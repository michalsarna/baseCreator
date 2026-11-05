//! Live camera capture. The programs build it in by default (`camera` feature).
use std::sync::mpsc::Receiver;

/// One captured picture: RGB bytes, row-major.
pub struct Frame {
    pub w: u32,
    pub h: u32,
    pub rgb: Vec<u8>,
}

/// Rotate an RGB picture by `quarter` quarter turns clockwise. Returns the new size and bytes.
pub fn rotate_rgb(rgb: &[u8], w: u32, h: u32, quarter: u8) -> (u32, u32, Vec<u8>) {
    let (w, h) = (w as usize, h as usize);
    let px = |x: usize, y: usize| -> &[u8] { &rgb[(y * w + x) * 3..(y * w + x) * 3 + 3] };
    match quarter % 4 {
        0 => (w as u32, h as u32, rgb.to_vec()),
        2 => {
            let mut out = Vec::with_capacity(rgb.len());
            for y in (0..h).rev() {
                for x in (0..w).rev() {
                    out.extend_from_slice(px(x, y));
                }
            }
            (w as u32, h as u32, out)
        }
        q => {
            // New picture is h wide and w tall.
            let mut out = Vec::with_capacity(rgb.len());
            for ny in 0..w {
                for nx in 0..h {
                    let (x, y) = if q == 1 { (ny, h - 1 - nx) } else { (w - 1 - ny, nx) };
                    out.extend_from_slice(px(x, y));
                }
            }
            (h as u32, w as u32, out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::rotate_rgb;

    #[test]
    fn rotations_move_pixels_clockwise() {
        // 3 wide, 2 tall: pixel values encode their position (R = x, G = y).
        let img: Vec<u8> = (0..2u8).flat_map(|y| (0..3u8).flat_map(move |x| [x, y, 0])).collect();
        let (w, h, r) = rotate_rgb(&img, 3, 2, 1);
        assert_eq!((w, h), (2, 3));
        // Clockwise: the old top-left pixel (0,0) ends up top-right of the new picture.
        assert_eq!(&r[3..6], &[0, 0, 0]);
        // The old bottom-left (0,1) becomes top-left.
        assert_eq!(&r[0..3], &[0, 1, 0]);
        let (w, h, r2) = rotate_rgb(&img, 3, 2, 2);
        assert_eq!((w, h), (3, 2));
        assert_eq!(&r2[0..3], &[2, 1, 0]);
        let (w, h, r3) = rotate_rgb(&img, 3, 2, 3);
        assert_eq!((w, h), (2, 3));
        assert_eq!(&r3[0..3], &[2, 0, 0]);
        // Four turns are the identity.
        let (_, _, a) = rotate_rgb(&img, 3, 2, 1);
        let (_, _, b) = rotate_rgb(&a, 2, 3, 3);
        assert_eq!(b, img);
    }
}

#[cfg(feature = "camera")]
mod imp {
    use super::*;
    use nokhwa::pixel_format::RgbFormat;
    use std::sync::mpsc::{channel, Sender};
    use nokhwa::utils::{ApiBackend, CameraIndex, FrameFormat, RequestedFormat, RequestedFormatType, Resolution};
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::time::Duration;

    pub const SUPPORTED: bool = true;

    /// Names of the cameras the system reports. Asks for permission first on macOS.
    pub fn list() -> Vec<String> {
        nokhwa::nokhwa_initialize(|_| {});
        // Give the permission prompt a moment on first use.
        for _ in 0..20 {
            if nokhwa::nokhwa_check() {
                break;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        nokhwa::query(ApiBackend::Auto).map(|v| v.into_iter().map(|c| c.human_name()).collect()).unwrap_or_default()
    }

    /// Open a camera. On macOS the library throws an exception out of some formats of some cameras, and not always the same ones (a
    /// webcam with several formats of one size): each try is guarded, and smaller sizes follow. A build for debugging cannot catch it,
    /// so there the camera only gets the format the system gives it first.
    fn open_camera(index: u32, prefer_yuyv: bool) -> Result<nokhwa::Camera, String> {
        use std::panic::{catch_unwind, AssertUnwindSafe};
        let macos = cfg!(target_os = "macos");
        // YUYV is asked for first where the backend can give it; otherwise whatever the camera offers.
        let preferred = [FrameFormat::YUYV, FrameFormat::MJPEG];
        let wanted = if prefer_yuyv { RequestedFormat::with_formats(RequestedFormatType::AbsoluteHighestResolution, &preferred) } else { RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestResolution) };
        let size = |w, h| RequestedFormat::new::<RgbFormat>(RequestedFormatType::HighestResolution(Resolution::new(w, h)));
        let plain = RequestedFormat::new::<RgbFormat>(RequestedFormatType::None);
        let tries: Vec<RequestedFormat> = if macos && cfg!(debug_assertions) {
            vec![plain]
        } else if macos {
            vec![wanted, size(1920, 1080), size(1280, 720), size(640, 480), wanted, size(1920, 1080), size(1280, 720), plain]
        } else {
            vec![wanted]
        };
        let mut last = String::new();
        for f in tries {
            match catch_unwind(AssertUnwindSafe(|| nokhwa::Camera::new(CameraIndex::Index(index), f))) {
                Ok(Ok(c)) => return Ok(c),
                Ok(Err(e)) => last = e.to_string(),
                Err(_) => last = "the system refused this format of the camera".to_string(),
            }
        }
        Err(last)
    }

    pub struct Worker {
        pub rx: Receiver<Result<Frame, String>>,
        stop: Arc<AtomicBool>,
    }

    impl Drop for Worker {
        fn drop(&mut self) {
            self.stop.store(true, Ordering::Relaxed);
        }
    }

    /// Stream frames (about 8 per second) from camera `index` until the worker is dropped.
    pub fn spawn(index: u32, prefer_yuyv: bool, ctx: egui::Context) -> Worker {
        let (tx, rx): (Sender<Result<Frame, String>>, _) = channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = stop.clone();
        std::thread::spawn(move || {
            let mut cam = match open_camera(index, prefer_yuyv) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Err(e));
                    return;
                }
            };
            if let Err(e) = cam.open_stream() {
                let _ = tx.send(Err(e.to_string()));
                return;
            }
            while !flag.load(Ordering::Relaxed) {
                match cam.frame().and_then(|f| f.decode_image::<RgbFormat>()) {
                    Ok(img) => {
                        let (w, h) = (img.width(), img.height());
                        if tx.send(Ok(Frame { w, h, rgb: img.into_raw() })).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                    Err(e) => {
                        let _ = tx.send(Err(e.to_string()));
                        break;
                    }
                }
                std::thread::sleep(Duration::from_millis(120));
            }
            let _ = cam.stop_stream();
        });
        Worker { rx, stop }
    }
}

#[cfg(not(feature = "camera"))]
mod imp {
    use super::*;
    use std::sync::mpsc::channel;

    pub const SUPPORTED: bool = false;

    pub fn list() -> Vec<String> {
        vec![]
    }

    pub struct Worker {
        pub rx: Receiver<Result<Frame, String>>,
    }

    pub fn spawn(_index: u32, _prefer_yuyv: bool, _ctx: egui::Context) -> Worker {
        let (_tx, rx) = channel();
        Worker { rx }
    }
}

pub use imp::{list, spawn, Worker, SUPPORTED};
