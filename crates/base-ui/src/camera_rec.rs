//! Recording what a camera sees: single photos, a video (Motion-JPEG in an AVI file, written here, no codec needed) and time-lapse
//! pictures, every few seconds or after each layer of a job, into a folder the person chooses.
use crate::i18n::{tr, trf};
use crate::theme;
use egui::{RichText, Ui};
use serde::{Deserialize, Serialize};
use std::io::{Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// When a time-lapse takes a picture.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LapseMode {
    /// Every `interval` seconds.
    #[default]
    Interval,
    /// When a layer of the job being sent to the machine is done.
    PerLayer,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RecSettings {
    /// Where the pictures and videos go; `None` is the folder `camera` next to the project (`camera00`, `camera01`… when it is there already),
    /// or the folder `camera` of the shared data folder when the project has no file yet.
    #[serde(default)]
    pub dir: Option<PathBuf>,
    /// Seconds between the pictures of a time-lapse.
    #[serde(default = "five")]
    pub interval: f64,
    #[serde(default)]
    pub mode: LapseMode,
    /// The file of a video.
    #[serde(default)]
    pub format: VideoFormat,
}

/// The file a video is written as.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum VideoFormat {
    /// The way that needs the least from the computer: the stream of an IP camera is copied into an MP4 file as it comes (nothing is
    /// decoded or encoded again); a camera that gives one picture after another is written as an AVI of Motion-JPEG pictures.
    #[default]
    Auto,
    /// An MP4 file: a copy of the stream. Only a stream (rtsp, udp, rtp, rtmp) can be copied; another camera is written as AVI.
    Mp4,
    /// An AVI file of Motion-JPEG pictures, whatever the camera is.
    Avi,
}

fn five() -> f64 {
    5.0
}

impl Default for RecSettings {
    fn default() -> RecSettings {
        RecSettings { dir: None, interval: 5.0, mode: LapseMode::Interval, format: VideoFormat::Auto }
    }
}

fn settings_file() -> Option<PathBuf> {
    base_core::paths::base_dir().map(|d| d.join("camera_recording.json"))
}

impl RecSettings {
    pub fn load() -> RecSettings {
        let mut s: RecSettings = settings_file().and_then(|f| std::fs::read_to_string(f).ok()).and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        s.interval = if s.interval.is_finite() { s.interval.clamp(0.2, 86_400.0) } else { 5.0 };
        s
    }

    pub fn store(&self) {
        if let (Some(f), Ok(t)) = (settings_file(), serde_json::to_string_pretty(self)) {
            let _ = std::fs::write(f, t);
        }
    }
}

// ---------------------------------------------------------------- the AVI file

/// A Motion-JPEG AVI file: every picture is a JPEG. The sizes and the rate are written at the end, when they are known.
pub struct AviWriter<W: Write + Seek> {
    w: W,
    width: u32,
    height: u32,
    /// (offset from the `movi` tag, size) of every frame.
    index: Vec<(u32, u32)>,
    max_frame: u32,
}

const HEADER_LEN: u64 = 212;
const MOVI_AT: u64 = 224;

fn le(v: u32) -> [u8; 4] {
    v.to_le_bytes()
}

impl<W: Write + Seek> AviWriter<W> {
    pub fn new(mut w: W, width: u32, height: u32) -> std::io::Result<AviWriter<W>> {
        // The header is written with zeros for what is not known yet and patched in `finish`.
        w.write_all(&vec![0u8; MOVI_AT as usize])?;
        Ok(AviWriter { w, width, height, index: vec![], max_frame: 0 })
    }

    pub fn frames(&self) -> usize {
        self.index.len()
    }

    pub fn push(&mut self, jpeg: &[u8]) -> std::io::Result<()> {
        let offset = self.w.stream_position()? - (MOVI_AT - 4);
        self.w.write_all(b"00dc")?;
        self.w.write_all(&le(jpeg.len() as u32))?;
        self.w.write_all(jpeg)?;
        if jpeg.len() % 2 == 1 {
            self.w.write_all(&[0])?;
        }
        self.index.push((offset as u32, jpeg.len() as u32));
        self.max_frame = self.max_frame.max(jpeg.len() as u32);
        Ok(())
    }

    /// Write the index and the header; `seconds` is how long the recording took (it gives the frame rate).
    pub fn finish(mut self, seconds: f64) -> std::io::Result<W> {
        let movi_end = self.w.stream_position()?;
        self.w.write_all(b"idx1")?;
        self.w.write_all(&le(self.index.len() as u32 * 16))?;
        for (off, size) in &self.index {
            self.w.write_all(b"00dc")?;
            self.w.write_all(&le(0x10))?;
            self.w.write_all(&le(*off))?;
            self.w.write_all(&le(*size))?;
        }
        let total = self.w.stream_position()?;
        let n = self.index.len() as u32;
        let fps = if seconds > 0.0 && n > 0 { (n as f64 / seconds).clamp(0.05, 120.0) } else { 10.0 };
        // The rate as a fraction of 1000 so that a slow time-lapse keeps its speed.
        let (scale, rate) = (1000u32, (fps * 1000.0).round().max(1.0) as u32);
        let usec = (1_000_000.0 / fps).round() as u32;
        let mut h: Vec<u8> = Vec::with_capacity(MOVI_AT as usize);
        let chunk = |h: &mut Vec<u8>, tag: &[u8; 4], body: &[u8]| {
            h.extend_from_slice(tag);
            h.extend_from_slice(&le(body.len() as u32));
            h.extend_from_slice(body);
        };
        h.extend_from_slice(b"RIFF");
        h.extend_from_slice(&le((total - 8) as u32));
        h.extend_from_slice(b"AVI ");
        h.extend_from_slice(b"LIST");
        h.extend_from_slice(&le(192));
        h.extend_from_slice(b"hdrl");
        let mut avih = vec![];
        for v in [usec, self.max_frame.saturating_mul(fps.ceil() as u32), 0, 0x10, n, 0, 1, self.max_frame, self.width, self.height, 0, 0, 0, 0] {
            avih.extend_from_slice(&le(v));
        }
        chunk(&mut h, b"avih", &avih);
        h.extend_from_slice(b"LIST");
        h.extend_from_slice(&le(116));
        h.extend_from_slice(b"strl");
        let mut strh = vec![];
        strh.extend_from_slice(b"vids");
        strh.extend_from_slice(b"MJPG");
        for v in [0, 0, 0, scale, rate, 0, n, self.max_frame, 0xFFFF_FFFF, 0] {
            strh.extend_from_slice(&le(v));
        }
        for v in [0i16, 0, self.width as i16, self.height as i16] {
            strh.extend_from_slice(&v.to_le_bytes());
        }
        chunk(&mut h, b"strh", &strh);
        let mut strf = vec![];
        for v in [40, self.width, self.height] {
            strf.extend_from_slice(&le(v));
        }
        strf.extend_from_slice(&1u16.to_le_bytes());
        strf.extend_from_slice(&24u16.to_le_bytes());
        strf.extend_from_slice(b"MJPG");
        for v in [self.width * self.height * 3, 0, 0, 0, 0] {
            strf.extend_from_slice(&le(v));
        }
        chunk(&mut h, b"strf", &strf);
        debug_assert_eq!(h.len() as u64, HEADER_LEN);
        h.extend_from_slice(b"LIST");
        h.extend_from_slice(&le((movi_end - (MOVI_AT - 4)) as u32));
        h.extend_from_slice(b"movi");
        debug_assert_eq!(h.len() as u64, MOVI_AT);
        self.w.seek(SeekFrom::Start(0))?;
        self.w.write_all(&h)?;
        self.w.seek(SeekFrom::Start(total))?;
        self.w.flush()?;
        Ok(self.w)
    }
}

fn jpeg_of(w: u32, h: u32, rgb: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, 85).encode(rgb, w, h, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
    Ok(out)
}

/// The first free `prefix-0001.ext`, `prefix-0002.ext`… in `dir` (for a folder, `ext` is empty).
pub fn next_name(dir: &Path, prefix: &str, ext: &str) -> PathBuf {
    let used: u32 = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let n = e.file_name().to_string_lossy().to_string();
            let rest = n.strip_prefix(prefix)?.strip_prefix('-')?;
            let digits: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            digits.parse::<u32>().ok()
        })
        .max()
        .unwrap_or(0);
    let name = if ext.is_empty() { format!("{prefix}-{:04}", used + 1) } else { format!("{prefix}-{:04}.{ext}", used + 1) };
    dir.join(name)
}

// ---------------------------------------------------------------- the layers of a job

/// Watches how far a job that is sent to the machine has got and says when a layer is done (for a picture after each layer).
#[derive(Clone, Debug, Default)]
pub struct LayerWatch {
    marks: Vec<usize>,
    next: usize,
    active: bool,
}

impl LayerWatch {
    /// A job starts; `marks` are the line numbers where its layers (after the first) begin (`link::layer_marks`).
    pub fn start(&mut self, marks: Vec<usize>) {
        self.marks = marks;
        self.next = 0;
        self.active = true;
    }

    /// `done` lines of the job are acknowledged: how many layers were done since the last call.
    pub fn progress(&mut self, done: usize) -> usize {
        if !self.active {
            return 0;
        }
        let mut n = 0;
        while self.next < self.marks.len() && done >= self.marks[self.next] {
            self.next += 1;
            n += 1;
        }
        n
    }

    /// The job ended: the last layer is done (1 when a job was watched).
    pub fn finished(&mut self) -> usize {
        let was = std::mem::replace(&mut self.active, false);
        self.marks.clear();
        usize::from(was)
    }

    /// The job was stopped: nothing more is done.
    pub fn stop(&mut self) {
        self.active = false;
        self.marks.clear();
    }
}

// ---------------------------------------------------------------- the recorder

/// How a video is written.
enum VideoKind {
    /// Motion-JPEG pictures in an AVI file, written here.
    Avi { avi: AviWriter<std::io::BufWriter<std::fs::File>>, size: (u32, u32) },
    /// A program (ffmpeg) copies the stream into an MP4 file.
    Copy { child: std::process::Child },
}

struct Video {
    kind: VideoKind,
    started: Instant,
    path: PathBuf,
}

/// The address of the stream a recording can copy, and its options.
#[derive(Clone, Debug, PartialEq)]
pub struct StreamSource {
    pub url: String,
    pub opts: crate::ffmpeg::StreamOptions,
}

struct Lapse {
    dir: PathBuf,
    shots: u32,
    last: Instant,
}

/// What a camera view records. The program gives it every picture (`feed`) and calls `tick` every frame of the window.
pub struct Recorder {
    pub settings: RecSettings,
    /// The last thing it did, for a line under the buttons.
    pub message: String,
    video: Option<Video>,
    lapse: Option<Lapse>,
    latest: Option<(u32, u32, Vec<u8>)>,
    /// The project file the pictures belong to: its folder holds the folder of the pictures, its name starts the names of the files.
    project: Option<PathBuf>,
    /// The folder chosen for a folder of a project, kept for the rest of the run.
    chosen: Option<(PathBuf, PathBuf)>,
    /// The stream of the camera, when it is one that can be copied (an address that ffmpeg reads).
    source: Option<StreamSource>,
}

impl Default for Recorder {
    fn default() -> Recorder {
        Recorder::new(RecSettings::load())
    }
}

impl Recorder {
    pub fn new(settings: RecSettings) -> Recorder {
        Recorder { settings, message: String::new(), video: None, lapse: None, latest: None, project: None, chosen: None, source: None }
    }

    /// Does the camera have to be kept running (a video or a time-lapse is being made)?
    pub fn active(&self) -> bool {
        self.video.is_some() || self.lapse.is_some()
    }

    pub fn recording(&self) -> bool {
        self.video.is_some()
    }

    pub fn lapsing(&self) -> bool {
        self.lapse.is_some()
    }

    /// Tell the recorder which project file the pictures are of (`None`: the project was never saved).
    pub fn set_project(&mut self, path: Option<&Path>) {
        if self.project.as_deref() != path {
            self.project = path.map(Path::to_path_buf);
        }
    }

    /// Tell the recorder which camera it records (`None`: none, or one that is not a stream).
    pub fn set_source(&mut self, cam: Option<&base_core::camimg::CameraSource>) {
        self.source = cam.and_then(|c| match &c.kind {
            base_core::camimg::CameraKind::Url(u) if base_core::camimg::is_ffmpeg_stream(u) => Some(StreamSource { url: u.trim().to_string(), opts: crate::camera_feed::options_of(c) }),
            _ => None,
        });
    }

    /// Is a video of this camera a copy of the stream (and not Motion-JPEG pictures)? It is when the person did not ask for AVI, the
    /// camera is a stream and ffmpeg is there.
    pub fn copies_stream(&self) -> bool {
        self.wants_copy() && crate::ffmpeg::path().is_some()
    }

    /// Would a video of this camera be a copy of the stream, if ffmpeg is there?
    fn wants_copy(&self) -> bool {
        self.settings.format != VideoFormat::Avi && self.source.is_some()
    }

    /// The start of the names of the files: the name of the project file, or nothing.
    fn stem(&self) -> String {
        let s = self.project.as_ref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
        s.chars().map(|c| if c.is_alphanumeric() || matches!(c, '-' | '_' | '.' | ' ') { c } else { '_' }).collect::<String>().trim().to_string()
    }

    /// `{project}-{kind}`, or just `kind` when there is no project file.
    fn prefix(&self, kind: &str) -> String {
        let s = self.stem();
        if s.is_empty() { kind.to_string() } else { format!("{s}-{kind}") }
    }

    /// The folder `camera` next to the project file: the first of `camera`, `camera00`, `camera01`… that is not there yet.
    pub fn free_camera_folder(base: &Path) -> PathBuf {
        let first = base.join("camera");
        if !first.exists() {
            return first;
        }
        (0..).map(|n| base.join(format!("camera{n:02}"))).find(|p| !p.exists()).expect("a free number")
    }

    /// The folder shown to the person: the chosen one, the one of this run, or the one the next file would make.
    pub fn folder_shown(&self) -> Option<PathBuf> {
        if let Some(d) = &self.settings.dir {
            return Some(d.clone());
        }
        let base = self.project.as_ref().and_then(|p| p.parent())?;
        match &self.chosen {
            Some((b, d)) if b == base => Some(d.clone()),
            _ => Some(Self::free_camera_folder(base)),
        }
    }

    /// The folder for the files; made when it is needed.
    pub fn folder(&mut self) -> Result<PathBuf, String> {
        let dir = if let Some(d) = self.settings.dir.clone() {
            d
        } else if let Some(base) = self.project.as_ref().and_then(|p| p.parent()).filter(|b| !b.as_os_str().is_empty()).map(Path::to_path_buf) {
            match &self.chosen {
                Some((b, d)) if *b == base => d.clone(),
                _ => {
                    let d = Self::free_camera_folder(&base);
                    self.chosen = Some((base, d.clone()));
                    d
                }
            }
        } else {
            base_core::paths::base_dir().map(|d| d.join("camera")).ok_or_else(|| "there is no folder".to_string())?
        };
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        Ok(dir)
    }

    fn fail(&mut self, e: impl std::fmt::Display) {
        self.message = trf("Could not save: {}", &[&e]);
    }

    /// A new picture from the camera: it goes into the video, and is kept for the time-lapse and the photos.
    pub fn feed(&mut self, now: Instant, w: u32, h: u32, rgb: Vec<u8>) {
        if let Some(Video { kind: VideoKind::Avi { avi, size }, .. }) = &mut self.video {
            if *size == (w, h) {
                match jpeg_of(w, h, &rgb).and_then(|j| avi.push(&j).map_err(|e| e.to_string())) {
                    Ok(()) => {}
                    Err(e) => {
                        self.fail(e);
                        self.stop_video(now);
                    }
                }
            }
        }
        self.latest = Some((w, h, rgb));
        self.tick(now);
    }

    /// Every frame of the window: the picture of a time-lapse that is due is taken.
    pub fn tick(&mut self, now: Instant) {
        // A copy of a stream that ended by itself (the camera went away, the address was wrong).
        let ended = match &mut self.video {
            Some(Video { kind: VideoKind::Copy { child }, .. }) => child.try_wait().ok().flatten().is_some(),
            _ => false,
        };
        if ended {
            self.stop_video(now);
        }
        let due = match (&self.lapse, self.settings.mode) {
            (Some(l), LapseMode::Interval) => now.duration_since(l.last) >= Duration::from_secs_f64(self.settings.interval),
            _ => false,
        };
        if due {
            self.lapse_shot(now, "frame");
        }
    }

    fn save_latest(&mut self, path: &Path) -> Result<(), String> {
        let (w, h, rgb) = self.latest.as_ref().ok_or_else(|| tr("No picture from the camera yet.").to_string())?;
        let jpeg = jpeg_of(*w, *h, rgb)?;
        std::fs::write(path, jpeg).map_err(|e| e.to_string())
    }

    /// One photo, as the next `photo-0001.jpg` of the folder.
    pub fn photo(&mut self) -> Option<PathBuf> {
        let dir = match self.folder() {
            Ok(d) => d,
            Err(e) => {
                self.fail(e);
                return None;
            }
        };
        let path = next_name(&dir, &self.prefix("photo"), "jpg");
        match self.save_latest(&path) {
            Ok(()) => {
                self.message = trf("Saved {}", &[&path.display()]);
                Some(path)
            }
            Err(e) => {
                self.fail(e);
                None
            }
        }
    }

    pub fn start_video(&mut self, now: Instant) {
        if self.video.is_some() {
            return;
        }
        let Some((w, h, _)) = self.latest.clone() else {
            self.message = tr("No picture from the camera yet.").to_string();
            return;
        };
        let copy = self.copies_stream();
        let result = self.folder().and_then(|dir| {
            if copy {
                let path = next_name(&dir, &self.prefix("video"), "mp4");
                let source = self.source.clone().expect("a stream to copy");
                let ffmpeg = crate::ffmpeg::path().ok_or_else(|| "ffmpeg".to_string())?;
                let child = std::process::Command::new(ffmpeg)
                    .args(crate::ffmpeg::record_args(&source.url, source.opts, &path.to_string_lossy()))
                    .stdin(std::process::Stdio::piped())
                    .stdout(std::process::Stdio::null())
                    .stderr(std::process::Stdio::null())
                    .spawn()
                    .map_err(|e| e.to_string())?;
                Ok(Video { kind: VideoKind::Copy { child }, started: now, path })
            } else {
                let path = next_name(&dir, &self.prefix("video"), "avi");
                let file = std::fs::File::create(&path).map_err(|e| e.to_string())?;
                let avi = AviWriter::new(std::io::BufWriter::new(file), w, h).map_err(|e| e.to_string())?;
                Ok(Video { kind: VideoKind::Avi { avi, size: (w, h) }, started: now, path })
            }
        });
        match result {
            Ok(v) => {
                let avi = matches!(v.kind, VideoKind::Avi { .. });
                self.video = Some(v);
                if avi {
                    // The picture that is on the screen is the first one.
                    if let Some((w, h, rgb)) = self.latest.clone() {
                        self.feed(now, w, h, rgb);
                    }
                    if self.settings.format == VideoFormat::Mp4 && self.source.is_none() {
                        self.message = tr("MP4 copies the stream of an IP camera; this camera is recorded as AVI.").to_string();
                    }
                }
            }
            Err(e) => self.fail(e),
        }
    }

    pub fn stop_video(&mut self, now: Instant) {
        let Some(v) = self.video.take() else { return };
        match v.kind {
            VideoKind::Avi { avi, .. } => {
                let frames = avi.frames();
                match avi.finish(now.duration_since(v.started).as_secs_f64()) {
                    Ok(_) => self.message = trf("Saved {} ({} pictures)", &[&v.path.display(), &frames]),
                    Err(e) => self.fail(e),
                }
            }
            VideoKind::Copy { mut child } => {
                // `q` makes ffmpeg close the file properly; a program that does not stop is killed (the file is fragmented, it plays anyway).
                if let Some(mut stdin) = child.stdin.take() {
                    let _ = stdin.write_all(b"q\n");
                }
                let t = Instant::now();
                while child.try_wait().ok().flatten().is_none() && t.elapsed() < Duration::from_secs(3) {
                    std::thread::sleep(Duration::from_millis(30));
                }
                if child.try_wait().ok().flatten().is_none() {
                    let _ = child.kill();
                    let _ = child.wait();
                }
                let size = std::fs::metadata(&v.path).map(|m| m.len()).unwrap_or(0);
                if size > 0 {
                    self.message = trf("Saved {}", &[&v.path.display()]);
                } else {
                    let _ = std::fs::remove_file(&v.path);
                    self.message = tr("The stream could not be recorded (the address, the camera, ffmpeg?).").to_string();
                }
            }
        }
    }

    /// Seconds and pictures of the video being recorded (a copy of a stream has no pictures to count: 0).
    pub fn video_progress(&self, now: Instant) -> Option<(u64, usize)> {
        self.video.as_ref().map(|v| (now.duration_since(v.started).as_secs(), if let VideoKind::Avi { avi, .. } = &v.kind { avi.frames() } else { 0 }))
    }

    /// Is the video being written a copy of a stream?
    pub fn recording_copy(&self) -> bool {
        matches!(self.video, Some(Video { kind: VideoKind::Copy { .. }, .. }))
    }

    pub fn start_lapse(&mut self, now: Instant) {
        if self.lapse.is_some() {
            return;
        }
        if self.latest.is_none() {
            self.message = tr("No picture from the camera yet.").to_string();
            return;
        }
        let result = self.folder().and_then(|dir| {
            let session = next_name(&dir, &self.prefix("timelapse"), "");
            std::fs::create_dir_all(&session).map_err(|e| e.to_string())?;
            Ok(session)
        });
        match result {
            Ok(dir) => {
                self.lapse = Some(Lapse { dir, shots: 0, last: now });
                // The first picture at once in the interval mode.
                if self.settings.mode == LapseMode::Interval {
                    self.lapse_shot(now, "frame");
                } else {
                    self.message = tr("Time-lapse: a picture is taken when a layer is done.").to_string();
                }
            }
            Err(e) => self.fail(e),
        }
    }

    pub fn stop_lapse(&mut self) {
        if let Some(l) = self.lapse.take() {
            self.message = trf("Time-lapse: {} pictures in {}", &[&l.shots, &l.dir.display()]);
        }
    }

    pub fn lapse_shots(&self) -> Option<u32> {
        self.lapse.as_ref().map(|l| l.shots)
    }

    fn lapse_shot(&mut self, now: Instant, kind: &str) {
        let Some(l) = &self.lapse else { return };
        let (dir, n) = (l.dir.clone(), l.shots + 1);
        let path = dir.join(format!("{}-{n:05}.jpg", self.prefix(kind)));
        match self.save_latest(&path) {
            Ok(()) => {
                if let Some(l) = &mut self.lapse {
                    l.shots = n;
                    l.last = now;
                }
            }
            Err(e) => {
                self.fail(e);
                self.stop_lapse();
            }
        }
    }

    /// A layer of the job that is being sent to the machine is done: a picture, in the "after each layer" mode.
    pub fn layer_done(&mut self, now: Instant) {
        if self.lapse.is_some() && self.settings.mode == LapseMode::PerLayer {
            self.lapse_shot(now, "layer");
        }
    }

    /// The buttons: photo, video, time-lapse and the folder.
    pub fn controls(&mut self, ui: &mut Ui) {
        let now = Instant::now();
        let has = self.latest.is_some();
        let mut changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(has, egui::Button::new(tr("Take a photo"))).clicked() {
                self.photo();
            }
            match self.video_progress(now) {
                Some((secs, frames)) => {
                    if ui.button(tr("Stop recording")).clicked() {
                        self.stop_video(now);
                    }
                    if self.recording_copy() {
                        ui.label(RichText::new(trf("Recording: {} s (the stream is copied)", &[&secs])).color(theme::text_dim()));
                    } else {
                        ui.label(RichText::new(trf("Recording: {} s, {} pictures", &[&secs, &frames])).color(theme::text_dim()));
                    }
                }
                None => {
                    if ui.add_enabled(has, egui::Button::new(tr("Record video"))).clicked() {
                        self.start_video(now);
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Video format"));
            let hint = tr("Automatic is the way that needs the least from the computer: the stream of an IP camera is copied into an MP4 file as it comes, nothing is decoded or encoded again. A camera that gives one picture after another is written as AVI.");
            let names = [(VideoFormat::Auto, tr("Automatic").to_string()), (VideoFormat::Mp4, "MP4".to_string()), (VideoFormat::Avi, "AVI".to_string())];
            ui.add_enabled_ui(!self.recording(), |ui| {
                for (f, name) in names {
                    changed |= ui.selectable_value(&mut self.settings.format, f, name).on_hover_text(&*hint).changed();
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Time-lapse"));
            changed |= ui.selectable_value(&mut self.settings.mode, LapseMode::Interval, tr("Every")).changed();
            ui.add_enabled_ui(self.settings.mode == LapseMode::Interval, |ui| {
                changed |= ui.add(egui::DragValue::new(&mut self.settings.interval).range(0.2..=86_400.0).speed(0.5).suffix(" s")).changed();
            });
            changed |= ui.selectable_value(&mut self.settings.mode, LapseMode::PerLayer, tr("After each layer")).on_hover_text(tr("Needs a job sent to the machine from the program (Start)")).changed();
            match self.lapse_shots() {
                Some(n) => {
                    if ui.button(tr("Stop time-lapse")).clicked() {
                        self.stop_lapse();
                    }
                    ui.label(RichText::new(trf("{} pictures", &[&n])).color(theme::text_dim()));
                }
                None => {
                    if ui.add_enabled(has, egui::Button::new(tr("Start time-lapse"))).clicked() {
                        self.start_lapse(now);
                    }
                }
            }
        });
        ui.horizontal_wrapped(|ui| {
            ui.label(tr("Folder"));
            let shown = self.folder_shown().map(|d| d.display().to_string()).unwrap_or_else(|| tr("the folder camera of the shared data").to_string());
            ui.label(RichText::new(shown).color(theme::text_dim()));
            if ui.button(tr("Choose the folder…")).clicked() {
                if let Some(d) = rfd::FileDialog::new().pick_folder() {
                    self.settings.dir = Some(d);
                    changed = true;
                }
            }
        });
        if !self.message.is_empty() {
            ui.label(RichText::new(&self.message).small().color(theme::text_dim()));
        }
        if changed {
            self.settings.store();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture(v: u8) -> (u32, u32, Vec<u8>) {
        (16, 8, vec![v; 16 * 8 * 3])
    }

    fn temp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("camrec-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn rec(dir: &Path) -> Recorder {
        Recorder::new(RecSettings { dir: Some(dir.to_path_buf()), interval: 2.0, mode: LapseMode::Interval, format: VideoFormat::Auto })
    }

    #[test]
    fn an_avi_has_the_right_sizes_an_index_and_its_frames_can_be_read_back_by_the_index() {
        let jpegs: Vec<Vec<u8>> = (0..3).map(|i| { let (w, h, p) = picture(40 * i + 20); jpeg_of(w, h, &p).unwrap() }).collect();
        let mut avi = AviWriter::new(std::io::Cursor::new(Vec::new()), 16, 8).unwrap();
        for j in &jpegs {
            avi.push(j).unwrap();
        }
        let bytes = avi.finish(1.5).unwrap().into_inner();
        let u32_at = |o: usize| u32::from_le_bytes([bytes[o], bytes[o + 1], bytes[o + 2], bytes[o + 3]]);
        assert_eq!(&bytes[0..4], b"RIFF");
        assert_eq!(u32_at(4) as usize, bytes.len() - 8, "the size of the RIFF");
        assert_eq!(&bytes[8..12], b"AVI ");
        assert_eq!(&bytes[20..24], b"hdrl");
        assert_eq!(&bytes[24..28], b"avih");
        // 3 pictures in 1.5 s: 2 a second, 500000 microseconds each; the width and height are in the header; the total is 3.
        assert_eq!(u32_at(32), 500_000);
        assert_eq!(u32_at(48), 3);
        assert_eq!((u32_at(64), u32_at(68)), (16, 8));
        assert_eq!(&bytes[212..216], b"LIST");
        assert_eq!(&bytes[220..224], b"movi");
        assert_eq!(u32_at(216) as usize, 4 + jpegs.iter().map(|j| 8 + j.len() + j.len() % 2).sum::<usize>(), "the size of the movi list");
        // The index points at the frames, relative to the "movi" tag.
        let idx = bytes.windows(4).rposition(|w| w == b"idx1").unwrap();
        assert_eq!(u32_at(idx + 4), 48);
        for (i, j) in jpegs.iter().enumerate() {
            let e = idx + 8 + i * 16;
            assert_eq!(&bytes[e..e + 4], b"00dc");
            let (off, size) = (u32_at(e + 8) as usize, u32_at(e + 12) as usize);
            let at = 220 + off;
            assert_eq!(&bytes[at..at + 4], b"00dc");
            assert_eq!(size, j.len());
            assert_eq!(&bytes[at + 8..at + 8 + size], &j[..]);
        }
    }

    #[test]
    fn files_go_to_a_camera_folder_next_to_the_project_and_carry_its_name() {
        let base = temp("project");
        let project = base.join("box.cre");
        let mut r = Recorder::new(RecSettings { dir: None, interval: 2.0, mode: LapseMode::Interval, format: VideoFormat::Auto });
        r.set_project(Some(&project));
        let (w, h, rgb) = picture(40);
        r.feed(Instant::now(), w, h, rgb);
        let first = r.photo().unwrap();
        assert_eq!(first, base.join("camera").join("box-photo-0001.jpg"));
        assert_eq!(r.photo().unwrap(), base.join("camera").join("box-photo-0002.jpg"), "the same folder for the rest of the run");
        // Another run finds the folder there and makes the next one.
        let mut again = Recorder::new(RecSettings { dir: None, interval: 2.0, mode: LapseMode::Interval, format: VideoFormat::Auto });
        again.set_project(Some(&project));
        let (w, h, rgb) = picture(50);
        again.feed(Instant::now(), w, h, rgb);
        assert_eq!(again.photo().unwrap(), base.join("camera00").join("box-photo-0001.jpg"));
        assert_eq!(Recorder::free_camera_folder(&base), base.join("camera01"));
        // A time-lapse folder and a video carry the name too.
        again.start_video(Instant::now());
        assert!(again.video.as_ref().unwrap().path.file_name().unwrap().to_string_lossy().starts_with("box-video-"));
        again.stop_video(Instant::now());
        again.start_lapse(Instant::now());
        assert!(again.lapse.as_ref().unwrap().dir.file_name().unwrap().to_string_lossy().starts_with("box-timelapse-"));
        let frame = std::fs::read_dir(&again.lapse.as_ref().unwrap().dir).unwrap().next().unwrap().unwrap().file_name().to_string_lossy().to_string();
        assert!(frame.starts_with("box-frame-"), "{frame}");
    }

    #[test]
    fn the_format_says_what_a_video_is_and_a_camera_that_is_not_a_stream_is_recorded_as_avi() {
        use base_core::camimg::CameraSource;
        let dir = temp("format");
        let mut r = rec(&dir);
        assert_eq!(r.settings.format, VideoFormat::Auto, "automatic to begin with");
        // A camera that is not a stream: AVI whatever is asked, and MP4 asked for says why.
        r.set_source(Some(&CameraSource::url("c", "http://127.0.0.1:1/x.mjpg", 0)));
        assert!(!r.wants_copy());
        r.settings.format = VideoFormat::Mp4;
        let (w, h, rgb) = picture(30);
        r.feed(Instant::now(), w, h, rgb);
        r.start_video(Instant::now());
        assert!(r.recording() && !r.recording_copy());
        assert!(r.message.contains("AVI"), "{}", r.message);
        r.stop_video(Instant::now());
        assert!(std::fs::read_dir(&dir).unwrap().flatten().any(|e| e.path().extension().is_some_and(|x| x == "avi")));
        // A stream is copied unless AVI is asked for.
        r.set_source(Some(&CameraSource::url("c", "rtsp://cam/stream1", 0)));
        for (f, copy) in [(VideoFormat::Auto, true), (VideoFormat::Mp4, true), (VideoFormat::Avi, false)] {
            r.settings.format = f;
            assert_eq!(r.wants_copy(), copy, "{f:?}");
        }
        r.set_source(None);
        assert!(!r.wants_copy());
        std::fs::remove_dir_all(dir).ok();
    }

    /// A real recording: a stream (MPEG-TS over UDP, made by a second ffmpeg) is copied into an MP4 file through the exact arguments of the
    /// program, and the file is read back and decoded. Run it with `CREATOR_FFMPEG=<the small ffmpeg>` and
    /// `LIVE_FFMPEG=<one that has the lavfi source, the mpeg4 encoder, the mpegts muxer and the mov demuxer>`.
    #[test]
    #[ignore]
    fn a_stream_is_copied_into_a_playable_mp4_without_encoding() {
        let Some(maker) = std::env::var_os("LIVE_FFMPEG") else { return };
        assert!(std::env::var_os("CREATOR_FFMPEG").is_some(), "CREATOR_FFMPEG is the ffmpeg of the program");
        let dir = temp("copy");
        let cam = base_core::camimg::CameraSource::url("c", "udp://127.0.0.1:18712", 0);
        let mut r = rec(&dir);
        r.set_source(Some(&cam));
        assert!(r.copies_stream(), "a UDP stream is copied");
        let (w, h, rgb) = picture(40);
        r.feed(Instant::now(), w, h, rgb);
        r.start_video(Instant::now());
        assert!(r.recording_copy(), "{}", r.message);
        let mut tx = std::process::Command::new(&maker)
            .args(["-hide_banner", "-loglevel", "error", "-re", "-f", "lavfi", "-i", "testsrc=size=320x240:rate=10", "-t", "9", "-c:v", "mpeg4", "-g", "10", "-f", "mpegts", "udp://127.0.0.1:18712?pkt_size=1316"])
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_secs(7));
        r.stop_video(Instant::now());
        let _ = tx.kill();
        let path = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|e| e == "mp4")).expect("an MP4 file");
        let bytes = std::fs::read(&path).unwrap();
        assert!(bytes.len() > 2000 && &bytes[4..8] == b"ftyp", "an MP4 file: {} bytes; {}", bytes.len(), r.message);
        // Read back and decoded (to Motion-JPEG pictures on the standard output, which are counted).
        let out = std::process::Command::new(&maker).args(["-hide_banner", "-loglevel", "error", "-i"]).arg(&path).args(["-f", "mjpeg", "-q:v", "8", "pipe:1"]).output().unwrap();
        let pictures = out.stdout.windows(3).filter(|w| *w == [0xFF, 0xD8, 0xFF]).count();
        assert!(out.status.success() && pictures >= 20, "the file decodes: {pictures} pictures; {}", String::from_utf8_lossy(&out.stderr));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_photo_goes_to_the_next_free_name_and_there_is_no_photo_without_a_picture() {
        let dir = temp("photo");
        let mut r = rec(&dir);
        assert!(r.photo().is_none(), "no picture yet: {}", r.message);
        let t = Instant::now();
        let (w, h, p) = picture(100);
        r.feed(t, w, h, p);
        assert_eq!(r.photo().unwrap().file_name().unwrap(), "photo-0001.jpg");
        assert_eq!(r.photo().unwrap().file_name().unwrap(), "photo-0002.jpg");
        let img = image::open(dir.join("photo-0002.jpg")).unwrap();
        assert_eq!((img.width(), img.height()), (16, 8));
        assert_eq!(next_name(&dir, "photo", "jpg").file_name().unwrap(), "photo-0003.jpg");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_video_is_written_while_pictures_come_and_closed_with_the_rate_it_had() {
        let dir = temp("video");
        let mut r = rec(&dir);
        let t = Instant::now();
        let (w, h, p) = picture(60);
        r.feed(t, w, h, p);
        r.start_video(t);
        assert!(r.recording() && r.active());
        for i in 1..=4u64 {
            let (w, h, p) = picture(60 + i as u8 * 10);
            r.feed(t + Duration::from_millis(250 * i), w, h, p);
        }
        // A picture of another size (the camera was turned) is left out of the video.
        r.feed(t + Duration::from_millis(1250), 8, 16, vec![1; 8 * 16 * 3]);
        r.stop_video(t + Duration::from_secs(1));
        assert!(!r.recording() && r.message.contains("video-0001.avi"), "{}", r.message);
        let bytes = std::fs::read(dir.join("video-0001.avi")).unwrap();
        let n = u32::from_le_bytes([bytes[48], bytes[49], bytes[50], bytes[51]]);
        assert_eq!(n, 5, "the one on the screen and four more");
        assert_eq!(u32::from_le_bytes([bytes[32], bytes[33], bytes[34], bytes[35]]), 200_000, "5 in a second");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_time_lapse_takes_a_picture_every_interval_or_when_a_layer_is_done_and_only_in_its_mode() {
        let dir = temp("lapse");
        let mut r = rec(&dir);
        let t = Instant::now();
        let (w, h, p) = picture(90);
        r.feed(t, w, h, p);
        r.start_lapse(t);
        assert_eq!(r.lapse_shots(), Some(1), "the first picture at once");
        r.tick(t + Duration::from_secs(1));
        assert_eq!(r.lapse_shots(), Some(1), "not yet");
        r.tick(t + Duration::from_secs(2));
        r.tick(t + Duration::from_millis(2500));
        assert_eq!(r.lapse_shots(), Some(2));
        r.layer_done(t + Duration::from_secs(3));
        assert_eq!(r.lapse_shots(), Some(2), "a finished layer does nothing in the interval mode");
        r.stop_lapse();
        assert!(r.message.contains("2 pictures") && !r.lapsing());
        assert!(dir.join("timelapse-0001").join("frame-00002.jpg").exists());
        // After each layer: no timer, a picture for every layer; a new session folder.
        r.settings.mode = LapseMode::PerLayer;
        r.start_lapse(t + Duration::from_secs(10));
        assert_eq!(r.lapse_shots(), Some(0));
        r.tick(t + Duration::from_secs(100));
        assert_eq!(r.lapse_shots(), Some(0), "no timer");
        r.layer_done(t + Duration::from_secs(101));
        r.layer_done(t + Duration::from_secs(102));
        assert_eq!(r.lapse_shots(), Some(2));
        assert!(dir.join("timelapse-0002").join("layer-00002.jpg").exists());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn a_layer_is_done_when_the_lines_of_the_next_one_begin_and_the_last_when_the_job_ends() {
        let mut w = LayerWatch::default();
        assert_eq!(w.progress(100), 0, "no job");
        w.start(vec![10, 30]);
        assert_eq!(w.progress(5), 0);
        assert_eq!(w.progress(10), 1);
        assert_eq!(w.progress(12), 0, "once");
        assert_eq!(w.progress(100), 1, "a jump over a mark still counts");
        assert_eq!(w.progress(200), 0);
        assert_eq!(w.finished(), 1, "the last layer");
        assert_eq!(w.finished(), 0);
        w.start(vec![3]);
        w.stop();
        assert_eq!((w.progress(10), w.finished()), (0, 0), "a stopped job does nothing");
    }

    #[test]
    fn the_settings_are_kept_in_range_and_old_files_read() {
        let s: RecSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, RecSettings::default());
        let s: RecSettings = serde_json::from_str("{\"interval\":0.0001,\"mode\":\"PerLayer\"}").unwrap();
        assert_eq!(s.mode, LapseMode::PerLayer);
        // The controls draw (with a picture and without).
        let ctx = egui::Context::default();
        let mut r = Recorder::new(RecSettings::default());
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| r.controls(ui));
            });
            out.textures_delta.clear();
            let (w, h, p) = picture(10);
            r.feed(Instant::now(), w, h, p);
        }
    }
}
