//! The tabs of the right-hand panel, the same in every program: the row of tabs (they can be dragged into another order,
//! which is remembered) with a button that collapses the panel, and the narrow strip with the tab names written
//! vertically that stays when it is collapsed (a click on a name opens the panel on that tab).
//!
//! The program keeps its own list of tabs; here a tab is only an id (a short fixed string) and a label.
use crate::i18n::tr;
use crate::{icons, theme};
use egui::{Align2, Color32, FontId, RichText, Sense};

/// Width of the collapsed strip.
pub const STRIP_W: f32 = 30.0;

/// The order of the tabs the user arranged, and whether the panel is collapsed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SideTabs {
    all: Vec<&'static str>,
    order: Vec<&'static str>,
    pub collapsed: bool,
}

/// `saved` (ids separated by commas, from the settings) in the order it gives; ids it does not mention keep their place at
/// the end, ids it mentions that do not exist are dropped.
pub fn arranged(all: &[&'static str], saved: &str) -> Vec<&'static str> {
    let mut out: Vec<&'static str> = vec![];
    for id in saved.split(',') {
        if let Some(t) = all.iter().find(|t| **t == id.trim()) {
            if !out.contains(t) {
                out.push(t);
            }
        }
    }
    for t in all {
        if !out.contains(t) {
            out.push(t);
        }
    }
    out
}

/// Move the tab at `from` to the place of the tab at `to`.
pub fn move_tab(order: &mut Vec<&'static str>, from: usize, to: usize) {
    if from < order.len() && to < order.len() && from != to {
        let t = order.remove(from);
        order.insert(to, t);
    }
}

impl SideTabs {
    /// `all` is every tab of the program in its default order; `saved` the order from the settings.
    pub fn new(all: &[&'static str], saved: &str, collapsed: bool) -> Self {
        SideTabs { all: all.to_vec(), order: arranged(all, saved), collapsed }
    }

    /// The order for the settings.
    pub fn order_string(&self) -> String {
        self.order.join(",")
    }

    /// Take an order from the settings (also from a backup).
    pub fn restore(&mut self, saved: &str, collapsed: bool) {
        self.order = arranged(&self.all, saved);
        self.collapsed = collapsed;
    }

    pub fn order(&self) -> &[&'static str] {
        &self.order
    }

    /// The row of tabs on top of the panel. `extra` are tabs that exist only sometimes (the camera): they come last and cannot
    /// be moved. Returns the tab that was clicked. The first button collapses the panel.
    pub fn bar(&mut self, ui: &mut egui::Ui, current: &str, extra: &[&'static str], label: &dyn Fn(&str) -> String) -> Option<&'static str> {
        let mut picked = None;
        let mut moved: Option<(usize, usize)> = None;
        let movable = self.order.len();
        let tabs: Vec<&'static str> = self.order.iter().copied().chain(extra.iter().copied()).collect();
        ui.horizontal_wrapped(|ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 24.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 4.0, theme::panel_dark());
            }
            icons::paint(ui, rect.shrink2(egui::vec2(3.0, 4.0)), "chevron-right", theme::text());
            if resp.on_hover_text(tr("Hide the panel (the tabs stay on a narrow strip)")).clicked() {
                self.collapsed = true;
            }
            let mut rects: Vec<egui::Rect> = vec![];
            let mut dragging: Option<usize> = None;
            for (i, id) in tabs.iter().enumerate() {
                let b = egui::Button::selectable(current == *id, RichText::new(label(id)).strong()).sense(Sense::click_and_drag());
                let r = ui.add(b);
                if r.clicked() {
                    picked = Some(*id);
                }
                if i < movable {
                    if r.dragged() {
                        dragging = Some(i);
                        ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                    }
                    if r.drag_stopped() {
                        moved = Some((i, usize::MAX));
                    }
                }
                rects.push(r.rect);
            }
            // Where a dragged tab would land: next to the tab nearest to the pointer.
            let pointer = ui.input(|i| i.pointer.interact_pos());
            let dragged = dragging.or(moved.map(|m| m.0));
            if let (Some(from), Some(p)) = (dragged, pointer) {
                let to = (0..movable).min_by(|a, b| rects[*a].distance_to_pos(p).total_cmp(&rects[*b].distance_to_pos(p)));
                if let Some(to) = to {
                    if dragging.is_some() && to != from {
                        let x = if to > from { rects[to].right() + 2.0 } else { rects[to].left() - 2.0 };
                        ui.painter().line_segment([egui::pos2(x, rects[to].top()), egui::pos2(x, rects[to].bottom())], egui::Stroke::new(2.0, theme::accent()));
                    }
                    if let Some(m) = &mut moved {
                        m.1 = to;
                    }
                }
            }
        });
        if let Some((from, to)) = moved {
            move_tab(&mut self.order, from, to);
        }
        picked
    }

    /// The strip of the collapsed panel: a button that opens it again and the names of the tabs written upwards. Returns the
    /// tab that was clicked (the panel is open again then).
    pub fn strip(&mut self, ui: &mut egui::Ui, current: &str, extra: &[&'static str], label: &dyn Fn(&str) -> String) -> Option<&'static str> {
        let mut picked = None;
        let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), Sense::click());
        if resp.hovered() {
            ui.painter().rect_filled(rect, 4.0, theme::panel_dark());
        }
        icons::paint(ui, rect.shrink2(egui::vec2(3.0, 4.0)), "chevron-left", theme::text());
        if resp.on_hover_text(tr("Show the panel")).clicked() {
            self.collapsed = false;
        }
        ui.add_space(6.0);
        let font = FontId::proportional(14.0);
        let tabs: Vec<&'static str> = self.order.iter().copied().chain(extra.iter().copied()).collect();
        for id in tabs {
            let name = label(id);
            let on = current == id;
            let ink = if on { theme::accent() } else { theme::text() };
            let galley = ui.painter().layout_no_wrap(name.clone(), font.clone(), ink);
            let size = egui::vec2(ui.available_width(), galley.size().x + 18.0);
            let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
            if on {
                ui.painter().rect_filled(rect, 4.0, theme::panel_dark());
            } else if resp.hovered() {
                ui.painter().rect_filled(rect, 4.0, theme::panel_dark().gamma_multiply(0.6));
            }
            // Written upwards: the text is turned by a quarter, round its left bottom corner.
            let at = egui::pos2(rect.center().x - galley.size().y / 2.0, rect.bottom() - 9.0);
            ui.painter().add(egui::epaint::TextShape::new(at, galley, Color32::PLACEHOLDER).with_angle(-std::f32::consts::FRAC_PI_2).with_override_text_color(ink));
            let _ = Align2::CENTER_CENTER;
            if resp.on_hover_text(name).clicked() {
                picked = Some(id);
                self.collapsed = false;
            }
        }
        picked
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const ALL: [&str; 4] = ["a", "b", "c", "d"];

    #[test]
    fn saved_order_is_kept_missing_are_added_unknown_dropped() {
        assert_eq!(arranged(&ALL, "c,a"), vec!["c", "a", "b", "d"]);
        assert_eq!(arranged(&ALL, "x, b ,b,d,y"), vec!["b", "d", "a", "c"]);
        assert_eq!(arranged(&ALL, ""), vec!["a", "b", "c", "d"]);
    }

    #[test]
    fn a_tab_is_moved_to_the_place_of_another() {
        let mut o = arranged(&ALL, "");
        move_tab(&mut o, 0, 2);
        assert_eq!(o, vec!["b", "c", "a", "d"]);
        move_tab(&mut o, 3, 0);
        assert_eq!(o, vec!["d", "b", "c", "a"]);
        move_tab(&mut o, 1, 9); // out of range: nothing
        assert_eq!(o, vec!["d", "b", "c", "a"]);
    }

    #[test]
    fn order_round_trips_through_the_settings() {
        let mut t = SideTabs::new(&ALL, "", false);
        move_tab(&mut t.order, 3, 0);
        let s = t.order_string();
        let mut u = SideTabs::new(&ALL, "", false);
        u.restore(&s, true);
        assert_eq!(u.order(), t.order());
        assert!(u.collapsed);
    }

    #[test]
    fn collapse_button_collapses_and_strip_opens_on_a_tab() {
        let ctx = egui::Context::default();
        let mut t = SideTabs::new(&ALL, "", false);
        let mut picked: Option<&'static str> = None;
        let mut frame = |t: &mut SideTabs, events: Vec<egui::Event>| {
            let mut input = egui::RawInput::default();
            input.screen_rect = Some(egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(400.0, 400.0)));
            input.events = events;
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show_inside(ui, |ui| {
                    let label = |id: &str| id.to_string();
                    let p = if t.collapsed { t.strip(ui, "a", &[], &label) } else { t.bar(ui, "a", &[], &label) };
                    picked = p.or(picked);
                });
            });
            out.textures_delta.clear();
        };
        let clicks = |at: egui::Pos2| {
            let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
            vec![vec![egui::Event::PointerMoved(at)], vec![button(true)], vec![button(false)], vec![]]
        };
        frame(&mut t, vec![]);
        frame(&mut t, vec![]);
        // The first button of the row is the collapse button.
        for e in clicks(egui::pos2(20.0, 20.0)) {
            frame(&mut t, e);
        }
        assert!(t.collapsed, "the button collapses the panel");
        // A name on the strip, below the button.
        for e in clicks(egui::pos2(15.0, 70.0)) {
            frame(&mut t, e);
        }
        assert!(!t.collapsed, "a click on a name opens the panel");
        assert!(picked.is_some());
    }
}
