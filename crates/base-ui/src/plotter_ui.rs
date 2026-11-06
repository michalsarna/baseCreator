//! The settings of a plotter that does not speak G-code (EBB, HPGL), as rows of the grid of a device's connection page.
use crate::i18n::tr;
use base_core::machine::GrblVersion;
use base_core::plotter::{FlowControl, Kinematics, PlotterSettings};
use egui::{Color32, RichText};

const WARN: Color32 = Color32::from_rgb(0xe8, 0x8a, 0x1f);

/// The rows for `version` (nothing for a GRBL); each row is a label and its fields, the grid's `end_row` included.
pub fn settings_rows(ui: &mut egui::Ui, version: GrblVersion, s: &mut PlotterSettings) {
    match version {
        GrblVersion::Ebb => {
            ui.label(tr("Motors"));
            egui::ComboBox::from_id_salt("plotter_kinematics").selected_text(tr(s.kinematics.label())).show_ui(ui, |ui| {
                for k in Kinematics::ALL {
                    ui.selectable_value(&mut s.kinematics, k, tr(k.label()));
                }
            });
            ui.end_row();
            ui.label(tr("Steps per mm"));
            ui.add(egui::DragValue::new(&mut s.steps_per_mm).range(1.0..=10_000.0).speed(0.5)).on_hover_text(tr("The AxiDraw has 80 steps per mm and counts Y towards the front, so its Y is reversed."));
            ui.end_row();
            ui.label(tr("Pen up"));
            ui.add(egui::Slider::new(&mut s.pen_up_pct, 0.0..=100.0).suffix(" %"));
            ui.end_row();
            ui.label(tr("Pen down"));
            ui.add(egui::Slider::new(&mut s.pen_down_pct, 0.0..=100.0).suffix(" %"));
            ui.end_row();
            ui.label(tr("Reverse"));
            ui.horizontal(|ui| {
                ui.checkbox(&mut s.reverse_x, "X");
                ui.checkbox(&mut s.reverse_y, "Y");
            });
            ui.end_row();
            ui.label("");
            ui.label(RichText::new(tr("The EBB has no limit switches: before connecting, put the pen at the corner where the drawing starts (its zero).")).color(WARN));
            ui.end_row();
        }
        GrblVersion::Hpgl => {
            ui.label(tr("Plotter units per mm"));
            ui.add(egui::DragValue::new(&mut s.hpgl_units_per_mm).range(1.0..=1000.0).speed(0.1));
            ui.end_row();
            ui.label(tr("Flow control"));
            ui.vertical(|ui| {
                egui::ComboBox::from_id_salt("plotter_flow").selected_text(tr(s.flow.label())).show_ui(ui, |ui| {
                    for f in FlowControl::ALL {
                        ui.selectable_value(&mut s.flow, f, tr(f.label()));
                    }
                });
                if s.flow == FlowControl::None {
                    ui.label(RichText::new(tr("Without flow control a plotter loses commands once its buffer is full.")).color(WARN));
                }
            });
            ui.end_row();
            ui.label("");
            ui.label(RichText::new(tr("A plotter answers nothing: a line counts as done once the port took it, and a stop lifts the pen while the plotter still draws what is in its buffer.")).color(crate::theme::text_dim()));
            ui.end_row();
        }
        _ => {}
    }
}
