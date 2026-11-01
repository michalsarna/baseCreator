//! The fields of a device for the text of its G-code: the very start and the very end of the file and the flavour for
//! controllers that read G-code in their own way (see `base_core::gcode_out`). The same page in every program.
use crate::i18n::tr;
use crate::theme;
use base_core::gcode_out::{DwellWord, Flavour, Frame};
use egui::RichText;

/// The first thing wrong with the variables of the very start and end, or `Ok`.
pub fn check_frame(frame: &Frame, allowed: &[&str]) -> Result<(), String> {
    base_core::gcode_out::check_texts(&[("Very start of the file", &frame.very_start), ("Very end of the file", &frame.very_end)], allowed)
}

/// The line that lists the variables a text may use.
pub fn variables_line(ui: &mut egui::Ui, allowed: &[&str]) {
    let names: Vec<String> = allowed.iter().map(|v| format!("{{{v}}}")).collect();
    ui.label(RichText::new(format!("{} {}", tr("Variables you can use:"), names.join(" "))).small().color(theme::text_dim()));
}

/// The very start and the very end of the file: two boxes for text, under a line that lists the variables. Returns whether
/// anything changed. A variable that is not in `allowed` is shown in red.
pub fn frame_ui(ui: &mut egui::Ui, frame: &mut Frame, allowed: &[&str]) -> bool {
    let mut changed = false;
    for (label, text) in [("Very start of the file", &mut frame.very_start), ("Very end of the file", &mut frame.very_end)] {
        ui.label(tr(label));
        changed |= ui.add(egui::TextEdit::multiline(text).desired_rows(2).desired_width(f32::INFINITY).font(egui::TextStyle::Monospace)).changed();
        if let Err(e) = base_core::gcode_out::check_variables(text, allowed) {
            ui.label(RichText::new(e).small().color(egui::Color32::from_rgb(0xc0, 0x30, 0x30)));
        }
    }
    variables_line(ui, allowed);
    changed
}

/// The flavour: a checkbox for each thing and the word of the dwell. Returns whether anything changed.
pub fn flavour_ui(ui: &mut egui::Ui, f: &mut Flavour) -> bool {
    let mut changed = false;
    ui.label(RichText::new(tr("Own file format")).strong());
    changed |= ui.checkbox(&mut f.decimal_point, tr("Decimal point after whole numbers")).changed();
    changed |= ui.checkbox(&mut f.percent, tr("% at the start and end")).changed();
    ui.horizontal(|ui| {
        ui.label(tr("Dwell word"));
        egui::ComboBox::from_id_salt("gcode_dwell_word").selected_text(tr(f.dwell.label())).show_ui(ui, |ui| {
            for w in DwellWord::ALL {
                changed |= ui.selectable_value(&mut f.dwell, w, tr(w.label())).changed();
            }
        });
    });
    ui.horizontal(|ui| {
        changed |= ui.checkbox(&mut f.line_numbers, tr("Line numbers")).changed();
        if f.line_numbers {
            ui.label(tr("Step"));
            changed |= ui.add(egui::DragValue::new(&mut f.number_step).range(1..=1000)).changed();
        }
    });
    changed |= ui.checkbox(&mut f.remove_comments, tr("Remove comments")).changed();
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fields_draw_and_a_bad_variable_is_reported() {
        let ctx = egui::Context::default();
        let mut frame = Frame { very_start: "%\nO{project_name}".into(), very_end: "{nonsense}".into() };
        let mut flavour = Flavour { line_numbers: true, ..Flavour::default() };
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            frame_ui(ui, &mut frame, base_core::gcode_out::COMMON_VARS);
            flavour_ui(ui, &mut flavour);
        });
        out.textures_delta.clear();
        assert!(check_frame(&frame, base_core::gcode_out::COMMON_VARS).is_err());
        frame.very_end.clear();
        assert!(check_frame(&frame, base_core::gcode_out::COMMON_VARS).is_ok());
    }
}
