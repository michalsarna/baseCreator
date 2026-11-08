//! Drag-value widgets that store millimetres but show and edit the profile's display units.
use egui::{self, Response};
use base_core::{SpeedUnit, Units};

fn drag(ui: &mut egui::Ui, v: &mut f64, k: f64, suffix: &str, decimals: usize, speed_mm: f64, range: Option<(f64, f64)>) -> Response {
    let mut d = *v / k;
    let mut w = egui::DragValue::new(&mut d).speed(speed_mm / k).suffix(suffix).max_decimals(decimals);
    if let Some((lo, hi)) = range {
        w = w.range(lo / k..=hi / k);
    }
    let r = ui.add(w);
    if r.changed() {
        *v = d * k;
    }
    r
}

/// A length stored in millimetres.
pub fn drag_len(ui: &mut egui::Ui, u: Units, v: &mut f64, speed_mm: f64, range: Option<(f64, f64)>) -> Response {
    drag(ui, v, u.to_mm(1.0), u.suffix(), u.decimals(), speed_mm, range)
}

/// A speed stored in mm/s (`per_min` false) or mm/min, shown in the device's length and time units.
pub fn drag_speed(ui: &mut egui::Ui, u: Units, su: SpeedUnit, per_min: bool, v: &mut f64, speed: f64, range: Option<(f64, f64)>) -> Response {
    drag(ui, v, su.factor(u, per_min), su.suffix(u), u.decimals(), speed, range)
}

/// Format a length given in millimetres.
pub fn fmt_len(u: Units, mm: f64, decimals_mm: usize) -> String {
    match u {
        Units::Mm => format!("{:.*}{}", decimals_mm, mm, u.suffix()),
        Units::Cm => format!("{:.*}{}", decimals_mm + 1, u.from_mm(mm), u.suffix()),
        Units::M => format!("{:.*}{}", decimals_mm + 3, u.from_mm(mm), u.suffix()),
        Units::Inch => format!("{:.3}{}", u.from_mm(mm), u.suffix()),
        Units::Foot => format!("{:.4}{}", u.from_mm(mm), u.suffix()),
    }
}

/// Format a speed stored in mm/s (`per_min` false) or mm/min.
pub fn fmt_speed(u: Units, su: SpeedUnit, per_min: bool, v: f64) -> String {
    let d = v / su.factor(u, per_min);
    match (su, u) {
        (SpeedUnit::PerMinute, Units::Mm) => format!("{:.0}{}", d, su.suffix(u)),
        _ => format!("{:.1}{}", d, su.suffix(u)),
    }
}

/// A length in the units of the program, with the length in the units of the machine in brackets when one is metric and the other
/// imperial ("25.40 mm [1.000 in]").
pub fn fmt_len_both(u: Units, machine: Units, mm: f64, decimals_mm: usize) -> String {
    if u.is_metric() == machine.is_metric() {
        fmt_len(u, mm, decimals_mm)
    } else {
        format!("{} [{}]", fmt_len(u, mm, decimals_mm), fmt_len(machine, mm, decimals_mm))
    }
}

/// The row of the program options that chooses the units of the program: "As the device" (`None`) or one of them. True when it
/// changed.
pub fn display_units_row(ui: &mut egui::Ui, units: &mut Option<Units>) -> bool {
    use crate::i18n::tr;
    let before = *units;
    ui.label(tr("Units"));
    let shown = units.map_or_else(|| tr("As the device").to_string(), |u| tr(u.label()).to_string());
    egui::ComboBox::from_id_salt("display_units").selected_text(shown).show_ui(ui, |ui| {
        ui.selectable_value(units, None, tr("As the device"));
        for u in Units::ALL {
            ui.selectable_value(units, Some(u), tr(u.label()));
        }
    })
    .response
    .on_hover_text(tr("The units of the sizes in the properties, the ruler, the dimensions and the previews. The G-code is always in the units of the device."));
    ui.end_row();
    *units != before
}
