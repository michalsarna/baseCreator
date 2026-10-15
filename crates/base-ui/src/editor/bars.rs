//! The tool palette (left strip) and the arrange bar (narrow strip with align, turn, mirror, group and order).
use super::{EditorHost, Tool};
use crate::i18n::tr;
use crate::menu::StdAct;
use crate::widgets::{icon_button, scroll_hints};
use crate::{icons, theme};
use egui::Color32;

/// One tool button; choosing a tool ends a temporary hand tool and a line being drawn.
pub fn tool_button<H: EditorHost>(h: &mut H, ui: &mut egui::Ui, t: Tool, tip: &'static str) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::click());
    let on = h.ed().tool == t;
    if on {
        ui.painter().rect_filled(rect, 6.0, theme::accent());
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, theme::panel_dark());
    }
    icons::paint(ui, rect.shrink(7.0), icons::tool_icon(t), if on { Color32::WHITE } else { theme::text() });
    if resp.on_hover_text(tr(tip)).clicked() {
        let ed = h.ed_mut();
        ed.tool = t;
        ed.space_prev = None;
        ed.pen_pts.clear();
        ed.show_shape_popup = matches!(t, Tool::Polygon | Tool::Spiral);
    }
    ui.add_space(2.0);
}

fn separator(ui: &mut egui::Ui) {
    ui.add_space(4.0);
    ui.separator();
    ui.add_space(4.0);
}

/// The tool palette: selecting and node editing, the tools that insert objects, then what `between` adds (the
/// application's own operations on the selection), then view navigation with the two fit buttons, then the generators and
/// the clipart from the internet.
pub fn tool_palette<H: EditorHost>(h: &mut H, ui: &mut egui::Ui, between: Option<&mut dyn FnMut(&mut H, &mut egui::Ui)>) {
    ui.add_space(6.0);
    let out = egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.vertical_centered(|ui| {
            // Selecting and node editing.
            for (t, tip, _) in Tool::ALL.into_iter().filter(|t| t.0.is_edit()) {
                tool_button(h, ui, t, tip);
            }
            // Tools that insert new objects.
            separator(ui);
            for (t, tip, _) in Tool::ALL.into_iter().filter(|t| !t.0.is_edit() && !t.0.is_nav()) {
                tool_button(h, ui, t, tip);
            }
            // The application's operations on the selected objects.
            if let Some(between) = between {
                separator(ui);
                between(h, ui);
            }
            // View navigation, apart from the rest.
            separator(ui);
            for (t, tip, _) in Tool::ALL.into_iter().filter(|t| t.0.is_nav()) {
                tool_button(h, ui, t, tip);
            }
            let ink = theme::text();
            let mut act = None;
            for (a, icon, tip) in [(StdAct::FitBed, "scan", tr("Fit work area (Ctrl+0)")), (StdAct::FitAll, "expand", tr("Fit all objects (Ctrl+9)"))] {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, 6.0, theme::panel_dark());
                }
                icons::paint(ui, rect.shrink(7.0), icon, ink);
                if resp.on_hover_text(tip).clicked() {
                    act = Some(a);
                }
                ui.add_space(2.0);
            }
            if let Some(a) = act {
                a.run(h);
            }
            // Where shapes come from besides the tools: generators (plugins) and clipart from the internet.
            separator(ui);
            let mut open_generators = false;
            let mut open_clipart = false;
            for (icon, tip) in [("shapes", tr("Generate shape…")), ("globe", tr("Online clipart"))] {
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, 6.0, theme::panel_dark());
                }
                icons::paint(ui, rect.shrink(7.0), icon, ink);
                if resp.on_hover_text(tip).clicked() {
                    if icon == "shapes" {
                        open_generators = true;
                    } else {
                        open_clipart = true;
                    }
                }
                ui.add_space(2.0);
            }
            if open_generators {
                h.open_shape_generators();
            }
            if open_clipart {
                h.open_online_clipart_search(ui.ctx());
            }
        })
    });
    scroll_hints(ui, &out);
}

/// Narrow strip between the work area and the side panel: align, turn, mirror and group the selection.
pub fn arrange_bar<H: EditorHost>(h: &mut H, ui: &mut egui::Ui) {
    let sel = h.ed().sel.clone();
    let any_unlocked = sel.iter().any(|id| h.scene().shape(*id).is_some_and(|s| !s.locked));
    let any_locked = sel.iter().any(|id| h.scene().shape(*id).is_some_and(|s| s.locked));
    let grouped = sel.iter().any(|id| h.scene().shape(*id).is_some_and(|s| s.group.is_some()));
    let multi = sel.len() >= 2;
    let sections: [Vec<(StdAct, bool, &str)>; 4] = [
        vec![
            (StdAct::Align(0), any_unlocked, "Align left"),
            (StdAct::Align(1), any_unlocked, "Align centre (H)"),
            (StdAct::Align(2), any_unlocked, "Align right"),
            (StdAct::Align(3), any_unlocked, "Align top"),
            (StdAct::Align(4), any_unlocked, "Align centre (V)"),
            (StdAct::Align(5), any_unlocked, "Align bottom"),
            (StdAct::CenterOnBed, any_unlocked, "Centre on bed"),
            (StdAct::CenterEachOther, multi, "Centre on each other"),
        ],
        vec![
            (StdAct::RotCcw, any_unlocked, "Rotate 90° CCW"),
            (StdAct::RotCw, any_unlocked, "Rotate 90° CW"),
            (StdAct::FlipH, any_unlocked, "Flip horizontal"),
            (StdAct::FlipV, any_unlocked, "Flip vertical"),
        ],
        vec![(StdAct::Group, multi, "Group"), (StdAct::Ungroup, grouped, "Ungroup"), (StdAct::Lock, any_unlocked, "Lock"), (StdAct::Unlock, any_locked, "Unlock")],
        vec![(StdAct::ToFront, any_unlocked, "Bring to front"), (StdAct::ToBack, any_unlocked, "Send to back")],
    ];
    let mut chosen = None;
    let out = egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.add_space(6.0);
        for (i, section) in sections.iter().enumerate() {
            if i > 0 {
                ui.add_space(2.0);
                ui.separator();
                ui.add_space(2.0);
            }
            ui.vertical_centered(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(2.0, 2.0);
                for (act, on, tip) in section {
                    if icon_button(ui, act.icon(), &tr(tip), *on, 28.0) {
                        chosen = Some(*act);
                    }
                }
            });
        }
    });
    scroll_hints(ui, &out);
    if let Some(a) = chosen {
        a.run(h);
    }
}

/// The name of the current tool in a box of fixed width, so the control bar does not jump around when it changes.
pub fn control_name<H: EditorHost>(h: &H, ui: &mut egui::Ui) {
    let name = Tool::ALL.iter().find(|t| t.0 == h.ed().tool).map(|t| t.1).unwrap_or("");
    ui.allocate_ui_with_layout(egui::vec2(96.0, 22.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
        // The box keeps its width whatever the name is: it grows to the minimum and never past the maximum.
        ui.set_min_size(egui::vec2(96.0, 22.0));
        ui.set_max_width(96.0);
        ui.add(egui::Label::new(egui::RichText::new(tr(name).split(" (").next().unwrap_or("")).strong()).truncate());
    });
    // Where the box ends, for the test that checks it does not move.
    ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new("tool_name_right"), ui.cursor().left()));
}

/// The four toggles of the control bar: the grid, the secondary grid, snapping to the grid and to objects.
pub fn grid_toggles<H: EditorHost>(h: &mut H, ui: &mut egui::Ui) {
    use crate::widgets::toggle_icon;
    let minor = h.ed().grid_prefs.minor_on;
    if toggle_icon(ui, "grid-2x2", &tr("Show or hide the grid"), h.ed().show_grid) {
        h.ed_mut().show_grid = !h.ed().show_grid;
    }
    if toggle_icon(ui, "grid-3x3", &tr("Show or hide the secondary grid"), minor) {
        h.ed_mut().grid_prefs.minor_on = !minor;
    }
    if toggle_icon(ui, "magnet", &tr("Snap to grid"), h.ed().snap) {
        h.ed_mut().snap = !h.ed().snap;
    }
    if toggle_icon(ui, "target", &tr("Snap to objects"), h.ed().snap_objects) {
        h.ed_mut().snap_objects = !h.ed().snap_objects;
    }
}

/// The size of the selection, dimmed, for the right end of the control bar (draws nothing without a selection).
pub fn selection_size<H: EditorHost>(h: &H, ui: &mut egui::Ui) {
    if let Some(b) = h.sel_bounds() {
        let u = h.units();
        ui.label(egui::RichText::new(format!("{} × {}", crate::units_ui::fmt_len(u, b.width(), 1), crate::units_ui::fmt_len(u, b.height(), 1))).color(theme::text_dim()));
    }
}

/// The zoom and the pointer position for the right end of the status bar (call it in a right-to-left layout).
pub fn zoom_and_cursor<H: EditorHost>(h: &H, ui: &mut egui::Ui) {
    ui.label(format!("{}%", (h.ed().view.zoom / 2.0 * 100.0).round()));
    if let Some(p) = h.ed().cursor_mm {
        let u = h.units();
        let (x, y) = (u.from_mm(p.x), u.from_mm(p.y));
        let d = if u == base_core::units::Units::Mm { 2 } else { 3 };
        ui.label(format!("X {:.*}  Y {:.*}{}", d, x, d, y, u.suffix()));
    }
}
