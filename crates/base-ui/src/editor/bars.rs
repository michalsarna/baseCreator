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
/// application's own operations on the selection), then view navigation with the two fit buttons.
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
