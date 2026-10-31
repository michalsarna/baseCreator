//! The ffmpeg that comes with the program: it turns the stream of an IP camera (RTSP, H.264 / H.265) into pictures the program can read.
//! It is looked for next to the program, in the resources of a macOS app and in a folder `ffmpeg` beside it, and only then in the PATH of
//! the system. The program starts it as a separate process and talks to it through pipes; nothing is linked.
use std::path::{Path, PathBuf};

fn exe_name() -> &'static str {
    if cfg!(windows) { "ffmpeg.exe" } else { "ffmpeg" }
}

/// The places to look in, in order, for a program whose executable is in `exe_dir`.
pub fn candidates(exe_dir: &Path) -> Vec<PathBuf> {
    let n = exe_name();
    vec![exe_dir.join(n), exe_dir.join("ffmpeg").join(n), exe_dir.join("../Resources").join(n), exe_dir.join("../Resources/ffmpeg").join(n), exe_dir.join("../lib/creator-ffmpeg").join(n)]
}

fn is_file(p: &Path) -> bool {
    std::fs::metadata(p).map(|m| m.is_file()).unwrap_or(false)
}

/// The ffmpeg to run: the one that came with the program, else the first in the PATH; `None` when there is none.
pub fn path() -> Option<PathBuf> {
    if let Some(dir) = std::env::current_exe().ok().and_then(|e| e.parent().map(Path::to_path_buf)) {
        if let Some(p) = candidates(&dir).into_iter().find(|p| is_file(p)) {
            return Some(p);
        }
    }
    let n = exe_name();
    std::env::var_os("PATH").and_then(|paths| std::env::split_paths(&paths).map(|d| d.join(n)).find(|p| is_file(p)))
}

/// The arguments that make ffmpeg read an RTSP stream and write JPEG pictures (about `fps` a second) to its standard output. The address
/// goes in as one argument, never through a shell.
pub fn rtsp_args(url: &str, fps: u32) -> Vec<String> {
    ["-hide_banner", "-loglevel", "error", "-nostdin", "-rtsp_transport", "tcp", "-i", url.trim(), "-an", "-vf", &format!("fps={}", fps.clamp(1, 30)), "-f", "mjpeg", "-q:v", "5", "pipe:1"].iter().map(|s| s.to_string()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_places_start_next_to_the_program_and_the_address_is_one_argument() {
        let c = candidates(Path::new("/opt/creator"));
        assert_eq!(c[0], Path::new("/opt/creator").join(exe_name()));
        assert!(c.iter().any(|p| p.to_string_lossy().contains("Resources")), "the resources of a macOS app");
        let a = rtsp_args("rtsp://user:pass@192.168.1.5:554/stream1", 10);
        assert_eq!(a.iter().position(|x| x == "-i").map(|i| a[i + 1].as_str()), Some("rtsp://user:pass@192.168.1.5:554/stream1"));
        assert!(a.contains(&"fps=10".to_string()) && a.last().map(String::as_str) == Some("pipe:1"));
        assert!(rtsp_args("rtsp://x", 500).contains(&"fps=30".to_string()), "the rate is limited");
        // Nothing here can be taken for an option: the address is after -i, whatever it starts with.
        assert_eq!(rtsp_args("-version", 5)[rtsp_args("-version", 5).iter().position(|x| x == "-i").unwrap() + 1], "-version");
    }

    #[test]
    fn a_missing_ffmpeg_is_none_and_a_file_in_the_search_folder_is_found() {
        let dir = std::env::temp_dir().join(format!("ffm-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        assert!(candidates(&dir).iter().all(|p| !is_file(p)));
        std::fs::write(dir.join(exe_name()), b"x").unwrap();
        assert_eq!(candidates(&dir).into_iter().find(|p| is_file(p)), Some(dir.join(exe_name())));
        std::fs::remove_dir_all(dir).ok();
    }
}
