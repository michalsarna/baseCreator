//! Password protection of a project, the parts every program shares: the question for the password when a protected file is
//! opened, the *Protection* part of Project setup, and the guard that keeps the drawing as it was for a person who opened the file
//! with the open password (`base_core::protect` does the cryptography).
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::popup;
use base_core::protect::{self, Access, Protection};
use base_core::scene::{Scene, Shape};
use egui::RichText;
use std::path::PathBuf;

const RED: egui::Color32 = crate::window::DELETE_RED;

/// The OK (green) and Cancel (red) buttons of a password question, half as big again as plain ones. Returns (ok, cancel).
fn ok_cancel(ui: &mut egui::Ui) -> (bool, bool) {
    let mut out = (false, false);
    ui.horizontal(|ui| {
        let size = egui::vec2(96.0, 30.0);
        let button = |text: &str, fill| egui::Button::new(RichText::new(text).size(16.0).color(egui::Color32::WHITE)).fill(fill).min_size(size);
        out.0 = ui.add(button(&tr("OK"), crate::window::SAVE_GREEN)).clicked();
        out.1 = ui.add(button(&tr("Cancel"), RED)).clicked();
    });
    out
}

/// The body of a window that asks for a password: the question, the field (Enter is OK), what went wrong, OK and Cancel.
fn ask_password(ui: &mut egui::Ui, question: &str, password: &mut String, error: Option<&str>) -> (bool, bool) {
    ui.label(question);
    ui.add_space(4.0);
    let r = ui.add(egui::TextEdit::singleline(password).password(true).desired_width(f32::INFINITY).hint_text(tr("Password")));
    if !r.has_focus() && error.is_none() && password.is_empty() {
        r.request_focus();
    }
    let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
    if let Some(e) = error {
        ui.colored_label(RED, e);
    }
    ui.add_space(8.0);
    let (ok, cancel) = ok_cancel(ui);
    (ok || enter, cancel)
}

/// A protected file waiting for its password.
pub struct Prompt {
    pub path: PathBuf,
    text: String,
    password: String,
    error: Option<String>,
    /// What the program wants to do with the file when it opens (open it in a tab, restore a tab, recover …), kept for it.
    pub purpose: u32,
}

impl Prompt {
    pub fn new(path: PathBuf, text: String, purpose: u32) -> Prompt {
        Prompt { path, text, password: String::new(), error: None, purpose }
    }
}

/// What the question for a password ended with.
pub enum Unlocked {
    /// The project's JSON and the protection to save it with (its `access` says what the password allows).
    Opened { path: PathBuf, json: String, protection: Protection, purpose: u32 },
    /// The person gave up: the file stays closed.
    Cancelled { path: PathBuf, purpose: u32 },
}

/// Ask for the password of the first waiting file. Returns when it was opened or given up.
pub fn prompt_window(ctx: &egui::Context, waiting: &mut Vec<Prompt>) -> Option<Unlocked> {
    let p = waiting.first_mut()?;
    let mut open = true;
    let (mut ok, mut cancel) = (false, false);
    let name = p.path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    popup(ctx, "protect_prompt", &tr("Protection"), [440.0, 185.0], &mut open, |ui| {
        (ok, cancel) = ask_password(ui, &trf("{} is protected. Enter its password:", &[&name]), &mut p.password, p.error.as_deref());
    });
    if cancel || !open {
        let p = waiting.remove(0);
        return Some(Unlocked::Cancelled { path: p.path, purpose: p.purpose });
    }
    if ok {
        match protect::open(&p.text, &p.password) {
            Ok((json, protection)) => {
                let p = waiting.remove(0);
                return Some(Unlocked::Opened { path: p.path, json, protection, purpose: p.purpose });
            }
            Err(protect::OpenError::WrongPassword) => {
                p.error = Some(tr("Wrong password.").to_string());
                p.password.clear();
            }
            // A damaged file: say why; Cancel gives up on it.
            Err(e) => p.error = Some(e.to_string()),
        }
    }
    None
}

/// The fields of the *Protection* part of Project setup.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Fields {
    /// What the person who opened the project may do: `None` when it is not protected.
    pub access: Option<Access>,
    /// The project has an open password too.
    pub has_open: bool,
    pub owner: String,
    pub owner2: String,
    pub open: String,
    pub open2: String,
    pub remove: bool,
    pub error: Option<String>,
    /// The owner password typed to unlock a project opened with the open password, and the click on Unlock (see [`setup_unlock`]).
    pub unlock: String,
    pub try_unlock: bool,
}

/// What Apply asks for.
#[derive(Debug)]
pub enum Change {
    /// Leave the protection as it is.
    Keep,
    /// Protect with these passwords (new ones, or the first).
    Set { owner: String, open: Option<String> },
    /// Save the project without a password again.
    Remove,
}

impl Fields {
    /// The fields for a project with this protection (or none). The passwords themselves are never shown.
    pub fn of(p: Option<&Protection>) -> Fields {
        Fields { access: p.map(|p| p.access), has_open: p.is_some_and(|p| p.has_open_password()), ..Default::default() }
    }

    /// Check what was typed; the message to show when it cannot be applied.
    pub fn check(&mut self) -> bool {
        self.error = None;
        if self.access == Some(Access::Open) || self.remove || (self.owner.is_empty() && self.open.is_empty()) {
            return true;
        }
        let err = if self.owner != self.owner2 || self.open != self.open2 {
            Some(tr("The passwords do not match.").to_string())
        } else if protect::check_passwords(&self.owner, Some(&self.open)).is_err() {
            Some(trf("At least {} characters; the open password must differ from the owner password.", &[&protect::MIN_PASSWORD]))
        } else {
            None
        };
        self.error = err;
        self.error.is_none()
    }

    /// What Apply should do (after `check` passed). The typed passwords are cleared.
    pub fn take_change(&mut self) -> Change {
        let ch = if self.access == Some(Access::Open) {
            Change::Keep
        } else if self.remove {
            if self.access.is_some() {
                Change::Remove
            } else {
                Change::Keep
            }
        } else if !self.owner.is_empty() {
            Change::Set { owner: self.owner.clone(), open: (!self.open.is_empty()).then(|| self.open.clone()) }
        } else {
            Change::Keep
        };
        use zeroize::Zeroize;
        for s in [&mut self.owner, &mut self.owner2, &mut self.open, &mut self.open2, &mut self.unlock] {
            s.zeroize();
        }
        ch
    }
}

/// Apply a change to the protection of a project (slow when new passwords are set: a key is made for each).
pub fn apply(change: Change, current: &mut Option<Protection>) -> Result<(), String> {
    match change {
        Change::Keep => Ok(()),
        Change::Remove => {
            *current = None;
            Ok(())
        }
        Change::Set { owner, open } => {
            *current = Some(Protection::new(&owner, open.as_deref())?);
            Ok(())
        }
    }
}

/// The *Protection* part of Project setup: a line, the title, what the passwords allow and the fields.
pub fn part(ui: &mut egui::Ui, f: &mut Fields) {
    crate::widgets::options_line(ui);
    ui.label(RichText::new(tr("Protection")).strong());
    if f.access == Some(Access::Open) {
        ui.label(tr("Opened with the open password: editing and copying are off."));
        ui.add_space(4.0);
        crate::widgets::options_grid(ui, "protect_unlock_grid", |ui| {
            ui.label(tr("Owner password"));
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut f.unlock).password(true).desired_width(220.0));
                let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let click = ui.add_enabled(!f.unlock.is_empty(), crate::widgets::apply_widget(None, &tr("Unlock"))).clicked();
                f.try_unlock |= (enter || click) && !f.unlock.is_empty();
            });
            ui.end_row();
        });
        if let Some(e) = &f.error {
            ui.colored_label(RED, e);
        }
        return;
    }
    ui.label(RichText::new(tr("Owner: may do everything. Open: may open, look, make G-code, run the job and change the material, but not edit or copy the drawing. Without a password the file cannot be read, and a forgotten password cannot be recovered.")).small().color(theme::text_dim()));
    if f.access == Some(Access::Owner) {
        ui.label(RichText::new(tr("The project is protected. Leave the fields empty to keep its passwords.")).small());
    }
    ui.add_space(4.0);
    let pw = |ui: &mut egui::Ui, s: &mut String| {
        ui.add(egui::TextEdit::singleline(s).password(true).desired_width(220.0));
    };
    // The open password goes with an owner password: without one its fields take nothing.
    let no_owner = f.owner.is_empty();
    if no_owner {
        f.open.clear();
        f.open2.clear();
    }
    ui.add_enabled_ui(!f.remove, |ui| {
        let mut line_y = None;
        let grid = crate::widgets::options_grid(ui, "protect_grid", |ui| {
            ui.label(tr("Owner password"));
            pw(ui, &mut f.owner);
            ui.end_row();
            ui.label(tr("Repeat the password"));
            pw(ui, &mut f.owner2);
            ui.end_row();
            // A line between the owner's passwords and the open ones (drawn over the whole grid below).
            line_y = Some(ui.cursor().top());
            ui.add_enabled(!no_owner, egui::Label::new(tr("Open password (optional)")));
            ui.add_enabled_ui(!no_owner, |ui| pw(ui, &mut f.open));
            ui.end_row();
            ui.add_enabled(!no_owner, egui::Label::new(tr("Repeat the password")));
            ui.add_enabled_ui(!no_owner, |ui| pw(ui, &mut f.open2));
            ui.end_row();
            ui.min_rect()
        });
        if let Some(y) = line_y {
            let y = y - 4.0;
            ui.painter().hline(grid.left()..=grid.right(), y, ui.visuals().widgets.noninteractive.bg_stroke);
        }
    });
    if f.access.is_some() {
        ui.checkbox(&mut f.remove, tr("Remove the protection"));
    }
    if let Some(e) = &f.error {
        ui.colored_label(RED, e);
    }
}

/// The owner password typed in the Protection part (Unlock): when it is right the project is the owner's from now on. True when it
/// was unlocked (the program turns editing on again: `EditorState::locked`).
pub fn setup_unlock(f: &mut Fields, lock: &mut DocLock) -> bool {
    if !std::mem::take(&mut f.try_unlock) {
        return false;
    }
    let pw = std::mem::take(&mut f.unlock);
    let r = lock.unlock_owner(&pw);
    drop(zeroize::Zeroizing::new(pw));
    match r {
        Ok(()) => {
            *f = lock.fields();
            true
        }
        Err(e) => {
            f.error = Some(e);
            false
        }
    }
}

/// The message when the owner password unlocked a project.
pub fn unlocked_message() -> String {
    tr("Unlocked with the owner password: everything is allowed.").to_string()
}

/// Keeps the drawing of a project opened with the open password as it was: any change of its shapes is put back.
#[derive(Default)]
pub struct Guard {
    shapes: Option<Vec<Shape>>,
}

impl Guard {
    /// Start guarding (the shapes as they are now), or stop when `on` is false.
    pub fn off(&mut self) {
        self.shapes = None;
    }

    pub fn set(&mut self, scene: &Scene, on: bool) {
        self.shapes = on.then(|| scene.shapes.clone());
    }
    pub fn on(&self) -> bool {
        self.shapes.is_some()
    }
    /// Put the shapes back when they were changed. Returns true when it did (tell the person why).
    pub fn check(&self, scene: &mut Scene) -> bool {
        match &self.shapes {
            Some(kept) if scene.shapes != *kept => {
                scene.shapes = kept.clone();
                true
            }
            _ => false,
        }
    }
}

/// The protection of one open project (one tab): the key to save it with and, for a project opened with the open password, the
/// guard of its drawing.
#[derive(Default)]
pub struct DocLock {
    pub protection: Option<Protection>,
    guard: Guard,
    /// The question for the owner password (from the menu of the tab), while it is open: the typed password and what went wrong.
    asking: Option<(String, Option<String>)>,
}

impl DocLock {
    /// The protection of a project just opened (or `None`), its drawing guarded when the open password opened it.
    pub fn new(protection: Option<Protection>, scene: &Scene) -> DocLock {
        let mut l = DocLock { protection, guard: Guard::default(), asking: None };
        l.guard.set(scene, l.locked());
        l
    }
    /// Opened with the open password: no editing, no copying.
    pub fn locked(&self) -> bool {
        self.protection.as_ref().is_some_and(|p| p.access == Access::Open)
    }
    pub fn protected(&self) -> bool {
        self.protection.is_some()
    }
    /// The text to write for the project's JSON: sealed when the project is protected.
    pub fn seal(&self, json: String) -> String {
        match &self.protection {
            Some(p) => p.seal(&json),
            None => json,
        }
    }
    /// Put back a guarded drawing that was changed; true when it did.
    pub fn check(&self, scene: &mut Scene) -> bool {
        self.guard.check(scene)
    }
    /// Another drawing of the same project is shown now (the other side or face of a job worked on from several sides): guard
    /// that one from here on.
    pub fn rearm(&mut self, scene: &Scene) {
        if self.locked() {
            self.guard.set(scene, true);
        }
    }
    /// The owner password for a project opened with the open password: right, and everything is allowed from now on (the guard
    /// stops). The message to show when it is not.
    pub fn unlock_owner(&mut self, password: &str) -> Result<(), String> {
        let Some(p) = self.protection.as_mut().filter(|p| p.access == Access::Open) else { return Ok(()) };
        match p.unlock_owner(password) {
            Ok(()) => {
                self.guard.off();
                Ok(())
            }
            Err(protect::OpenError::WrongPassword) => Err(tr("Wrong password.").to_string()),
            Err(e) => Err(e.to_string()),
        }
    }

    /// Ask for the owner password (*Unlock with the owner password…* in the menu of the tab).
    pub fn ask_owner(&mut self) {
        if self.locked() {
            self.asking = Some((String::new(), None));
        }
    }

    /// The window that asks for the owner password, while it is asked for. True when the project was unlocked by it (the program
    /// turns editing on again and says so).
    pub fn owner_window(&mut self, ctx: &egui::Context, name: &str) -> bool {
        let Some((mut pw, err)) = self.asking.take() else { return false };
        let mut open = true;
        let (mut ok, mut cancel) = (false, false);
        popup(ctx, "protect_owner", &tr("Protection"), [440.0, 185.0], &mut open, |ui| {
            (ok, cancel) = ask_password(ui, &trf("Enter the owner password of {}:", &[&name]), &mut pw, err.as_deref());
        });
        if cancel || !open || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            drop(zeroize::Zeroizing::new(pw));
            return false;
        }
        if !ok {
            self.asking = Some((pw, err));
            return false;
        }
        let r = self.unlock_owner(&pw);
        drop(zeroize::Zeroizing::new(pw));
        match r {
            Ok(()) => true,
            Err(e) => {
                self.asking = Some((String::new(), Some(e)));
                false
            }
        }
    }

    /// The fields of the Protection part for this project.
    pub fn fields(&self) -> Fields {
        Fields::of(self.protection.as_ref())
    }
}

/// The Protection part with its own Apply button, for a window that applies everything else at once. Returns true when the
/// protection was changed (the project needs saving).
pub fn part_with_apply(ui: &mut egui::Ui, f: &mut Fields, lock: &mut DocLock) -> bool {
    part(ui, f);
    if f.access == Some(Access::Open) {
        return false;
    }
    let mut changed = false;
    // Blue, so it stands out from the fields above it.
    let blue = egui::Button::new(RichText::new(tr("Apply")).color(egui::Color32::WHITE)).fill(crate::window::NEW_BLUE);
    if ui.add(blue).clicked() && f.check() {
        let ch = f.take_change();
        let real = !matches!(ch, Change::Keep);
        match apply(ch, &mut lock.protection) {
            Ok(()) => changed = real,
            Err(e) => f.error = Some(e),
        }
        let access = lock.protection.as_ref().map(|p| p.access);
        let has_open = lock.protection.as_ref().is_some_and(|p| p.has_open_password());
        f.access = access;
        f.has_open = has_open;
        f.remove = false;
    }
    changed
}

/// The message when something tried to change or copy a protected drawing.
pub fn locked_message() -> String {
    tr("Opened with the open password: editing and copying are off.").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::geom::Xf;
    use base_core::scene::Kind;

    /// A random password for the tests (no password is written into the code).
    fn pw() -> String {
        protect::random_password()
    }

    #[test]
    fn the_guard_puts_the_drawing_back() {
        let mut s = Scene::default();
        s.add(0, Kind::Rect { w: 10.0, h: 5.0 }, Xf::IDENTITY);
        let mut g = Guard::default();
        assert!(!g.check(&mut s), "off: nothing to do");
        g.set(&s, true);
        s.shapes[0].xf = Xf::translate(3.0, 0.0);
        s.add(0, Kind::Rect { w: 1.0, h: 1.0 }, Xf::IDENTITY);
        assert!(g.check(&mut s));
        assert_eq!(s.shapes.len(), 1);
        assert_eq!(s.shapes[0].xf, Xf::IDENTITY);
        assert!(!g.check(&mut s));
    }

    #[test]
    fn the_owner_password_unlocks_the_drawing_in_setup() {
        use base_core::protect::Kdf;
        let fast = Kdf { m_kib: 8 * 1024, t: 1, p: 1 };
        let (owner, open) = (pw(), pw());
        let p = Protection::with_kdf(&owner, Some(&open), fast).unwrap();
        let (_, opened) = protect::open(&p.seal("{}"), &open).unwrap();
        let mut s = Scene::default();
        s.add(0, Kind::Rect { w: 10.0, h: 5.0 }, Xf::IDENTITY);
        let mut lock = DocLock::new(Some(opened), &s);
        let mut f = lock.fields();
        assert!(!setup_unlock(&mut f, &mut lock), "nothing asked");
        f.unlock = open.clone();
        f.try_unlock = true;
        assert!(!setup_unlock(&mut f, &mut lock) && f.error.is_some() && lock.locked());
        f.unlock = owner.clone();
        f.try_unlock = true;
        assert!(setup_unlock(&mut f, &mut lock));
        assert!(!lock.locked() && f.access == Some(Access::Owner) && f.unlock.is_empty());
        s.shapes.clear();
        assert!(!lock.check(&mut s), "the guard is off");
    }

    #[test]
    fn the_fields_check_the_passwords() {
        let mut f = Fields::default();
        assert!(f.check(), "empty: nothing to do");
        assert!(matches!(f.take_change(), Change::Keep));
        let (owner, open) = (pw(), pw());
        f.owner = owner.clone();
        f.owner2 = owner[..owner.len() - 1].to_string();
        assert!(!f.check());
        f.owner2 = owner.clone();
        assert!(f.check());
        f.open = owner.clone();
        f.open2 = owner.clone();
        assert!(!f.check(), "the open password must differ");
        f.open = open.clone();
        f.open2 = open.clone();
        assert!(f.check());
        assert!(matches!(f.take_change(), Change::Set { owner: ref w, open: Some(ref o) } if *w == owner && *o == open));
        assert!(f.owner.is_empty(), "the typed passwords are cleared");
        let mut r = Fields { access: Some(Access::Owner), remove: true, ..Default::default() };
        assert!(r.check());
        assert!(matches!(r.take_change(), Change::Remove));
        let mut o = Fields { access: Some(Access::Open), owner: pw(), ..Default::default() };
        assert!(o.check());
        assert!(matches!(o.take_change(), Change::Keep), "the open password changes nothing");
    }
}
