//! Plotters that do not speak G-code: the EiBotBoard (EBB) of the AxiDraw and the EggBot, and HPGL plotters and cutters on a
//! serial port. The programs make G-code as for any machine; the link turns every line into the commands of the plotter with a
//! [`Translator`], so the buttons, the progress, the pauses and the stop work the same way.
//!
//! What is read of the G-code is what the programs write: `G0`/`G1` with `X Y Z F`, `G4 P` (seconds), `G20`/`G21`, `G90`/`G91`,
//! `G92` and `G10 L20` (a new work zero), `M3`/`M4 S`/`M5` (a servo that lifts the pen) and `$H` (back to the corner the plotter
//! started at). Everything else is left out.
use serde::{Deserialize, Serialize};

/// How the two motors of an EBB move the pen.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kinematics {
    /// AxiDraw: both motors drive one belt; motor 1 moves by X + Y, motor 2 by X − Y.
    #[default]
    AxiDraw,
    /// EggBot and similar: motor 1 turns the pen arm (Y), motor 2 the egg (X), as `doXYMove` of evil-mad's plotink sends them
    /// (`SM,time,Y,X`); the EggBot's own program reverses the pen arm, as `reverse_y` does.
    EggBot,
}

impl Kinematics {
    pub const ALL: [Kinematics; 2] = [Kinematics::AxiDraw, Kinematics::EggBot];
    pub fn label(self) -> &'static str {
        match self {
            Kinematics::AxiDraw => "AxiDraw (both motors on one belt)",
            Kinematics::EggBot => "EggBot (one motor for each axis)",
        }
    }
}

/// How a plotter on a serial port says it cannot take more: plotters do not answer every command, they stop the flow.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum FlowControl {
    /// RTS/CTS lines of the cable.
    #[default]
    Hardware,
    /// XON/XOFF characters.
    Software,
    /// Nothing: only for a plotter with a buffer larger than the job.
    None,
}

impl FlowControl {
    pub const ALL: [FlowControl; 3] = [FlowControl::Hardware, FlowControl::Software, FlowControl::None];
    pub fn label(self) -> &'static str {
        match self {
            FlowControl::Hardware => "Hardware (RTS/CTS)",
            FlowControl::Software => "Software (XON/XOFF)",
            FlowControl::None => "None",
        }
    }
}

/// The settings of a plotter that does not speak G-code, kept with the connection of a device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PlotterSettings {
    #[serde(default)]
    pub kinematics: Kinematics,
    /// Motor steps for one millimetre of belt (EBB). The AxiDraw has 80 with 1/16 microstepping.
    #[serde(default = "d_steps")]
    pub steps_per_mm: f64,
    /// The height of the pen when it is up and when it is down, 0 to 100 % of the servo's range (EBB).
    #[serde(default = "d_up")]
    pub pen_up_pct: f64,
    #[serde(default = "d_down")]
    pub pen_down_pct: f64,
    /// Turn the direction of an axis round. The AxiDraw counts Y towards the front, the programs towards the back.
    #[serde(default)]
    pub reverse_x: bool,
    #[serde(default = "yes")]
    pub reverse_y: bool,
    /// Plotter units in a millimetre (HPGL: 40).
    #[serde(default = "d_units")]
    pub hpgl_units_per_mm: f64,
    #[serde(default)]
    pub flow: FlowControl,
}

fn d_steps() -> f64 {
    80.0
}
fn d_up() -> f64 {
    60.0
}
fn d_down() -> f64 {
    30.0
}
fn d_units() -> f64 {
    40.0
}
fn yes() -> bool {
    true
}

impl Default for PlotterSettings {
    fn default() -> Self {
        PlotterSettings { kinematics: Kinematics::AxiDraw, steps_per_mm: 80.0, pen_up_pct: 60.0, pen_down_pct: 30.0, reverse_x: false, reverse_y: true, hpgl_units_per_mm: 40.0, flow: FlowControl::Hardware }
    }
}

impl PlotterSettings {
    pub fn check(&self) -> Result<(), String> {
        let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
        if !ok(self.steps_per_mm, 1.0, 10_000.0) || !ok(self.hpgl_units_per_mm, 1.0, 1000.0) {
            return Err("the steps or the units of the plotter are out of range".into());
        }
        if !ok(self.pen_up_pct, 0.0, 100.0) || !ok(self.pen_down_pct, 0.0, 100.0) {
            return Err("the pen heights of the plotter are out of range".into());
        }
        Ok(())
    }
}

/// The language of the plotter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Language {
    Ebb,
    Hpgl,
}

/// How the G-code says the pen is up or down: by the height of the Z axis, or by the S value of a servo (`M3 S…`). The value
/// nearer to `down` is down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PenSense {
    Z { up: f64, down: f64 },
    Servo { up: f64, down: f64 },
}

impl Default for PenSense {
    fn default() -> Self {
        PenSense::Z { up: 3.0, down: 0.0 }
    }
}

fn nearer_down(v: f64, up: f64, down: f64) -> bool {
    (v - down).abs() < (v - up).abs()
}

/// What the link needs to talk to a plotter.
#[derive(Clone, Debug, PartialEq)]
pub struct Setup {
    pub language: Language,
    pub settings: PlotterSettings,
    pub pen: PenSense,
    /// The speed of `G0` moves, mm/min.
    pub rapid: f64,
}

impl Setup {
    pub fn new(language: Language) -> Setup {
        Setup { language, settings: PlotterSettings::default(), pen: PenSense::default(), rapid: 3000.0 }
    }
}

/// The servo of the AxiDraw at 0 % and at 100 %, in units of 83.3 ns (`axidraw_conf.py`: `servo_min`, `servo_max`).
pub const EBB_SERVO_MIN: f64 = 9855.0;
pub const EBB_SERVO_MAX: f64 = 27831.0;
/// The EBB makes at most 25 000 steps a second on a motor; a little less is asked.
pub const EBB_MAX_STEPS_PER_MS: f64 = 24.0;
/// The longest pause one `SM` command may be.
const EBB_MAX_DELAY_MS: u64 = 100_000;
/// The slowest pen-down speed, mm/min, when the G-code never set one.
const DEFAULT_FEED: f64 = 1000.0;

/// The servo position (`SC,4` / `SC,5`) of a pen height in percent.
pub fn ebb_servo(pct: f64) -> u32 {
    (EBB_SERVO_MIN + (EBB_SERVO_MAX - EBB_SERVO_MIN) * pct.clamp(0.0, 100.0) / 100.0).round() as u32
}

/// Reads G-code line by line and writes the commands of the plotter, keeping where the pen is.
#[derive(Clone, Debug)]
pub struct Translator {
    setup: Setup,
    /// Work position (what the G-code means), mm.
    pos: [f64; 2],
    /// Machine = work + offset (`G92`, `G10 L20`).
    offset: [f64; 2],
    /// Where the motors are (EBB: motor steps) or the pen is (HPGL: plotter units), counted from where the plotter started.
    sent: [i64; 2],
    pen_down: bool,
    absolute: bool,
    inches: bool,
    /// `G0` or `G1`.
    motion: u8,
    feed: f64,
    servo_on: bool,
    /// HPGL: the speed last sent (`VS`).
    speed: Option<u32>,
}

impl Translator {
    pub fn new(setup: Setup) -> Translator {
        Translator { setup, pos: [0.0; 2], offset: [0.0; 2], sent: [0; 2], pen_down: false, absolute: true, inches: false, motion: 0, feed: DEFAULT_FEED, servo_on: false, speed: None }
    }

    pub fn language(&self) -> Language {
        self.setup.language
    }

    /// The work position, mm.
    pub fn work_position(&self) -> [f64; 2] {
        self.pos
    }

    /// The machine position (from where the plotter started), mm.
    pub fn machine_position(&self) -> [f64; 2] {
        [self.pos[0] + self.offset[0], self.pos[1] + self.offset[1]]
    }

    pub fn pen_is_down(&self) -> bool {
        self.pen_down
    }

    /// What is sent once the link is open: the motors on with 1/16 steps (this also sets the step counters to zero), the pen
    /// heights and the pen up; for HPGL a reset, the first pen and the pen up.
    pub fn start(&mut self) -> Vec<String> {
        self.pen_down = false;
        self.sent = [0; 2];
        self.pos = [0.0; 2];
        self.offset = [0.0; 2];
        match self.setup.language {
            Language::Ebb => {
                let s = &self.setup.settings;
                vec!["EM,1,1".into(), format!("SC,4,{}", ebb_servo(s.pen_up_pct)), format!("SC,5,{}", ebb_servo(s.pen_down_pct)), "SP,1".into()]
            }
            Language::Hpgl => vec!["IN;".into(), "SP1;".into(), "PU;".into()],
        }
    }

    /// What stops the plotter at once: the EBB drops its queue (`ES`), lifts the pen and says where the motors are (`QS`);
    /// an HPGL plotter only lifts the pen (what it has in its buffer is still drawn).
    pub fn stop(&mut self) -> Vec<String> {
        self.pen_down = false;
        match self.setup.language {
            Language::Ebb => vec!["ES".into(), "SP,1".into(), "QS".into()],
            Language::Hpgl => vec!["PU;".into()],
        }
    }

    /// The answer to `QS` (`1200,-340`): where the motors really are, after a stop cut a move short.
    pub fn sync_steps(&mut self, line: &str) -> bool {
        if self.setup.language != Language::Ebb {
            return false;
        }
        let mut it = line.trim().split(',');
        let (Some(a), Some(b), None) = (it.next(), it.next(), it.next()) else { return false };
        let (Ok(a), Ok(b)) = (a.trim().parse::<i64>(), b.trim().parse::<i64>()) else { return false };
        self.sent = [a, b];
        // Back from steps to millimetres.
        let s = &self.setup.settings;
        let k = s.steps_per_mm;
        let (tx, ty) = match s.kinematics {
            Kinematics::AxiDraw => ((a + b) as f64 / (2.0 * k), (a - b) as f64 / (2.0 * k)),
            Kinematics::EggBot => (b as f64 / k, a as f64 / k),
        };
        let mx = if s.reverse_x { -tx } else { tx };
        let my = if s.reverse_y { -ty } else { ty };
        self.pos = [mx - self.offset[0], my - self.offset[1]];
        true
    }

    /// The commands of one line of G-code (none for a line that changes nothing on the plotter).
    pub fn translate(&mut self, line: &str) -> Vec<String> {
        let code = strip_comments(line).to_ascii_uppercase();
        let code = code.trim();
        let mut out = vec![];
        if code.is_empty() {
            return out;
        }
        if code == "$H" {
            self.pen(false, &mut out);
            let m = self.machine_position();
            let target = [self.pos[0] - m[0], self.pos[1] - m[1]];
            self.move_to(target, self.setup.rapid, &mut out);
            return out;
        }
        // A jog of GRBL 1.1: the same move, relative for just this line.
        let (code, jog) = match code.strip_prefix("$J=") {
            Some(rest) => (rest.trim(), true),
            None => (code, false),
        };
        if code.starts_with('$') {
            return out;
        }
        let words = words(code);
        let num = |l: char| words.iter().find(|(c, _)| *c == l).map(|(_, v)| *v);
        let gs: Vec<f64> = words.iter().filter(|(c, _)| *c == 'G').map(|(_, v)| *v).collect();
        let ms: Vec<f64> = words.iter().filter(|(c, _)| *c == 'M').map(|(_, v)| *v).collect();
        let has = |list: &[f64], v: f64| list.iter().any(|g| (g - v).abs() < 1e-6);
        let was_absolute = self.absolute;
        for g in &gs {
            match *g as i64 {
                20 if g.fract() == 0.0 => self.inches = true,
                21 if g.fract() == 0.0 => self.inches = false,
                90 if g.fract() == 0.0 => self.absolute = true,
                91 if g.fract() == 0.0 => self.absolute = false,
                0 => self.motion = 0,
                1 => self.motion = 1,
                _ => {}
            }
        }
        let unit = if self.inches { 25.4 } else { 1.0 };
        if let Some(f) = num('F').filter(|f| *f > 0.0) {
            self.feed = f * unit;
        }
        if has(&gs, 4.0) {
            // G4 P is seconds in GRBL.
            let ms_total = (num('P').unwrap_or(0.0).max(0.0) * 1000.0).round() as u64;
            self.dwell(ms_total, &mut out);
            return out;
        }
        // A new work zero: G92 X.. Y.., or G10 L20 P.. X.. Y.. (the programs' "set position").
        let set_zero = has(&gs, 92.0) || (has(&gs, 10.0) && num('L').is_some_and(|l| (l - 20.0).abs() < 1e-6));
        if set_zero {
            let m = self.machine_position();
            for (i, l) in ['X', 'Y'].into_iter().enumerate() {
                if let Some(v) = num(l) {
                    self.pos[i] = v * unit;
                    self.offset[i] = m[i] - self.pos[i];
                }
            }
            return out;
        }
        if has(&gs, 10.0) || has(&gs, 28.0) || has(&gs, 53.0) && num('X').is_none() && num('Y').is_none() {
            return out;
        }
        // The pen: the Z axis, or the servo.
        match self.setup.pen {
            PenSense::Z { up, down } => {
                if let Some(z) = num('Z') {
                    let z = z * unit;
                    let abs_z = if self.absolute && !jog { z } else if self.pen_down { down + z } else { up + z };
                    self.pen(nearer_down(abs_z, up, down), &mut out);
                }
            }
            PenSense::Servo { up, down } => {
                if has(&ms, 5.0) {
                    self.servo_on = false;
                    self.pen(false, &mut out);
                } else if has(&ms, 3.0) || has(&ms, 4.0) || (self.servo_on && num('S').is_some()) {
                    self.servo_on = true;
                    if let Some(s) = num('S') {
                        self.pen(nearer_down(s, up, down), &mut out);
                    }
                }
            }
        }
        if num('X').is_some() || num('Y').is_some() {
            let mut target = self.pos;
            for (i, l) in ['X', 'Y'].into_iter().enumerate() {
                if let Some(v) = num(l) {
                    let v = v * unit;
                    target[i] = if self.absolute && !jog { v } else { self.pos[i] + v };
                }
            }
            if has(&gs, 53.0) {
                // Machine coordinates for this line only.
                target = [target[0] - self.offset[0], target[1] - self.offset[1]];
            }
            let feed = if jog || self.motion == 1 { self.feed } else { self.setup.rapid };
            self.move_to(target, feed, &mut out);
        }
        if jog {
            self.absolute = was_absolute;
        }
        out
    }

    fn dwell(&mut self, ms: u64, out: &mut Vec<String>) {
        if self.setup.language != Language::Ebb {
            return;
        }
        let mut left = ms;
        while left > 0 {
            let d = left.min(EBB_MAX_DELAY_MS);
            out.push(format!("SM,{d},0,0"));
            left -= d;
        }
    }

    fn pen(&mut self, down: bool, out: &mut Vec<String>) {
        if down == self.pen_down {
            return;
        }
        self.pen_down = down;
        out.push(match (self.setup.language, down) {
            (Language::Ebb, true) => "SP,0".into(),
            (Language::Ebb, false) => "SP,1".into(),
            (Language::Hpgl, true) => "PD;".into(),
            (Language::Hpgl, false) => "PU;".into(),
        });
    }

    /// Where the motors (or the pen of an HPGL plotter) have to be for a machine position.
    fn device_target(&self, machine: [f64; 2]) -> [i64; 2] {
        let s = &self.setup.settings;
        let x = if s.reverse_x { -machine[0] } else { machine[0] };
        let y = if s.reverse_y { -machine[1] } else { machine[1] };
        match self.setup.language {
            Language::Hpgl => {
                // HPGL counts Y up from the front, as the programs do: the reversing is for the EBB only.
                let u = s.hpgl_units_per_mm;
                [(machine[0] * u).round() as i64, (machine[1] * u).round() as i64]
            }
            Language::Ebb => {
                let k = s.steps_per_mm;
                match s.kinematics {
                    Kinematics::AxiDraw => [((x + y) * k).round() as i64, ((x - y) * k).round() as i64],
                    Kinematics::EggBot => [(y * k).round() as i64, (x * k).round() as i64],
                }
            }
        }
    }

    fn move_to(&mut self, target: [f64; 2], feed: f64, out: &mut Vec<String>) {
        let dist = ((target[0] - self.pos[0]).powi(2) + (target[1] - self.pos[1]).powi(2)).sqrt();
        self.pos = target;
        let t = self.device_target(self.machine_position());
        // The rounded positions are kept, and the differences between them sent: rounding each move would add up.
        let d = [t[0] - self.sent[0], t[1] - self.sent[1]];
        if d == [0, 0] {
            return;
        }
        self.sent = t;
        match self.setup.language {
            Language::Ebb => {
                let by_feed = dist / feed.max(1.0) * 60_000.0;
                let by_rate = d[0].abs().max(d[1].abs()) as f64 / EBB_MAX_STEPS_PER_MS;
                let ms = by_feed.max(by_rate).ceil().max(1.0) as u64;
                out.push(format!("SM,{ms},{},{}", d[0], d[1]));
            }
            Language::Hpgl => {
                if self.pen_down {
                    // VS is centimetres a second.
                    let vs = (feed / 600.0).round().clamp(1.0, 38.0) as u32;
                    if self.speed != Some(vs) {
                        self.speed = Some(vs);
                        out.push(format!("VS{vs};"));
                    }
                }
                out.push(format!("PA{},{};", t[0], t[1]));
            }
        }
    }
}

fn strip_comments(line: &str) -> String {
    let mut out = String::new();
    let mut depth = 0;
    for c in line.chars() {
        match c {
            ';' if depth == 0 => break,
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out
}

/// The words of a line: a letter and its number (`G1`, `X-2.5`).
fn words(code: &str) -> Vec<(char, f64)> {
    let mut out = vec![];
    let chars: Vec<char> = code.chars().filter(|c| !c.is_whitespace()).collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        i += 1;
        if !c.is_ascii_alphabetic() {
            continue;
        }
        let start = i;
        while i < chars.len() && (chars[i].is_ascii_digit() || matches!(chars[i], '.' | '-' | '+')) {
            i += 1;
        }
        if let Ok(v) = chars[start..i].iter().collect::<String>().parse::<f64>() {
            out.push((c, v));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ebb() -> Translator {
        let mut t = Translator::new(Setup { language: Language::Ebb, settings: PlotterSettings { reverse_y: false, ..Default::default() }, pen: PenSense::Z { up: 3.0, down: 0.0 }, rapid: 6000.0 });
        t.start();
        t
    }

    fn sm(cmd: &str) -> (u64, i64, i64) {
        let p: Vec<&str> = cmd.split(',').collect();
        assert_eq!(p[0], "SM", "{cmd}");
        (p[1].parse().unwrap(), p[2].parse().unwrap(), p[3].parse().unwrap())
    }

    #[test]
    fn the_axidraw_mixes_the_motors_and_takes_the_time_from_the_speed() {
        let mut t = ebb();
        // 10 mm in X at 600 mm/min is one second; both motors turn by 800 steps.
        assert_eq!(t.translate("G1 X10 F600"), vec!["SM,1000,800,800"]);
        // 10 mm in Y: the motors turn in opposite directions.
        assert_eq!(t.translate("G1 Y10"), vec!["SM,1000,800,-800"]);
        assert_eq!(t.work_position(), [10.0, 10.0]);
        // A G0 goes at the rapid speed: 14.14 mm at 6000 mm/min.
        let (ms, a, b) = sm(&t.translate("G0 X0 Y0")[0]);
        assert_eq!((a, b, ms), (-1600, 0, 142));
        assert!(t.translate("G0 X0 Y0").is_empty(), "no move, no command");
        // But never faster than the motors can step: 100 mm is 8000 steps.
        let (ms, ..) = sm(&t.translate("G1 X100 F100000")[0]);
        assert_eq!(ms, (8000.0 / EBB_MAX_STEPS_PER_MS).ceil() as u64);
    }

    #[test]
    fn the_eggbot_has_a_motor_for_each_axis_and_the_axes_can_be_reversed() {
        // Motor 1 is the pen arm (Y), motor 2 the egg (X).
        let mut t = Translator::new(Setup { language: Language::Ebb, settings: PlotterSettings { kinematics: Kinematics::EggBot, reverse_x: true, reverse_y: false, ..Default::default() }, pen: PenSense::default(), rapid: 6000.0 });
        let (_, a, b) = sm(&t.translate("G1 X1 Y2 F600")[0]);
        assert_eq!((a, b), (160, -80));
        // With the default reversed Y, as the EggBot's own program drives the pen arm.
        let mut e = Translator::new(Setup { language: Language::Ebb, settings: PlotterSettings { kinematics: Kinematics::EggBot, ..Default::default() }, pen: PenSense::default(), rapid: 6000.0 });
        let (_, a, b) = sm(&e.translate("G1 X1 Y2 F600")[0]);
        assert_eq!((a, b), (-160, 80));
        assert!(e.sync_steps("-80,40"));
        assert_eq!(e.work_position(), [0.5, 1.0]);
    }

    #[test]
    fn rounding_does_not_add_up_over_many_small_moves() {
        let mut t = ebb();
        let mut total = [0i64; 2];
        for i in 1..=1000 {
            for c in t.translate(&format!("G1 X{:.4} F3000", i as f64 * 0.0137)) {
                let (_, a, b) = sm(&c);
                total[0] += a;
                total[1] += b;
            }
        }
        // 13.7 mm is 1096 steps on each motor, however the moves were cut.
        assert_eq!(total, [1096, 1096]);
    }

    #[test]
    fn the_pen_follows_z_or_the_servo_and_a_pause_becomes_a_delay() {
        let mut t = ebb();
        assert_eq!(t.translate("G0 Z0"), vec!["SP,0"]);
        assert!(t.translate("G0 Z0.2").is_empty(), "still nearer down");
        assert_eq!(t.translate("G0 Z3 (up)"), vec!["SP,1"]);
        assert_eq!(t.translate("G4 P0.15"), vec!["SM,150,0,0"]);
        assert_eq!(t.translate("G4 P250").len(), 3, "long pauses are cut into pieces of 100 s");
        let mut s = Translator::new(Setup { language: Language::Ebb, settings: PlotterSettings::default(), pen: PenSense::Servo { up: 800.0, down: 300.0 }, rapid: 6000.0 });
        assert_eq!(s.translate("M3 S300"), vec!["SP,0"]);
        assert_eq!(s.translate("M3 S800"), vec!["SP,1"]);
        assert_eq!(s.translate("M3 S300"), vec!["SP,0"]);
        assert_eq!(s.translate("M5"), vec!["SP,1"]);
    }

    #[test]
    fn the_start_sets_the_servo_and_a_stop_lifts_the_pen_and_asks_where_the_motors_are() {
        let mut t = ebb();
        let start = t.start();
        assert_eq!(start, vec!["EM,1,1", "SC,4,20641", "SC,5,15248", "SP,1"]);
        assert_eq!((ebb_servo(0.0), ebb_servo(100.0)), (9855, 27831));
        t.translate("G0 Z0");
        assert_eq!(t.stop(), vec!["ES", "SP,1", "QS"]);
        assert!(!t.pen_is_down());
        // The motors stopped half way to X10: the position comes back from the step counters.
        t.translate("G1 X10 F600");
        assert!(t.sync_steps("400,400"));
        assert_eq!(t.work_position(), [5.0, 0.0]);
        assert_eq!(t.translate("G1 X10"), vec!["SM,500,400,400"]);
        assert!(!t.sync_steps("OK") && !t.sync_steps("1"));
    }

    #[test]
    fn relative_moves_jogs_inches_new_zero_and_home() {
        let mut t = ebb();
        t.translate("G91 G21");
        t.translate("G1 X1 Y1 F600");
        t.translate("G1 X1");
        t.translate("G90");
        assert_eq!(t.work_position(), [2.0, 1.0]);
        t.translate("$J=G91 G21 X-1 F600");
        assert_eq!(t.work_position(), [1.0, 1.0]);
        t.translate("G1 X3");
        assert_eq!(t.work_position(), [3.0, 1.0], "the jog left the absolute mode on");
        t.translate("G10 L20 P1 X0 Y0");
        assert_eq!((t.work_position(), t.machine_position()), ([0.0, 0.0], [3.0, 1.0]));
        t.translate("G20");
        t.translate("G1 X1");
        assert!((t.machine_position()[0] - 28.4).abs() < 1e-9);
        t.translate("G21");
        t.translate("G0 Z0");
        let home = t.translate("$H");
        assert_eq!(home[0], "SP,1", "the pen goes up first");
        assert_eq!(t.machine_position(), [0.0, 0.0]);
        for l in ["$X", "$$", "$I", "$C", "G54", "M0", "M30", "(comment)", "; note", "G28"] {
            assert!(t.translate(l).is_empty(), "{l}");
        }
    }

    #[test]
    fn hpgl_draws_with_absolute_plotter_units_and_counts_y_up() {
        let mut t = Translator::new(Setup { language: Language::Hpgl, settings: PlotterSettings::default(), pen: PenSense::Z { up: 3.0, down: 0.0 }, rapid: 6000.0 });
        assert_eq!(t.start(), vec!["IN;", "SP1;", "PU;"]);
        assert_eq!(t.translate("G0 X10 Y5"), vec!["PA400,200;"]);
        assert_eq!(t.translate("G0 Z0"), vec!["PD;"]);
        assert_eq!(t.translate("G1 X20 F1200"), vec!["VS2;", "PA800,200;"]);
        assert_eq!(t.translate("G1 Y10"), vec!["PA800,400;"], "the speed is sent only when it changes");
        assert_eq!(t.translate("G0 Z3"), vec!["PU;"]);
        assert!(t.translate("G4 P0.2").is_empty());
        assert_eq!(t.stop(), vec!["PU;"]);
    }

    #[test]
    fn the_settings_are_checked_and_an_old_file_gets_the_defaults() {
        assert!(PlotterSettings::default().check().is_ok());
        assert!(PlotterSettings { pen_up_pct: 120.0, ..Default::default() }.check().is_err());
        assert!(PlotterSettings { steps_per_mm: f64::NAN, ..Default::default() }.check().is_err());
        let s: PlotterSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(s, PlotterSettings::default());
    }
}
