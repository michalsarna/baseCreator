//! The Control tab of a machine: the position (work and machine), the work coordinate system and its zero, the steps and
//! speed of jogging, the big jog buttons, the spindle, the probe and the macros. The same in every program that drives a
//! GRBL machine.
use crate::i18n::{tr, trf};
use crate::machine_ctl::Machine;
use crate::theme;
use base_core::control::{Axis, Control, ProbeKind};
use egui::{Color32, RichText, Sense, Ui};
use std::time::{Duration, Instant};

/// What the tab remembers between frames.
pub struct ControlUi {
    /// The distance of one click, as typed or chosen with a button (mm).
    pub xy_text: String,
    pub z_text: String,
    /// Hold a button to move, release to stop, instead of one step per click.
    pub continuous: bool,
    /// The speeds shown: what the person changed since the device's own were loaded.
    pub feed_xy: Option<f64>,
    pub feed_z: Option<f64>,
    /// The speeds of the device the changes above were made against; when the device's change, they are forgotten.
    feeds_of_device: Option<(f64, f64)>,
    /// The spindle speed typed for a test, rpm (the first time: a part of the top speed).
    pub spindle_rpm: Option<f64>,
    held: Option<[i8; 3]>,
    last_segment: Option<Instant>,
    /// The axis whose work position is being typed, and the text.
    edit: Option<(usize, String)>,
    /// The diameter of the tool for edge probing, typed by the person; the program's guess until then.
    pub tool_d: Option<f64>,
}

impl Default for ControlUi {
    fn default() -> ControlUi {
        ControlUi { xy_text: String::new(), z_text: String::new(), continuous: false, feed_xy: None, feed_z: None, feeds_of_device: None, spindle_rpm: None, held: None, last_segment: None, edit: None, tool_d: None }
    }
}

/// The spindle of the machine, for the switch of the Control tab.
#[derive(Clone, Copy, Debug)]
pub struct SpindleCtx {
    pub min_rpm: f64,
    pub max_rpm: f64,
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
    /// Show the spindle section (a machine with a spindle).
    pub spindle: Option<SpindleCtx>,
}

const AXES: [(Axis, &str); 3] = [(Axis::X, "X"), (Axis::Y, "Y"), (Axis::Z, "Z")];

/// The size of a jog button: big enough to hit with a finger or a mouse in a hurry.
const JOG: [f32; 2] = [70.0, 60.0];

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

/// The value typed in a field of steps; `None` when it is not a sane distance.
fn parse_step(text: &str) -> Option<f64> {
    let v: f64 = text.trim().replace(',', ".").parse().ok()?;
    (v.is_finite() && v > 0.0 && v <= 1000.0).then_some(v)
}

/// A button with its tip, which also shows while the button is switched off.
fn tip(r: egui::Response, text: &str) -> egui::Response {
    r.on_hover_text(text).on_disabled_hover_text(text)
}

fn heading(ui: &mut Ui, text: &str) {
    ui.label(RichText::new(text).strong().size(16.0));
}

/// The line between two sections.
fn section_break(ui: &mut Ui) {
    ui.add_space(6.0);
    ui.separator();
    ui.add_space(6.0);
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
        ui.label(RichText::new(tr("Connect to the machine first (Device tab)")).color(theme::text_dim()));
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
        let state = if m.state.is_empty() { "–".to_string() } else { m.state.clone() };
        ui.label(RichText::new(state).strong().size(17.0).color(if alarm { Color32::from_rgb(0xc0, 0x30, 0x30) } else { theme::text() }));
        if m.fw.info.is_known() {
            ui.label(RichText::new(m.fw.info.label()).color(theme::text_dim()));
        }
        if alarm && tip(ui.button(tr("Unlock")), &tr("Clear an alarm ($X)")).clicked() {
            m.unlock();
        }
    });
    if let Some(code) = m.alarm.filter(|c| *c > 0) {
        if let Some(t) = base_core::grbl_codes::alarm(code) {
            ui.label(RichText::new(trf("Alarm {}: {}", &[&code, &tr(t)])).color(Color32::from_rgb(0xc0, 0x30, 0x30)));
        }
    }
    section_break(ui);

    // ---- the position, with the zero buttons on the left ----
    heading(ui, &tr("Position"));
    ui.add_space(2.0);
    egui::Grid::new("dro").num_columns(4).spacing([14.0, 6.0]).show(ui, |ui| {
        ui.label("");
        ui.label("");
        ui.label(RichText::new(tr("Work")).color(theme::text_dim()));
        ui.label(RichText::new(tr("Machine")).color(theme::text_dim()));
        ui.end_row();
        for (i, (axis, name)) in AXES.iter().enumerate() {
            if tip(ui.add_enabled(!busy, egui::Button::new(RichText::new(format!("{name}0")).size(16.0)).min_size(egui::vec2(46.0, 32.0))), &tr("The tool is here: make this the zero of the work")).clicked() {
                m.set_position(&[(*axis, 0.0)]);
            }
            ui.label(RichText::new(*name).strong().size(24.0));
            if st.edit.as_ref().is_some_and(|e| e.0 == i) {
                let (_, text) = st.edit.as_mut().unwrap();
                let r = ui.add(egui::TextEdit::singleline(text).desired_width(120.0).font(egui::FontId::monospace(22.0)));
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
                .add(egui::Button::new(RichText::new(fmt_pos(m.wpos[i])).font(egui::FontId::monospace(24.0))).frame(false))
                .on_hover_text(tr("Click to type the position of the tool in the work coordinates"))
                .clicked()
            {
                st.edit = Some((i, if m.wpos[i].is_finite() { format!("{:.3}", m.wpos[i]) } else { "0".into() }));
            }
            ui.label(RichText::new(fmt_pos(m.mpos[i])).font(egui::FontId::monospace(17.0)).color(theme::text_dim()));
            ui.end_row();
        }
    });
    section_break(ui);

    // ---- the zero of the work and the ways to it ----
    ui.horizontal_wrapped(|ui| {
        if ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Zero all")).size(15.0)).min_size(egui::vec2(0.0, 32.0))).clicked() {
            m.set_position(&[(Axis::X, 0.0), (Axis::Y, 0.0), (Axis::Z, 0.0)]);
        }
        if ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Zero X and Y")).size(15.0)).min_size(egui::vec2(0.0, 32.0))).clicked() {
            m.set_position(&[(Axis::X, 0.0), (Axis::Y, 0.0)]);
        }
    });
    ui.horizontal_wrapped(|ui| {
        if tip(ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Go to X0 Y0")).size(15.0)).min_size(egui::vec2(0.0, 32.0))), &tr("Lift to the safe height, then go to the work zero")).clicked() {
            m.lines([format!("G90 G0 Z{}", c.safe_z), "G90 G0 X0 Y0".to_string()]);
        }
        if ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Z to safe height")).size(15.0)).min_size(egui::vec2(0.0, 32.0))).clicked() {
            m.lines([format!("G90 G0 Z{}", c.safe_z)]);
        }
    });
    section_break(ui);

    // ---- the coordinate system ----
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Coordinates")).color(theme::text_dim()));
        let names = ["G54", "G55", "G56", "G57", "G58", "G59"];
        let mut wcs = m.wcs.clamp(1, 6) as usize - 1;
        egui::ComboBox::from_id_salt("wcs").selected_text(names[wcs]).width(70.0).show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                if ui.selectable_value(&mut wcs, i, *n).clicked() {
                    m.select_wcs(i as u8 + 1);
                }
            }
        });
    });
    if !m.pins.is_empty() {
        ui.label(RichText::new(trf("Inputs active: {}", &[&pins_text(&m.pins)])).small().color(theme::text_dim()));
    }
    section_break(ui);

    // ---- how far one click moves ----
    heading(ui, &tr("Move"));
    ui.add_space(2.0);
    let xy_steps = c.control.jog.xy_steps.clone();
    let z_steps = c.control.jog.z_steps.clone();
    if st.xy_text.is_empty() {
        st.xy_text = step_label(xy_steps.get(1).or(xy_steps.first()).copied().unwrap_or(1.0));
    }
    if st.z_text.is_empty() {
        st.z_text = step_label(z_steps.get(2).or(z_steps.first()).copied().unwrap_or(1.0));
    }
    let pick = |ui: &mut Ui, name: &str, text: &mut String, steps: &[f64]| {
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(name).strong());
            let ok = parse_step(text).is_some();
            let r = ui.add(egui::TextEdit::singleline(text).desired_width(64.0).text_color(if ok { theme::text() } else { Color32::from_rgb(0xc0, 0x30, 0x30) }));
            r.on_hover_text(tr("How far one click moves the tool, in millimetres"));
            ui.label(RichText::new("mm").color(theme::text_dim()));
            let now = parse_step(text);
            for s in steps {
                if ui.add(egui::Button::selectable(now == Some(*s), step_label(*s))).clicked() {
                    *text = step_label(*s);
                }
            }
        });
    };
    pick(ui, "XY", &mut st.xy_text, &xy_steps);
    pick(ui, "Z", &mut st.z_text, &z_steps);
    let step_xy = parse_step(&st.xy_text).unwrap_or(1.0);
    let step_z = parse_step(&st.z_text).unwrap_or(1.0);
    section_break(ui);

    // ---- the speed, from the device each time it changes ----
    let cfg = (c.control.jog.xy_feed, c.control.jog.z_feed);
    if st.feeds_of_device != Some(cfg) {
        st.feeds_of_device = Some(cfg);
        st.feed_xy = None;
        st.feed_z = None;
    }
    let mut feed_xy = st.feed_xy.unwrap_or(cfg.0);
    let mut feed_z = st.feed_z.unwrap_or(cfg.1);
    heading(ui, &tr("Speed"));
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("XY").strong());
        ui.add(egui::DragValue::new(&mut feed_xy).range(1.0..=100_000.0).speed(50.0).suffix(" mm/min"));
        ui.add_space(10.0);
        ui.label(RichText::new("Z").strong());
        ui.add(egui::DragValue::new(&mut feed_z).range(1.0..=100_000.0).speed(10.0).suffix(" mm/min"));
    });
    if feed_xy != cfg.0 {
        st.feed_xy = Some(feed_xy);
    }
    if feed_z != cfg.1 {
        st.feed_z = Some(feed_z);
    }
    section_break(ui);

    // ---- the big jog buttons, in the middle ----
    let mut want: Option<[i8; 3]> = None;
    let continuous = st.continuous;
    ui.add_enabled_ui(!busy, |ui| {
        let grid_w = 3.0 * JOG[0] + 2.0 * 3.0;
        let total = grid_w + 14.0 + JOG[0];
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            egui::Grid::new("jog_xy").spacing([3.0, 3.0]).show(ui, |ui| {
                // The picture of the machine seen from above: Y up is towards the back.
                let cells: [[(&str, [i8; 3]); 3]; 3] = [[("↖", [-1, 1, 0]), ("▲", [0, 1, 0]), ("↗", [1, 1, 0])], [("◀", [-1, 0, 0]), ("•", [0, 0, 0]), ("▶", [1, 0, 0])], [("↙", [-1, -1, 0]), ("▼", [0, -1, 0]), ("↘", [1, -1, 0])]];
                for row in cells {
                    for (t, d) in row {
                        if d == [0, 0, 0] {
                            if ui.add_sized(JOG, egui::Button::new(RichText::new("■").size(26.0).color(Color32::from_rgb(0xc0, 0x30, 0x30)))).on_hover_text(tr("Stop the move")).clicked() {
                                m.cancel_jog();
                            }
                        } else if let Some(w) = jog_button(ui, t, d, continuous) {
                            if continuous {
                                want = Some(w);
                            } else {
                                m.jog(d[0] as f64 * step_xy, d[1] as f64 * step_xy, 0.0, feed_xy);
                            }
                        }
                    }
                    ui.end_row();
                }
            });
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                for (t, d) in [("Z▲", [0, 0, 1]), ("Z▼", [0, 0, -1])] {
                    if let Some(w) = jog_button(ui, t, d, continuous) {
                        if continuous {
                            want = Some(w);
                        } else {
                            m.jog(0.0, 0.0, d[2] as f64 * step_z, feed_z);
                        }
                    }
                }
            });
        });
    });
    // Hold to move: short moves one after the other while the button is down, a jog cancel when it is let go.
    if st.continuous {
        drive_hold(ui, m, st, want, feed_xy, feed_z);
    } else if st.held.take().is_some() {
        m.cancel_jog();
    }
    ui.add_space(4.0);
    ui.horizontal_wrapped(|ui| {
        tip(ui.checkbox(&mut st.continuous, tr("Hold to move")), &tr("Instead of one step per click, the tool moves while the button is held down"));
        tip(ui.checkbox(&mut m.keyboard_jog, tr("Keyboard")), &tr("Arrow keys move X and Y, Page Up and Page Down move Z, by the step chosen above. Off by default: the arrows also move the selected shapes."));
    });
    if m.keyboard_jog && !ui.ctx().egui_wants_keyboard_input() {
        let (dx, dy, dz) = ui.input(|i| {
            let k = |key| i.key_pressed(key) as i8;
            (k(egui::Key::ArrowRight) - k(egui::Key::ArrowLeft), k(egui::Key::ArrowUp) - k(egui::Key::ArrowDown), k(egui::Key::PageUp) - k(egui::Key::PageDown))
        });
        if (dx, dy, dz) != (0, 0, 0) && !busy {
            m.jog(dx as f64 * step_xy, dy as f64 * step_xy, dz as f64 * step_z, if dz != 0 { feed_z } else { feed_xy });
        }
    }

    // ---- the spindle ----
    if let Some(sp) = c.spindle {
        section_break(ui);
        heading(ui, &tr("Spindle"));
        ui.add_space(2.0);
        let mut rpm = st.spindle_rpm.unwrap_or((sp.max_rpm * 0.5).max(sp.min_rpm));
        ui.horizontal_wrapped(|ui| {
            ui.label(RichText::new(tr("Spindle speed")).color(theme::text_dim()));
            ui.add(egui::DragValue::new(&mut rpm).range(0.0..=1_000_000.0).speed(100.0).suffix(" rpm"));
            let go = tip(ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Switch on")).size(15.0)).min_size(egui::vec2(0.0, 32.0))), &tr("Switches the spindle on to try it (M3 S…) and off again (M5). A job does this by itself."));
            if go.clicked() {
                m.line(format!("M3 S{}", fmt_num(m.spindle_s(rpm, sp.max_rpm))));
            }
            if ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Switch off")).size(15.0)).min_size(egui::vec2(0.0, 32.0))).clicked() {
                m.line("M5");
            }
        });
        st.spindle_rpm = Some(rpm);
    }

    // ---- the probe ----
    if c.probe {
        section_break(ui);
        heading(ui, &tr("Probe"));
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            let closed = m.probe_closed();
            let (txt, col) = if closed { (tr("closed"), Color32::from_rgb(0xe8, 0x8a, 0x1f)) } else { (tr("open"), theme::text_dim()) };
            ui.label(RichText::new(trf("input {}", &[&txt])).color(col));
            let z_tip = match c.control.probe.kind {
                ProbeKind::Plate => tr("Put the plate under the tool: the tool goes down until it touches, then Z is set to the thickness of the plate"),
                ProbeKind::Touch3d => tr("The probe goes down until it touches; Z is set to the offset of the probe"),
            };
            let tool_d = st.tool_d.unwrap_or(c.tool_diameter);
            if tip(ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Probe Z")).size(15.0)).min_size(egui::vec2(0.0, 32.0))), &z_tip).clicked() {
                if let Err(e) = m.probe(c.control, Axis::Z, -1, tool_d) {
                    say = Some(e);
                }
            }
        });
        if let Some((p, touched)) = m.probe {
            ui.label(
                RichText::new(trf("Last probe: X {} Y {} Z {} ({})", &[&fmt_pos(p[0]), &fmt_pos(p[1]), &fmt_pos(p[2]), &if touched { tr("touched") } else { tr("no contact") }])).small().color(theme::text_dim()),
            );
        }
    }

    // ---- macros ----
    if !c.control.macros.is_empty() {
        section_break(ui);
        heading(ui, &tr("Macros"));
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            for mac in &c.control.macros {
                let first = mac.code.lines().find(|l| !l.trim().is_empty()).unwrap_or("").to_string();
                if tip(ui.add_enabled(!busy, egui::Button::new(&mac.name)), &first).clicked() {
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

fn fmt_num(v: f64) -> String {
    let s = format!("{v:.3}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
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

/// A big jog button. One step per click (`continuous` off): only the click counts, not the frames the mouse button is down
/// (that sent the move five times). Hold to move: `Some(direction)` while it is pressed.
fn jog_button(ui: &mut Ui, text: &str, dir: [i8; 3], continuous: bool) -> Option<[i8; 3]> {
    let r = ui.add_sized(JOG, egui::Button::new(RichText::new(text).size(26.0)).sense(Sense::click_and_drag()));
    let hit = if continuous { r.clicked() || r.is_pointer_button_down_on() } else { r.clicked() };
    hit.then_some(dir)
}

fn drive_hold(ui: &mut Ui, m: &mut Machine, st: &mut ControlUi, want: Option<[i8; 3]>, feed_xy: f64, feed_z: f64) {
    match want {
        Some(d) => {
            let due = st.last_segment.map_or(true, |t| t.elapsed() >= Duration::from_millis(120));
            if due || st.held != Some(d) {
                // A little more than the time between two segments, so the tool never stops while the button is down.
                let secs = 0.3;
                let (fx, fz) = (feed_xy / 60.0 * secs, feed_z / 60.0 * secs);
                m.jog(d[0] as f64 * fx, d[1] as f64 * fx, d[2] as f64 * fz, if d[2] != 0 { feed_z } else { feed_xy });
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
            spindle: Some(SpindleCtx { min_rpm: 1000.0, max_rpm: 24000.0 }),
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

    /// Holding the mouse button on a step button for a moment (several frames) is one click, so one move. It used to send the
    /// move on every frame the button was down, five or six times.
    #[test]
    fn a_click_on_a_jog_button_is_one_move_however_long_the_button_is_down() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0));
        let mut pos = egui::pos2(10.0, 10.0);
        let mut moves = (0, 0);
        let mut frame = |events: Vec<egui::Event>, moves: &mut (u32, u32), pos: &mut egui::Pos2| {
            let input = egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
            let mut o = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let at = ui.cursor().min;
                    *pos = at + egui::vec2(JOG[0] / 2.0, JOG[1] / 2.0);
                    if jog_button(ui, "▲", [0, 1, 0], false).is_some() {
                        moves.0 += 1;
                    }
                    if jog_button(ui, "▼", [0, -1, 0], true).is_some() {
                        moves.1 += 1;
                    }
                });
            });
            o.textures_delta.clear();
        };
        frame(vec![], &mut moves, &mut pos);
        frame(vec![], &mut moves, &mut pos);
        let at = pos;
        let button = |pressed| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        frame(vec![egui::Event::PointerMoved(at)], &mut moves, &mut pos);
        frame(vec![button(true)], &mut moves, &mut pos);
        for _ in 0..8 {
            frame(vec![], &mut moves, &mut pos);
        }
        frame(vec![button(false)], &mut moves, &mut pos);
        frame(vec![], &mut moves, &mut pos);
        assert_eq!(moves.0, 1, "one click, one move");
        // Hold to move (the second button) wants the direction on every frame the button is down: that is what it is for.
        let at2 = at + egui::vec2(0.0, JOG[1] + 8.0);
        let button2 = |pressed| egui::Event::PointerButton { pos: at2, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() };
        frame(vec![egui::Event::PointerMoved(at2)], &mut moves, &mut pos);
        frame(vec![button2(true)], &mut moves, &mut pos);
        for _ in 0..5 {
            frame(vec![], &mut moves, &mut pos);
        }
        frame(vec![button2(false)], &mut moves, &mut pos);
        assert!(moves.1 >= 5, "held: {}", moves.1);
    }
}
