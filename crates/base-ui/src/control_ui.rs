//! The Control tab of a machine: the position (work and machine), the work coordinate system and its zero, the steps and
//! speed of jogging, the big jog buttons, the spindle, the probe and the macros. The same in every program that drives a
//! GRBL machine.
use crate::i18n::{tr, trf};
use crate::machine_ctl::Machine;
use crate::theme;
use base_core::control::{Axis, Control, ProbeKind};
use egui::{Color32, RichText, Sense, Ui};
use std::time::{Duration, Instant};

/// A place the tool was at, remembered for going back to it: the machine position.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SavedPos {
    pub name: String,
    pub p: [f64; 3],
}

const MAX_SAVED: usize = 24;

fn positions_file() -> Option<std::path::PathBuf> {
    base_core::paths::base_dir().map(|d| d.join("saved_positions.json"))
}

fn load_positions() -> Vec<SavedPos> {
    let Some(f) = positions_file() else { return vec![] };
    let Ok(text) = std::fs::read_to_string(f) else { return vec![] };
    let mut v: Vec<SavedPos> = serde_json::from_str(&text).unwrap_or_default();
    v.retain(|s| s.p.iter().all(|x| x.is_finite()) && s.name.len() <= 60);
    v.truncate(MAX_SAVED);
    v
}

fn store_positions(v: &[SavedPos]) {
    if let Some(f) = positions_file() {
        if let Ok(text) = serde_json::to_string_pretty(v) {
            let _ = std::fs::write(f, text);
        }
    }
}

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
    /// Places to go back to (machine positions), kept in a file of the shared data folder; the name being typed for the next.
    pub positions: Option<Vec<SavedPos>>,
    pub pos_name: String,
}

impl Default for ControlUi {
    fn default() -> ControlUi {
        ControlUi { xy_text: String::new(), z_text: String::new(), continuous: false, feed_xy: None, feed_z: None, feeds_of_device: None, spindle_rpm: None, held: None, last_segment: None, edit: None, tool_d: None, positions: None, pos_name: String::new() }
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
    /// The buttons that move the work zero: Zero all, X0 Y0 Z0, typing a position, the coordinate system. A program whose
    /// jobs count from the machine's zero shows the positions and leaves these out.
    pub zeroing: bool,
    /// The machine has a Z axis (the Z rows, the Z buttons and the lift before a move go with it).
    pub z_axis: bool,
    /// Hold to move works (it needs the jog cancel of GRBL).
    pub hold_to_move: bool,
}

/// What the Control tab needs from the machine it drives: the `Machine` of the programs that stream a GRBL job themselves,
/// and the laser of LightCreator, which has its own controllers and limits behind the same buttons.
pub trait ControlTarget {
    fn connected(&self) -> bool;
    /// The state word of the last report (`Idle`, `Run`, `Alarm`…); empty when unknown.
    fn state(&self) -> String;
    /// The firmware, when it is known (`Grbl 1.1h`).
    fn fw_label(&self) -> Option<String>;
    fn alarm_code(&self) -> Option<u32>;
    fn is_alarm(&self) -> bool;
    fn wpos(&self) -> [f64; 3];
    fn mpos(&self) -> [f64; 3];
    fn pins(&self) -> String;
    fn wcs(&self) -> u8;
    /// A job is on and not at a pause: the machine must not be moved by hand.
    fn busy(&self) -> bool;
    fn keyboard_jog(&self) -> bool;
    fn set_keyboard_jog(&mut self, on: bool);
    /// The work position of the axes (`G10 L20`).
    fn set_position(&mut self, values: &[(Axis, f64)]);
    /// One relative move: x and y count Y up towards the back, as the pad does.
    fn jog(&mut self, dx: f64, dy: f64, dz: f64, feed: f64);
    fn cancel_jog(&mut self);
    fn unlock(&mut self);
    fn select_wcs(&mut self, n: u8);
    fn lines(&mut self, ls: Vec<String>);
    fn line(&mut self, l: String);
    fn run_macro(&mut self, code: &str, safe_z: f64, job_box: Option<([f64; 3], [f64; 3])>) -> Result<usize, String>;
    fn probe_closed(&self) -> bool;
    fn last_probe(&self) -> Option<([f64; 3], bool)>;
    fn probe(&mut self, c: &Control, axis: Axis, dir: i8, tool_diameter: f64) -> Result<(), String>;
    fn spindle_s(&self, rpm: f64, max_rpm: f64) -> f64;
    /// The overrides the controller reports (feed, rapids, spindle in percent); `None` where there are none (GRBL before 1.1).
    fn overrides(&self) -> Option<(u32, u32, u32)> {
        None
    }
    /// A realtime byte (the overrides are changed by them).
    fn realtime(&mut self, _b: u8) {}
    /// Can this machine go to a machine position by itself (the saved positions need it)? A laser with its own limits and margins cannot.
    fn can_go_machine(&self) -> bool {
        false
    }
    /// Go to a machine position (a saved one) at `feed` mm/min, up first when needed; Z only with `with_z`.
    fn go_machine(&mut self, _p: [f64; 3], _feed: f64, _with_z: bool) {}
    /// Find the centre of a hole with the probe and make it the zero of X and Y.
    fn center_hole(&mut self, _c: &Control) -> Result<(), String> {
        Err(tr("The machine cannot do this.").to_string())
    }
    fn center_running(&self) -> bool {
        false
    }
}

impl ControlTarget for Machine {
    fn overrides(&self) -> Option<(u32, u32, u32)> {
        self.overrides
    }
    fn realtime(&mut self, b: u8) {
        Machine::realtime(self, b)
    }
    fn can_go_machine(&self) -> bool {
        true
    }
    fn go_machine(&mut self, p: [f64; 3], feed: f64, with_z: bool) {
        Machine::go_machine(self, p, feed, with_z)
    }
    fn center_hole(&mut self, c: &Control) -> Result<(), String> {
        Machine::center_start(self, c)
    }
    fn center_running(&self) -> bool {
        Machine::center_running(self)
    }
    fn connected(&self) -> bool {
        self.connected
    }
    fn state(&self) -> String {
        self.state.clone()
    }
    fn fw_label(&self) -> Option<String> {
        self.fw.info.is_known().then(|| self.fw.info.label())
    }
    fn alarm_code(&self) -> Option<u32> {
        self.alarm
    }
    fn is_alarm(&self) -> bool {
        Machine::is_alarm(self)
    }
    fn wpos(&self) -> [f64; 3] {
        self.wpos
    }
    fn mpos(&self) -> [f64; 3] {
        self.mpos
    }
    fn pins(&self) -> String {
        self.pins.clone()
    }
    fn wcs(&self) -> u8 {
        self.wcs
    }
    fn busy(&self) -> bool {
        (self.job_running() || self.job_active()) && self.barrier.is_none()
    }
    fn keyboard_jog(&self) -> bool {
        self.keyboard_jog
    }
    fn set_keyboard_jog(&mut self, on: bool) {
        self.keyboard_jog = on;
    }
    fn set_position(&mut self, values: &[(Axis, f64)]) {
        Machine::set_position(self, values)
    }
    fn jog(&mut self, dx: f64, dy: f64, dz: f64, feed: f64) {
        Machine::jog(self, dx, dy, dz, feed)
    }
    fn cancel_jog(&mut self) {
        Machine::cancel_jog(self)
    }
    fn unlock(&mut self) {
        Machine::unlock(self)
    }
    fn select_wcs(&mut self, n: u8) {
        Machine::select_wcs(self, n)
    }
    fn lines(&mut self, ls: Vec<String>) {
        Machine::lines(self, ls)
    }
    fn line(&mut self, l: String) {
        Machine::line(self, l)
    }
    fn run_macro(&mut self, code: &str, safe_z: f64, job_box: Option<([f64; 3], [f64; 3])>) -> Result<usize, String> {
        Machine::run_macro(self, code, safe_z, job_box)
    }
    fn probe_closed(&self) -> bool {
        Machine::probe_closed(self)
    }
    fn last_probe(&self) -> Option<([f64; 3], bool)> {
        self.probe
    }
    fn probe(&mut self, c: &Control, axis: Axis, dir: i8, tool_diameter: f64) -> Result<(), String> {
        Machine::probe(self, c, axis, dir, tool_diameter)
    }
    fn spindle_s(&self, rpm: f64, max_rpm: f64) -> f64 {
        Machine::spindle_s(self, rpm, max_rpm)
    }
}

/// The wide red STOP button of the Control tab. Only live while the machine is working (`enabled`); returns true when pressed.
pub fn stop_button(ui: &mut Ui, enabled: bool) -> bool {
    let fill = if enabled { Color32::from_rgb(0xc0, 0x30, 0x30) } else { Color32::from_rgb(0xc0, 0x30, 0x30).gamma_multiply(0.35) };
    let b = egui::Button::new(RichText::new(tr("STOP")).color(Color32::WHITE).strong()).fill(fill);
    ui.add_enabled_ui(enabled, |ui| ui.add_sized([ui.available_width(), 40.0], b)).inner.clicked()
}

/// What the four quick buttons ask for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Quick {
    Home,
    Unlock,
    Pause,
    Resume,
}

/// Home, Unlock, Pause and Resume in a block of 2 × 2, each with an icon of its own colour (Home blue, Unlock red, Pause
/// orange, Resume green). The same in the Device tab and at the top of the Control tab of every program. `moves` says that
/// Home and Unlock may be used now (connected, no job), `link` that Pause and Resume may (connected).
pub fn quick_buttons(ui: &mut Ui, moves: bool, link: bool) -> Option<Quick> {
    use crate::window::{NEW_BLUE, OPEN_ORANGE, SAVE_GREEN};
    let mut hit = None;
    let width = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(90.0);
    let items = [
        (Quick::Home, "house", NEW_BLUE, tr("Home"), moves, None),
        (Quick::Unlock, "lock-open", Color32::from_rgb(0xc0, 0x30, 0x30), tr("Unlock"), moves, Some(tr("Clear an alarm ($X)"))),
        (Quick::Pause, "pause", OPEN_ORANGE, tr("Pause"), link, Some(tr("Feed hold"))),
        (Quick::Resume, "play", SAVE_GREEN, tr("Resume"), link, None),
    ];
    egui::Grid::new("quick_buttons").num_columns(2).spacing([ui.spacing().item_spacing.x, 4.0]).show(ui, |ui| {
        for (i, (what, icon, ink, text, on, tooltip)) in items.into_iter().enumerate() {
            let b = egui::Button::image_and_text(crate::icons::slot(Some(icon), if on { ink } else { ink.gamma_multiply(0.45) }), RichText::new(text).size(15.0));
            let mut r = ui.add_enabled(on, b.min_size(egui::vec2(width, 32.0)));
            if let Some(t) = tooltip {
                r = r.on_hover_text(t).on_disabled_hover_text(t);
            }
            if r.clicked() {
                hit = Some(what);
            }
            if i % 2 == 1 {
                ui.end_row();
            }
        }
    });
    hit
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
pub fn panel(ui: &mut Ui, m: &mut dyn ControlTarget, st: &mut ControlUi, c: &ControlCtx) -> Option<String> {
    let mut say = None;
    let on = m.connected();
    ui.add_enabled_ui(on, |ui| {
        say = body(ui, m, st, c);
    });
    if !on {
        ui.add_space(4.0);
        ui.label(RichText::new(tr("Connect to the machine first (Device tab)")).color(theme::text_dim()));
    }
    say
}

fn body(ui: &mut Ui, m: &mut dyn ControlTarget, st: &mut ControlUi, c: &ControlCtx) -> Option<String> {
    let mut say = None;
    // At a pause of the job (a change of the bit) the machine is free to be moved, probed and zeroed.
    let busy = m.busy();
    let (wpos, mpos) = (m.wpos(), m.mpos());
    // ---- state ----
    ui.horizontal_wrapped(|ui| {
        let alarm = m.is_alarm();
        let state = if m.state().is_empty() { "–".to_string() } else { m.state() };
        ui.label(RichText::new(state).strong().size(17.0).color(if alarm { Color32::from_rgb(0xc0, 0x30, 0x30) } else { theme::text() }));
        if let Some(fw) = m.fw_label() {
            ui.label(RichText::new(fw).color(theme::text_dim()));
        }
        if alarm && tip(ui.button(tr("Unlock")), &tr("Clear an alarm ($X)")).clicked() {
            m.unlock();
        }
    });
    if let Some(code) = m.alarm_code().filter(|c| *c > 0) {
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
            if i == 2 && !c.z_axis {
                continue;
            }
            if !c.zeroing {
                ui.label("");
            } else if tip(ui.add_enabled(!busy, egui::Button::new(RichText::new(format!("{name}0")).size(16.0)).min_size(egui::vec2(46.0, 32.0))), &tr("The tool is here: make this the zero of the work")).clicked() {
                m.set_position(&[(*axis, 0.0)]);
            }
            ui.label(RichText::new(*name).strong().size(24.0));
            if !c.zeroing {
                ui.label(RichText::new(fmt_pos(wpos[i])).font(egui::FontId::monospace(24.0)));
            } else if st.edit.as_ref().is_some_and(|e| e.0 == i) {
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
                .add(egui::Button::new(RichText::new(fmt_pos(wpos[i])).font(egui::FontId::monospace(24.0))).frame(false))
                .on_hover_text(tr("Click to type the position of the tool in the work coordinates"))
                .clicked()
            {
                st.edit = Some((i, if wpos[i].is_finite() { format!("{:.3}", wpos[i]) } else { "0".into() }));
            }
            ui.label(RichText::new(fmt_pos(mpos[i])).font(egui::FontId::monospace(17.0)).color(theme::text_dim()));
            ui.end_row();
        }
    });
    section_break(ui);

    // ---- the zero of the work and the ways to it ----
    if c.zeroing {
    let half = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(60.0);
    let big = |t: String| egui::Button::new(RichText::new(t).size(15.0)).min_size(egui::vec2(half, 32.0));
    egui::Grid::new("zero_grid").num_columns(2).show(ui, |ui| {
        if ui.add_enabled(!busy, big(tr("Zero all").to_string())).clicked() {
            let mut all = vec![(Axis::X, 0.0), (Axis::Y, 0.0)];
            if c.z_axis {
                all.push((Axis::Z, 0.0));
            }
            m.set_position(&all);
        }
        if ui.add_enabled(!busy, big(tr("Zero X and Y").to_string())).clicked() {
            m.set_position(&[(Axis::X, 0.0), (Axis::Y, 0.0)]);
        }
        ui.end_row();
        if tip(ui.add_enabled(!busy, big(tr("Go to X0 Y0").to_string())), &tr("Lift to the safe height, then go to the work zero")).clicked() {
            let mut go = vec![];
            if c.z_axis {
                go.push(format!("G90 G0 Z{}", c.safe_z));
            }
            go.push("G90 G0 X0 Y0".to_string());
            m.lines(go);
        }
        if c.z_axis && ui.add_enabled(!busy, big(tr("Z to safe height").to_string())).clicked() {
            m.lines(vec![format!("G90 G0 Z{}", c.safe_z)]);
        }
        ui.end_row();
    });
    section_break(ui);
    }

    // ---- the coordinate system ----
    if c.zeroing {
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new(tr("Coordinates")).color(theme::text_dim()));
        let names = ["G54", "G55", "G56", "G57", "G58", "G59"];
        let mut wcs = m.wcs().clamp(1, 6) as usize - 1;
        egui::ComboBox::from_id_salt("wcs").selected_text(names[wcs]).width(70.0).show_ui(ui, |ui| {
            for (i, n) in names.iter().enumerate() {
                if ui.selectable_value(&mut wcs, i, *n).clicked() {
                    m.select_wcs(i as u8 + 1);
                }
            }
        });
    });
    }
    let pins = m.pins();
    if !pins.is_empty() {
        ui.label(RichText::new(trf("Inputs active: {}", &[&pins_text(&pins)])).small().color(theme::text_dim()));
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
    if c.z_axis {
        pick(ui, "Z", &mut st.z_text, &z_steps);
    }
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
    // ---- the big jog buttons, in the middle ----
    let mut want: Option<[i8; 3]> = None;
    if !c.hold_to_move {
        st.continuous = false;
    }
    let continuous = st.continuous;
    ui.add_enabled_ui(!busy, |ui| {
        let grid_w = 3.0 * JOG[0] + 2.0 * 3.0;
        let total = grid_w + 14.0 + JOG[0];
        ui.horizontal(|ui| {
            ui.add_space(((ui.available_width() - total) / 2.0).max(0.0));
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            let hit = xy_pad(ui, continuous);
            if hit.stop {
                m.cancel_jog();
            }
            if let Some(d) = hit.dir {
                if continuous {
                    want = Some(d);
                } else {
                    m.jog(d[0] as f64 * step_xy, d[1] as f64 * step_xy, 0.0, feed_xy);
                }
            }
            if c.z_axis {
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 3.0;
                for (t, d) in [("Z▲", [0, 0, 1]), ("Z▼", [0, 0, -1])] {
                    if let Some(w) = jog_button(ui, t, d, continuous, None) {
                        if continuous {
                            want = Some(w);
                        } else {
                            m.jog(0.0, 0.0, d[2] as f64 * step_z, feed_z);
                        }
                    }
                }
            });
            }
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
        if c.hold_to_move {
            tip(ui.checkbox(&mut st.continuous, tr("Hold to move")), &tr("Instead of one step per click, the tool moves while the button is held down"));
        }
        let mut kb = m.keyboard_jog();
        tip(ui.checkbox(&mut kb, tr("Keyboard")), &tr("Arrow keys move X and Y, Page Up and Page Down move Z, by the step chosen above. Off by default: the arrows also move the selected shapes."));
        m.set_keyboard_jog(kb);
    });
    if m.keyboard_jog() && !ui.ctx().egui_wants_keyboard_input() {
        let (dx, dy, dz) = ui.input(|i| {
            let k = |key| i.key_pressed(key) as i8;
            (k(egui::Key::ArrowRight) - k(egui::Key::ArrowLeft), k(egui::Key::ArrowUp) - k(egui::Key::ArrowDown), k(egui::Key::PageUp) - k(egui::Key::PageDown))
        });
        if (dx, dy, dz) != (0, 0, 0) && !busy {
            let dz = if c.z_axis { dz } else { 0 };
            m.jog(dx as f64 * step_xy, dy as f64 * step_xy, dz as f64 * step_z, if dz != 0 { feed_z } else { feed_xy });
        }
    }

    // ---- the speed of a move ----
    section_break(ui);
    heading(ui, &tr("Speed"));
    ui.add_space(2.0);
    ui.horizontal_wrapped(|ui| {
        ui.label(RichText::new("XY").strong());
        ui.add(egui::DragValue::new(&mut feed_xy).range(1.0..=100_000.0).speed(50.0).suffix(" mm/min"));
        if c.z_axis {
            ui.add_space(10.0);
            ui.label(RichText::new("Z").strong());
            ui.add(egui::DragValue::new(&mut feed_z).range(1.0..=100_000.0).speed(10.0).suffix(" mm/min"));
        }
    });
    if feed_xy != cfg.0 {
        st.feed_xy = Some(feed_xy);
    }
    if feed_z != cfg.1 {
        st.feed_z = Some(feed_z);
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
            let centre_tip = tr("The probe hangs in a hole, about in the middle: it touches the four sides, goes to the middle and makes it the zero of X and Y. Do the Z first.");
            if tip(ui.add_enabled(!busy && !m.center_running(), egui::Button::new(RichText::new(tr("Find the centre")).size(15.0)).min_size(egui::vec2(0.0, 32.0))), &centre_tip).clicked() {
                if let Err(e) = m.center_hole(c.control) {
                    say = Some(e);
                }
            }
        });
        if let Some((p, touched)) = m.last_probe() {
            ui.label(
                RichText::new(trf("Last probe: X {} Y {} Z {} ({})", &[&fmt_pos(p[0]), &fmt_pos(p[1]), &fmt_pos(p[2]), &if touched { tr("touched") } else { tr("no contact") }])).small().color(theme::text_dim()),
            );
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
        });
        let half = ((ui.available_width() - ui.spacing().item_spacing.x) / 2.0).max(60.0);
        egui::Grid::new("spindle_switch").num_columns(2).show(ui, |ui| {
            let go = tip(ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Switch on")).size(15.0)).min_size(egui::vec2(half, 32.0))), &tr("Switches the spindle on to try it (M3 S…) and off again (M5). A job does this by itself."));
            if go.clicked() {
                let s_value = fmt_num(m.spindle_s(rpm, sp.max_rpm));
                m.line(format!("M3 S{s_value}"));
            }
            if ui.add_enabled(!busy, egui::Button::new(RichText::new(tr("Switch off")).size(15.0)).min_size(egui::vec2(half, 32.0))).clicked() {
                m.line("M5".to_string());
            }
        });
        st.spindle_rpm = Some(rpm);
    }

    // ---- coolant ----
    if c.spindle.is_some() {
        section_break(ui);
        heading(ui, &tr("Coolant"));
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            for (label, code, hint) in [("Mist", "M7", "Switches the mist coolant on (M7)"), ("Flood", "M8", "Switches the flood coolant, or the dust collector on a machine that has it wired so, on (M8)"), ("Off", "M9", "Switches all coolant off (M9)")] {
                if tip(ui.add_enabled(!busy, egui::Button::new(tr(label))), &tr(hint)).clicked() {
                    m.line(code.to_string());
                }
            }
        });
    }

    // ---- the overrides of a job in progress (GRBL 1.1 and newer report them) ----
    if let Some((feed, rapid, spindle)) = m.overrides() {
        section_break(ui);
        heading(ui, &tr("Overrides"));
        ui.add_space(2.0);
        // (name, current percent, [(label, byte)], the byte of 100 %)
        let rows: [(&str, u32, [(&str, u8); 4], u8); 2] = [("Feed", feed, [("-10", 0x92), ("-1", 0x94), ("+1", 0x93), ("+10", 0x91)], 0x90), ("Spindle", spindle, [("-10", 0x9B), ("-1", 0x9D), ("+1", 0x9C), ("+10", 0x9A)], 0x99)];
        egui::Grid::new("overrides").num_columns(2).spacing([10.0, 4.0]).show(ui, |ui| {
            for (name, now, steps, reset) in rows {
                ui.label(RichText::new(format!("{} {now} %", tr(name))).strong());
                ui.horizontal(|ui| {
                    for (label, byte) in steps {
                        if ui.button(label).clicked() {
                            m.realtime(byte);
                        }
                    }
                    if tip(ui.button("100 %"), &tr("Back to the speed the program asks for")).clicked() {
                        m.realtime(reset);
                    }
                });
                ui.end_row();
            }
            ui.label(RichText::new(format!("{} {rapid} %", tr("Rapids"))).strong());
            ui.horizontal(|ui| {
                for (label, byte) in [("25 %", 0x97u8), ("50 %", 0x96), ("100 %", 0x95)] {
                    if ui.button(label).clicked() {
                        m.realtime(byte);
                    }
                }
            });
            ui.end_row();
        });
    }

    // ---- places to go back to ----
    if c.hold_to_move && m.can_go_machine() {
        section_break(ui);
        heading(ui, &tr("Saved positions"));
        ui.add_space(2.0);
        let list = st.positions.get_or_insert_with(load_positions);
        let mut changed = false;
        let mut go: Option<[f64; 3]> = None;
        let mut remove: Option<usize> = None;
        for (i, sp) in list.iter().enumerate() {
            ui.horizontal(|ui| {
                if tip(ui.add_enabled(!busy, egui::Button::new(&sp.name)), &trf("Go to X {} Y {} Z {} (machine position), up first when needed", &[&fmt_pos(sp.p[0]), &fmt_pos(sp.p[1]), &fmt_pos(sp.p[2])])).clicked() {
                    go = Some(sp.p);
                }
                if tip(ui.small_button("✖"), &tr("Forget this position")).clicked() {
                    remove = Some(i);
                }
            });
        }
        if let Some(i) = remove {
            list.remove(i);
            changed = true;
        }
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut st.pos_name).desired_width(120.0).hint_text(tr("Name")));
            let can = list.len() < MAX_SAVED && m.mpos().iter().all(|v| v.is_finite());
            if tip(ui.add_enabled(can, egui::Button::new(tr("Save here"))), &tr("Remember where the tool is now")).clicked() {
                let name = if st.pos_name.trim().is_empty() { format!("P{}", list.len() + 1) } else { st.pos_name.trim().chars().take(40).collect() };
                list.push(SavedPos { name, p: mpos });
                st.pos_name.clear();
                changed = true;
            }
        });
        if changed {
            store_positions(list);
        }
        if let Some(p) = go {
            m.go_machine(p, feed_xy, c.z_axis);
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
/// What the person did on the XY pad this frame.
#[derive(Default)]
pub struct PadHit {
    /// The direction of a click (or, when holding, of a button that is down): -1, 0 or 1 for X and Y (the third is 0).
    pub dir: Option<[i8; 3]>,
    /// The square in the middle was clicked: stop the move.
    pub stop: bool,
}

/// The nine buttons of the XY jog pad, the same in every program: arrows (Y up is towards the back of the machine), the
/// four diagonals and a stop square in the middle. With `continuous` a button reports on every frame it is held down,
/// otherwise once per click.
pub fn xy_pad(ui: &mut Ui, continuous: bool) -> PadHit {
    let mut hit = PadHit::default();
    egui::Grid::new("jog_xy").spacing([3.0, 3.0]).show(ui, |ui| {
        let cells: [[(&str, [i8; 3]); 3]; 3] = [[("arrow-up-left", [-1, 1, 0]), ("arrow-up", [0, 1, 0]), ("arrow-up-right", [1, 1, 0])], [("arrow-left", [-1, 0, 0]), ("square", [0, 0, 0]), ("arrow-right", [1, 0, 0])], [("arrow-down-left", [-1, -1, 0]), ("arrow-down", [0, -1, 0]), ("arrow-down-right", [1, -1, 0])]];
        for row in cells {
            for (icon, d) in row {
                if d == [0, 0, 0] {
                    if jog_button(ui, icon, d, false, Some(Color32::from_rgb(0xe8, 0x8a, 0x1f))).is_some() {
                        hit.stop = true;
                    }
                } else if let Some(w) = jog_button(ui, icon, d, continuous, None) {
                    hit.dir = Some(w);
                }
            }
            ui.end_row();
        }
    });
    hit
}

/// One jog button: an icon of the shared store (or a short text when there is no such icon). `accent` fills it.
fn jog_button(ui: &mut Ui, what: &str, dir: [i8; 3], continuous: bool, accent: Option<Color32>) -> Option<[i8; 3]> {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(JOG[0], JOG[1]), Sense::click_and_drag());
    let hovered = r.hovered() && ui.is_enabled();
    let fill = match (accent, hovered) {
        (Some(c), true) => c.gamma_multiply(0.85),
        (Some(c), false) => c,
        (None, true) => theme::panel_dark(),
        (None, false) => ui.visuals().widgets.inactive.weak_bg_fill,
    };
    ui.painter().rect_filled(rect, 4.0, fill);
    ui.painter().rect_stroke(rect, 4.0, egui::Stroke::new(1.0, theme::border()), egui::StrokeKind::Inside);
    let ink = if accent.is_some() { Color32::WHITE } else if ui.is_enabled() { theme::text() } else { theme::text().gamma_multiply(0.4) };
    if crate::icons::svg_text(what).is_some() {
        crate::icons::paint(ui, egui::Rect::from_center_size(rect.center(), egui::vec2(30.0, 30.0)), what, ink);
    } else {
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, what, egui::FontId::proportional(26.0), ink);
    }
    let r = if what == "square" { r.on_hover_text(tr("Stop the move")) } else { r };
    let hit = if continuous { r.clicked() || r.is_pointer_button_down_on() } else { r.clicked() };
    hit.then_some(dir)
}

fn drive_hold(ui: &mut Ui, m: &mut dyn ControlTarget, st: &mut ControlUi, want: Option<[i8; 3]>, feed_xy: f64, feed_z: f64) {
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
            zeroing: true,
            z_axis: true,
            hold_to_move: true,
        };
        m.overrides = Some((100, 110, 90));
        st.positions = Some(vec![SavedPos { name: "Park".into(), p: [-1.0, -2.0, -3.0] }]);
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
        let frame = |events: Vec<egui::Event>, moves: &mut (u32, u32), pos: &mut egui::Pos2| {
            let input = egui::RawInput { screen_rect: Some(screen), events, ..Default::default() };
            let mut o = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let at = ui.cursor().min;
                    *pos = at + egui::vec2(JOG[0] / 2.0, JOG[1] / 2.0);
                    if jog_button(ui, "▲", [0, 1, 0], false, None).is_some() {
                        moves.0 += 1;
                    }
                    if jog_button(ui, "▼", [0, -1, 0], true, None).is_some() {
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
