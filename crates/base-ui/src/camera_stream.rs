//! Live picture of the machine from a URL: an MJPEG stream or a JPEG snapshot address (http / https).
//! The worker reads the byte stream, cuts out complete JPEG pictures and decodes at most ten per second.
use crate::camera::Frame;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use std::time::{Duration, Instant};

pub enum Msg {
    Frame(Frame),
    Error(String),
}

pub struct StreamWorker {
    pub rx: Receiver<Msg>,
    stop: Arc<AtomicBool>,
}

impl Drop for StreamWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }
}

/// Position of a complete JPEG (start of image .. end of image) in `buf`, if there is one.
pub fn find_jpeg(buf: &[u8]) -> Option<(usize, usize)> {
    let start = buf.windows(2).position(|w| w == [0xFF, 0xD8])?;
    let end = buf[start + 2..].windows(2).position(|w| w == [0xFF, 0xD9])? + start + 2 + 2;
    Some((start, end))
}

fn decode(jpeg: &[u8]) -> Option<Frame> {
    let img = image::load_from_memory_with_format(jpeg, image::ImageFormat::Jpeg).ok()?.to_rgb8();
    Some(Frame { w: img.width(), h: img.height(), rgb: img.into_raw() })
}

pub fn spawn(url: String, ctx: egui::Context) -> StreamWorker {
    let (tx, rx) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::spawn(move || {
        let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(5)).timeout_read(Duration::from_secs(3)).build();
        let mut last_error = String::new();
        let mut last_frame = Instant::now() - Duration::from_secs(1);
        while !flag.load(Ordering::Relaxed) {
            match agent.get(&url).call() {
                Ok(resp) => {
                    last_error.clear();
                    let mut reader = resp.into_reader();
                    let mut buf: Vec<u8> = Vec::new();
                    let mut chunk = vec![0u8; 32 * 1024];
                    while !flag.load(Ordering::Relaxed) {
                        match reader.read(&mut chunk) {
                            Ok(0) => break,
                            Ok(n) => buf.extend_from_slice(&chunk[..n]),
                            Err(e) => {
                                if !matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock) {
                                    let _ = tx.send(Msg::Error(e.to_string()));
                                }
                                break;
                            }
                        }
                        let mut newest: Option<(usize, usize)> = None;
                        let mut from = 0;
                        while let Some((s, e)) = find_jpeg(&buf[from..]) {
                            newest = Some((from + s, from + e));
                            from += e;
                        }
                        if let Some((s, e)) = newest {
                            if last_frame.elapsed() >= Duration::from_millis(100) {
                                if let Some(f) = decode(&buf[s..e]) {
                                    last_frame = Instant::now();
                                    if tx.send(Msg::Frame(f)).is_err() {
                                        return;
                                    }
                                    ctx.request_repaint();
                                }
                            }
                            buf.drain(..e);
                        }
                        if buf.len() > 8 * 1024 * 1024 {
                            buf.clear();
                        }
                    }
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg != last_error {
                        let _ = tx.send(Msg::Error(msg.clone()));
                        ctx.request_repaint();
                        last_error = msg;
                    }
                }
            }
            // Snapshot addresses end after one picture; streams that drop are reconnected.
            let t = Instant::now();
            while t.elapsed() < Duration::from_millis(400) && !flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(50));
            }
        }
    });
    StreamWorker { rx, stop }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::net::TcpListener;

    fn tiny_jpeg() -> Vec<u8> {
        let img = image::RgbImage::from_pixel(16, 8, image::Rgb([200, 30, 30]));
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(img).write_to(&mut out, image::ImageFormat::Jpeg).unwrap();
        out.into_inner()
    }

    #[test]
    fn finds_jpegs_in_a_byte_stream() {
        let j = tiny_jpeg();
        let mut buf = b"--frame\r\nContent-Type: image/jpeg\r\n\r\n".to_vec();
        let start = buf.len();
        buf.extend_from_slice(&j);
        buf.extend_from_slice(b"\r\n--frame\r\n");
        assert_eq!(find_jpeg(&buf), Some((start, start + j.len())));
        assert!(find_jpeg(&buf[..start + 10]).is_none());
        assert!(decode(&j).is_some());
    }

    #[test]
    fn receives_frames_from_an_mjpeg_server() {
        let jpeg = tiny_jpeg();
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let mut req = [0u8; 1024];
            let _ = s.read(&mut req);
            let _ = s.write_all(b"HTTP/1.0 200 OK\r\nContent-Type: multipart/x-mixed-replace; boundary=frame\r\n\r\n");
            for _ in 0..5 {
                let _ = s.write_all(b"--frame\r\nContent-Type: image/jpeg\r\n\r\n");
                let _ = s.write_all(&jpeg);
                let _ = s.write_all(b"\r\n");
                std::thread::sleep(Duration::from_millis(150));
            }
        });
        let w = spawn(format!("http://127.0.0.1:{port}/stream.mjpg"), egui::Context::default());
        let got = w.rx.recv_timeout(Duration::from_secs(5)).expect("a message");
        match got {
            Msg::Frame(f) => assert_eq!((f.w, f.h), (16, 8)),
            Msg::Error(e) => panic!("error: {e}"),
        }
    }
}
