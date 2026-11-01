//! The ffmpeg that comes with the program: it turns the stream of an IP camera (RTSP, H.264 / H.265) into pictures the program can read.
//! It is looked for next to the program, in the resources of a macOS app and in a folder `ffmpeg` beside it, and only then in the PATH of
//! the system. The program starts it as a separate process and talks to it through pipes; nothing is linked.
use std::path::{Path, PathBuf};

fn exe_name() -> &'static str {
    if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }
}

/// The places to look in, in order, for the program `app` (the name of its executable) whose executable is in `exe_dir`. A Linux package
/// puts it in `/usr/lib/<app>/`, so that programs installed together do not own the same file.
pub fn candidates(exe_dir: &Path, app: &str) -> Vec<PathBuf> {
    let n = exe_name();
    vec![exe_dir.join(n), exe_dir.join("ffmpeg").join(n), exe_dir.join("../Resources").join(n), exe_dir.join("../Resources/ffmpeg").join(n), exe_dir.join("../lib").join(app).join(n)]
}

fn is_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

/// The ffmpeg to run: the one that came with the program, else the first in the PATH; `None` when there is none.
pub fn path() -> Option<PathBuf> {
    // `CREATOR_FFMPEG` names the program to run (to try another build, and for the tests).
    if let Some(p) = std::env::var_os("CREATOR_FFMPEG").map(PathBuf::from).filter(|p| is_file(p)) {
        return Some(p);
    }
    if let Some(exe) = std::env::current_exe().ok() {
        let app = exe.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        let dir = exe.parent().map(Path::to_path_buf).unwrap_or_default();
        if let Some(p) = candidates(&dir, &app).into_iter().find(|p| is_file(p)) {
            return Some(p);
        }
    }
    let n = exe_name();
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(n)).find(|p| is_file(p)))
}

/// What the person chose for a stream (the options of the camera).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct StreamOptions {
    /// RTSP over UDP instead of TCP.
    pub udp: bool,
    /// No buffering: the newest picture sooner.
    pub low_latency: bool,
}

/// The arguments that make ffmpeg read a stream (RTSP, RTMP, UDP, RTP) and write JPEG pictures (about `fps` a second) to its standard
/// output. The address goes in as one argument, never through a shell. A UDP, RTP or RTMP read ends after ten seconds without data; a UDP
/// or RTP stream is read without the buffering that adds a delay. RTSP gets no `-rw_timeout`: its demuxer opens its own sockets and
/// ffmpeg refuses the option ("Option rw_timeout not found") before the stream is opened. A stream that stopped is also caught by the
/// watchdog of `camera_stream`, whatever the kind.
pub fn stream_args(url: &str, fps: u32) -> Vec<String> {
    stream_args_with(url, fps, StreamOptions::default())
}

/// [`stream_args`] with the options of the camera.
pub fn stream_args_with(url: &str, fps: u32, opts: StreamOptions) -> Vec<String> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    let live = lower.starts_with("udp://") || lower.starts_with("rtp://");
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin"].iter().map(|s| s.to_string()).collect();
    if lower.starts_with("rtsp") {
        a.extend(["-rtsp_transport", if opts.udp { "udp" } else { "tcp" }].iter().map(|s| s.to_string()));
    } else {
        // Microseconds, an option of the protocol layer: the sockets of udp://, rtp:// and rtmp:// take it.
        a.extend(["-rw_timeout", "10000000"].iter().map(|s| s.to_string()));
    }
    if live || opts.low_latency {
        a.extend(["-fflags", "nobuffer", "-flags", "low_delay"].iter().map(|s| s.to_string()));
    }
    if live {
        // A live stream is probed for a moment, not for the 5 MB or 5 seconds ffmpeg waits for by default.
        a.extend(["-probesize", "200000", "-analyzeduration", "1000000"].iter().map(|s| s.to_string()));
    }
    a.extend(["-i", url, "-an", "-vf", &format!("fps={}", fps.clamp(1, 30)), "-f", "mjpeg", "-q:v", "5", "pipe:1"].iter().map(|s| s.to_string()));
    a
}

/// The arguments that copy a stream into an MP4 file as it comes: no decoding and no encoding, so no load on the processor and no loss
/// of quality. The file is fragmented, so it plays even when the program is stopped before the recording is closed properly.
pub fn record_args(url: &str, opts: StreamOptions, path: &str) -> Vec<String> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-y"].iter().map(|s| s.to_string()).collect();
    if lower.starts_with("rtsp") {
        a.extend(["-rtsp_transport", if opts.udp { "udp" } else { "tcp" }].iter().map(|s| s.to_string()));
    } else {
        a.extend(["-rw_timeout", "10000000"].iter().map(|s| s.to_string()));
    }
    if lower.starts_with("udp://") || lower.starts_with("rtp://") {
        a.extend(["-probesize", "200000", "-analyzeduration", "1000000"].iter().map(|s| s.to_string()));
    }
    a.extend(["-i", url, "-an", "-c:v", "copy", "-movflags", "+frag_keyframe+empty_moov+default_base_moof", "-f", "mp4", path].iter().map(|s| s.to_string()));
    a
}

/// The arguments for an RTSP stream (see [`stream_args`]).
pub fn rtsp_args(url: &str, fps: u32) -> Vec<String> {
    stream_args(url, fps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_places_start_next_to_the_program_and_the_address_is_one_argument() {
        let c = candidates(Path::new("/opt/creator"), "lightcreator");
        assert_eq!(c[0], Path::new("/opt/creator").join(exe_name()));
        assert!(c.iter().any(|p| p.to_string_lossy().contains("Resources")), "the resources of a macOS app");
        let a = rtsp_args("rtsp://user:pass@192.168.1.5:554/stream1", 10);
        assert_eq!(a.iter().position(|x| x == "-i").map(|i| a[i + 1].as_str()), Some("rtsp://user:pass@192.168.1.5:554/stream1"));
        assert!(a.contains(&"fps=10".to_string()) && a.last().map(String::as_str) == Some("pipe:1"));
        assert!(rtsp_args("rtsp://x", 500).contains(&"fps=30".to_string()), "the rate is limited");
        // The RTSP demuxer refuses -rw_timeout, which would keep every RTSP camera from opening.
        assert!(a.windows(2).any(|w| w == ["-rtsp_transport", "tcp"]) && !a.contains(&"-rw_timeout".to_string()));
        assert!(!stream_args("rtsps://x/y", 5).contains(&"-rw_timeout".to_string()));
        // A UDP stream is read without buffering; RTMP only has the timeout.
        let u = stream_args("udp://@:5000", 10);
        assert!(u.windows(2).any(|w| w == ["-fflags", "nobuffer"]) && u.windows(2).any(|w| w == ["-probesize", "200000"]) && !u.contains(&"-rtsp_transport".to_string()));
        let m = stream_args("rtmp://server/live/key", 10);
        assert!(!m.contains(&"-fflags".to_string()) && m.contains(&"-rw_timeout".to_string()));
        // Nothing here can be taken for an option: the address is after -i, whatever it starts with.
        assert_eq!(rtsp_args("-version", 5)[rtsp_args("-version", 5).iter().position(|x| x == "-i").unwrap() + 1], "-version");
    }

    #[test]
    fn the_options_of_a_camera_change_the_arguments_and_a_copy_has_no_encoding() {
        let opt = |udp, low_latency| StreamOptions { udp, low_latency };
        let t = stream_args_with("rtsp://c/s", 10, opt(false, false));
        assert!(t.windows(2).any(|w| w == ["-rtsp_transport", "tcp"]) && !t.contains(&"-fflags".to_string()), "{t:?}");
        let u = stream_args_with("rtsp://c/s", 10, opt(true, true));
        assert!(u.windows(2).any(|w| w == ["-rtsp_transport", "udp"]) && u.windows(2).any(|w| w == ["-fflags", "nobuffer"]) && !u.contains(&"-rw_timeout".to_string()));
        assert!(stream_args_with("rtmp://s/l/k", 10, opt(false, true)).windows(2).any(|w| w == ["-flags", "low_delay"]));
        assert_eq!(stream_args("rtsp://c/s", 10), stream_args_with("rtsp://c/s", 10, StreamOptions::default()), "the old arguments are the same");
        let r = record_args("rtsp://user:pw@c/s", opt(true, false), "/tmp/a b.mp4");
        assert!(r.windows(2).any(|w| w == ["-c:v", "copy"]) && r.windows(2).any(|w| w == ["-rtsp_transport", "udp"]), "{r:?}");
        assert_eq!(r.last().map(String::as_str), Some("/tmp/a b.mp4"), "the file is one argument");
        assert!(!r.contains(&"-vf".to_string()) && !r.iter().any(|a| a.contains("mjpeg")), "nothing is encoded again");
        assert!(!record_args("rtsp://c/s", opt(false, false), "x.mp4").contains(&"-rw_timeout".to_string()), "the RTSP demuxer refuses it");
        assert!(record_args("udp://@:5000", opt(false, false), "x.mp4").windows(2).any(|w| w == ["-probesize", "200000"]));
    }

    /// A real RTSP stream through the exact arguments of the program: a second ffmpeg publishes a test picture to the first one, which listens
    /// (`-rtsp_flags listen`). Run it with `LIVE_FFMPEG=<ffmpeg that has the rtsp muxer and the testsrc>` and `FFMPEG_FOR_ARGS=<the small one>`.
    #[test]
    #[ignore]
    fn rtsp_arguments_open_a_real_rtsp_stream() {
        let (Some(maker), Some(reader)) = (std::env::var_os("LIVE_FFMPEG"), std::env::var_os("FFMPEG_FOR_ARGS")) else { return };
        let url = "rtsp://127.0.0.1:18554/x";
        let mut args = stream_args(url, 10);
        let at = args.iter().position(|x| x == "-i").unwrap();
        args.splice(at..at, ["-rtsp_flags", "listen"].iter().map(|s| s.to_string()));
        let mut rx = std::process::Command::new(reader).args(&args).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().unwrap();
        std::thread::sleep(std::time::Duration::from_secs(1));
        let mut tx = std::process::Command::new(maker)
            .args(["-hide_banner", "-loglevel", "error", "-re", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=10", "-t", "4", "-c:v", "mpeg4", "-f", "rtsp", "-rtsp_transport", "tcp", url])
            .spawn()
            .unwrap();
        let mut out = rx.stdout.take().unwrap();
        let mut buf = vec![];
        let mut chunk = [0u8; 65536];
        let t = std::time::Instant::now();
        while t.elapsed() < std::time::Duration::from_secs(8) {
            use std::io::Read;
            match out.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(n) => buf.extend_from_slice(&chunk[..n]),
            }
            if buf.windows(2).filter(|w| *w == [0xFF, 0xD8]).count() >= 3 {
                break;
            }
        }
        let _ = rx.kill();
        let _ = tx.kill();
        let mut err = String::new();
        use std::io::Read;
        if let Some(mut e) = rx.stderr.take() {
            let _ = e.read_to_string(&mut err);
        }
        assert!(buf.windows(2).filter(|w| *w == [0xFF, 0xD8]).count() >= 3, "pictures from the RTSP stream; ffmpeg said: {err}");
    }

    #[test]
    fn a_missing_ffmpeg_is_none_and_a_file_in_the_search_folder_is_found() {
        let dir = std::env::temp_dir().join(format!("ffm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(candidates(&dir, "x").iter().all(|p| !is_file(p)));
        std::fs::write(dir.join(exe_name()), b"x").unwrap();
        assert_eq!(candidates(&dir, "x").into_iter().find(|p| is_file(p)), Some(dir.join(exe_name())));
        std::fs::remove_dir_all(dir).ok();
    }
}
