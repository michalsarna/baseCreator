//! The sections of the Properties tab that every application shows for a selection: position and size, rotation,
//! mirror and centre, alignment. An application adds its own sections (a layer, text, picture) below.
use super::{EditorCommands, EditorHost};
use crate::i18n::tr;
use crate::units_ui::drag_len;
use crate::widgets::{button_grid, section_title};
use crate::{icons, theme};
use crate::menu::StdAct;
use base_core::geom::Xf;

/// Draw the sections for the selection. Returns false (and draws nothing) when nothing is selected.
pub fn properties_sections<H: EditorHost>(h: &mut H, ui: &mut egui::Ui) -> bool {
    let Some(b) = h.sel_bounds() else { return false };
    let units = h.units();
    let (mut x, mut y, mut w, mut hh) = (b.min.x, b.min.y, b.width(), b.height());
    let mut changed = None;
    section_title(ui, "move", &tr("Position and size"));
    let mut started = false;
    egui::Grid::new("props").num_columns(4).spacing([6.0, 6.0]).show(ui, |ui| {
        ui.spacing_mut().interact_size.x = 78.0;
        let f = |ui: &mut egui::Ui, label: &str, v: &mut f64| -> egui::Response {
            ui.label(label);
            drag_len(ui, units, v, 0.1, None)
        };
        let rx = f(ui, "X", &mut x);
        let ry = f(ui, "Y", &mut y);
        ui.end_row();
        let rw = f(ui, "W", &mut w);
        let rh = f(ui, "H", &mut hh);
        ui.end_row();
        for (r, which) in [(&rx, 0), (&ry, 1), (&rw, 2), (&rh, 3)] {
            if r.drag_started() || r.gained_focus() {
                started = true;
            }
            if r.changed() {
                changed = Some(which);
            }
        }
    });
    if started {
        h.checkpoint();
    }
    let mut lock = h.ed().lock_aspect;
    ui.checkbox(&mut lock, tr("Lock aspect ratio"));
    h.ed_mut().lock_aspect = lock;
    if let Some(which) = changed {
        let t = match which {
            0 | 1 => Xf::translate(x - b.min.x, y - b.min.y),
            _ => {
                let (mut sx, mut sy) = (if b.width() > 1e-9 { w / b.width() } else { 1.0 }, if b.height() > 1e-9 { hh / b.height() } else { 1.0 });
                if lock {
                    if which == 2 {
                        sy = sx;
                    } else {
                        sx = sy;
                    }
                }
                Xf::scale_about(sx.max(1e-4), sy.max(1e-4), b.min)
            }
        };
        h.transform_selection(t);
    }
    // Rotate, mirror and align: each section in its own block, buttons of one size.
    ui.separator();
    section_title(ui, "rotate-cw", &tr("Rotation"));
    let bw = (ui.available_width() - 12.0) / 3.0;
    let mut rot = h.ed().rot_input;
    let (mut apply, mut quarter) = (false, false);
    egui::Grid::new("props_rotate").num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
        ui.allocate_ui_with_layout(egui::vec2(bw, 24.0), egui::Layout::left_to_right(egui::Align::Center), |ui| {
            // A label as well would not fit in every language; the section title says what this is.
            ui.add(egui::DragValue::new(&mut rot).suffix("°")).on_hover_text(tr("Rotate"));
        });
        let ink = theme::text();
        apply = ui.add_sized([bw, 24.0], egui::Button::image_and_text(icons::slot(Some("check"), ink), tr("Apply")).truncate()).clicked();
        quarter = ui.add_sized([bw, 24.0], egui::Button::image_and_text(icons::slot(Some("rotate-cw"), ink), "90°")).clicked();
        ui.end_row();
    });
    h.ed_mut().rot_input = rot;
    if apply {
        h.rotate_sel(rot);
    }
    if quarter {
        h.rotate_sel(90.0);
    }
    ui.separator();
    section_title(ui, "flip-horizontal", &tr("Mirror and centre"));
    let flips = [(tr("Flip H"), tr("Flip horizontal"), "flip-horizontal"), (tr("Flip V"), tr("Flip vertical"), "flip-vertical"), (tr("Centre"), tr("Centre on bed"), "focus")];
    match button_grid(ui, "props_flip", &flips) {
        Some(0) => h.flip(true),
        Some(1) => h.flip(false),
        Some(2) => h.center_on_bed(),
        _ => {}
    }
    ui.separator();
    section_title(ui, "align-center-vertical", &tr("Alignment"));
    let aligns = [
        (tr("Left"), tr("Align left"), "align-start-vertical"),
        (tr("Mid-H"), tr("Align centre"), "align-center-vertical"),
        (tr("Right"), tr("Align right"), "align-end-vertical"),
        (tr("Top"), tr("Align top"), "align-start-horizontal"),
        (tr("Mid-V"), tr("Align middle"), "align-center-horizontal"),
        (tr("Bottom"), tr("Align bottom"), "align-end-horizontal"),
    ];
    if let Some(i) = button_grid(ui, "props_align", &aligns) {
        h.align(i as u8);
    }
    // A picture: adjust it, or trace it into outlines.
    let single_image = h.ed().sel.len() == 1 && h.scene().shape(h.ed().sel[0]).is_some_and(|s| matches!(s.kind, base_core::scene::Kind::Image(_)));
    if single_image {
        ui.separator();
        section_title(ui, "image", &tr("Image"));
        let bw = (ui.available_width() - ui.spacing().item_spacing.x) / 2.0;
        let ink = theme::text();
        let mut act = None;
        ui.horizontal(|ui| {
            for (a, label) in [(StdAct::AdjustImage, "Adjust image…"), (StdAct::TraceImage, "Trace image…")] {
                let btn = egui::Button::image_and_text(icons::slot(Some(a.icon()), ink), tr(label)).truncate();
                if ui.add_sized([bw, 24.0], btn).on_hover_text(tr(label)).clicked() {
                    act = Some(a);
                }
            }
        });
        if let Some(a) = act {
            a.run(h);
        }
    }
    true
}
