//! Tools of a machine: calibration of the steps per millimetre and of the squareness of the axes, and the diagnostics
//! window with the input pins, the statistics of the machine and its maintenance reminders.
use crate::i18n::{tr, trf};
use crate::machine_ctl::Machine;
use crate::theme;
use crate::window::{popup, save_button_enabled};
use base_core::calibration;
use base_core::control::Control;
use base_core::diag;
use base_core::grbl_codes;
use egui::{Color32, RichText};

const ORANGE: Color32 = Color32::from_rgb(0xe8, 0x8a, 0x1f);
const GREEN: Color32 = Color32::from_rgb(0x2e, 0xa0, 0x4f);

/// The calibration window.
pub struct CalUi {
    pub open: bool,
    axis: usize,
    distance: f64,
    measured: f64,
    width: f64,
    height: f64,
    d1: f64,
    d2: f64,
}

impl Default for CalUi {
    fn default() -> CalUi {
        CalUi {
            open: false,
            axis: 0,
            distance: 100.0,
            measured: 0.0,
            width: 300.0,
            height: 200.0,
            d1: 0.0,
            d2: 0.0,
        }
    }
}

/// What the calibration window needs from the program: the link and the settings of the controller.
pub trait CalHost {
    fn connected(&self) -> bool;
    /// The value of a setting (`$100` …) as it was read from the controller.
    fn setting(&self, id: u32) -> Option<f64>;
    fn reading(&self) -> bool;
    fn read_settings(&mut self);
    fn job_running(&self) -> bool;
    /// Send lines to the controller.
    fn send(&mut self, lines: Vec<String>);
    /// Ask whether to write a setting, and write it when the answer is yes.
    fn ask_write(&mut self, id: u32, value: String);
}

impl CalHost for Machine {
    fn connected(&self) -> bool {
        self.connected
    }
    fn setting(&self, id: u32) -> Option<f64> {
        self.fw
            .values
            .get(&id)
            .and_then(|v| v.trim().parse::<f64>().ok())
    }
    fn reading(&self) -> bool {
        self.fw.reading
    }
    fn read_settings(&mut self) {
        self.read_all();
    }
    fn job_running(&self) -> bool {
        Machine::job_running(self)
    }
    fn send(&mut self, lines: Vec<String>) {
        self.lines(lines);
    }
    fn ask_write(&mut self, id: u32, value: String) {
        self.fw.write_ask = Some((id, value));
    }
}

pub fn calibration_window<H: CalHost>(
    ctx: &egui::Context,
    st: &mut CalUi,
    m: &mut H,
) -> Option<String> {
    if !st.open {
        return None;
    }
    let mut open = st.open;
    popup(
        ctx,
        "calibration_window",
        &tr("Calibration"),
        [560.0, 560.0],
        &mut open,
        |ui| {
            if !m.connected() {
                ui.label(
                    RichText::new(tr("Connect to the machine first (Device tab)"))
                        .color(theme::text_dim()),
                );
                return;
            }
            // ---- steps per millimetre ----
            ui.label(RichText::new(tr("Steps per millimetre")).strong());
            ui.label(RichText::new(tr("The machine moves by a distance you ask for; measure how far it really went and the right number of steps is worked out.")).color(theme::text_dim()));
            ui.horizontal(|ui| {
                for (i, n) in ["X", "Y", "Z"].iter().enumerate() {
                    if ui.add(egui::Button::selectable(st.axis == i, *n)).clicked() {
                        st.axis = i;
                    }
                }
                ui.label(tr("Distance"));
                ui.add(
                    egui::DragValue::new(&mut st.distance)
                        .range(1.0..=2000.0)
                        .suffix(" mm"),
                );
            });
            let id = 100 + st.axis as u32;
            let current = m.setting(id);
            ui.horizontal(|ui| {
                match current {
                    Some(c) => ui.label(trf("${} is now {} steps/mm", &[&id, &c])),
                    None => ui.label(
                        RichText::new(tr("The settings have not been read yet.")).color(ORANGE),
                    ),
                };
                if ui
                    .add_enabled(
                        !m.reading(),
                        egui::Button::new(tr("Read the settings ($$)")),
                    )
                    .clicked()
                {
                    m.read_settings();
                }
            });
            ui.label(tr("1. Make a mark where the tool is (or put a dial gauge there). 2. Press Move. 3. Measure the distance and type it below."));
            ui.horizontal(|ui| {
                let axis = ["X", "Y", "Z"][st.axis];
                if ui
                    .add_enabled(
                        !m.job_running(),
                        egui::Button::new(trf("Move {} by the distance", &[&axis])),
                    )
                    .clicked()
                {
                    let d = st.distance;
                    m.send(vec![
                        "G91".to_string(),
                        format!("G1 {axis}{d} F1000"),
                        "G90".to_string(),
                    ]);
                }
                ui.label(tr("Measured"));
                ui.add(
                    egui::DragValue::new(&mut st.measured)
                        .range(0.0..=5000.0)
                        .speed(0.1)
                        .suffix(" mm"),
                );
            });
            let new =
                current.and_then(|c| calibration::corrected_steps(c, st.distance, st.measured));
            match (new, current) {
                (Some(n), Some(c)) => {
                    ui.label(
                        RichText::new(trf("New value: ${} = {} (was {})", &[&id, &n, &c])).strong(),
                    );
                    if save_button_enabled(ui, (n - c).abs() > 1e-9, &trf("Write ${}", &[&id]))
                        .clicked()
                    {
                        m.ask_write(id, format!("{n}"));
                    }
                }
                _ => {
                    ui.label(
                        RichText::new(tr("Type the measured distance to see the new value."))
                            .color(theme::text_dim()),
                    );
                }
            }
            ui.add_space(10.0);
            ui.separator();
            // ---- squareness ----
            ui.label(RichText::new(tr("Squareness of the axes")).strong());
            ui.label(RichText::new(tr("Mark or cut a rectangle and measure its two diagonals: from the front left corner to the back right one (A) and from the front right to the back left (B). They are equal when the axes are square.")).color(theme::text_dim()));
            egui::Grid::new("squaring")
                .num_columns(4)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label(tr("Width"));
                    ui.add(
                        egui::DragValue::new(&mut st.width)
                            .range(1.0..=5000.0)
                            .suffix(" mm"),
                    );
                    ui.label(tr("Height"));
                    ui.add(
                        egui::DragValue::new(&mut st.height)
                            .range(1.0..=5000.0)
                            .suffix(" mm"),
                    );
                    ui.end_row();
                    ui.label(tr("Diagonal A"));
                    ui.add(
                        egui::DragValue::new(&mut st.d1)
                            .range(0.0..=10000.0)
                            .speed(0.1)
                            .suffix(" mm"),
                    );
                    ui.label(tr("Diagonal B"));
                    ui.add(
                        egui::DragValue::new(&mut st.d2)
                            .range(0.0..=10000.0)
                            .speed(0.1)
                            .suffix(" mm"),
                    );
                    ui.end_row();
                });
            match calibration::skew(st.width, st.height, st.d1, st.d2) {
                Some(s) if s.shift_mm.abs() < 0.05 => {
                    ui.label(
                        RichText::new(trf(
                            "The axes are square within {} mm over the height.",
                            &[&format!("{:.2}", s.shift_mm.abs())],
                        ))
                        .color(GREEN),
                    );
                }
                Some(s) => {
                    ui.label(RichText::new(trf("Out of square by {}°: one end of the Y axis is {} mm off over the whole height.", &[&format!("{:.3}", s.angle_deg.abs()), &format!("{:.2}", s.shift_mm.abs())])).color(ORANGE));
                    let which = if s.angle_deg > 0.0 {
                        tr("Diagonal A is the longer one: the Y axis leans towards +X.")
                    } else {
                        tr("Diagonal B is the longer one: the Y axis leans towards −X.")
                    };
                    ui.label(which);
                }
                None => {
                    ui.label(
                        RichText::new(tr("Type both diagonals to see the result."))
                            .color(theme::text_dim()),
                    );
                }
            }
        },
    );
    st.open = open;
    None
}

/// The diagnostics window: what the machine and the program say, to read or to send to somebody.
#[derive(Default)]
pub struct DiagUi {
    pub open: bool,
}

/// What the diagnostics window needs from the program.
pub struct DiagInfo<'a> {
    /// `MillCreator`.
    pub title: &'a str,
    pub version: &'a str,
    /// The device as text (JSON) for the file.
    pub device: String,
    /// The G-code of the project, if any.
    pub gcode: Option<&'a str>,
}

const PINS: [(char, &str); 8] = [
    ('X', "X limit switch"),
    ('Y', "Y limit switch"),
    ('Z', "Z limit switch"),
    ('P', "Probe"),
    ('D', "Safety door"),
    ('H', "Feed hold"),
    ('R', "Reset"),
    ('S', "Cycle start"),
];

/// The text of the report: the machine, the firmware, the statistics.
pub fn report_text(title: &str, version: &str, m: &Machine, c: &Control) -> String {
    let mut s = format!("{title} {version}\n");
    s.push_str(&format!(
        "Controller: {}\n",
        if m.fw.info.is_known() {
            m.fw.info.label()
        } else {
            "unknown".into()
        }
    ));
    s.push_str(&format!(
        "State: {}\nConnected: {}\n",
        if m.state.is_empty() { "-" } else { &m.state },
        m.connected
    ));
    s.push_str(&format!(
        "Position (work): {:?}\nPosition (machine): {:?}\nActive inputs: {}\n",
        m.wpos,
        m.mpos,
        if m.pins.is_empty() { "none" } else { &m.pins }
    ));
    let st = &c.stats;
    s.push_str(&format!(
        "Jobs: {}\nRun time: {:.2} h\nAlarms: {} {:?}\nErrors: {} {:?}\n",
        st.jobs,
        st.hours(),
        st.alarms,
        st.alarm_codes,
        st.errors,
        st.error_codes
    ));
    for r in &c.reminders {
        s.push_str(&format!(
            "Reminder {}: every {} h, {} h left\n",
            r.name,
            r.every_hours,
            r.hours_left(st).round()
        ));
    }
    s
}

pub fn diagnostics_window(
    ctx: &egui::Context,
    st: &mut DiagUi,
    m: &mut Machine,
    c: &mut Control,
    info: &DiagInfo,
) -> Option<String> {
    if !st.open {
        return None;
    }
    let mut say = None;
    let mut open = st.open;
    popup(
        ctx,
        "diagnostics_window",
        &tr("Diagnostics and statistics"),
        [560.0, 600.0],
        &mut open,
        |ui| {
            ui.label(RichText::new(tr("Inputs")).strong());
            if !m.connected {
                ui.label(
                    RichText::new(tr("Connect to the machine to see its inputs."))
                        .color(theme::text_dim()),
                );
            } else {
                egui::Grid::new("pins")
                    .num_columns(4)
                    .spacing([14.0, 4.0])
                    .show(ui, |ui| {
                        for (i, (l, name)) in PINS.iter().enumerate() {
                            let on = m.pins.contains(*l);
                            let (txt, col) = if on {
                                ("●", ORANGE)
                            } else {
                                ("○", theme::text_dim())
                            };
                            ui.label(RichText::new(txt).color(col));
                            ui.label(tr(name));
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                ui.label(RichText::new(tr("An input that is active while nothing touches it points to a wiring or setting problem ($5, $6).")).small().color(theme::text_dim()));
            }
            ui.add_space(6.0);
            ui.label(RichText::new(tr("Controller")).strong());
            ui.label(if m.fw.info.is_known() {
                m.fw.info.label()
            } else {
                tr("Not read yet (Machine → Firmware settings).").to_string()
            });
            ui.add_space(6.0);
            ui.label(RichText::new(tr("Statistics of this machine")).strong());
            let s = &c.stats;
            egui::Grid::new("stats")
                .num_columns(2)
                .spacing([14.0, 2.0])
                .show(ui, |ui| {
                    ui.label(tr("Jobs"));
                    ui.label(s.jobs.to_string());
                    ui.end_row();
                    ui.label(tr("Time of running"));
                    ui.label(format!("{:.1} h", s.hours()));
                    ui.end_row();
                    ui.label(tr("Alarms"));
                    ui.label(s.alarms.to_string());
                    ui.end_row();
                    ui.label(tr("Errors"));
                    ui.label(s.errors.to_string());
                    ui.end_row();
                });
            let mut top: Vec<(&u32, &u32, bool)> = s
                .alarm_codes
                .iter()
                .map(|(k, v)| (k, v, true))
                .chain(s.error_codes.iter().map(|(k, v)| (k, v, false)))
                .collect();
            top.sort_by(|a, b| b.1.cmp(a.1));
            for (code, n, alarm) in top.into_iter().take(5) {
                let t = if alarm {
                    grbl_codes::alarm(*code)
                } else {
                    grbl_codes::error(*code)
                };
                let name = if alarm { "ALARM" } else { "error" };
                ui.label(
                    RichText::new(format!("{name}:{code} × {n}  {}", t.map(tr).unwrap_or("")))
                        .small()
                        .color(theme::text_dim()),
                );
            }
            if ui.button(tr("Clear the statistics")).clicked() {
                c.stats = Default::default();
                for r in &mut c.reminders {
                    r.done_at_hours = 0.0;
                }
            }
            ui.add_space(6.0);
            ui.label(RichText::new(tr("Maintenance")).strong());
            if c.reminders.is_empty() {
                ui.label(
                    RichText::new(tr(
                        "No reminders. Add them in the device settings (Maintenance).",
                    ))
                    .color(theme::text_dim()),
                );
            }
            let stats = c.stats.clone();
            for r in &mut c.reminders {
                ui.horizontal(|ui| {
                    if r.due(&stats) {
                        ui.label(
                            RichText::new(trf(
                                "{}: due ({} h over)",
                                &[&r.name, &format!("{:.0}", -r.hours_left(&stats))],
                            ))
                            .color(ORANGE)
                            .strong(),
                        );
                    } else {
                        ui.label(trf(
                            "{}: {} h left",
                            &[&r.name, &format!("{:.0}", r.hours_left(&stats))],
                        ));
                    }
                    if ui.small_button(tr("Done")).clicked() {
                        r.done_at_hours = stats.hours();
                    }
                });
            }
            ui.add_space(10.0);
            if save_button_enabled(ui, true, &tr("Export diagnostics (zip)…")).clicked() {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let name = format!(
                    "{}-diagnostics-{}.zip",
                    info.title.to_lowercase(),
                    base_core::backup::ymd(now)
                );
                if let Some(dest) = rfd::FileDialog::new()
                    .add_filter("Zip", &["zip"])
                    .set_file_name(&name)
                    .save_file()
                {
                    let console: String = m
                        .console
                        .iter()
                        .rev()
                        .take(500)
                        .map(|l| {
                            format!(
                                "{} {}\n",
                                match l.dir {
                                    crate::link::Dir::Tx => ">",
                                    crate::link::Dir::Rx => "<",
                                    crate::link::Dir::Info => "#",
                                },
                                l.text
                            )
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect();
                    let settings = base_core::grbl_settings::backup_text(
                        &m.fw.values,
                        &m.fw.info.raw,
                        &base_core::backup::ymd(now),
                    );
                    let report = report_text(info.title, info.version, m, c);
                    let gcode = info.gcode.unwrap_or_default().to_string();
                    let settings_text = if m.fw.values.is_empty() {
                        String::new()
                    } else {
                        settings
                    };
                    let parts = [
                        diag::Part {
                            name: "report.txt",
                            text: &report,
                        },
                        diag::Part {
                            name: "grbl-settings.txt",
                            text: &settings_text,
                        },
                        diag::Part {
                            name: "console.txt",
                            text: &console,
                        },
                        diag::Part {
                            name: "device.json",
                            text: &info.device,
                        },
                        diag::Part {
                            name: "program.gcode",
                            text: &gcode,
                        },
                    ];
                    say = Some(match diag::write(&dest, &parts) {
                        Ok(_) => trf("Diagnostics saved: {}", &[&dest.display()]),
                        Err(e) => trf("Export failed: {}", &[&e]),
                    });
                }
            }
        },
    );
    st.open = open;
    say
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_windows_draw_and_the_report_has_the_numbers() {
        let ctx = egui::Context::default();
        let mut m = Machine::new(&ctx);
        let mut c = Control::default();
        c.stats.job_done(3600);
        c.stats.alarm(1);
        c.reminders.push(base_core::control::Reminder {
            name: "Oil".into(),
            every_hours: 0.5,
            done_at_hours: 0.0,
        });
        let mut cal = CalUi {
            open: true,
            ..Default::default()
        };
        let mut diag_ui = DiagUi { open: true };
        let info = DiagInfo {
            title: "MillCreator",
            version: "v1",
            device: "{}".into(),
            gcode: None,
        };
        for connected in [false, true] {
            m.connected = connected;
            m.pins = "XP".into();
            m.fw.values.insert(100, "800.000".into());
            cal.measured = 98.0;
            cal.d1 = 360.5;
            cal.d2 = 360.0;
            let mut out = ctx.run_ui(egui::RawInput::default(), |_ui| {
                calibration_window(&ctx, &mut cal, &mut m);
                diagnostics_window(&ctx, &mut diag_ui, &mut m, &mut c, &info);
            });
            out.textures_delta.clear();
        }
        let r = report_text("MillCreator", "v1", &m, &c);
        assert!(
            r.contains("Jobs: 1") && r.contains("Run time: 1.00 h") && r.contains("Reminder Oil"),
            "{r}"
        );
        assert_eq!(PINS.len(), 8);
    }
}
