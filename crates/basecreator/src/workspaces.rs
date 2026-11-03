//! Open drawings as tabs (the bar and the closing are in `base_ui::doc_tabs`): each tab holds a `Workspace`, the part of
//! the program that belongs to one drawing. The live fields of the program are the shown tab; the others are parked here
//! and exchanged with the live ones when a tab is picked.
use crate::section::{self as layers, DrawLayer, Page};
use crate::{App, History, Screen};
use base_core::project::Project;
use base_core::scene::Scene;
use base_ui::doc_tabs::{self, TabHost, MAX_TABS};
use base_ui::editor::DocView;
use eframe::egui;
use std::path::PathBuf;

const APP: &str = "basecreator";

/// What belongs to one drawing. `clean` and `autosaved_clean` stand for "the revision equals the saved one": the revisions
/// themselves are not kept, because the live revision is a new number every time a tab is shown.
pub struct Workspace {
    pub scene: Scene,
    pub history: History,
    pub path: Option<PathBuf>,
    pub description: String,
    pub kept: Project,
    pub layers: Vec<DrawLayer>,
    pub page: Page,
    pub clean: bool,
    pub autosaved_clean: bool,
    pub view: DocView,
    /// The number of the recovery file of this tab.
    pub slot: usize,
}

impl Workspace {
    fn empty(layers: Vec<DrawLayer>, page: Page, slot: usize) -> Workspace {
        Workspace {
            scene: Scene::default(),
            history: History::default(),
            path: None,
            description: String::new(),
            kept: Project::default(),
            layers,
            page,
            clean: true,
            autosaved_clean: true,
            view: DocView::default(),
            slot,
        }
    }

    /// Unsaved work, as `App::is_dirty` says it for the shown tab.
    pub fn dirty(&self) -> bool {
        !self.clean && (!self.scene.shapes.is_empty() || self.path.is_some())
    }

    pub fn to_project(&self) -> Project {
        let mut p = Project::new(self.scene.clone());
        p.description = self.description.clone();
        p.set_section(layers::SECTION, &layers::DrawSection { format: layers::SECTION_FORMAT, page: self.page, layers: self.layers.clone() });
        p.restore_sections(&self.kept.other_sections(layers::SECTION));
        p.restore_extra(self.kept.extra());
        base_ui::editor::background_section(&self.view.background, &self.view.background_pending, &mut p);
        p
    }
}

/// The parked tabs: `parked[i]` is the tab `i`, `None` for the shown one.
#[derive(Default)]
pub struct Tabs {
    pub parked: Vec<Option<Workspace>>,
    pub active: usize,
    /// The shown tab is being closed by its cross: the question is on the screen.
    pub closing: bool,
}

impl App {
    /// The shown tab with a parked one: a swap of every field that belongs to a drawing. The pattern has no `..`, so a field
    /// added to `Workspace` has to be handled here.
    fn swap_ws(&mut self, w: &mut Workspace) {
        let Workspace { scene, history, path, description, kept, layers, page, clean, autosaved_clean, view, slot } = w;
        std::mem::swap(&mut self.scene, scene);
        std::mem::swap(&mut self.history, history);
        std::mem::swap(&mut self.path, path);
        std::mem::swap(&mut self.description, description);
        std::mem::swap(&mut self.kept, kept);
        std::mem::swap(&mut self.layers, layers);
        std::mem::swap(&mut self.page, page);
        std::mem::swap(&mut self.slot, slot);
        self.ed.swap_doc_view(view);
        // The revisions: a new live number, and the saved ones either equal to it or not.
        let was_clean = self.rev == self.saved_rev;
        let was_autosaved_clean = !self.autosave.changed(self.rev);
        self.rev += 1;
        self.saved_rev = if *clean { self.rev } else { self.rev.wrapping_sub(1) };
        self.autosave = Default::default();
        if *autosaved_clean {
            self.autosave.in_sync(self.rev);
        }
        *clean = was_clean;
        *autosaved_clean = was_autosaved_clean;
    }

    pub fn tab_count(&self) -> usize {
        self.tabs_state.parked.len().max(1)
    }

    fn ensure_tabs(&mut self) {
        if self.tabs_state.parked.is_empty() {
            self.tabs_state.parked.push(None);
        }
    }

    /// Unsaved work in the shown tab: neither saved by hand nor written to its own file.
    pub fn is_dirty(&self) -> bool {
        self.rev != self.saved_rev && (!self.scene.shapes.is_empty() || self.path.is_some())
    }

    pub fn any_dirty(&self) -> bool {
        self.is_dirty() || self.tabs_state.parked.iter().flatten().any(|w| w.dirty())
    }

    /// The slots in use (and the ones of the recovery files that wait to be opened).
    fn used_slots(&self) -> Vec<usize> {
        let mut v: Vec<usize> = self.tabs_state.parked.iter().flatten().map(|w| w.slot).collect();
        v.push(self.slot);
        v.extend(self.recovery.iter().map(|f| f.slot));
        v
    }

    /// A new, empty tab, shown now (from the start screen a blank tab is used as it is). False (with a message) when ten tabs are open.
    pub fn new_tab(&mut self) -> bool {
        // From the start screen a blank tab is used as it is.
        let reuse = self.screen == Screen::Start && self.shown_tab_is_blank();
        if !reuse && !self.make_room() {
            return false;
        }
        self.new_design();
        true
    }

    /// Park the shown tab and show a fresh one (not yet initialised: the caller fills it). False when there is no room.
    fn make_room(&mut self) -> bool {
        self.ensure_tabs();
        if self.tabs_state.parked.len() >= MAX_TABS {
            self.status = doc_tabs::limit_message();
            return false;
        }
        let slot = doc_tabs::free_slot(&self.used_slots());
        let mut w = Workspace::empty(layers::default_layers(), self.new_page, slot);
        self.swap_ws(&mut w);
        let at = self.tabs_state.active;
        self.tabs_state.parked[at] = Some(w);
        self.tabs_state.parked.push(None);
        self.tabs_state.active = self.tabs_state.parked.len() - 1;
        true
    }

    /// Is the shown tab one that a file can simply replace: no name, nothing in it.
    fn shown_tab_is_blank(&self) -> bool {
        self.path.is_none() && self.scene.shapes.is_empty() && !self.is_dirty()
    }

    /// Before a file is put into the shown tab: a blank tab is used, otherwise a new one is made. False when the project is
    /// open in another tab already (that tab is shown) or there is no room. `project` is false for a picture-like drawing
    /// (SVG, PDF), which has no file of its own to compare.
    pub fn tab_for_file(&mut self, file: &std::path::Path, project: bool) -> bool {
        self.ensure_tabs();
        if project {
            let same = |p: &PathBuf| std::fs::canonicalize(p).ok() == std::fs::canonicalize(file).ok();
            if self.path.as_ref().is_some_and(same) {
                return false;
            }
            let other = self.tabs_state.parked.iter().position(|w| w.as_ref().is_some_and(|w| w.path.as_ref().is_some_and(same)));
            if let Some(i) = other {
                self.switch_to(i);
                return false;
            }
        }
        if self.shown_tab_is_blank() {
            return true;
        }
        self.make_room()
    }

    pub fn switch_to(&mut self, i: usize) {
        self.ensure_tabs();
        if i == self.tabs_state.active || i >= self.tabs_state.parked.len() {
            return;
        }
        let Some(mut w) = self.tabs_state.parked[i].take() else { return };
        self.swap_ws(&mut w);
        let at = self.tabs_state.active;
        self.tabs_state.parked[at] = Some(w);
        self.tabs_state.active = i;
    }

    /// Close the shown tab (its work is saved or given up by now). The last tab turns into a new, empty drawing and the
    /// start screen is shown.
    pub fn close_shown_tab(&mut self) {
        self.ensure_tabs();
        base_core::autosave::clear_slot(&crate::data_paths().base, APP, self.slot);
        let n = self.tabs_state.parked.len();
        if n <= 1 {
            // The last tab: what is in it is saved or given up by now. The drawing is cleared (so the exit saves nothing)
            // and the program closes.
            self.new_design();
            self.close_program = true;
            return;
        }
        let at = self.tabs_state.active;
        let next = if at + 1 < n { at + 1 } else { at - 1 };
        let Some(mut w) = self.tabs_state.parked[next].take() else { return };
        self.swap_ws(&mut w);
        // `w` now holds the closed tab.
        self.tabs_state.parked.remove(at);
        self.tabs_state.active = if next > at { next - 1 } else { next };
    }

    /// Close the shown tab, asking first when it has unsaved work (the cross of the tab, and Ctrl+W).
    pub fn close_tab_request(&mut self) {
        if self.is_dirty() {
            self.tabs_state.closing = true;
        } else {
            self.close_shown_tab();
        }
    }

    /// Write the work of the tabs that are not shown (the shown one is written by `autosave_tick`).
    pub fn autosave_parked(&mut self) {
        self.parked_saved_at = std::time::Instant::now();
        let base = crate::data_paths().base.clone();
        for w in self.tabs_state.parked.iter_mut().flatten() {
            if !w.dirty() || w.autosaved_clean {
                continue;
            }
            let text = w.to_project().to_json();
            let ok = match &w.path {
                Some(p) => base_core::autosave::write_atomic(p, &text).is_ok(),
                None => base_core::autosave::save_slot(&base, APP, w.slot, &text).is_ok(),
            };
            if ok {
                w.autosaved_clean = true;
                if w.path.is_some() {
                    w.clean = true;
                }
            }
        }
    }

    pub fn parked_need_autosave(&self) -> bool {
        self.tabs_state.parked.iter().flatten().any(|w| w.dirty() && !w.autosaved_clean)
    }

    /// The questions about unsaved work: when the program is closed, tab by tab (`doc_tabs::quit_tick`), and when one tab is
    /// closed by its cross.
    pub fn close_dialogs(&mut self, ctx: &egui::Context) {
        if self.close_program {
            self.close_program = false;
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        if self.quitting {
            match doc_tabs::quit_tick(self, ctx, crate::SAVE_GREEN) {
                doc_tabs::Quit::Done => {
                    self.quitting = false;
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                doc_tabs::Quit::Stopped => self.quitting = false,
                doc_tabs::Quit::Asking => {}
            }
        }
        if !self.tabs_state.closing {
            return;
        }
        match doc_tabs::ask_save(ctx, &doc_tabs::title_of(self.path.as_deref()), crate::SAVE_GREEN) {
            Some(doc_tabs::Answer::Save) => {
                self.save(false);
                self.tabs_state.closing = false;
                if !self.is_dirty() {
                    self.close_shown_tab();
                }
            }
            Some(doc_tabs::Answer::Discard) => {
                self.tabs_state.closing = false;
                self.close_shown_tab();
            }
            Some(doc_tabs::Answer::Cancel) => self.tabs_state.closing = false,
            None => {}
        }
    }

    /// Close the program, after asking about every tab with unsaved work.
    pub fn request_close(&mut self, ctx: &egui::Context) {
        if self.any_dirty() && !self.allow_close {
            self.quitting = true;
        } else {
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    /// The window's own close button or the system asks to close.
    pub fn close_requested_tick(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested()) && self.any_dirty() && !self.allow_close {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.quitting = true;
        }
    }
}

impl TabHost for App {
    fn tab_count(&self) -> usize {
        App::tab_count(self)
    }
    fn active_tab(&self) -> usize {
        self.tabs_state.active
    }
    fn tab_dirty(&self, i: usize) -> bool {
        if i == self.tabs_state.active || self.tabs_state.parked.len() <= 1 {
            self.is_dirty()
        } else {
            self.tabs_state.parked[i].as_ref().is_some_and(|w| w.dirty())
        }
    }
    fn tab_title(&self, i: usize) -> String {
        if i == self.tabs_state.active || self.tabs_state.parked.len() <= 1 {
            doc_tabs::title_of(self.path.as_deref())
        } else {
            doc_tabs::title_of(self.tabs_state.parked[i].as_ref().and_then(|w| w.path.as_deref()))
        }
    }
    fn switch_tab(&mut self, i: usize) {
        self.switch_to(i);
    }
    fn save_active(&mut self) {
        self.save(false);
    }
    fn discard_active(&mut self) {
        base_core::autosave::clear_slot(&crate::data_paths().base, APP, self.slot);
    }
    fn close_active_tab(&mut self) {
        self.close_shown_tab();
    }
}
