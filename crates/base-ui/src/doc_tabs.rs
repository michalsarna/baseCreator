//! Tabs of open drawings: the bar with the tabs, the question asked about unsaved work, and the order in which the tabs are
//! closed when the program is. What a tab holds is up to the program (a `Workspace` it swaps with the live fields); this
//! module knows only titles and whether there is unsaved work.
use crate::i18n::{tr, trf};
use crate::theme;
use egui::{Color32, RichText, Sense};
use std::path::Path;

/// The most drawings that are open at once.
pub const MAX_TABS: usize = 10;

/// What the bar needs to know about a tab.
pub struct TabInfo {
    pub title: String,
    pub dirty: bool,
}

/// What the person did in the bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabAction {
    Select(usize),
    Close(usize),
    New,
    /// The extra tab with this `id` (see [`ExtraTab`]) was picked, or closed.
    Extra(&'static str),
    CloseExtra(&'static str),
}

/// The title of a tab: the name of the file, or "Untitled".
pub fn title_of(path: Option<&Path>) -> String {
    path.and_then(|p| p.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| tr("Untitled").to_string())
}

/// The lowest recovery slot that no tab uses.
pub fn free_slot(used: &[usize]) -> usize {
    (0..MAX_TABS).find(|s| !used.contains(s)).unwrap_or(0)
}

const TAB_H: f32 = 26.0;
const MAX_CHARS: usize = 22;

fn shorten(s: &str) -> String {
    if s.chars().count() <= MAX_CHARS {
        s.to_string()
    } else {
        format!("{}…", s.chars().take(MAX_CHARS - 1).collect::<String>())
    }
}

/// A tab that is not a drawing (the live view of the camera, the preview): it comes after the drawings and has no unsaved
/// work.
pub struct ExtraTab {
    /// Tells the tab in the [`TabAction`]s.
    pub id: &'static str,
    pub title: String,
    /// Shown instead of the drawing at `active`.
    pub shown: bool,
}

/// Draws one tab; returns (clicked, closed).
fn one_tab(ui: &mut egui::Ui, id: impl std::hash::Hash + std::fmt::Debug, title: &str, dirty: bool, is_active: bool) -> (bool, bool) {
    let font = egui::FontId::proportional(13.0);
    let text = shorten(title);
    let galley = ui.painter().layout_no_wrap(text, font, if is_active { theme::text() } else { theme::text_dim() });
    let w = galley.size().x + 14.0 + 22.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, TAB_H), Sense::click());
    let close_rect = egui::Rect::from_min_size(egui::pos2(rect.right() - 22.0, rect.top()), egui::vec2(22.0, TAB_H));
    let close = ui.interact(close_rect, ui.make_persistent_id(("doc_tab_close", id)), Sense::click());
    let fill = if is_active {
        theme::workspace()
    } else if resp.hovered() {
        theme::panel_dark()
    } else {
        theme::panel()
    };
    ui.painter().rect_filled(rect, egui::CornerRadius { nw: 5, ne: 5, sw: 0, se: 0 }, fill);
    if is_active {
        ui.painter().rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), 2.0)), 0.0, theme::accent());
    }
    ui.painter().galley(egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y / 2.0), galley, Color32::PLACEHOLDER);
    // A dot for unsaved work, which turns into the cross while the pointer is on the close button.
    let (mark, colour) = if close.hovered() {
        ("×", theme::text())
    } else if dirty {
        ("●", theme::accent())
    } else {
        ("×", theme::text_dim())
    };
    ui.painter().text(close_rect.center(), egui::Align2::CENTER_CENTER, mark, egui::FontId::proportional(if mark == "●" { 9.0 } else { 15.0 }), colour);
    let resp = if title.chars().count() > MAX_CHARS { resp.on_hover_text(title) } else { resp };
    close.clone().on_hover_text(tr("Close tab"));
    (resp.clicked() || resp.middle_clicked(), close.clicked() || resp.middle_clicked())
}

/// The bar: a tab for each drawing (a dot when it has unsaved work, a cross to close it) and a "+" for a new one. A middle
/// click closes a tab.
pub fn tab_bar(ui: &mut egui::Ui, tabs: &[TabInfo], active: usize) -> Option<TabAction> {
    tab_bar_with(ui, tabs, active, &[])
}

/// [`tab_bar`] with more tabs after the drawings. While one of them is shown no drawing is the active one.
pub fn tab_bar_with(ui: &mut egui::Ui, tabs: &[TabInfo], active: usize, extras: &[ExtraTab]) -> Option<TabAction> {
    let mut action = None;
    egui::ScrollArea::horizontal().id_salt("doc_tabs_scroll").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            for (i, t) in tabs.iter().enumerate() {
                let is_active = i == active && !extras.iter().any(|e| e.shown);
                let (clicked, closed) = one_tab(ui, i, &t.title, t.dirty, is_active);
                if closed {
                    action = Some(TabAction::Close(i));
                } else if clicked && !is_active {
                    action = Some(TabAction::Select(i));
                }
            }
            for e in extras {
                let (clicked, closed) = one_tab(ui, e.id, &e.title, false, e.shown);
                if closed {
                    action = Some(TabAction::CloseExtra(e.id));
                } else if clicked && !e.shown {
                    action = Some(TabAction::Extra(e.id));
                }
            }
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(TAB_H, TAB_H), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 5.0, theme::panel_dark());
            }
            ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(18.0), theme::text());
            if resp.on_hover_text(tr("New tab")).clicked() {
                action = Some(TabAction::New);
            }
        });
    });
    action
}

/// The name of the way to show the camera view as a tab of the work area.
pub fn work_tab_label() -> &'static str {
    tr("Tab in the work area")
}

/// The button that puts the camera view in a tab of the work area (or takes it out); true when it was clicked.
pub fn work_tab_toggle(ui: &mut egui::Ui, on: bool) -> bool {
    crate::widgets::toggle_icon(ui, "panel-top", work_tab_label(), on)
}

/// The small button of a floating window (its title line) that pins the window as a tab of the work area; true when clicked.
pub fn pin_as_tab_button(ui: &mut egui::Ui) -> bool {
    crate::widgets::icon_button(ui, "panel-top", work_tab_label(), true, 22.0)
}

/// The small button of such a tab (above its content) that makes it a floating window again; true when clicked.
pub fn back_to_window_button(ui: &mut egui::Ui) -> bool {
    crate::widgets::icon_button(ui, "picture-in-picture-2", tr("Back to a window"), true, 22.0)
}

/// What a tab shortcut asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabKey {
    /// Show the tab with this index (Ctrl+1 … Ctrl+9 are the tabs 1 … 9, Ctrl+0 the tenth).
    Select(usize),
    /// Close the shown tab (Ctrl+W), asking when it has unsaved work.
    Close,
}

/// The digit keys in the order of the tabs: 1 … 9, then 0.
const DIGITS: [egui::Key; MAX_TABS] = [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3, egui::Key::Num4, egui::Key::Num5, egui::Key::Num6, egui::Key::Num7, egui::Key::Num8, egui::Key::Num9, egui::Key::Num0];

/// Ctrl+digit (Cmd on a Mac) shows a tab, Ctrl+W closes the shown one. Nothing while a field has the keyboard.
pub fn shortcut(ctx: &egui::Context) -> Option<TabKey> {
    if ctx.egui_wants_keyboard_input() {
        return None;
    }
    ctx.input_mut(|i| {
        if i.consume_key(egui::Modifiers::COMMAND, egui::Key::W) {
            return Some(TabKey::Close);
        }
        DIGITS.iter().position(|k| i.consume_key(egui::Modifiers::COMMAND, *k)).map(TabKey::Select)
    })
}

/// What the person answered when asked about a tab with unsaved work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Save,
    Discard,
    Cancel,
}

/// The question: save the work of this tab before closing? `None` while nothing was chosen (Escape is Cancel).
pub fn ask_save(ctx: &egui::Context, title: &str, green: Color32) -> Option<Answer> {
    let mut choice = None;
    egui::Window::new("close_dialog").title_bar(false).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.set_min_width(420.0);
        ui.add_space(6.0);
        ui.label(RichText::new(tr("Save your work before closing?")).size(18.0).strong());
        ui.add_space(4.0);
        ui.label(RichText::new(title).size(15.0).strong());
        ui.label(RichText::new(tr("The design has not been saved to a file.")).size(14.0).color(theme::text_dim()));
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let size = egui::vec2((ui.available_width() - 2.0 * ui.spacing().item_spacing.x) / 3.0, 38.0);
            let save = egui::Button::new(RichText::new(tr("Save")).size(15.0).color(Color32::WHITE).strong()).fill(green);
            if ui.add_sized(size, save).clicked() {
                choice = Some(Answer::Save);
            }
            let discard = egui::Button::new(RichText::new(tr("Discard")).size(15.0).color(Color32::WHITE).strong()).fill(Color32::from_rgb(0xc0, 0x30, 0x30));
            if ui.add_sized(size, discard).clicked() {
                choice = Some(Answer::Discard);
            }
            if ui.add_sized(size, egui::Button::new(RichText::new(tr("Cancel")).size(15.0))).clicked() {
                choice = Some(Answer::Cancel);
            }
        });
        ui.add_space(4.0);
    });
    if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
        choice = Some(Answer::Cancel);
    }
    choice
}

/// The message for a new tab that would be the eleventh.
pub fn limit_message() -> String {
    trf("At most {} tabs can be open.", &[&MAX_TABS])
}

/// What the program has to offer for closing it tab by tab.
pub trait TabHost {
    fn tab_count(&self) -> usize;
    fn active_tab(&self) -> usize;
    /// Has the tab unsaved work?
    fn tab_dirty(&self, i: usize) -> bool;
    fn tab_title(&self, i: usize) -> String;
    fn switch_tab(&mut self, i: usize);
    /// Save the shown tab (a file dialog when it has no name); after a cancelled dialog it stays dirty.
    fn save_active(&mut self);
    /// Throw the unsaved work of the shown tab away (and its recovery file).
    fn discard_active(&mut self);
    /// Close the shown tab; the one next to it is shown, or the start screen when it was the last.
    fn close_active_tab(&mut self);
}

/// The first tab, from the left, with unsaved work.
pub fn next_dirty<H: TabHost>(h: &H) -> Option<usize> {
    (0..h.tab_count()).find(|&i| h.tab_dirty(i))
}

/// How the closing of the program goes on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quit {
    /// Another tab (or this one again) has to be asked about.
    Asking,
    /// Nothing is unsaved any more: the window can close.
    Done,
    /// The person said Cancel (or cancelled the file dialog): the program stays open.
    Stopped,
}

/// Show the tab that is the next to be asked about and say what is to happen: `Done` when no tab has unsaved work.
pub fn quit_prepare<H: TabHost>(h: &mut H) -> Quit {
    match next_dirty(h) {
        None => Quit::Done,
        Some(i) => {
            if h.active_tab() != i {
                h.switch_tab(i);
            }
            Quit::Asking
        }
    }
}

/// Carry out the answer about the shown tab: Save keeps it only when it became clean (a cancelled file dialog stops the
/// closing), then the tab is closed and the next one is asked about.
pub fn quit_answer<H: TabHost>(h: &mut H, a: Answer) -> Quit {
    match a {
        Answer::Cancel => Quit::Stopped,
        Answer::Discard => {
            h.discard_active();
            h.close_active_tab();
            quit_prepare(h)
        }
        Answer::Save => {
            h.save_active();
            if h.tab_dirty(h.active_tab()) {
                return Quit::Stopped;
            }
            h.close_active_tab();
            quit_prepare(h)
        }
    }
}

/// Called every frame while the program is closing. Returns what to do next; `Done` means the window may close now.
pub fn quit_tick<H: TabHost>(h: &mut H, ctx: &egui::Context, green: Color32) -> Quit {
    match quit_prepare(h) {
        Quit::Done => Quit::Done,
        _ => {
            let title = h.tab_title(h.active_tab());
            match ask_save(ctx, &title, green) {
                Some(a) => quit_answer(h, a),
                None => Quit::Asking,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_extra_tab_is_drawn_after_the_drawings_and_takes_the_active_mark() {
        let ctx = egui::Context::default();
        let tabs = [TabInfo { title: "a".into(), dirty: false }, TabInfo { title: "b".into(), dirty: true }];
        let extras = [ExtraTab { id: "camera", title: "Camera".into(), shown: true }, ExtraTab { id: "preview", title: "Preview".into(), shown: false }];
        let mut out = ctx.run_ui(Default::default(), |ui| {
            assert_eq!(tab_bar_with(ui, &tabs, 0, &extras), None);
            assert_eq!(tab_bar(ui, &tabs, 0), None);
        });
        out.textures_delta.clear();
        assert_eq!(work_tab_label(), "Tab in the work area");
    }

    use super::*;

    /// Tabs with a title and unsaved work (or not), the one shown, and what a Save does: `saves` is what the file dialog
    /// does (false = cancelled).
    struct Fake {
        tabs: Vec<(String, bool)>,
        active: usize,
        saves: bool,
        log: Vec<String>,
    }

    impl Fake {
        fn new(tabs: &[(&str, bool)]) -> Fake {
            Fake { tabs: tabs.iter().map(|(t, d)| (t.to_string(), *d)).collect(), active: 0, saves: true, log: vec![] }
        }
    }

    impl TabHost for Fake {
        fn tab_count(&self) -> usize {
            self.tabs.len()
        }
        fn active_tab(&self) -> usize {
            self.active
        }
        fn tab_dirty(&self, i: usize) -> bool {
            self.tabs[i].1
        }
        fn tab_title(&self, i: usize) -> String {
            self.tabs[i].0.clone()
        }
        fn switch_tab(&mut self, i: usize) {
            self.active = i;
            self.log.push(format!("show {}", self.tabs[i].0));
        }
        fn save_active(&mut self) {
            self.log.push(format!("save {}", self.tabs[self.active].0));
            if self.saves {
                self.tabs[self.active].1 = false;
            }
        }
        fn discard_active(&mut self) {
            self.log.push(format!("discard {}", self.tabs[self.active].0));
        }
        fn close_active_tab(&mut self) {
            self.log.push(format!("close {}", self.tabs[self.active].0));
            self.tabs.remove(self.active);
            self.active = self.active.min(self.tabs.len().saturating_sub(1));
        }
    }

    #[test]
    fn clean_tabs_close_without_a_question() {
        let mut h = Fake::new(&[("a", false), ("b", false)]);
        assert_eq!(quit_prepare(&mut h), Quit::Done);
        assert!(h.log.is_empty());
    }

    #[test]
    fn every_tab_with_unsaved_work_is_asked_about_in_turn_and_closed() {
        let mut h = Fake::new(&[("a", true), ("b", false), ("c", true)]);
        assert_eq!(quit_prepare(&mut h), Quit::Asking);
        assert_eq!(h.active, 0);
        assert_eq!(quit_answer(&mut h, Answer::Save), Quit::Asking, "a is saved and closed, c is next");
        assert_eq!(h.tab_title(h.active_tab()), "c", "the next tab with unsaved work is shown");
        assert_eq!(quit_answer(&mut h, Answer::Discard), Quit::Done);
        assert_eq!(h.log, ["save a", "close a", "show c", "discard c", "close c"]);
    }

    #[test]
    fn cancel_stops_the_closing_and_leaves_the_tab_open() {
        let mut h = Fake::new(&[("a", true), ("b", true)]);
        quit_prepare(&mut h);
        assert_eq!(quit_answer(&mut h, Answer::Discard), Quit::Asking);
        assert_eq!(quit_answer(&mut h, Answer::Cancel), Quit::Stopped);
        assert_eq!(h.tabs.len(), 1, "a was closed, b is still there");
        assert_eq!(h.tabs[0].0, "b");
    }

    #[test]
    fn a_cancelled_file_dialog_stops_the_closing_too() {
        let mut h = Fake::new(&[("a", true)]);
        h.saves = false;
        quit_prepare(&mut h);
        assert_eq!(quit_answer(&mut h, Answer::Save), Quit::Stopped);
        assert_eq!(h.tabs.len(), 1, "the tab with its work is kept");
    }

    #[test]
    fn a_free_slot_is_the_lowest_unused_one() {
        assert_eq!(free_slot(&[]), 0);
        assert_eq!(free_slot(&[0, 1, 3]), 2);
        assert_eq!(free_slot(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]), 0, "never happens: the tabs are at most ten");
    }

    #[test]
    fn ctrl_digits_pick_the_tabs_and_ctrl_w_closes() {
        let ctx = egui::Context::default();
        let press = |key: egui::Key, mods: egui::Modifiers| {
            let input = egui::RawInput { events: vec![egui::Event::Key { key, physical_key: Some(key), pressed: true, repeat: false, modifiers: mods }], ..Default::default() };
            let mut got = None;
            let mut out = ctx.run_ui(input, |_| got = shortcut(&ctx));
            out.textures_delta.clear();
            got
        };
        assert_eq!(press(egui::Key::Num1, egui::Modifiers::COMMAND), Some(TabKey::Select(0)));
        assert_eq!(press(egui::Key::Num9, egui::Modifiers::COMMAND), Some(TabKey::Select(8)));
        assert_eq!(press(egui::Key::Num0, egui::Modifiers::COMMAND), Some(TabKey::Select(9)));
        assert_eq!(press(egui::Key::W, egui::Modifiers::COMMAND), Some(TabKey::Close));
        assert_eq!(press(egui::Key::Num1, egui::Modifiers::NONE), None, "a digit alone is not a shortcut");
        assert_eq!(press(egui::Key::W, egui::Modifiers::NONE), None);
    }

    #[test]
    fn long_titles_are_shortened() {
        assert_eq!(shorten("short.cre"), "short.cre");
        let long = "a-very-long-name-of-a-project-file.cre";
        assert!(shorten(long).chars().count() <= MAX_CHARS && shorten(long).ends_with('…'));
    }
}
