//! The picture of the camera of a device, kept up to date: the worker that reads it (an address, an RTSP stream, a camera of the
//! computer), the picture as it is shown (lens correction, adjustments, the turn) as a texture, and the last picture for whoever needs
//! it (a background, a recording). One of these serves every view of the camera.
use crate::camera::{self, rotate_rgb};
use crate::camera_stream::{self, Msg};
use base_core::camimg::{CameraKind, CameraSource};

/// The picture of a camera as it is shown: lens correction, adjustments and the turn.
pub fn process_frame(cam: &CameraSource, rgb: Vec<u8>, w: u32, h: u32) -> (u32, u32, Vec<u8>) {
    let (wu, hu) = (w as usize, h as usize);
    let mut out = if cam.lens.is_none() { rgb } else { base_core::camimg::undistort(&rgb, wu, hu, &cam.lens) };
    base_core::camimg::adjust(&mut out, wu, hu, &cam.image);
    rotate_rgb(&out, w, h, cam.rotation % 4)
}

pub struct CamFeed {
    key: String,
    url: Option<camera_stream::StreamWorker>,
    local: Option<camera::Worker>,
    pub tex: Option<egui::TextureHandle>,
    /// The last picture as it is shown: size and RGB.
    pub last: Option<(usize, usize, Vec<u8>)>,
    pub error: String,
}

impl CamFeed {
    /// Start reading the camera `cam`.
    pub fn start(ctx: &egui::Context, cam: &CameraSource) -> CamFeed {
        let mut st = CamFeed { key: Self::key_of(cam), url: None, local: None, tex: None, last: None, error: String::new() };
        match &cam.kind {
            CameraKind::Url(u) => st.url = Some(camera_stream::spawn(u.clone(), ctx.clone())),
            CameraKind::Local(i) if camera::SUPPORTED => st.local = Some(camera::spawn(*i, cam.image.prefer_yuyv, ctx.clone())),
            CameraKind::Local(_) => st.error = crate::i18n::tr("This build cannot use the cameras of the computer.").to_string(),
        }
        st
    }

    pub fn key_of(cam: &CameraSource) -> String {
        format!("{:?}", cam.kind)
    }

    /// Is this the feed of the camera that `cam` is (the same source)?
    pub fn is_for(&self, cam: &CameraSource) -> bool {
        self.key == Self::key_of(cam)
    }

    /// Take what the worker has sent: the newest picture is processed, shown in the texture and kept. Returns true when there is a new one.
    pub fn pump(&mut self, ctx: &egui::Context, cam: &CameraSource) -> bool {
        let mut latest: Option<(u32, u32, Vec<u8>)> = None;
        if let Some(w) = &self.url {
            while let Ok(m) = w.rx.try_recv() {
                match m {
                    Msg::Frame(f) => {
                        self.error.clear();
                        latest = Some((f.w, f.h, f.rgb));
                    }
                    Msg::Error(e) => self.error = e,
                }
            }
        }
        if let Some(w) = &self.local {
            while let Ok(m) = w.rx.try_recv() {
                match m {
                    Ok(f) => {
                        self.error.clear();
                        latest = Some((f.w, f.h, f.rgb));
                    }
                    Err(e) => self.error = e,
                }
            }
        }
        let Some((w, h, rgb)) = latest else { return false };
        let (w, h, rgb) = process_frame(cam, rgb, w, h);
        let img = egui::ColorImage::from_rgb([w as usize, h as usize], &rgb);
        match &mut self.tex {
            Some(t) if t.size() == [w as usize, h as usize] => t.set(img, egui::TextureOptions::LINEAR),
            _ => self.tex = Some(ctx.load_texture("camera-view", img, egui::TextureOptions::LINEAR)),
        }
        self.last = Some((w as usize, h as usize, rgb));
        true
    }

    /// The last picture as an image for a texture.
    pub fn last_image(&self) -> Option<egui::ColorImage> {
        self.last.as_ref().map(|(w, h, rgb)| egui::ColorImage::from_rgb([*w, *h], rgb))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_picture_is_turned_by_the_camera_settings_and_the_feed_knows_its_source() {
        let mut cam = CameraSource::url("c", "http://127.0.0.1:1/x", 1);
        let (w, h, rgb) = process_frame(&cam, vec![10; 4 * 2 * 3], 4, 2);
        assert_eq!((w, h, rgb.len()), (2, 4, 24), "a quarter turn swaps the sides");
        cam.rotation = 0;
        assert_eq!(process_frame(&cam, vec![10; 4 * 2 * 3], 4, 2).0, 4);
        let ctx = egui::Context::default();
        let feed = CamFeed::start(&ctx, &cam);
        assert!(feed.is_for(&cam));
        assert!(!feed.is_for(&CameraSource::url("c", "http://other/x", 0)));
        assert!(feed.last_image().is_none());
    }
}
