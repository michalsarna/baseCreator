//! Where the programs keep their data. LightCreator, MillCreator and PenCreator share one folder:
//!
//! ```text
//! <data>/baseCreator/
//!   clipart/             the gallery, the same for all programs (index.json + SVG files)
//!   apps/<program>/      what belongs to one program: app.ron (settings, device profiles, materials, tools)
//! ```
//!
//! `<data>` is `~/Library/Application Support` on macOS, `%APPDATA%` on Windows and `$XDG_DATA_HOME`
//! (or `~/.local/share`) elsewhere. The environment variable `BASECREATOR_DIR` replaces the whole
//! `<data>/baseCreator` folder.
use crate::clipart::Library;
use std::path::{Path, PathBuf};

/// The shared folder, or `None` when the system gives no home folder.
pub fn base_dir() -> Option<PathBuf> {
    if let Some(d) = std::env::var_os("BASECREATOR_DIR").filter(|d| !d.is_empty()) {
        return Some(PathBuf::from(d));
    }
    let data = if cfg!(target_os = "macos") {
        std::env::home_dir()?.join("Library").join("Application Support")
    } else if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?)
    } else {
        std::env::var_os("XDG_DATA_HOME").map(PathBuf::from).filter(|p| p.is_absolute()).or_else(|| std::env::home_dir().map(|h| h.join(".local").join("share")))?
    };
    Some(data.join("baseCreator"))
}

/// The gallery shared by all programs.
pub fn gallery_dir(base: &Path) -> PathBuf {
    base.join("clipart")
}

/// The folder of one program (`lightcreator`, `millcreator`, `pencreator`).
pub fn app_dir(base: &Path, app: &str) -> PathBuf {
    base.join("apps").join(app)
}

/// The file the program keeps its settings in.
pub fn settings_file(base: &Path, app: &str) -> PathBuf {
    app_dir(base, app).join("app.ron")
}

/// The places a program uses.
#[derive(Clone, Debug)]
pub struct DataPaths {
    pub base: PathBuf,
    pub gallery: PathBuf,
    pub settings: PathBuf,
}

/// Work out the places for `app` and make its folder. Without a home folder everything goes to the temp folder.
pub fn prepare(app: &str) -> DataPaths {
    let base = base_dir().unwrap_or_else(|| std::env::temp_dir().join("baseCreator"));
    prepare_in(&base, app)
}

pub fn prepare_in(base: &Path, app: &str) -> DataPaths {
    let p = DataPaths { base: base.to_path_buf(), gallery: gallery_dir(base), settings: settings_file(base, app) };
    let _ = std::fs::create_dir_all(app_dir(base, app));
    p
}

/// Add the pictures of the library in `from` to the one in `to`. Returns how many were added.
pub fn merge_gallery(from: &Path, to: &Path) -> usize {
    let src = Library::open(from);
    let mut dst = Library::open(to);
    let mut added = 0;
    for c in &src.items {
        let Some(svg) = src.svg(&c.id) else { continue };
        if dst.items.iter().any(|d| d.name == c.name && d.category == c.category && dst.svg(&d.id).as_deref() == Some(svg.as_str())) {
            continue;
        }
        if dst.add(&svg, &c.name, &c.category, &c.source, &c.license, &c.author).is_ok() {
            added += 1;
        }
    }
    added
}
