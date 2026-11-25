//! The person who uses the programs (name, e-mail and a free text), recorded in the projects they save (see
//! `base_core::authorship`), and the window File → Project information. The person is set in the *Author* part of Program options
//! and kept in one file of the shared folder (`author.json`), so it is the same in every program.
use crate::i18n::{tr, trf};
use crate::theme;
use base_core::authorship::{self, Authorship, Entry, Signer};
use egui::RichText;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// The file of the person, in the shared folder.
pub const FILE: &str = "author.json";

/// Who saves the projects on this computer.
#[derive(Clone, Debug, Default, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Person {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub email: String,
    /// Anything else: a company, a website...
    #[serde(default)]
    pub other: String,
}

impl Person {
    fn trimmed(&self) -> Person {
        Person { name: self.name.trim().to_string(), email: self.email.trim().to_string(), other: self.other.trim().to_string() }
    }

    /// Nothing set: the projects get no person.
    pub fn is_empty(&self) -> bool {
        self.name.trim().is_empty() && self.email.trim().is_empty()
    }
}

/// The file and the person read from it. Nothing until a program calls `init` (the tests of the programs do not).
static STATE: Mutex<Option<(PathBuf, Person)>> = Mutex::new(None);

/// Read the person from the shared folder `base`. Called once when the program starts.
pub fn init(base: &Path) {
    let file = base.join(FILE);
    let person = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<Person>(&t).ok()).unwrap_or_default();
    *STATE.lock().unwrap_or_else(|e| e.into_inner()) = Some((file, person));
}

/// The person set in Program options; empty when none is.
pub fn person() -> Person {
    STATE.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(_, p)| p.clone()).unwrap_or_default()
}

/// The name set in Program options; empty when none is.
pub fn name() -> String {
    person().name
}

/// Set the person and write it to the shared folder (where the program said with `init`).
pub fn set_person(p: &Person) -> Result<(), String> {
    let mut st = STATE.lock().unwrap_or_else(|e| e.into_inner());
    let Some((file, kept)) = st.as_mut() else {
        return Ok(());
    };
    *kept = p.clone();
    let text = serde_json::to_string_pretty(&p.trimmed()).map_err(|e| e.to_string())?;
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, text).map_err(|e| e.to_string())
}

/// Set only the name (the e-mail and the rest stay).
pub fn set_name(name: &str) -> Result<(), String> {
    set_person(&Person { name: name.to_string(), ..person() })
}

/// A save of the project by the person of this computer, now, with `program` and its `version` (`v0.9.10`). `auto` is an
/// automatic save (it adds no edit). Called on the authorship the program carries for the project, before the file is made of it.
pub fn stamp(a: &mut Authorship, program: &str, version: &str, auto: bool) {
    let p = person();
    a.stamp(Signer { name: &p.name, email: &p.email, other: &p.other, program, version }, authorship::now_secs(), auto);
}

/// The *Author* part of Program options, under the grid of the other options: a line, a title and the name, the e-mail and the
/// rest. True when something was changed.
pub fn options_section(ui: &mut egui::Ui) -> bool {
    let id = egui::Id::unique("authorship_person_edit");
    let mut p = ui.data_mut(|d| d.get_temp::<Person>(id)).unwrap_or_else(person);
    ui.add_space(4.0);
    ui.separator();
    ui.label(RichText::new(tr("Author")).strong()).on_hover_text(tr("Recorded in the projects you save, as their creator or a contributor. The same in every program."));
    ui.add_space(2.0);
    let mut changed = false;
    egui::Grid::new("authorship_options_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
        for (label, text, hint) in [(tr("Your name"), &mut p.name, ""), (tr("Email"), &mut p.email, "name@example.com"), (tr("Other"), &mut p.other, tr("A company, a website or anything else to keep with your name"))] {
            ui.label(label);
            changed |= ui.add(egui::TextEdit::singleline(text).desired_width(260.0).char_limit(200).hint_text(hint)).changed();
            ui.end_row();
        }
    });
    if changed {
        let _ = set_person(&p);
    }
    ui.data_mut(|d| d.insert_temp(id, p));
    changed
}

/// What the window shows: the file of the project (`None` while it has no name) and who made it.
pub struct Info<'a> {
    pub path: Option<&'a Path>,
    pub authorship: &'a Authorship,
}

/// The window's content as text, separate from the drawing so that it can be tested.
pub struct View {
    /// (label, value) of the file.
    pub file: Vec<(&'static str, String)>,
    /// (label, value, more) of the authors: `more` is the e-mail and the rest, shown smaller under the value.
    pub authors: Vec<(&'static str, String, String)>,
    /// The edits, newest first: time, person, program.
    pub edits: Vec<[String; 3]>,
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

/// The name, or the e-mail when there is no name.
fn who(e: &Entry) -> String {
    if !e.name.trim().is_empty() {
        e.name.clone()
    } else if !e.email.trim().is_empty() {
        e.email.clone()
    } else {
        tr("(no name)").to_string()
    }
}

/// The e-mail (when the name is shown) and the rest.
fn more(e: &Entry) -> String {
    let email = if e.name.trim().is_empty() { "" } else { e.email.trim() };
    [email, e.other.trim()].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ")
}

fn program(e: &Entry) -> String {
    format!("{} {}", e.program, e.version).trim().to_string()
}

pub fn view(info: &Info) -> View {
    let mut file = Vec::new();
    match info.path {
        None => file.push((tr("Name"), tr("Not saved yet").to_string())),
        Some(p) => {
            file.push((tr("Name"), p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
            file.push((tr("Folder"), p.parent().map(|d| d.display().to_string()).unwrap_or_default()));
            if let Ok(m) = std::fs::metadata(p) {
                file.push((tr("Size"), size_text(m.len())));
                // Some file systems do not keep the time a file was made.
                if let Some(t) = time_text(m.created()) {
                    file.push((tr("File created"), t));
                }
                if let Some(t) = time_text(m.modified()) {
                    file.push((tr("File changed"), t));
                }
            }
        }
    }
    let a = info.authorship;
    let mut authors = Vec::new();
    if let Some(c) = &a.creator {
        authors.push((tr("Project created"), authorship::shown(&c.at), program(c)));
        authors.push((tr("Creator"), who(c), more(c)));
    }
    if a.contributors.is_empty() {
        authors.push((tr("Contributors"), tr("Nobody yet").to_string(), String::new()));
    }
    for (i, c) in a.contributors.iter().enumerate() {
        let m = [more(c), format!("{} · {}", authorship::shown(&c.at), program(c))].into_iter().filter(|s| !s.is_empty()).collect::<Vec<_>>().join(" · ");
        authors.push((if i == 0 { tr("Contributors") } else { "" }, who(c), m));
    }
    let edits = a.edits.iter().rev().map(|e| [authorship::shown(&e.at), who(e), program(e)]).collect();
    View { file, authors, edits }
}

/// A title of a part of the window, with a line over it (but the first).
fn part(ui: &mut egui::Ui, title: &str, first: bool) {
    if !first {
        ui.add_space(6.0);
        ui.separator();
    }
    ui.label(RichText::new(title).strong());
    ui.add_space(2.0);
}

/// Label and value in two columns, the value wrapped.
fn pairs(ui: &mut egui::Ui, id: &str, rows: impl Iterator<Item = (&'static str, String, String)>) {
    egui::Grid::new(id).num_columns(2).min_col_width(130.0).spacing([16.0, 6.0]).show(ui, |ui| {
        for (label, value, more) in rows {
            ui.label(RichText::new(label).color(theme::text_dim()));
            ui.vertical(|ui| {
                ui.add(egui::Label::new(value).wrap());
                if !more.is_empty() {
                    ui.add(egui::Label::new(RichText::new(more).small().color(theme::text_dim())).wrap());
                }
            });
            ui.end_row();
        }
    });
}

/// The window File → Project information.
pub fn window(ctx: &egui::Context, open: &mut bool, info: &Info) {
    if !*open {
        return;
    }
    let v = view(info);
    crate::window::popup(ctx, "project_information_2", tr("Project information"), [560.0, 560.0], open, |ui| {
        part(ui, tr("File"), true);
        pairs(ui, "project_information_file", v.file.iter().map(|(l, v)| (*l, v.clone(), String::new())));
        part(ui, tr("Authors"), false);
        pairs(ui, "project_information_authors", v.authors.iter().cloned());
        if person().is_empty() {
            ui.add_space(4.0);
            ui.add(egui::Label::new(RichText::new(tr("Set your name in Settings → Program options to be recorded in the projects you save.")).small().color(theme::text_dim())).wrap());
        }
        part(ui, &trf("Edits ({})", &[&v.edits.len()]), false);
        if v.edits.is_empty() {
            ui.label(RichText::new(tr("Nobody yet")).color(theme::text_dim()));
            return;
        }
        egui::ScrollArea::vertical().id_salt("project_information_edits").auto_shrink([false, false]).show(ui, |ui| {
            let w = ui.available_width();
            egui::Grid::new("project_information_edits_grid").num_columns(3).striped(true).min_col_width((w / 3.0 - 12.0).max(80.0)).spacing([12.0, 4.0]).show(ui, |ui| {
                for h in [tr("Time"), tr("Person"), tr("Program")] {
                    ui.label(RichText::new(h).color(theme::text_dim()));
                }
                ui.end_row();
                for row in &v.edits {
                    for cell in row {
                        ui.label(cell);
                    }
                    ui.end_row();
                }
            });
        });
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_lists_the_file_the_authors_and_the_edits() {
        let dir = std::env::temp_dir().join(format!("authorship_ui_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("box.cre");
        std::fs::write(&file, "x".repeat(2048)).unwrap();
        let mut a = Authorship::default();
        a.stamp(Signer { name: "Anna", email: "anna@example.org", other: "Studio A", program: "LightCreator", version: "v0.9.9" }, 1_795_000_000, false);
        a.stamp(Signer { name: "Jan", email: "", other: "", program: "MillCreator", version: "v0.9.10" }, 1_795_000_600, false);
        let v = view(&Info { path: Some(&file), authorship: &a });
        let get = |k: &str| v.file.iter().find(|(l, _)| *l == k).map(|(_, v)| v.clone());
        assert_eq!(get("Name").as_deref(), Some("box.cre"));
        assert_eq!(get("Size").as_deref(), Some("2.0 KB"));
        assert!(get("File changed").is_some());
        let who = |k: &str| v.authors.iter().find(|(l, ..)| *l == k).map(|(_, v, m)| (v.clone(), m.clone()));
        assert_eq!(who("Creator"), Some(("Anna".to_string(), "anna@example.org · Studio A".to_string())));
        assert!(who("Project created").unwrap().1.contains("LightCreator v0.9.9"));
        let (name, more) = who("Contributors").unwrap();
        assert_eq!(name, "Jan");
        assert!(more.ends_with("MillCreator v0.9.10"), "{more}");
        assert_eq!(v.edits.len(), 2);
        assert_eq!(v.edits[0][1], "Jan", "newest first");
        let empty = view(&Info { path: None, authorship: &Authorship::default() });
        assert_eq!(empty.file[0].1, "Not saved yet");
        assert!(empty.edits.is_empty());
        // The window draws.
        let ctx = egui::Context::default();
        let mut open = true;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| window(ui.ctx(), &mut open, &Info { path: Some(&file), authorship: &a }));
        out.textures_delta.clear();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_person_is_kept_in_the_shared_folder() {
        let dir = std::env::temp_dir().join(format!("authorship_name_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        init(&dir);
        assert!(person().is_empty());
        set_person(&Person { name: "Anna Nowak ".into(), email: "anna@example.org".into(), other: "Studio A".into() }).unwrap();
        init(&dir);
        assert_eq!(person(), Person { name: "Anna Nowak".into(), email: "anna@example.org".into(), other: "Studio A".into() });
        set_name("Anna").unwrap();
        assert_eq!(person().email, "anna@example.org", "setting the name keeps the rest");
        let mut a = Authorship::default();
        stamp(&mut a, "BaseCreator", "v2.7.9", false);
        let c = a.creator.unwrap();
        assert_eq!((c.name.as_str(), c.email.as_str(), c.other.as_str()), ("Anna", "anna@example.org", "Studio A"));
        // The section of Program options draws.
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            options_section(ui);
        });
        out.textures_delta.clear();
        *STATE.lock().unwrap() = None;
        let _ = std::fs::remove_dir_all(&dir);
    }
}
