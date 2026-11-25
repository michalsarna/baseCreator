//! The name of the person who uses the programs, recorded in the projects they save (see `base_core::authorship`), and the window
//! File → Project information. The name is set in Program options and kept in one file of the shared folder (`author.json`), so it
//! is the same in every program.
use crate::i18n::{tr, trf};
use crate::theme;
use base_core::authorship::{self, Authorship, Entry, Signer};
use egui::RichText;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The file of the name, in the shared folder.
pub const FILE: &str = "author.json";

/// The file and the name read from it. Nothing until a program calls `init` (the tests of the programs do not).
static STATE: Mutex<Option<(PathBuf, String)>> = Mutex::new(None);

#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Stored {
    #[serde(default)]
    name: String,
}

/// Read the name from the shared folder `base`. Called once when the program starts.
pub fn init(base: &Path) {
    let file = base.join(FILE);
    let name = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Stored>(&t).ok()).map(|s| s.name).unwrap_or_default();
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some((file, name));
}

/// The name set in Program options; empty when none is.
pub fn name() -> String {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(_, n)| n.clone()).unwrap_or_default()
}

/// Set the name and write it to the shared folder (where the program said with `init`).
pub fn set_name(name: &str) -> Result<(), String> {
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let Some((file, kept)) = st.as_mut() else {
        return Ok(());
    };
    *kept = name.to_string();
    let text = serde_json::to_string_pretty(&Stored { name: name.trim().to_string() }).map_err(|e| e.to_string())?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, text).map_err(|e| e.to_string())
}

/// A save of the project by the person of this computer, now, with `program` and its `version` (`v0.9.10`). `auto` is an
/// automatic save. Called on the authorship the program carries for the project, before the file is made of it.
pub fn stamp(a: &mut Authorship, program: &str, version: &str, auto: bool) {
    let name = name();
    a.stamp(Signer { name: &name, program, version }, authorship::now_secs(), auto);
}

/// The rows of Program options (a grid of two columns): the name. True when it was changed.
pub fn options_rows(ui: &mut egui::Ui) -> bool {
    let id = egui::Id::unique("authorship_name_edit");
    let mut text = ui.data_mut(|d| d.get_temp::<String>(id)).unwrap_or_else(name);
    ui.label(tr("Your name"));
    let r = ui.add(egui::TextEdit::singleline(&mut text).desired_width(220.0).char_limit(120)).on_hover_text(tr("Recorded in the projects you save, as their creator or a contributor. The same in every program."));
    ui.end_row();
    let changed = r.changed();
    if changed {
        let _ = set_name(&text);
    }
    ui.data_mut(|d| d.insert_temp(id, text));
    changed
}

/// What the window shows: the file of the project (`None` while it has no name) and who made it.
pub struct Info<'a> {
    pub path: Option<&'a Path>,
    pub authorship: &'a Authorship,
}

fn size_text(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn time_text(t: std::io::Result<std::time::SystemTime>) -> Option<String> {
    let secs = t.ok()?.duration_since(std::time::UNIX_EPOCH).ok()?.as_secs();
    Some(authorship::shown(&authorship::iso(secs)))
}

fn who(e: &Entry) -> String {
    if e.name.trim().is_empty() {
        tr("(no name)").to_string()
    } else {
        e.name.clone()
    }
}

fn program(e: &Entry) -> String {
    format!("{} {}", e.program, e.version).trim().to_string()
}

/// The lines of the window, as (label, value) pairs; the edits come apart, newest first. Separate from the drawing to be tested.
pub fn lines(info: &Info) -> (Vec<(&'static str, String)>, Vec<String>) {
    let mut rows = Vec::new();
    match info.path {
        None => rows.push((tr("File"), tr("Not saved yet").to_string())),
        Some(p) => {
            rows.push((tr("File"), p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
            rows.push((tr("Folder"), p.parent().map(|d| d.display().to_string()).unwrap_or_default()));
            let meta = std::fs::metadata(p).ok();
            if let Some(m) = &meta {
                rows.push((tr("Size"), size_text(m.len())));
                // Some file systems do not keep the time a file was made.
                if let Some(t) = time_text(m.created()) {
                    rows.push((tr("File created"), t));
                }
                if let Some(t) = time_text(m.modified()) {
                    rows.push((tr("File changed"), t));
                }
            }
        }
    }
    let a = info.authorship;
    if let Some(c) = &a.creator {
        rows.push((tr("Project created"), format!("{} · {}", authorship::shown(&c.at), program(c))));
        rows.push((tr("Creator"), who(c)));
    }
    if a.contributors.is_empty() {
        rows.push((tr("Contributors"), tr("Nobody yet").to_string()));
    } else {
        for (i, c) in a.contributors.iter().enumerate() {
            rows.push((if i == 0 { tr("Contributors") } else { "" }, format!("{} · {} · {}", who(c), authorship::shown(&c.at), program(c))));
        }
    }
    let edits = a.edits.iter().rev().map(|e| format!("{} · {} · {}", authorship::shown(&e.at), who(e), program(e))).collect();
    (rows, edits)
}

/// The window File → Project information.
pub fn window(ctx: &egui::Context, open: &mut bool, info: &Info) {
    if !*open {
        return;
    }
    let (rows, edits) = lines(info);
    crate::window::popup(ctx, "project_information", tr("Project information"), [480.0, 420.0], open, |ui| {
        egui::Grid::new("project_information_grid").num_columns(2).spacing([16.0, 6.0]).striped(true).show(ui, |ui| {
            for (label, value) in &rows {
                ui.label(RichText::new(*label).color(theme::text_dim()));
                ui.add(egui::Label::new(value.as_str()).wrap());
                ui.end_row();
            }
        });
        if name().trim().is_empty() {
            ui.add_space(6.0);
            ui.label(RichText::new(tr("Set your name in Settings → Program options to be recorded in the projects you save.")).color(theme::text_dim()).small());
        }
        ui.add_space(8.0);
        ui.label(RichText::new(trf("Edits ({})", &[&edits.len()])).strong());
        egui::ScrollArea::vertical().id_salt("project_information_edits").max_height(180.0).auto_shrink([false, true]).show(ui, |ui| {
            if edits.is_empty() {
                ui.label(RichText::new(tr("Nobody yet")).color(theme::text_dim()));
            }
            for e in &edits {
                ui.label(e);
            }
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_lists_the_file_the_creator_the_contributors_and_the_edits() {
        let dir = std::env::temp_dir().join(format!("authorship_ui_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("box.cre");
        std::fs::write(&file, "x".repeat(2048)).unwrap();
        let mut a = Authorship::default();
        a.stamp(Signer { name: "Anna", program: "LightCreator", version: "v0.9.9" }, 1_795_000_000, false);
        a.stamp(Signer { name: "Jan", program: "MillCreator", version: "v0.9.10" }, 1_795_000_600, false);
        let (rows, edits) = lines(&Info { path: Some(&file), authorship: &a });
        let get = |k: &str| rows.iter().find(|(l, _)| *l == k).map(|(_, v)| v.clone());
        assert_eq!(get("File").as_deref(), Some("box.cre"));
        assert_eq!(get("Size").as_deref(), Some("2.0 KB"));
        assert!(get("File changed").is_some());
        assert_eq!(get("Creator").as_deref(), Some("Anna"));
        assert!(get("Project created").unwrap().contains("LightCreator v0.9.9"));
        assert!(get("Contributors").unwrap().starts_with("Jan · "));
        assert_eq!(edits.len(), 2);
        assert!(edits[0].contains("Jan"), "newest first");
        let (rows, edits) = lines(&Info { path: None, authorship: &Authorship::default() });
        assert_eq!(rows[0].1, "Not saved yet");
        assert!(edits.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
        // The window draws.
        let ctx = egui::Context::default();
        let mut open = true;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), &mut open, &Info { path: Some(&file), authorship: &a }));
        out.textures_delta.clear();
    }

    #[test]
    fn the_name_is_kept_in_the_shared_folder() {
        let dir = std::env::temp_dir().join(format!("authorship_name_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(&dir);
        assert_eq!(name(), "");
        set_name("Anna Nowak").unwrap();
        init(&dir);
        assert_eq!(name(), "Anna Nowak");
        let mut a = Authorship::default();
        stamp(&mut a, "BaseCreator", "v2.7.8", false);
        assert_eq!(a.creator.unwrap().name, "Anna Nowak");
        *STATE.lock().unwrap() = None;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
