//! The central area in panes (the machinery is in `base_ui::work_panes`): every pane has a bar of tabs and a tab is a drawing.
use crate::App;
use base_ui::editor::{show_other_doc, EditorCanvas, Look};
use base_ui::work_panes::{Content, PaneEvent, PaneHost};
use eframe::egui;

impl App {
    /// The slots of all open drawings, in the order of their tabs.
    fn doc_slots(&self) -> Vec<usize> {
        (0..self.tab_count()).map(|i| if i == self.tabs_state.active || self.tabs_state.parked.len() <= 1 { self.slot } else { self.tabs_state.parked[i].as_ref().map_or(self.slot, |w| w.slot) }).collect()
    }

    /// Every tab there is: the drawings.
    fn pane_tabs(&self) -> Vec<Content> {
        self.doc_slots().into_iter().map(Content::Doc).collect()
    }

    /// The central area: the panes.
    pub(crate) fn central_panes(&mut self, ui: &mut egui::Ui) {
        let mut panes = std::mem::take(&mut self.panes);
        panes.normalize(&self.pane_tabs(), self.slot);
        let event = panes.show(self, ui);
        self.panes = panes;
        self.pane_event(event);
    }

    /// The windows of the tabs that were taken out (drawn outside the panels).
    pub(crate) fn pane_windows(&mut self, ctx: &egui::Context) {
        if self.panes.windows.is_empty() {
            return;
        }
        let mut panes = std::mem::take(&mut self.panes);
        panes.normalize(&self.pane_tabs(), self.slot);
        let event = panes.show_windows(self, ctx);
        self.panes = panes;
        self.pane_event(event);
    }

    /// What was done in the panes: a drawing clicked becomes the active one, a cross closes, the "+" makes a new drawing.
    pub(crate) fn pane_event(&mut self, event: PaneEvent) {
        match event {
            PaneEvent::Activate(slot) => self.activate_slot(slot),
            PaneEvent::Close(Content::Doc(slot)) => {
                self.activate_slot(slot);
                self.close_tab_request();
            }
            PaneEvent::NewTab(_) => {
                self.new_tab();
            }
            PaneEvent::Close(_) | PaneEvent::Nothing => {}
        }
    }

    /// Make the drawing with this recovery slot the active one.
    pub(crate) fn activate_slot(&mut self, slot: usize) {
        if slot != self.slot {
            if let Some(i) = self.doc_slots().iter().position(|s| *s == slot) {
                self.switch_to(i);
            }
        }
    }
}

impl PaneHost for App {
    fn window_logo(&self) -> Option<egui::TextureHandle> {
        self.logo.clone()
    }
    fn tab_title(&self, c: Content) -> String {
        match c {
            Content::Doc(slot) if slot == self.slot => base_ui::doc_tabs::title_of(self.path.as_deref()),
            Content::Doc(slot) => base_ui::doc_tabs::title_of(self.tabs_state.parked.iter().flatten().find(|w| w.slot == slot).and_then(|w| w.path.as_deref())),
            _ => String::new(),
        }
    }

    fn tab_dirty(&self, c: Content) -> bool {
        match c {
            Content::Doc(slot) if slot == self.slot => self.is_dirty(),
            Content::Doc(slot) => self.tabs_state.parked.iter().flatten().find(|w| w.slot == slot).is_some_and(|w| w.dirty()),
            _ => false,
        }
    }

    fn draw_pane(&mut self, ui: &mut egui::Ui, content: Content, view: &mut Option<base_ui::editor::View>) -> PaneEvent {
        match content {
            Content::Doc(slot) if slot == self.slot => self.canvas(ui),
            Content::Doc(slot) => {
                let Some(w) = self.tabs_state.parked.iter().flatten().find(|w| w.slot == slot) else { return PaneEvent::Nothing };
                let colour = |l: usize| w.layers.get(l).and_then(|l| l.visible.then(|| l.color32()));
                let look = Look { workspace: self.ed.grid_prefs.workspace_fill(), bed: self.ed.grid_prefs.bed_fill(self.scheme.is_dark()), layer_colour: &colour };
                // Every drawing has its own page.
                if show_other_doc(ui, &w.scene, (w.page.w, w.page.h), &look, view) {
                    return PaneEvent::Activate(slot);
                }
            }
            _ => {}
        }
        PaneEvent::Nothing
    }
}
