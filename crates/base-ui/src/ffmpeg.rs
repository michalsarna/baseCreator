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

/// The arguments that make ffmpeg read a stream (RTSP, RTMP, UDP, RTP) and write JPEG pictures (about `fps` a second) to its standard
/// output. The address goes in as one argument, never through a shell. A dead connection ends after ten seconds without data (the
/// program opens it again); a UDP or RTP stream is read without the buffering that adds a delay.
pub fn stream_args(url: &str, fps: u32) -> Vec<String> {
    let url = url.trim();
    let lower = url.to_ascii_lowercase();
    let mut a: Vec<String> = ["-hide_banner", "-loglevel", "error", "-nostdin"].iter().map(|s| s.to_string()).collect();
    if lower.starts_with("rtsp") {
        a.extend(["-rtsp_transport", "tcp"].iter().map(|s| s.to_string()));
    } else if lower.starts_with("udp://") || lower.starts_with("rtp://") {
        // A live stream is probed for a moment, not for the 5 MB or 5 seconds ffmpeg waits for by default.
        a.extend(["-fflags", "nobuffer", "-flags", "low_delay", "-probesize", "200000", "-analyzeduration", "1000000"].iter().map(|s| s.to_string()));
    }
    // Microseconds; ends the read of a socket that gives nothing (the unplugged camera), whatever the kind of stream.
    a.extend(["-rw_timeout", "10000000", "-i", url, "-an", "-vf", &format!("fps={}", fps.clamp(1, 30)), "-f", "mjpeg", "-q:v", "5", "pipe:1"].iter().map(|s| s.to_string()));
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
        assert!(a.windows(2).any(|w| w == ["-rtsp_transport", "tcp"]) && a.windows(2).any(|w| w == ["-rw_timeout", "10000000"]));
        // A UDP stream is read without buffering; RTMP only has the timeout.
        let u = stream_args("udp://@:5000", 10);
        assert!(u.windows(2).any(|w| w == ["-fflags", "nobuffer"]) && u.windows(2).any(|w| w == ["-probesize", "200000"]) && !u.contains(&"-rtsp_transport".to_string()));
        let m = stream_args("rtmp://server/live/key", 10);
        assert!(!m.contains(&"-fflags".to_string()) && m.contains(&"-rw_timeout".to_string()));
        // Nothing here can be taken for an option: the address is after -i, whatever it starts with.
        assert_eq!(rtsp_args("-version", 5)[rtsp_args("-version", 5).iter().position(|x| x == "-i").unwrap() + 1], "-version");
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
