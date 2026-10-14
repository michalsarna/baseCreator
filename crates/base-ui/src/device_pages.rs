//! Pages of the device window for the things a machine needs besides its size: jogging, the probe, macros and the
//! G-code around a job, maintenance reminders, and the values read from the controller. Shared by the programs that drive a machine.
use crate::i18n::{tr, trf};
use crate::machine_ctl::Machine;
use crate::theme;
use crate::window::{delete_icon, DELETE_RED};
use base_core::control::{
    Control, Macro, ProbeKind, Reminder, MAX_MACROS, MAX_REMINDERS, VARIABLES,
};
use base_core::firmware::Suggestion;
use egui::{Color32, RichText, Ui};

fn steps_row(ui: &mut Ui, label: &str, steps: &mut Vec<f64>) {
    ui.horizontal_wrapped(|ui| {
        ui.label(label);
        let mut remove = None;
        let count = steps.len();
        for (i, s) in steps.iter_mut().enumerate() {
            ui.add(
                egui::DragValue::new(s)
                    .range(0.01..=1000.0)
                    .speed(0.05)
                    .max_decimals(3),
            );
            if count > 1
                && ui
                    .small_button("−")
                    .on_hover_text(tr("Remove this step"))
                    .clicked()
            {
                remove = Some(i);
            }
        }
        if let Some(i) = remove {
            steps.remove(i);
        }
        if steps.len() < 6
            && ui
                .small_button("+")
                .on_hover_text(tr("Add a step"))
                .clicked()
        {
            let next = steps.last().map_or(1.0, |l| l * 2.0).min(1000.0);
            steps.push(next);
        }
    });
}

/// Jog steps and feeds, and the check of the program before a run.
pub fn jog_page(ui: &mut Ui, c: &mut Control) {
    ui.label(RichText::new(tr("Jogging")).strong());
    steps_row(ui, &tr("Steps of X and Y (mm)"), &mut c.jog.xy_steps);
    steps_row(ui, &tr("Steps of Z (mm)"), &mut c.jog.z_steps);
    ui.horizontal(|ui| {
        ui.label(tr("Speed XY"));
        ui.add(
            egui::DragValue::new(&mut c.jog.xy_feed)
                .range(1.0..=100_000.0)
                .speed(50.0)
                .suffix(" mm/min"),
        );
        ui.label(tr("Speed Z"));
        ui.add(
            egui::DragValue::new(&mut c.jog.z_feed)
                .range(1.0..=100_000.0)
                .speed(10.0)
                .suffix(" mm/min"),
        );
    });
    ui.add_space(6.0);
    ui.checkbox(&mut c.check_first, tr("Check the program before every run")).on_hover_text(tr("The controller goes through the program in check mode ($C): nothing moves, and the lines it does not understand are reported before the real run"));
}

/// The probe: a touch plate or a 3D probe.
pub fn probe_page(ui: &mut Ui, c: &mut Control) {
    let p = &mut c.probe;
    ui.label(RichText::new(tr("Probe")).strong());
    ui.horizontal(|ui| {
        for k in ProbeKind::ALL {
            if ui
                .add(egui::Button::selectable(p.kind == k, tr(k.label())))
                .clicked()
            {
                p.kind = k;
            }
        }
    });
    let mm = |ui: &mut Ui, label: &str, v: &mut f64, hi: f64| {
        ui.label(label);
        ui.add(
            egui::DragValue::new(v)
                .range(0.0..=hi)
                .speed(0.1)
                .max_decimals(3)
                .suffix(" mm"),
        );
        ui.end_row();
    };
    egui::Grid::new("probe_grid")
        .num_columns(2)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            match p.kind {
                ProbeKind::Plate => {
                    mm(ui, &tr("Thickness of the plate"), &mut p.z_offset, 200.0);
                    mm(ui, &tr("Thickness of its side"), &mut p.side_offset, 200.0);
                }
                ProbeKind::Touch3d => {
                    mm(ui, &tr("Offset of Z at the touch"), &mut p.z_offset, 200.0);
                    mm(ui, &tr("Diameter of the ball"), &mut p.ball_diameter, 50.0);
                }
            }
            mm(ui, &tr("Fail distance"), &mut p.fail_distance, 1000.0);
            mm(
                ui,
                &tr("Retract between the touches"),
                &mut p.retract,
                100.0,
            );
            mm(ui, &tr("Lift after the zero is set"), &mut p.lift, 200.0);
            ui.label(tr("Quick touch"));
            ui.add(
                egui::DragValue::new(&mut p.seek_feed)
                    .range(1.0..=10_000.0)
                    .speed(5.0)
                    .suffix(" mm/min"),
            );
            ui.end_row();
            ui.label(tr("Slow touch"));
            ui.add(
                egui::DragValue::new(&mut p.slow_feed)
                    .range(1.0..=10_000.0)
                    .speed(1.0)
                    .suffix(" mm/min"),
            );
            ui.end_row();
        });
    if let Err(e) = p.check() {
        ui.label(RichText::new(tr(e)).color(DELETE_RED));
    }
    ui.add_space(4.0);
    ui.label(RichText::new(tr("The probe must be connected to the probe input of the controller. Test it by touching the plate: the input shows as closed on the Control tab.")).small().color(theme::text_dim()));
}

/// Macros and the G-code run around a job.
pub fn macros_page(ui: &mut Ui, c: &mut Control) {
    ui.label(RichText::new(tr("Macros")).strong());
    ui.label(RichText::new(tr("A macro is G-code that a button on the Control tab sends. It may use variables in brackets.")).color(theme::text_dim()));
    ui.label(
        RichText::new(trf(
            "Variables: {}",
            &[&VARIABLES
                .iter()
                .map(|v| format!("[{v}]"))
                .collect::<Vec<_>>()
                .join(" ")],
        ))
        .small()
        .color(theme::text_dim()),
    );
    let mut remove = None;
    for (i, m) in c.macros.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut m.name)
                    .desired_width(160.0)
                    .hint_text(tr("Name")),
            );
            if delete_icon(ui, &tr("Delete this macro")).clicked() {
                remove = Some(i);
            }
        });
        ui.add(
            egui::TextEdit::multiline(&mut m.code)
                .desired_width(f32::INFINITY)
                .desired_rows(3)
                .font(egui::TextStyle::Monospace),
        );
        // A line that cannot be sent is shown at once.
        let vars: std::collections::BTreeMap<String, f64> =
            VARIABLES.iter().map(|v| (v.to_string(), 0.0)).collect();
        if let Err(e) = base_core::control::expand(&m.code, &vars) {
            ui.label(RichText::new(e).small().color(DELETE_RED));
        }
        ui.add_space(4.0);
    }
    if let Some(i) = remove {
        c.macros.remove(i);
    }
    if c.macros.len() < MAX_MACROS && ui.button(tr("Add a macro")).clicked() {
        c.macros.push(Macro {
            name: trf("Macro {}", &[&(c.macros.len() + 1)]),
            code: String::new(),
        });
    }
    ui.add_space(8.0);
    ui.label(RichText::new(tr("Around a job")).strong());
    ui.label(tr("Before the job starts"));
    ui.add(
        egui::TextEdit::multiline(&mut c.hooks.before_job)
            .desired_width(f32::INFINITY)
            .desired_rows(2)
            .font(egui::TextStyle::Monospace),
    );
    ui.label(tr("After the job (finished or stopped)"));
    ui.add(
        egui::TextEdit::multiline(&mut c.hooks.after_job)
            .desired_width(f32::INFINITY)
            .desired_rows(2)
            .font(egui::TextStyle::Monospace),
    );
}

/// Maintenance reminders and the running time.
pub fn maintenance_page(ui: &mut Ui, c: &mut Control) {
    ui.label(RichText::new(tr("Maintenance")).strong());
    ui.label(
        RichText::new(trf(
            "The machine has run {} hours in {} jobs.",
            &[&format!("{:.1}", c.stats.hours()), &c.stats.jobs],
        ))
        .color(theme::text_dim()),
    );
    let hours = c.stats.hours();
    let mut remove = None;
    for (i, r) in c.reminders.iter_mut().enumerate() {
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut r.name)
                    .desired_width(200.0)
                    .hint_text(tr("What to do")),
            );
            ui.label(tr("every"));
            ui.add(
                egui::DragValue::new(&mut r.every_hours)
                    .range(0.0..=100_000.0)
                    .speed(1.0)
                    .suffix(" h"),
            );
            if ui
                .small_button(tr("Done"))
                .on_hover_text(tr("It was just done"))
                .clicked()
            {
                r.done_at_hours = hours;
            }
            if delete_icon(ui, &tr("Delete this reminder")).clicked() {
                remove = Some(i);
            }
        });
    }
    if let Some(i) = remove {
        c.reminders.remove(i);
    }
    if c.reminders.len() < MAX_REMINDERS && ui.button(tr("Add a reminder")).clicked() {
        c.reminders.push(Reminder {
            name: tr("Oil the rails").to_string(),
            every_hours: 50.0,
            done_at_hours: hours,
        });
    }
}

/// What the controller says about the machine, with a button to use it. `true` when the button was pressed.
pub fn suggestion_ui(ui: &mut Ui, m: &mut Machine, s: &Suggestion) -> bool {
    let mut apply = false;
    ui.label(RichText::new(tr("From the controller")).strong());
    if !m.connected {
        ui.label(
            RichText::new(tr("Connect to the machine first (Device tab)")).color(theme::text_dim()),
        );
        return false;
    }
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(
                !m.fw.reading,
                egui::Button::new(if m.fw.reading {
                    tr("Reading…")
                } else {
                    tr("Read the machine")
                }),
            )
            .on_hover_text(tr(
                "Reads the firmware ($I) and the settings ($$) of the controller",
            ))
            .clicked()
        {
            m.read_all();
        }
        if m.fw.info.is_known() {
            ui.label(m.fw.info.label());
        }
    });
    if s.is_empty() {
        ui.label(RichText::new(tr("Nothing read yet.")).color(theme::text_dim()));
        return false;
    }
    egui::Grid::new("suggestion")
        .num_columns(2)
        .spacing([14.0, 2.0])
        .show(ui, |ui| {
            if let Some(a) = s.area {
                ui.label(tr("Travel of X, Y, Z"));
                ui.label(format!("{} × {} × {} mm", a[0], a[1], a[2]));
                ui.end_row();
            }
            if let Some((lo, hi)) = s.spindle {
                ui.label(tr("Spindle speed"));
                ui.label(format!("{lo} – {hi}"));
                ui.end_row();
            }
            if let Some(r) = s.rapid {
                ui.label(tr("Fastest move of X and Y"));
                ui.label(format!("{r} mm/min"));
                ui.end_row();
            }
            ui.label(tr("Homing"));
            ui.label(if s.homing { tr("on") } else { tr("off") });
            ui.end_row();
            ui.label(tr("Soft limits"));
            ui.label(if s.soft_limits { tr("on") } else { tr("off") });
            ui.end_row();
        });
    if s.inches {
        ui.label(RichText::new(tr("The controller reports in inches ($13): the positions on the Control tab are not in millimetres.")).color(Color32::from_rgb(0xe8, 0x8a, 0x1f)));
    }
    if ui.button(tr("Use these values for the device")).clicked() {
        apply = true;
    }
    apply
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pages_draw() {
        let ctx = egui::Context::default();
        let mut c = Control::default();
        c.macros.push(Macro {
            name: "Park".into(),
            code: "G0 Z[safez]\n$RST=*".into(),
        });
        c.reminders.push(Reminder {
            name: "Oil".into(),
            every_hours: 10.0,
            done_at_hours: 0.0,
        });
        let mut m = Machine::new(&ctx);
        m.connected = true;
        let s = Suggestion {
            area: Some([300.0, 180.0, 45.0]),
            spindle: Some((0.0, 24000.0)),
            rapid: Some(3000.0),
            ..Default::default()
        };
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                jog_page(ui, &mut c);
                probe_page(ui, &mut c);
                macros_page(ui, &mut c);
                maintenance_page(ui, &mut c);
                suggestion_ui(ui, &mut m, &s);
            });
        });
        out.textures_delta.clear();
    }
}

/// The copies of the settings of the controller in the device window: on the page of the machine.
pub fn copies_section(ui: &mut Ui, m: &mut Machine, c: &mut Control) -> Option<String> {
    let act = crate::firmware_ui::copies_ui(ui, &c.fw_copies, !m.fw.values.is_empty(), &[])?;
    crate::firmware_ui::run_copy_act(m, &mut c.fw_copies, act)
}
