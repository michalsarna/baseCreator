//! Automatic saving. A project that has a file is written to that file; a project without a name is written to a
//! recovery file in the folder of the program (`apps/<program>/recovery.cre`), which is removed when the project is saved
//! under a name. When a program starts and finds a recovery file, its start screen offers to recover it.
use crate::paths;
use crate::project::Project;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const FILE: &str = "recovery.cre";
/// How often a changed project is saved.
pub const INTERVAL: Duration = Duration::from_secs(20);

pub fn recovery_path(base: &Path, app: &str) -> PathBuf {
    paths::app_dir(base, app).join(FILE)
}

/// Write a file so that a program that reads it at the same time (or a crash in the middle) never sees half of it.
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("{}.tmp", std::process::id()));
    std::fs::write(&tmp, text).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// Save the text of an unnamed project to the recovery file.
pub fn save_recovery(base: &Path, app: &str, text: &str) -> Result<(), String> {
    write_atomic(&recovery_path(base, app), text)
}

/// Remove the recovery file (the project was saved under a name, or the person does not want it).
pub fn clear(base: &Path, app: &str) {
    let _ = std::fs::remove_file(recovery_path(base, app));
}

/// A recovery file that was found.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub path: PathBuf,
    pub shapes: usize,
}

/// Look for a recovery file. One that is empty or cannot be read is removed (there is nothing to recover).
pub fn find(base: &Path, app: &str) -> Option<Found> {
    let path = recovery_path(base, app);
    let text = std::fs::read_to_string(&path).ok()?;
    match Project::from_json(&text) {
        Ok(p) if !p.scene.shapes.is_empty() => Some(Found { path, shapes: p.scene.shapes.len() }),
        _ => {
            // Keep a file that is damaged next to the others, so it is not lost, but do not offer it again.
            let _ = std::fs::rename(&path, path.with_extension("damaged"));
            None
        }
    }
}

/// When to save: after `INTERVAL`, and only when the project changed since the last save.
#[derive(Debug)]
pub struct Timer {
    saved_rev: Option<u64>,
    last: std::time::Instant,
}

impl Default for Timer {
    fn default() -> Self {
        Timer { saved_rev: None, last: std::time::Instant::now() }
    }
}

impl Timer {
    /// Is it time to save a project that is at revision `rev`?
    pub fn due(&self, rev: u64) -> bool {
        self.saved_rev != Some(rev) && self.last.elapsed() >= INTERVAL
    }

    /// Is it time, right now? (on exit: the interval does not matter)
    pub fn changed(&self, rev: u64) -> bool {
        self.saved_rev != Some(rev)
    }

    /// The project at `rev` was saved.
    pub fn saved(&mut self, rev: u64) {
        self.saved_rev = Some(rev);
        self.last = std::time::Instant::now();
    }

    /// The project at `rev` is what is on disk already (it was opened or saved by the person).
    pub fn in_sync(&mut self, rev: u64) {
        self.saved_rev = Some(rev);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Kind, Scene};

    fn project_with(n: usize) -> String {
        let mut s = Scene::default();
        for i in 0..n {
            s.add(0, Kind::Rect { w: 5.0, h: 5.0 }, crate::geom::Xf::translate(i as f64 * 10.0, 0.0));
        }
        Project::new(s).to_json()
    }

    #[test]
    fn a_saved_recovery_file_is_found_and_cleared() {
        let base = std::env::temp_dir().join(format!("bc-autosave-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        assert!(find(&base, "millcreator").is_none());
        save_recovery(&base, "millcreator", &project_with(3)).unwrap();
        let f = find(&base, "millcreator").unwrap();
        assert_eq!(f.shapes, 3);
        // Another program has its own.
        assert!(find(&base, "pencreator").is_none());
        clear(&base, "millcreator");
        assert!(find(&base, "millcreator").is_none());
        std::fs::remove_dir_all(base).ok();
    }

    #[test]
    fn an_empty_or_damaged_file_is_not_offered() {
        let base = std::env::temp_dir().join(format!("bc-autosave-bad-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        save_recovery(&base, "a", &project_with(0)).unwrap();
        assert!(find(&base, "a").is_none());
        save_recovery(&base, "a", "{ not a project").unwrap();
        assert!(find(&base, "a").is_none());
        assert!(!recovery_path(&base, "a").exists());
        assert!(recovery_path(&base, "a").with_extension("damaged").exists(), "kept, not lost");
        std::fs::remove_dir_all(base).ok();
    }

    #[test]
    fn the_timer_saves_only_changes_and_not_too_often() {
        let mut t = Timer::default();
        assert!(!t.due(1), "not before the interval");
        t.last -= INTERVAL;
        assert!(t.due(1));
        t.saved(1);
        t.last -= INTERVAL;
        assert!(!t.due(1), "nothing changed");
        assert!(t.due(2));
        assert!(t.changed(2));
    }
}
