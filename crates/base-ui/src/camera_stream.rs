//! Live picture of the machine from a URL: an MJPEG stream or a JPEG snapshot address (http / https).
//! The worker reads the byte stream, cuts out complete JPEG pictures and decodes at most ten per second.
use crate::camera::Frame;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub enum Msg {
    Frame(Frame),
    Error(String),
}

pub struct StreamWorker {
    pub rx: Receiver<Msg>,
    stop: Arc<AtomicBool>,
    /// The process of an RTSP stream (ffmpeg): ended with the worker.
    child: Arc<Mutex<Option<std::process::Child>>>,
}

impl Drop for StreamWorker {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Ok(mut c) = self.child.lock() {
            if let Some(child) = c.as_mut() {
                let _ = child.kill();
            }
        }
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

/// The picture pieces of a byte stream: the newest complete JPEG in `buf` is returned and everything up to its end is dropped.
fn take_newest(buf: &mut Vec<u8>) -> Option<Vec<u8>> {
    let mut newest: Option<(usize, usize)> = None;
    let mut from = 0;
    while let Some((s, e)) = find_jpeg(&buf[from..]) {
        newest = Some((from + s, from + e));
        from += e;
    }
    let (s, e) = newest?;
    let jpeg = buf[s..e].to_vec();
    buf.drain(..e);
    Some(jpeg)
}

/// The picture of an RTSP camera: a program (ffmpeg) that reads the stream and writes JPEG pictures to its standard output; they are cut
/// out and decoded like those of an MJPEG address. `command` says what to run (`None`: there is nothing to run); the stream is started
/// again when the program ends.
pub fn spawn_process(ctx: egui::Context, command: impl Fn() -> Option<(std::path::PathBuf, Vec<String>)> + Send + 'static) -> StreamWorker {
    spawn_process_with(ctx, command, SILENCE)
}

/// How long a program may give nothing before it is taken for dead: a stream that stopped (an unplugged camera, a UDP sender that went away)
/// does not always end the read, and ffmpeg then waits for ever.
const SILENCE: Duration = Duration::from_secs(20);

/// [`spawn_process`] with the silence after which the program is killed and started again.
fn spawn_process_with(ctx: egui::Context, command: impl Fn() -> Option<(std::path::PathBuf, Vec<String>)> + Send + 'static, silence: Duration) -> StreamWorker {
    let (tx, rx) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    let child: Arc<Mutex<Option<std::process::Child>>> = Arc::default();
    let shared = child.clone();
    std::thread::spawn(move || {
        let wait = |secs: u64| {
            let t = Instant::now();
            while t.elapsed() < Duration::from_secs(secs) && !flag.load(Ordering::Relaxed) {
                std::thread::sleep(Duration::from_millis(50));
            }
        };
        let mut last_error = String::new();
        let mut say = |msg: String, tx: &std::sync::mpsc::Sender<Msg>, ctx: &egui::Context| {
            if msg != last_error {
                let _ = tx.send(Msg::Error(msg.clone()));
                ctx.request_repaint();
                last_error = msg;
            }
        };
        while !flag.load(Ordering::Relaxed) {
            let Some((path, args)) = command() else {
                say(crate::i18n::tr("The stream of this camera needs ffmpeg, and it was not found next to the program or in the PATH of the system.").to_string(), &tx, &ctx);
                wait(5);
                continue;
            };
            let spawned = std::process::Command::new(&path).args(&args).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::null()).spawn();
            let mut proc = match spawned {
                Ok(c) => c,
                Err(e) => {
                    say(e.to_string(), &tx, &ctx);
                    wait(5);
                    continue;
                }
            };
            let Some(mut out) = proc.stdout.take() else { continue };
            if let Ok(mut c) = shared.lock() {
                *c = Some(proc);
            }
            let mut buf: Vec<u8> = Vec::new();
            let mut chunk = vec![0u8; 64 * 1024];
            let mut got_any = false;
            // The watchdog: when the program gives nothing for `silence` it is killed, which ends the read below.
            let started = Instant::now();
            let last = Arc::new(std::sync::atomic::AtomicU64::new(0));
            let done = Arc::new(AtomicBool::new(false));
            {
                let (last, done, flag, shared) = (last.clone(), done.clone(), flag.clone(), shared.clone());
                std::thread::spawn(move || {
                    while !done.load(Ordering::Relaxed) && !flag.load(Ordering::Relaxed) {
                        std::thread::sleep(Duration::from_millis(100));
                        if started.elapsed().saturating_sub(Duration::from_millis(last.load(Ordering::Relaxed))) >= silence {
                            if let Ok(mut c) = shared.lock() {
                                if let Some(p) = c.as_mut() {
                                    let _ = p.kill();
                                }
                            }
                            break;
                        }
                    }
                });
            }
            while !flag.load(Ordering::Relaxed) {
                match out.read(&mut chunk) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        buf.extend_from_slice(&chunk[..n]);
                        last.store(started.elapsed().as_millis() as u64, Ordering::Relaxed);
                    }
                }
                if let Some(jpeg) = take_newest(&mut buf) {
                    if let Some(f) = decode(&jpeg) {
                        got_any = true;
                        if tx.send(Msg::Frame(f)).is_err() {
                            break;
                        }
                        ctx.request_repaint();
                    }
                }
                if buf.len() > 8 * 1024 * 1024 {
                    buf.clear();
                }
            }
            done.store(true, Ordering::Relaxed);
            if let Ok(mut c) = shared.lock() {
                if let Some(mut proc) = c.take() {
                    let _ = proc.kill();
                    let _ = proc.wait();
                }
            }
            if !flag.load(Ordering::Relaxed) {
                say(crate::i18n::tr(if got_any { "The stream ended: it is opened again." } else { "The stream could not be opened (address, user and password?): it is tried again." }).to_string(), &tx, &ctx);
                wait(3);
            }
        }
    });
    StreamWorker { rx, stop, child }
}

pub fn spawn(url: String, ctx: egui::Context) -> StreamWorker {
    if base_core::camimg::is_ffmpeg_stream(&url) {
        return spawn_process(ctx, move || crate::ffmpeg::path().map(|p| (p, crate::ffmpeg::stream_args(&url, 10))));
    }
    let (tx, rx) = channel();
    let stop = Arc::new(AtomicBool::new(false));
    let flag = stop.clone();
    std::thread::spawn(move || {
        let agent = ureq::Agent::from(ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(5))).timeout_recv_response(Some(Duration::from_secs(5))).timeout_recv_body(None).build());
        let mut last_error = String::new();
        let mut last_frame = Instant::now() - Duration::from_secs(1);
        while !flag.load(Ordering::Relaxed) {
            match agent.get(&url).call() {
                Ok(resp) => {
                    last_error.clear();
                    let mut reader = resp.into_body().into_reader();
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
    StreamWorker { rx, stop, child: Arc::default() }
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

    /// A stand-in for ffmpeg: a script that writes the pictures of `jpeg` (a file) to its standard output.
    #[cfg(unix)]
    fn fake_ffmpeg(dir: &std::path::Path, jpeg: &std::path::Path, times: usize) -> std::path::PathBuf {
        use std::os::unix::fs::PermissionsExt;
        let script = dir.join("fake-ffmpeg.sh");
        std::fs::write(&script, format!("#!/bin/sh\ni=0\nwhile [ $i -lt {times} ]; do cat '{}'; sleep 0.1; i=$((i+1)); done\nsleep 5\n", jpeg.display())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        script
    }

    #[cfg(unix)]
    #[test]
    fn an_rtsp_stream_comes_as_pictures_from_the_program_that_reads_it_and_ends_with_the_worker() {
        let dir = std::env::temp_dir().join(format!("rtsp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let jpeg = dir.join("one.jpg");
        std::fs::write(&jpeg, tiny_jpeg()).unwrap();
        let script = fake_ffmpeg(&dir, &jpeg, 4);
        let w = spawn_process(egui::Context::default(), move || Some((script.clone(), vec![])));
        let mut frames = 0;
        let t = Instant::now();
        while frames < 3 && t.elapsed() < Duration::from_secs(8) {
            if let Ok(Msg::Frame(f)) = w.rx.recv_timeout(Duration::from_millis(500)) {
                assert_eq!((f.w, f.h), (16, 8));
                frames += 1;
            }
        }
        assert!(frames >= 3, "{frames} pictures");
        // Dropping the worker ends the process: the script sleeps 5 s after the pictures, and the child is killed at once.
        let child = w.child.clone();
        drop(w);
        std::thread::sleep(Duration::from_millis(300));
        let gone = child.lock().unwrap().as_mut().map_or(true, |c| c.try_wait().map(|s| s.is_some()).unwrap_or(true));
        assert!(gone, "the process is killed with the worker");
        std::fs::remove_dir_all(dir).ok();
    }

    /// A program that starts and then gives nothing (a stream that stopped) is killed after the silence and started again.
    #[cfg(unix)]
    #[test]
    fn a_program_that_gives_nothing_is_killed_and_started_again() {
        use std::os::unix::fs::PermissionsExt;
        let dir = std::env::temp_dir().join(format!("silent-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let log = dir.join("starts.log");
        let script = dir.join("silent.sh");
        std::fs::write(&script, format!("#!/bin/sh\necho x >> '{}'\nexec sleep 60\n", log.display())).unwrap();
        std::fs::set_permissions(&script, std::fs::Permissions::from_mode(0o755)).unwrap();
        let s2 = script.clone();
        let w = spawn_process_with(egui::Context::default(), move || Some((s2.clone(), vec![])), Duration::from_millis(600));
        match w.rx.recv_timeout(Duration::from_secs(10)).expect("a message after the silence") {
            Msg::Error(e) => assert!(e.contains("could not be opened"), "{e}"),
            Msg::Frame(_) => panic!("no picture"),
        }
        // After the pause the program is started again.
        let t = Instant::now();
        while std::fs::read_to_string(&log).map(|s| s.lines().count()).unwrap_or(0) < 2 && t.elapsed() < Duration::from_secs(10) {
            std::thread::sleep(Duration::from_millis(100));
        }
        assert!(std::fs::read_to_string(&log).unwrap().lines().count() >= 2, "started again");
        drop(w);
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn without_ffmpeg_the_worker_says_so_and_a_program_that_cannot_start_is_an_error() {
        let w = spawn_process(egui::Context::default(), || None);
        match w.rx.recv_timeout(Duration::from_secs(3)).expect("a message") {
            Msg::Error(e) => assert!(e.contains("ffmpeg"), "{e}"),
            Msg::Frame(_) => panic!("no picture without a program"),
        }
        let w = spawn_process(egui::Context::default(), || Some((std::path::PathBuf::from("/nonexistent/ffmpeg-nowhere"), vec![])));
        assert!(matches!(w.rx.recv_timeout(Duration::from_secs(3)), Ok(Msg::Error(_))));
        // The addresses: rtsp is a camera address now, spaces and options are not.
        assert!(base_core::camimg::valid_url("rtsp://user:pw@192.168.1.5:554/stream1") && base_core::camimg::valid_url("rtsps://cam.local/x"));
        assert!(!base_core::camimg::valid_url("rtsp://") && !base_core::camimg::valid_url("rtsp://a b/c") && !base_core::camimg::valid_url("ftp://x/y"));
        assert!(base_core::camimg::is_rtsp("RTSP://cam/x") && !base_core::camimg::is_rtsp("http://cam/rtsp://"));
        // RTMP, UDP and RTP streams are read by ffmpeg too; a UDP address needs a port, not a host.
        for ok in ["rtmp://server/live/key", "udp://@:5000", "udp://239.0.0.1:1234?pkt_size=1316", "rtp://0.0.0.0:5004", "UDP://127.0.0.1:9"] {
            assert!(base_core::camimg::valid_url(ok) && base_core::camimg::is_ffmpeg_stream(ok), "{ok}");
        }
        for bad in ["udp://", "udp://host", "udp://host:port", "rtp://:", "rtmp://", "udp://a b:5", "srt://x:1"] {
            assert!(!base_core::camimg::valid_url(bad), "{bad}");
        }
        assert!(!base_core::camimg::is_ffmpeg_stream("http://cam/udp://x:1"));
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
