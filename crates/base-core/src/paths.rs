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

/// Work out the places for `app`, move what an older version kept in `legacy` (its own folder) into
/// the shared folder, and make the folders. Without a home folder everything goes to the temp folder.
/// The old folder is left as it was.
pub fn prepare(app: &str, legacy: Option<&Path>) -> DataPaths {
    let base = base_dir().unwrap_or_else(|| std::env::temp_dir().join("baseCreator"));
    prepare_in(&base, app, legacy)
}

pub fn prepare_in(base: &Path, app: &str, legacy: Option<&Path>) -> DataPaths {
    let p = DataPaths { base: base.to_path_buf(), gallery: gallery_dir(base), settings: settings_file(base, app) };
    let _ = std::fs::create_dir_all(app_dir(base, app));
    if let Some(old) = legacy {
        migrate(&p, app, old);
    }
    p
}

/// One time per program: copy the old settings (unless there are new ones) and add the old pictures to
/// the shared gallery (a picture that is already there, with the same content, is not added twice).
fn migrate(p: &DataPaths, app: &str, old: &Path) {
    let marker = app_dir(&p.base, app).join(".migrated");
    if marker.exists() {
        return;
    }
    let old_settings = old.join("app.ron");
    if old_settings.is_file() && !p.settings.exists() {
        let _ = std::fs::copy(&old_settings, &p.settings);
    }
    let old_gallery = old.join("clipart");
    if old_gallery.join("index.json").is_file() && old_gallery != p.gallery {
        merge_gallery(&old_gallery, &p.gallery);
    }
    let _ = std::fs::write(marker, "settings and gallery copied from the program's own folder\n");
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
