//! The Control tab of a machine: the position (work and machine), the work coordinate system and its zero, jogging,
//! the probe and the macros. The same in every program that drives a GRBL machine.
use crate::i18n::{tr, trf};
use crate::machine_ctl::Machine;
use crate::theme;
use base_core::control::{Axis, Control, ProbeKind};
use egui::{Color32, RichText, Sense, Ui};
use std::time::{Duration, Instant};

/// What the tab remembers between frames.
pub struct ControlUi {
    pub step_xy: usize,
    pub step_z: usize,
    /// Hold a button to move, release to stop, instead of one step per click.
    pub continuous: bool,
    pub feed_xy: Option<f64>,
    pub feed_z: Option<f64>,
    held: Option<[i8; 3]>,
    last_segment: Option<Instant>,
    /// The axis whose work position is being typed, and the text.
    edit: Option<(usize, String)>,
    /// The diameter of the tool for edge probing, typed by the person; the program's guess until then.
    pub tool_d: Option<f64>,
}

impl Default for ControlUi {
    fn default() -> ControlUi {
        ControlUi {
            step_xy: 1,
            step_z: 1,
            continuous: false,
            feed_xy: None,
            feed_z: None,
            held: None,
            last_segment: None,
            edit: None,
            tool_d: None,
        }
    }
}

/// What the tab needs to know about the machine and the project.
pub struct ControlCtx<'a> {
    pub control: &'a Control,
    /// The height above the work zero where the tool moves freely, mm.
    pub safe_z: f64,
    /// The diameter of the tool that is in the machine now (the bit, the pen); 0 when there is none.
    pub tool_diameter: f64,
    /// The box of the job, for the variables of macros.
    pub job_box: Option<([f64; 3], [f64; 3])>,
    /// Show the probe section.
    pub probe: bool,
    /// How far the axes go, mm.
    pub travel: [f64; 3],
}

const AXES: [(Axis, &str); 3] = [(Axis::X, "X"), (Axis::Y, "Y"), (Axis::Z, "Z")];

fn fmt_pos(v: f64) -> String {
    if v.is_finite() {
        format!("{v:.3}")
    } else {
        "–".into()
    }
}

fn step_label(v: f64) -> String {
    format!("{v}")
}

/// Draw the tab. Returns a line for the status bar when something was refused or went wrong.
pub fn panel(ui: &mut Ui, m: &mut Machine, st: &mut ControlUi, c: &ControlCtx) -> Option<String> {
    let mut say = None;
    let on = m.connected;
    ui.add_enabled_ui(on, |ui| {
        say = body(ui, m, st, c);
    });
    if !on {
        ui.add_space(4.0);
        ui.label(
            RichText::new(tr("Connect to the machine first (Device tab)")).color(theme::text_dim()),
        );
    }
    say
}

fn body(ui: &mut Ui, m: &mut Machine, st: &mut ControlUi, c: &ControlCtx) -> Option<String> {
    let mut say = None;
    // At a pause of the job (a change of the bit) the machine is free to be moved, probed and zeroed.
    let busy = (m.job_running() || m.job_active()) && m.barrier.is_none();
    // ---- state ----
    ui.horizontal_wrapped(|ui| {
        let alarm = m.is_alarm();
        let state = if m.state.is_empty() {
            "–".to_string()
        } else {
            m.state.clone()
        };
        ui.label(RichText::new(state).strong().color(if alarm {
            Color32::from_rgb(0xc0, 0x30, 0x30)
        } else {
            theme::text()
        }));
        if m.fw.info.is_known() {
            ui.label(RichText::new(m.fw.info.label()).color(theme::text_dim()));
        }
        if alarm
            && ui
                .button(tr("Unlock"))
                .on_hover_text(tr("Clear an alarm ($X)"))
                .clicked()
        {
            m.unlock();
        }
    });
    if let Some(code) = m.alarm.filter(|c| *c > 0) {
        if let Some(t) = base_core::grbl_codes::alarm(code) {
            ui.label(
                RichText::new(trf("Alarm {}: {}", &[&code, &tr(t)]))
                    .color(Color32::from_rgb(0xc0, 0x30, 0x30)),
            );
        }
    }
    ui.add_space(4.0);

    // ---- the position ----
    ui.label(RichText::new(tr("Position")).strong());
    egui::Grid::new("dro")
        .num_columns(4)
        .spacing([12.0, 4.0])
        .show(ui, |ui| {
            ui.label("");
            ui.label(RichText::new(tr("Work")).color(theme::text_dim()));
            ui.label(RichText::new(tr("Machine")).color(theme::text_dim()));
            ui.label("");
            ui.end_row();
            for (i, (axis, name)) in AXES.iter().enumerate() {
                ui.label(RichText::new(*name).strong());
                if st.edit.as_ref().is_some_and(|e| e.0 == i) {
                    let (_, text) = st.edit.as_mut().unwrap();
                    let r = ui.add(
                        egui::TextEdit::singleline(text)
                            .desired_width(80.0)
                            .font(egui::TextStyle::Monospace),
                    );
                    r.request_focus();
                    let enter = ui.input(|inp| inp.key_pressed(egui::Key::Enter));
                    let esc = ui.input(|inp| inp.key_pressed(egui::Key::Escape));
                    if enter {
                        if let Ok(v) = text.trim().replace(',', ".").parse::<f64>() {
                            m.set_position(&[(*axis, v)]);
                        }
                        st.edit = None;
                    } else if esc || (r.lost_focus() && !enter) {
                        st.edit = None;
                    }
                } else if ui
                    .add(
                        egui::Button::new(RichText::new(fmt_pos(m.wpos[i])).monospace())
                            .frame(false),
                    )
                    .on_hover_text(tr(
                        "Click to type the position of the tool in the work coordinates",
                    ))
                    .clicked()
                {
                    st.edit = Some((
                        i,
                        if m.wpos[i].is_finite() {
                            format!("{:.3}", m.wpos[i])
                        } else {
                            "0".into()
                        },
                    ));
                }
                ui.label(
                    RichText::new(fmt_pos(m.mpos[i]))
                        .monospace()
                        .color(theme::text_dim()),
                );
                if ui
                    .add_enabled(!busy, egui::Button::new(format!("{name}0")))
                    .on_hover_text(tr("The tool is here: make this the zero of the work"))
                    .clicked()
                {
                    m.set_position(&[(*axis, 0.0)]);
                }
                ui.end_row();
            }
        });
    ui.horizontal_wrapped(|ui| {
        if ui
            .add_enabled(!busy, egui::Button::new(tr("Zero all")))
            .clicked()
        {
            m.set_position(&[(Axis::X, 0.0), (Axis::Y, 0.0), (Axis::Z, 0.0)]);
        }
        if ui
            .add_enabled(!busy, egui::Button::new(tr("Zero X and Y")))
            .clicked()
        {
            m.set_position(&[(Axis::X, 0.0), (Axis::Y, 0.0)]);
        }
        ui.label(RichText::new(tr("Coordinates")).color(theme::text_dim()));
        let names = ["G54", "G55", "G56", "G57", "G58", "G59"];
        let mut wcs = m.wcs.clamp(1, 6) as usize - 1;
        egui::ComboBox::from_id_salt("wcs")
            .selected_text(names[wcs])
            .width(60.0)
            .show_ui(ui, |ui| {
                for (i, n) in names.iter().enumerate() {
                    if ui.selectable_value(&mut wcs, i, *n).clicked() {
                        m.select_wcs(i as u8 + 1);
                    }
                }
            });
    });
    if !m.pins.is_empty() {
        ui.label(
            RichText::new(trf("Inputs active: {}", &[&pins_text(&m.pins)]))
                .small()
                .color(theme::text_dim()),
        );
    }
    ui.add_space(6.0);

    // ---- jogging ----
    ui.label(RichText::new(tr("Move")).strong());
    let xy_steps = c.control.jog.xy_steps.clone();
    let z_steps = c.control.jog.z_steps.clone();
    st.step_xy = st.step_xy.min(xy_steps.len().saturating_sub(1));
    st.step_z = st.step_z.min(z_steps.len().saturating_sub(1));
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("XY").color(theme::text_dim()));
        for (i, s) in xy_steps.iter().enumerate() {
            if ui
                .add(egui::Button::selectable(st.step_xy == i, step_label(*s)))
                .clicked()
            {
                st.step_xy = i;
            }
        }
        ui.label(RichText::new("Z").color(theme::text_dim()));
        for (i, s) in z_steps.iter().enumerate() {
            if ui
                .add(egui::Button::selectable(st.step_z == i, step_label(*s)))
                .clicked()
            {
                st.step_z = i;
            }
        }
    });
    let mut feed_xy = st.feed_xy.unwrap_or(c.control.jog.xy_feed);
    let mut feed_z = st.feed_z.unwrap_or(c.control.jog.z_feed);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Speed XY")).color(theme::text_dim()));
        ui.add(
            egui::DragValue::new(&mut feed_xy)
                .range(1.0..=100_000.0)
                .speed(50.0)
                .suffix(" mm/min"),
        );
        ui.label(RichText::new(tr("Z")).color(theme::text_dim()));
        ui.add(
            egui::DragValue::new(&mut feed_z)
                .range(1.0..=100_000.0)
                .speed(10.0)
                .suffix(" mm/min"),
        );
        ui.checkbox(&mut st.continuous, tr("Hold to move"))
            .on_hover_text(tr(
                "Instead of one step per click, the tool moves while the button is held down",
            ));
    });
    st.feed_xy = Some(feed_xy);
    st.feed_z = Some(feed_z);
    let step_xy = xy_steps.get(st.step_xy).copied().unwrap_or(1.0);
    let step_z = z_steps.get(st.step_z).copied().unwrap_or(1.0);
    let mut want: Option<[i8; 3]> = None;
    ui.add_enabled_ui(!busy, |ui| {
        ui.horizontal(|ui| {
            egui::Grid::new("jog_xy")
                .spacing([2.0, 2.0])
                .show(ui, |ui| {
                    // The picture of the machine seen from above: Y up is towards the back.
                    let cells: [[(&str, [i8; 3]); 3]; 3] = [
                        [("↖", [-1, 1, 0]), ("▲", [0, 1, 0]), ("↗", [1, 1, 0])],
                        [("◀", [-1, 0, 0]), ("•", [0, 0, 0]), ("▶", [1, 0, 0])],
                        [("↙", [-1, -1, 0]), ("▼", [0, -1, 0]), ("↘", [1, -1, 0])],
                    ];
                    for row in cells {
                        for (t, d) in row {
                            if d == [0, 0, 0] {
                                if ui
                                    .add_sized(
                                        [34.0, 30.0],
                                        egui::Button::new(
                                            RichText::new("■")
                                                .color(Color32::from_rgb(0xc0, 0x30, 0x30)),
                                        ),
                                    )
                                    .on_hover_text(tr("Stop the move"))
                                    .clicked()
                                {
                                    m.cancel_jog();
                                }
                            } else if let Some(w) = jog_button(ui, t, d) {
                                want = Some(w);
                                if !st.continuous {
                                    m.jog(
                                        d[0] as f64 * step_xy,
                                        d[1] as f64 * step_xy,
                                        0.0,
                                        feed_xy,
                                    );
                                    want = None;
                                }
                            }
                        }
                        ui.end_row();
                    }
                });
            ui.add_space(8.0);
            ui.vertical(|ui| {
                for (t, d) in [("Z▲", [0, 0, 1]), ("Z▼", [0, 0, -1])] {
                    if let Some(w) = jog_button(ui, t, d) {
                        want = Some(w);
                        if !st.continuous {
                            m.jog(0.0, 0.0, d[2] as f64 * step_z, feed_z);
                            want = None;
                        }
                    }
                }
            });
        });
    });
    // Hold to move: short moves one after the other while the button is down, a jog cancel when it is let go.
    if st.continuous {
        drive_hold(ui, m, st, want, step_xy, step_z, feed_xy, feed_z);
    } else if st.held.take().is_some() {
        m.cancel_jog();
    }
    ui.horizontal_wrapped(|ui| {
        ui.checkbox(&mut m.keyboard_jog, tr("Keyboard")).on_hover_text(tr("Arrow keys move X and Y, Page Up and Page Down move Z. Off by default: the arrows also move the selected shapes."));
        if ui.add_enabled(!busy, egui::Button::new(tr("Go to X0 Y0"))).on_hover_text(tr("Lift to the safe height, then go to the work zero")).clicked() {
            m.lines([format!("G90 G0 Z{}", c.safe_z), "G90 G0 X0 Y0".to_string()]);
        }
        if ui.add_enabled(!busy, egui::Button::new(tr("Z to safe height"))).clicked() {
            m.lines([format!("G90 G0 Z{}", c.safe_z)]);
        }
    });
    if m.keyboard_jog && !ui.ctx().egui_wants_keyboard_input() {
        let (dx, dy, dz) = ui.input(|i| {
            let k = |key| i.key_pressed(key) as i8;
            (
                k(egui::Key::ArrowRight) - k(egui::Key::ArrowLeft),
                k(egui::Key::ArrowUp) - k(egui::Key::ArrowDown),
                k(egui::Key::PageUp) - k(egui::Key::PageDown),
            )
        });
        if (dx, dy, dz) != (0, 0, 0) && !busy {
            m.jog(
                dx as f64 * step_xy,
                dy as f64 * step_xy,
                dz as f64 * step_z,
                if dz != 0 { feed_z } else { feed_xy },
            );
        }
    }
    ui.add_space(6.0);

    // ---- the probe ----
    if c.probe {
        ui.label(RichText::new(tr("Probe")).strong());
        ui.horizontal_wrapped(|ui| {
            let closed = m.probe_closed();
            let (txt, col) = if closed {
                (tr("closed"), Color32::from_rgb(0xe8, 0x8a, 0x1f))
            } else {
                (tr("open"), theme::text_dim())
            };
            ui.label(RichText::new(tr(c.control.probe.kind.label())).color(theme::text_dim()));
            ui.label(RichText::new(trf("input {}", &[&txt])).color(col));
        });
        let mut tool_d = st.tool_d.unwrap_or(c.tool_diameter);
        if c.control.probe.kind == ProbeKind::Plate {
            ui.horizontal(|ui| {
                ui.label(RichText::new(tr("Tool diameter")).color(theme::text_dim()));
                ui.add(
                    egui::DragValue::new(&mut tool_d)
                        .range(0.0..=100.0)
                        .speed(0.1)
                        .max_decimals(3)
                        .suffix(" mm"),
                );
            });
            st.tool_d = Some(tool_d);
        }
        ui.horizontal_wrapped(|ui| {
            let mut go = |ui: &mut Ui, text: &str, tip: &str, axis: Axis, dir: i8| {
                if ui.add_enabled(!busy, egui::Button::new(text)).on_hover_text(tip).clicked() {
                    if let Err(e) = m.probe(c.control, axis, dir, tool_d) {
                        say = Some(e);
                    }
                }
            };
            let z_tip = match c.control.probe.kind {
                ProbeKind::Plate => tr("Put the plate under the tool: the tool goes down until it touches, then Z is set to the thickness of the plate"),
                ProbeKind::Touch3d => tr("The probe goes down until it touches; Z is set to the offset of the probe"),
            };
            go(ui, &tr("Probe Z"), z_tip, Axis::Z, -1);
            let edge = tr("Put the tool next to the edge: it moves towards it until it touches, and the edge becomes zero");
            go(ui, "X+", edge, Axis::X, 1);
            go(ui, "X−", edge, Axis::X, -1);
            go(ui, "Y+", edge, Axis::Y, 1);
            go(ui, "Y−", edge, Axis::Y, -1);
        });
        if let Some((p, touched)) = m.probe {
            ui.label(
                RichText::new(trf(
                    "Last probe: X {} Y {} Z {} ({})",
                    &[
                        &fmt_pos(p[0]),
                        &fmt_pos(p[1]),
                        &fmt_pos(p[2]),
                        &if touched {
                            tr("touched")
                        } else {
                            tr("no contact")
                        },
                    ],
                ))
                .small()
                .color(theme::text_dim()),
            );
        }
        ui.add_space(6.0);
    }

    // ---- macros ----
    if !c.control.macros.is_empty() {
        ui.label(RichText::new(tr("Macros")).strong());
        ui.horizontal_wrapped(|ui| {
            for mac in &c.control.macros {
                let first = mac
                    .code
                    .lines()
                    .find(|l| !l.trim().is_empty())
                    .unwrap_or("")
                    .to_string();
                if ui
                    .add_enabled(!busy, egui::Button::new(&mac.name))
                    .on_hover_text(first)
                    .clicked()
                {
                    match m.run_macro(&mac.code, c.safe_z, c.job_box) {
                        Ok(n) => say = Some(trf("Macro {}: {} lines sent", &[&mac.name, &n])),
                        Err(e) => say = Some(trf("Macro {}: {}", &[&mac.name, &e])),
                    }
                }
            }
        });
    }
    say
}

fn pins_text(pins: &str) -> String {
    pins.chars()
        .map(|c| match c {
            'X' => tr("X limit"),
            'Y' => tr("Y limit"),
            'Z' => tr("Z limit"),
            'P' => tr("probe"),
            'D' => tr("door"),
            'H' => tr("hold"),
            'R' => tr("reset"),
            'S' => tr("start"),
            _ => "?",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// A jog button; `Some(direction)` while it is pressed or was clicked.
fn jog_button(ui: &mut Ui, text: &str, dir: [i8; 3]) -> Option<[i8; 3]> {
    let r = ui.add_sized(
        [34.0, 30.0],
        egui::Button::new(text).sense(Sense::click_and_drag()),
    );
    (r.clicked() || r.is_pointer_button_down_on()).then_some(dir)
}

#[allow(clippy::too_many_arguments)]
fn drive_hold(
    ui: &mut Ui,
    m: &mut Machine,
    st: &mut ControlUi,
    want: Option<[i8; 3]>,
    step_xy: f64,
    step_z: f64,
    feed_xy: f64,
    feed_z: f64,
) {
    let _ = (step_xy, step_z);
    match want {
        Some(d) => {
            let due = st
                .last_segment
                .map_or(true, |t| t.elapsed() >= Duration::from_millis(120));
            if due || st.held != Some(d) {
                // A little more than the time between two segments, so the tool never stops while the button is down.
                let secs = 0.3;
                let (fx, fz) = (feed_xy / 60.0 * secs, feed_z / 60.0 * secs);
                m.jog(
                    d[0] as f64 * fx,
                    d[1] as f64 * fx,
                    d[2] as f64 * fz,
                    if d[2] != 0 { feed_z } else { feed_xy },
                );
                st.last_segment = Some(Instant::now());
            }
            st.held = Some(d);
            ui.ctx().request_repaint_after(Duration::from_millis(60));
        }
        None => {
            if st.held.take().is_some() {
                m.cancel_jog();
                st.last_segment = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_ctl::Machine;

    #[test]
    fn the_tab_draws_connected_or_not_and_the_texts_are_plain() {
        let ctx = egui::Context::default();
        let mut m = Machine::new(&ctx);
        let mut st = ControlUi::default();
        let mut control = Control::default();
        control.macros.push(base_core::control::Macro {
            name: "Park".into(),
            code: "G0 Z5".into(),
        });
        let c = ControlCtx {
            control: &control,
            safe_z: 5.0,
            tool_diameter: 3.0,
            job_box: None,
            probe: true,
            travel: [300.0, 200.0, 40.0],
        };
        for connected in [false, true] {
            m.connected = connected;
            m.wpos = [1.0, 2.0, 3.0];
            m.mpos = [-9.0, -18.0, -3.0];
            m.pins = "P".into();
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    panel(ui, &mut m, &mut st, &c);
                });
            });
            out.textures_delta.clear();
        }
        assert_eq!(pins_text("XP"), "X limit, probe");
        assert_eq!(fmt_pos(f64::NAN), "–");
        assert_eq!(fmt_pos(1.0), "1.000");
    }
}
