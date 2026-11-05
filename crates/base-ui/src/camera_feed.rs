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

/// The options of a stream as the program that runs ffmpeg takes them.
pub fn options_of(cam: &CameraSource) -> crate::ffmpeg::StreamOptions {
    crate::ffmpeg::StreamOptions { udp: cam.rtsp_udp, low_latency: cam.low_latency }
}

/// The choices under the address of a stream: RTSP over UDP and low latency. Only an address that ffmpeg reads has them. Returns whether
/// anything was changed.
pub fn stream_options_ui(ui: &mut egui::Ui, cam: &mut CameraSource) -> bool {
    let CameraKind::Url(u) = &cam.kind else { return false };
    if !base_core::camimg::is_ffmpeg_stream(u) {
        return false;
    }
    let tcp_udp = base_core::camimg::is_rtsp(u);
    let mut changed = false;
    ui.horizontal_wrapped(|ui| {
        if tcp_udp {
            changed |= ui.checkbox(&mut cam.rtsp_udp, crate::i18n::tr("Use UDP for RTSP")).on_hover_text(crate::i18n::tr("Less delay than TCP, but the picture can break on a bad network. Off: TCP.")).changed();
        }
        changed |= ui.checkbox(&mut cam.low_latency, crate::i18n::tr("Low latency")).on_hover_text(crate::i18n::tr("Shows the newest picture sooner, without buffering; the picture can stutter.")).changed();
    });
    changed
}

pub struct CamFeed {
    key: String,
    url: Option<camera_stream::StreamWorker>,
    local: Option<camera::Worker>,
    pub tex: Option<egui::TextureHandle>,
    /// The last picture as it is shown: size and RGB.
    pub last: Option<(usize, usize, Vec<u8>)>,
    pub error: String,
    /// How many pictures have come; another number means a new picture.
    pub frames: u64,
}

impl CamFeed {
    /// Start reading the camera `cam`.
    pub fn start(ctx: &egui::Context, cam: &CameraSource) -> CamFeed {
        let mut st = CamFeed { key: Self::key_of(cam), url: None, local: None, tex: None, last: None, error: String::new(), frames: 0 };
        match &cam.kind {
            CameraKind::Url(u) => st.url = Some(camera_stream::spawn_with(u.clone(), options_of(cam), ctx.clone())),
            CameraKind::Local(i) if camera::SUPPORTED => st.local = Some(camera::spawn(*i, cam.image.prefer_yuyv, ctx.clone())),
            CameraKind::Local(_) => st.error = crate::i18n::tr("This build cannot use the cameras of the computer.").to_string(),
        }
        st
    }

    pub fn key_of(cam: &CameraSource) -> String {
        // The options are in the key: another choice opens the stream again.
        format!("{:?}{}{}", cam.kind, cam.rtsp_udp, cam.low_latency)
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
        self.frames += 1;
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

/// The cameras of a device in its settings, the same in every program: which one is on, an address or a camera of this computer, the
/// test, the turn, the picture and the lens. The camera to test goes to `test`.
pub fn camera_page(ui: &mut egui::Ui, cameras: &mut Vec<CameraSource>, active: &mut usize, test: &mut Option<CameraSource>) {
    use crate::i18n::{tr, trf};
    use egui::RichText;
    ui.label(RichText::new(tr("Cameras that watch the machine. The one that is ticked gives the live picture; ticking another switches the first off.")).color(crate::theme::text_dim()));
    let mut remove = None;
    let mut toggle: Option<(usize, bool)> = None;
    for i in 0..cameras.len() {
        let on_now = *active == i;
        let c = &mut cameras[i];
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.horizontal_wrapped(|ui| {
                // The tick before the name switches this camera on (and the others off): it is the one in use.
                let mut on = on_now && c.enabled;
                if ui.checkbox(&mut on, "").on_hover_text(tr("Switch this camera on: it gives the live picture")).changed() {
                    toggle = Some((i, on));
                }
                ui.add(egui::TextEdit::singleline(&mut c.name).hint_text(tr("Camera name")).desired_width(150.0));
                if crate::window::delete_icon(ui, &tr("Delete this camera")).clicked() {
                    remove = Some(i);
                }
            });
            ui.horizontal_wrapped(|ui| {
                let local = matches!(c.kind, CameraKind::Local(_));
                let mut want_local = local;
                ui.radio_value(&mut want_local, false, tr("Address"));
                ui.radio_value(&mut want_local, true, tr("Camera of this computer"));
                if want_local != local {
                    c.kind = if want_local { CameraKind::Local(0) } else { CameraKind::Url(String::new()) };
                    if !want_local {
                        c.enabled = false;
                    }
                }
            });
            match &mut c.kind {
                CameraKind::Url(u) => {
                    address_field(ui, u);
                }
                CameraKind::Local(n) => {
                    if camera::SUPPORTED {
                        ui.horizontal(|ui| {
                            ui.label(tr("Camera number"));
                            ui.add(egui::DragValue::new(n).range(0..=15));
                        });
                    } else {
                        ui.label(RichText::new(tr("This build cannot use the cameras of the computer.")).color(crate::theme::text_dim()));
                    }
                }
            }
            stream_options_ui(ui, c);
            let testable = match &c.kind {
                CameraKind::Url(u) => base_core::camimg::valid_url(u),
                CameraKind::Local(_) => camera::SUPPORTED,
            };
            if ui.add_enabled(testable, egui::Button::new(tr("Test"))).on_hover_text(tr("Opens the picture of this camera as set here, without saving")).clicked() {
                *test = Some(c.clone());
            }
            ui.horizontal(|ui| {
                ui.label(tr("Camera rotation"));
                for q in 0..4u8 {
                    ui.selectable_value(&mut c.rotation, q, format!("{}°", q as u32 * 90));
                }
            });
            egui::CollapsingHeader::new(tr("Picture")).id_salt(("cam_img", i)).show(ui, |ui| {
                egui::Grid::new(("cam_img_grid", i)).num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                    ui.label(tr("Brightness"));
                    ui.add(egui::Slider::new(&mut c.image.brightness, -100..=100));
                    ui.end_row();
                    ui.label(tr("Contrast"));
                    ui.add(egui::Slider::new(&mut c.image.contrast, 0..=100));
                    ui.end_row();
                    ui.label(tr("White balance"));
                    ui.horizontal(|ui| {
                        let mut auto = c.image.white_balance.is_none();
                        if ui.checkbox(&mut auto, tr("Auto")).changed() {
                            c.image.white_balance = if auto { None } else { Some(6500) };
                        }
                        if let Some(k) = &mut c.image.white_balance {
                            ui.add(egui::DragValue::new(k).range(2500..=10_000).speed(50.0).suffix(" K"));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Noise reduction"));
                    ui.add(egui::Slider::new(&mut c.image.denoise, 0.0..=0.95));
                    ui.end_row();
                    ui.label(tr("Format"));
                    ui.checkbox(&mut c.image.prefer_yuyv, tr("Prefer YUYV")).on_hover_text(tr("For a camera of this computer: ask for the uncompressed YUYV format where the system allows it."));
                    ui.end_row();
                });
            });
            egui::CollapsingHeader::new(tr("Lens correction")).id_salt(("cam_lens", i)).show(ui, |ui| {
                ui.label(RichText::new(tr("Coefficients of the lens distortion (radial k1–k3, tangential p1, p2) for coordinates measured from the centre in units of the longer side of the picture. All 0 = no correction. Entered by hand: there is no automatic calibration.")).color(crate::theme::text_dim()));
                ui.horizontal_wrapped(|ui| {
                    for (name, v) in [("k1", &mut c.lens.k1), ("k2", &mut c.lens.k2), ("k3", &mut c.lens.k3), ("p1", &mut c.lens.p1), ("p2", &mut c.lens.p2)] {
                        ui.label(name);
                        ui.add(egui::DragValue::new(v).speed(0.001).range(-5.0..=5.0).max_decimals(4));
                    }
                    if ui.small_button(tr("Reset")).clicked() {
                        c.lens = base_core::camimg::Lens::default();
                    }
                });
            });
        });
    }
    if let Some((i, on)) = toggle {
        if on {
            activate(cameras, active, i);
        } else if let Some(c) = cameras.get_mut(i) {
            c.enabled = false;
        }
    }
    if let Some(i) = remove {
        cameras.remove(i);
        *active = (*active).min(cameras.len().saturating_sub(1));
    }
    if cameras.len() < 8 && ui.button(tr("Add a camera")).clicked() {
        let name = trf("Camera {}", &[&(cameras.len() + 1)]);
        cameras.push(CameraSource { enabled: true, ..CameraSource::url(&name, "", 0) });
        let last = cameras.len() - 1;
        activate(cameras, active, last);
    }
}

/// The address of a camera. What is pasted often brings a line break or a space along: they are cut off both ends.
pub fn address_field(ui: &mut egui::Ui, u: &mut String) {
    use crate::i18n::tr;
    use egui::{Color32, RichText};
    let r = ui.add(egui::TextEdit::singleline(u).hint_text("http://raspberrypi.local:8080/stream.mjpg").desired_width(340.0)).on_hover_text(tr("Live picture of the machine: an MJPEG stream, a JPEG snapshot address (http or https), or a stream that ffmpeg reads (rtsp, udp, rtp, rtmp)."));
    if r.changed() {
        let clean = clean_address(u);
        if clean != *u {
            *u = clean;
        }
    }
    let red = Color32::from_rgb(0xc0, 0x30, 0x30);
    if u.is_empty() {
        ui.label(RichText::new(tr("Enter the address of the camera, or delete the camera.")).color(red));
    } else if !base_core::camimg::valid_url(u) {
        ui.label(RichText::new(tr("Not a camera address (http, https, rtsp, udp, rtp or rtmp).")).color(red));
    }
}

/// The address without what stands around it.
pub fn clean_address(u: &str) -> String {
    u.trim().to_string()
}

/// A camera by address without one: the device cannot be saved like that.
pub fn lacks_address(cameras: &[CameraSource]) -> bool {
    cameras.iter().any(|c| matches!(&c.kind, CameraKind::Url(u) if u.trim().is_empty()))
}

/// This camera on, the others off.
pub fn activate(cameras: &mut [CameraSource], active: &mut usize, i: usize) {
    if i < cameras.len() {
        *active = i;
        for (j, c) in cameras.iter_mut().enumerate() {
            c.enabled = j == i;
        }
    }
}

/// The *Test* button of a camera: its picture in a small window, as it is being set, without changing the saved device.
pub struct CamTest {
    cam: CameraSource,
    feed: CamFeed,
}

impl CamTest {
    /// `None` for a camera that cannot be opened.
    pub fn start(cam: CameraSource, ctx: &egui::Context) -> Option<CamTest> {
        let ok = match &cam.kind {
            CameraKind::Url(u) => base_core::camimg::valid_url(u),
            CameraKind::Local(_) => camera::SUPPORTED,
        };
        ok.then(|| CamTest { feed: CamFeed::start(ctx, &cam), cam })
    }

    /// The window. It closes itself by taking the test away.
    pub fn window(ctx: &egui::Context, slot: &mut Option<CamTest>) {
        use crate::i18n::{tr, trf};
        let Some(t) = slot.as_mut() else { return };
        t.feed.pump(ctx, &t.cam);
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        let mut open = true;
        let title = trf("Camera test: {}", &[&t.cam.name]);
        crate::window::popup(ctx, "cam_test_window", &title, [480.0, 380.0], &mut open, |ui| {
            if !t.feed.error.is_empty() {
                let text = match &t.cam.kind {
                    CameraKind::Url(u) => t.feed.error.replace(u.as_str(), "…"),
                    _ => t.feed.error.clone(),
                };
                crate::widgets::message(ui, egui::RichText::new(trf("Stream error: {}", &[&text])).color(egui::Color32::from_rgb(0xc0, 0x30, 0x30)));
            }
            match &t.feed.tex {
                Some(tex) => {
                    let sz = tex.size_vec2();
                    let k = (ui.available_width() / sz.x).min((ui.available_height() - 30.0).max(100.0) / sz.y).min(2.0);
                    ui.image((tex.id(), sz * k));
                }
                None => {
                    ui.add_space(30.0);
                    ui.label(tr("Waiting for the picture…"));
                }
            }
        });
        if !open {
            *slot = None;
        }
    }
}

#[cfg(test)]
mod address_tests {
    use super::*;

    #[test]
    fn a_pasted_address_loses_what_stands_around_it() {
        assert_eq!(clean_address("\r\n http://h/s \n"), "http://h/s");
        assert_eq!(clean_address("\n\n"), "");
    }

    #[test]
    fn only_a_camera_by_address_needs_an_address() {
        let mut c = CameraSource::url("x", "", 0);
        assert!(lacks_address(std::slice::from_ref(&c)));
        c.kind = CameraKind::Url(" \n".into());
        assert!(lacks_address(std::slice::from_ref(&c)));
        c.kind = CameraKind::Url("http://h/s".into());
        assert!(!lacks_address(std::slice::from_ref(&c)));
        c.kind = CameraKind::Local(0);
        assert!(!lacks_address(&[c]));
        assert!(!lacks_address(&[]));
    }
}
