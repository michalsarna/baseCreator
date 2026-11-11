//! A rotary attachment in the programs that drive GRBL with three axes (MillCreator, PenCreator): the page of the device settings,
//! and the lines on the work area where the design wraps round the workpiece (`base_core::machine::Rotary`, the Y motor turns the
//! work).
use crate::i18n::{tr, trf};
use crate::theme;
use crate::units_ui::{drag_len, fmt_len};
use base_core::geom::Pt;
use base_core::machine::{Rotary, RotaryKind};
use base_core::units::Units;
use egui::{Color32, RichText, Stroke};

/// The title of the page in the device settings.
pub const PAGE: &str = "Rotary";

/// The colour of the lines of the way round.
const INK: Color32 = Color32::from_rgb(0x2e, 0xa0, 0xc0);

/// The page: on or off, the kind, the diameters, the microsteps and what the controller needs. True when something changed.
pub fn page(ui: &mut egui::Ui, units: Units, r: &mut Rotary) -> bool {
    let before = r.clone();
    ui.checkbox(&mut r.enabled, tr("A rotary attachment is connected"));
    ui.add_enabled_ui(r.enabled, |ui| {
        egui::Grid::new("dev_rotary").num_columns(2).min_col_width(150.0).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Kind"));
            ui.horizontal(|ui| {
                ui.radio_value(&mut r.kind, RotaryKind::Chuck, tr("Chuck"));
                ui.radio_value(&mut r.kind, RotaryKind::Roller, tr("Rollers"));
            });
            ui.end_row();
            ui.label(tr("Workpiece diameter"));
            drag_len(ui, units, &mut r.diameter, 0.5, Some((1.0, 1000.0)));
            ui.end_row();
            if r.kind == RotaryKind::Roller {
                ui.label(tr("Roller diameter"));
                drag_len(ui, units, &mut r.roller_diameter, 0.5, Some((1.0, 1000.0)));
                ui.end_row();
            }
            ui.label(tr("Microsteps per turn"));
            ui.add(egui::DragValue::new(&mut r.microsteps_per_rev).range(1.0..=1_000_000.0)).on_hover_text(tr("Microsteps of the motor for one full turn of what it drives (the chuck, or one roller)."));
            ui.end_row();
            ui.label(tr("Circumference"));
            ui.label(fmt_len(units, r.circumference(), 2)).on_hover_text(tr("The way round the workpiece: the design may not be higher."));
            ui.end_row();
        });
        ui.label(RichText::new(tr("GRBL 1.1 has three axes: the A axis is not available, Y is used.")).color(Color32::from_rgb(0xe8, 0x8a, 0x1f)));
        ui.label(trf("The controller needs {} steps per mm on the Y axis ($101).", &[&format!("{:.3}", r.steps_per_mm())]));
    });
    ui.add_space(4.0);
    ui.label(RichText::new(tr("With a rotary attachment the Y axis turns the work: the drawing may not be higher than the way round the workpiece. The model is simple (a chuck turns by the workpiece diameter, rollers move the surface like their own); a real machine needs testing.")).color(theme::text_dim()));
    *r != before
}

/// The heights of the design (from its top edge, millimetres) where it comes round to its start again: one line for every whole way
/// round that fits into the work area of the height `work_h`.
pub fn wrap_lines(r: &Rotary, work_h: f64) -> Vec<f64> {
    let c = r.circumference();
    if !r.enabled || c <= 1.0 {
        return vec![];
    }
    (1..).map(|k| c * k as f64).take_while(|y| *y < work_h).take(1000).collect()
}

/// Draw the lines of the way round across the work area (`bed` on the screen, `to` maps design millimetres onto it), the first one
/// with the circumference written over it, as LightCreator does.
pub fn paint(painter: &egui::Painter, to: &dyn Fn(Pt) -> egui::Pos2, bed: egui::Rect, units: Units, r: &Rotary, work_h: f64) {
    let lines = wrap_lines(r, work_h);
    for (i, y) in lines.iter().enumerate() {
        let sy = to(Pt::new(0.0, *y)).y;
        painter.line_segment([egui::pos2(bed.min.x, sy), egui::pos2(bed.max.x, sy)], Stroke::new(1.0, INK));
        if i == 0 {
            painter.text(egui::pos2(bed.min.x + 4.0, sy - 2.0), egui::Align2::LEFT_BOTTOM, format!("{} {}", tr("Circumference"), fmt_len(units, r.circumference(), 1)), egui::FontId::proportional(11.0), INK);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lines_come_at_every_way_round_inside_the_work_area() {
        let mut r = Rotary { diameter: 50.0, ..Rotary::default() };
        assert!(wrap_lines(&r, 400.0).is_empty(), "off");
        r.enabled = true;
        let c = r.circumference();
        assert_eq!(wrap_lines(&r, 400.0), vec![c, 2.0 * c]);
        assert!(wrap_lines(&r, c - 1.0).is_empty());
    }
}
