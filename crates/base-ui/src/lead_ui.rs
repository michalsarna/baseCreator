//! The rows of a layer's settings that set its leads (`base_core::lead`), the same in every program.
use crate::i18n::tr;
use crate::units_ui::drag_len;
use base_core::lead::{Lead, LeadKind};
use base_core::units::Units;

/// Rows of a two-column grid: the lead in, the lead out, their length (or radius) and angle, the overlap. True when something changed.
pub fn rows(ui: &mut egui::Ui, units: Units, l: &mut Lead) -> bool {
    let mut changed = false;
    let hint = tr("The tool comes onto the line from the waste side and leaves it the same way, so the mark where it starts is not on the part. A lead that does not fit is made smaller or left out.");
    for (label, id, kind) in [("Lead in", "lead_in", &mut l.lead_in), ("Lead out", "lead_out", &mut l.lead_out)] {
        ui.label(tr(label)).on_hover_text(hint);
        egui::ComboBox::from_id_salt(id).selected_text(tr(kind.label())).width(120.0).show_ui(ui, |ui| {
            for k in LeadKind::ALL {
                changed |= ui.selectable_value(kind, k, tr(k.label())).changed();
            }
        });
        ui.end_row();
    }
    let any = |k: LeadKind| l.lead_in == k || l.lead_out == k;
    if any(LeadKind::Line) || any(LeadKind::Arc) {
        ui.label(tr(if any(LeadKind::Line) { "Lead length" } else { "Lead radius" })).on_hover_text(tr("The length of a straight lead, the radius of an arc."));
        changed |= drag_len(ui, units, &mut l.length, 0.1, Some((0.1, 1000.0))).changed();
        ui.end_row();
    }
    if any(LeadKind::Line) {
        ui.label(tr("Lead angle"));
        changed |= ui.add(egui::DragValue::new(&mut l.angle).range(5.0..=90.0).suffix(" °")).on_hover_text(tr("90° comes straight at the line, a small angle almost along it.")).changed();
        ui.end_row();
    }
    ui.label(tr("Overlap"));
    changed |= drag_len(ui, units, &mut l.overlap, 0.1, Some((0.0, 1000.0))).on_hover_text(tr("How far the cut goes on past its start, so that the start and the end meet cleanly.")).changed();
    ui.end_row();
    changed
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_rows_draw() {
        let ctx = egui::Context::default();
        let mut l = base_core::lead::Lead { lead_in: base_core::lead::LeadKind::Line, lead_out: base_core::lead::LeadKind::Arc, ..Default::default() };
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                egui::Grid::new("g").show(ui, |ui| super::rows(ui, base_core::units::Units::Mm, &mut l));
            });
        });
        out.textures_delta.clear();
    }
}
