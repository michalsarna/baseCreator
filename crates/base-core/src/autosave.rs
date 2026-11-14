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

/// The most tabs (open projects) a program has; each one has its own recovery file, numbered by its slot.
pub const SLOTS: usize = 10;

pub fn recovery_path(base: &Path, app: &str) -> PathBuf {
    slot_path(base, app, 0)
}

/// The recovery file of a tab: slot 0 is `recovery.cre` (as it always was), the others `recovery-<n>.cre`.
pub fn slot_path(base: &Path, app: &str, slot: usize) -> PathBuf {
    if slot == 0 {
        paths::app_dir(base, app).join(FILE)
    } else {
        paths::app_dir(base, app).join(format!("recovery-{slot}.cre"))
    }
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
    save_slot(base, app, 0, text)
}

/// Save the text of an unnamed project of tab `slot`.
pub fn save_slot(base: &Path, app: &str, slot: usize, text: &str) -> Result<(), String> {
    write_atomic(&slot_path(base, app, slot), text)
}

/// Remove the recovery file (the project was saved under a name, or the person does not want it).
pub fn clear(base: &Path, app: &str) {
    clear_slot(base, app, 0);
}

/// Remove the recovery file of tab `slot`.
pub fn clear_slot(base: &Path, app: &str, slot: usize) {
    let _ = std::fs::remove_file(slot_path(base, app, slot));
}

/// A recovery file that was found.
#[derive(Clone, Debug, PartialEq)]
pub struct Found {
    pub path: PathBuf,
    pub shapes: usize,
    pub slot: usize,
}

/// Look for the recovery file of slot 0. One that is empty or cannot be read is removed (there is nothing to recover).
pub fn find(base: &Path, app: &str) -> Option<Found> {
    find_slot(base, app, 0)
}

/// Every recovery file there is, the tabs that were left unsaved by an earlier run, in the order of their slots.
pub fn find_all(base: &Path, app: &str) -> Vec<Found> {
    (0..SLOTS).filter_map(|slot| find_slot(base, app, slot)).collect()
}

/// Look for the recovery file of tab `slot`.
pub fn find_slot(base: &Path, app: &str, slot: usize) -> Option<Found> {
    let path = slot_path(base, app, slot);
    let text = std::fs::read_to_string(&path).ok()?;
    match Project::from_json(&text) {
        Ok(p) if !p.scene.shapes.is_empty() => Some(Found { path, shapes: p.scene.shapes.len(), slot }),
        _ => {
            // Keep a file that is damaged next to the others, so it is not lost, but do not offer it again.
            let _ = std::fs::rename(&path, path.with_extension("damaged"));
            None
        }
    }
}

/// The auto-save of the program options, the same in every program: on or off, and how often (seconds).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub on: bool,
    pub secs: u32,
}

impl Options {
    pub const DEFAULT_SECS: u32 = 120;
    pub const MIN_SECS: u32 = 10;
    pub const MAX_SECS: u32 = 3600;

    /// How long between two saves.
    pub fn every(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.secs.clamp(Self::MIN_SECS, Self::MAX_SECS) as u64)
    }

    /// From the settings of a program (`autosave_on`, `autosave_secs`), with the defaults for what is missing or wrong.
    pub fn from_settings(get: impl Fn(&str) -> Option<String>) -> Options {
        Options {
            on: get("autosave_on").map(|v| v != "0").unwrap_or(true),
            secs: get("autosave_secs").and_then(|v| v.parse().ok()).filter(|v| (Self::MIN_SECS..=Self::MAX_SECS).contains(v)).unwrap_or(Self::DEFAULT_SECS),
        }
    }

    /// The settings to keep.
    pub fn settings(&self) -> [(&'static str, String); 2] {
        [("autosave_on", if self.on { "1" } else { "0" }.to_string()), ("autosave_secs", self.secs.to_string())]
    }
}

impl Default for Options {
    fn default() -> Self {
        Options { on: true, secs: Self::DEFAULT_SECS }
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
        self.due_every(rev, INTERVAL)
    }

    /// The same with the interval of the program options.
    pub fn due_every(&self, rev: u64, every: std::time::Duration) -> bool {
        self.saved_rev != Some(rev) && self.last.elapsed() >= every
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

    #[test]
    fn the_options_are_read_from_the_settings_with_defaults() {
        let o = Options::from_settings(|k| match k {
            "autosave_on" => Some("0".into()),
            "autosave_secs" => Some("45".into()),
            _ => None,
        });
        assert_eq!(o, Options { on: false, secs: 45 });
        assert_eq!(Options::from_settings(|_| None), Options::default());
        assert_eq!(Options::from_settings(|k| (k == "autosave_secs").then(|| "5".into())).secs, Options::DEFAULT_SECS, "too short");
        let back = Options::from_settings(|k| o.settings().iter().find(|(n, _)| *n == k).map(|(_, v)| v.clone()));
        assert_eq!(back, o);
        assert_eq!(o.every(), std::time::Duration::from_secs(45));
        let mut t = Timer::default();
        t.last = std::time::Instant::now() - std::time::Duration::from_secs(50);
        assert!(t.due_every(1, o.every()) && !t.due_every(1, std::time::Duration::from_secs(60)));
    }

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
    fn every_tab_has_its_own_recovery_file() {
        let base = std::env::temp_dir().join(format!("bc-autosave-slots-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        save_slot(&base, "a", 0, &project_with(1)).unwrap();
        save_slot(&base, "a", 3, &project_with(4)).unwrap();
        assert_eq!(slot_path(&base, "a", 0), recovery_path(&base, "a"), "slot 0 is the file there always was");
        let all = find_all(&base, "a");
        assert_eq!(all.iter().map(|f| (f.slot, f.shapes)).collect::<Vec<_>>(), vec![(0, 1), (3, 4)]);
        clear_slot(&base, "a", 0);
        assert_eq!(find_all(&base, "a").len(), 1);
        assert!(find(&base, "a").is_none(), "find looks at slot 0 only");
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
