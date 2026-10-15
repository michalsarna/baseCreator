//! What a machine needs besides its size: how far a jog goes, how the probe is set up, the macros and the G-code run
//! around a job, and the statistics and maintenance reminders of the machine. Kept with the device by every program that
//! drives a GRBL controller.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const MAX_MACROS: usize = 16;
pub const MAX_REMINDERS: usize = 8;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Jog {
    /// The distances of the step buttons for X and Y, mm.
    #[serde(deserialize_with = "de_xy_steps")]
    pub xy_steps: Vec<f64>,
    /// The distances for Z, mm.
    #[serde(deserialize_with = "de_z_steps")]
    pub z_steps: Vec<f64>,
    /// Feed of X and Y jogs, mm/min.
    pub xy_feed: f64,
    pub z_feed: f64,
}

/// The step buttons a device starts with.
pub const XY_STEPS: [f64; 5] = [0.1, 1.0, 10.0, 50.0, 100.0];
pub const Z_STEPS: [f64; 7] = [0.1, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0];

impl Default for Jog {
    fn default() -> Jog {
        Jog { xy_steps: XY_STEPS.to_vec(), z_steps: Z_STEPS.to_vec(), xy_feed: 3000.0, z_feed: 500.0 }
    }
}

/// Devices saved with the buttons of the first versions (never changed by the person) get the new ones.
fn de_steps<'de, D: serde::Deserializer<'de>>(d: D, old: &[f64], new: &[f64]) -> Result<Vec<f64>, D::Error> {
    let v = Vec::<f64>::deserialize(d)?;
    Ok(if v == old { new.to_vec() } else { v })
}

fn de_xy_steps<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<f64>, D::Error> {
    de_steps(d, &[0.1, 1.0, 10.0, 50.0], &XY_STEPS)
}

fn de_z_steps<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Vec<f64>, D::Error> {
    de_steps(d, &[0.05, 0.5, 5.0, 20.0], &Z_STEPS)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProbeKind {
    /// A touch plate of a known thickness: the tool touches it.
    Plate,
    /// A 3D touch probe with a ball of a known diameter.
    Touch3d,
}

impl ProbeKind {
    pub const ALL: [ProbeKind; 2] = [ProbeKind::Plate, ProbeKind::Touch3d];
    pub fn label(self) -> &'static str {
        match self {
            ProbeKind::Plate => "Touch plate",
            ProbeKind::Touch3d => "3D touch probe",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Probe {
    pub kind: ProbeKind,
    /// The height of the plate (or what the probe adds) that Z gets at the moment of touch, mm.
    pub z_offset: f64,
    /// The thickness of the plate's side, between the tool and the edge of the work, mm (touch plate only).
    pub side_offset: f64,
    /// Diameter of the ball of a 3D probe, mm.
    pub ball_diameter: f64,
    /// How far a probe move may go before it fails, mm.
    pub fail_distance: f64,
    /// Feed of the first, quick touch and of the slow, exact one, mm/min.
    pub seek_feed: f64,
    pub slow_feed: f64,
    /// How far the tool backs off between the two touches, mm.
    pub retract: f64,
    /// How far the tool moves away after the zero is set, mm.
    pub lift: f64,
}

impl Default for Probe {
    fn default() -> Probe {
        Probe { kind: ProbeKind::Plate, z_offset: 19.0, side_offset: 10.0, ball_diameter: 2.0, fail_distance: 30.0, seek_feed: 150.0, slow_feed: 40.0, retract: 2.0, lift: 10.0 }
    }
}

impl Probe {
    pub fn check(&self) -> Result<(), &'static str> {
        let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
        if !ok(self.z_offset, 0.0, 200.0) || !ok(self.side_offset, 0.0, 200.0) || !ok(self.ball_diameter, 0.0, 50.0) {
            return Err("a probe size is out of range");
        }
        if !ok(self.fail_distance, 1.0, 1000.0) || !ok(self.retract, 0.1, 100.0) || !ok(self.lift, 0.0, 200.0) {
            return Err("a probe distance is out of range");
        }
        if !ok(self.seek_feed, 1.0, 10_000.0) || !ok(self.slow_feed, 1.0, 10_000.0) || self.slow_feed > self.seek_feed {
            return Err("the slow probe feed must be between 1 and the quick one");
        }
        if self.fail_distance <= self.retract * 2.0 {
            return Err("the fail distance must be longer than twice the retract");
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    X,
    Y,
    Z,
}

impl Axis {
    fn letter(self) -> char {
        match self {
            Axis::X => 'X',
            Axis::Y => 'Y',
            Axis::Z => 'Z',
        }
    }
}

fn n(v: f64) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" { "0".into() } else { s.to_string() }
}

/// G-code of one probe cycle. Z goes down. For X and Y, `dir` is +1 or -1: the way the tool moves towards the edge,
/// which becomes zero (the tool of `tool_diameter` mm, or the ball, is taken into account). `wcs` is 1 for G54 … 6 for G59.
/// The cycle ends with the tool `lift` mm away from where it touched.
pub fn probe_lines(p: &Probe, axis: Axis, dir: i8, tool_diameter: f64, wcs: u8) -> Vec<String> {
    let l = axis.letter();
    let sign = if axis == Axis::Z { -1.0 } else if dir < 0 { -1.0 } else { 1.0 };
    let set = if axis == Axis::Z {
        p.z_offset
    } else {
        let r = match p.kind {
            ProbeKind::Plate => tool_diameter / 2.0 + p.side_offset,
            ProbeKind::Touch3d => p.ball_diameter / 2.0,
        };
        -sign * r
    };
    vec![
        "G21".into(),
        "G91".into(),
        format!("G38.2 {l}{} F{}", n(sign * p.fail_distance), n(p.seek_feed)),
        format!("G0 {l}{}", n(-sign * p.retract)),
        format!("G38.2 {l}{} F{}", n(sign * p.retract * 2.0), n(p.slow_feed)),
        "G90".into(),
        format!("G10 L20 P{wcs} {l}{}", n(set)),
        "G91".into(),
        format!("G0 {l}{}", n(-sign * p.lift)),
        "G90".into(),
    ]
}

/// Lines that set the work position of the axes in `axes` to the given values (`G10 L20`), for the system `wcs`.
pub fn set_position_lines(wcs: u8, values: &[(Axis, f64)]) -> Vec<String> {
    let words: Vec<String> = values.iter().map(|(a, v)| format!("{}{}", a.letter(), n(*v))).collect();
    if words.is_empty() {
        return vec![];
    }
    vec![format!("G10 L20 P{wcs} {}", words.join(" "))]
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Macro {
    pub name: String,
    pub code: String,
}

/// G-code run before a job starts and after it ends (finished or stopped).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Hooks {
    pub before_job: String,
    pub after_job: String,
}

/// Variables a macro may use: `[posx]`, `[posy]`, `[posz]` (work position), `[mposx]`…, `[xmin]` … `[zmax]` (the box of
/// the job), `[safez]`, `[prbx]`, `[prby]`, `[prbz]` (where the last probe stopped).
pub const VARIABLES: &[&str] = &["posx", "posy", "posz", "mposx", "mposy", "mposz", "xmin", "xmax", "ymin", "ymax", "zmin", "zmax", "safez", "prbx", "prby", "prbz"];

/// Is this line something a macro must never send?
pub fn is_forbidden(line: &str) -> bool {
    let l = line.trim().to_uppercase();
    l.starts_with("$RST") || l.starts_with("$SLP")
}

/// The lines of a macro with its variables replaced; comments (`;`) and empty lines are dropped. An unknown variable, a
/// missing value or a forbidden line is an error that names it.
pub fn expand(code: &str, vars: &BTreeMap<String, f64>) -> Result<Vec<String>, String> {
    let mut out = vec![];
    for raw in code.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(';') {
            continue;
        }
        let mut text = String::new();
        let mut rest = line;
        while let Some(i) = rest.find('[') {
            text.push_str(&rest[..i]);
            let Some(j) = rest[i..].find(']') else { return Err(format!("a [ is not closed in: {line}")) };
            let name = rest[i + 1..i + j].trim().to_lowercase();
            if name.starts_with("msg") || name.contains(':') {
                // A GRBL message comment, not a variable.
                text.push_str(&rest[i..i + j + 1]);
            } else {
                let v = vars.get(&name).ok_or_else(|| format!("unknown variable [{name}]"))?;
                if !v.is_finite() {
                    return Err(format!("[{name}] has no value yet"));
                }
                text.push_str(&n(*v));
            }
            rest = &rest[i + j + 1..];
        }
        text.push_str(rest);
        if is_forbidden(&text) {
            return Err(format!("not allowed in a macro: {text}"));
        }
        out.push(text);
    }
    Ok(out)
}

/// A job that was run, an alarm or an error, kept per machine.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub jobs: u32,
    pub run_seconds: u64,
    pub alarms: u32,
    pub errors: u32,
    pub alarm_codes: BTreeMap<u32, u32>,
    pub error_codes: BTreeMap<u32, u32>,
}

impl Stats {
    pub fn job_done(&mut self, seconds: u64) {
        self.jobs = self.jobs.saturating_add(1);
        self.run_seconds = self.run_seconds.saturating_add(seconds);
    }
    pub fn alarm(&mut self, code: u32) {
        self.alarms = self.alarms.saturating_add(1);
        *self.alarm_codes.entry(code).or_insert(0) += 1;
    }
    pub fn error(&mut self, code: u32) {
        self.errors = self.errors.saturating_add(1);
        *self.error_codes.entry(code).or_insert(0) += 1;
    }
    pub fn hours(&self) -> f64 {
        self.run_seconds as f64 / 3600.0
    }
}

/// "Oil the rails every 50 hours of running."
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Reminder {
    pub name: String,
    pub every_hours: f64,
    /// The running hours of the machine when it was last done.
    pub done_at_hours: f64,
}

impl Reminder {
    pub fn due(&self, stats: &Stats) -> bool {
        self.every_hours > 0.0 && stats.hours() - self.done_at_hours >= self.every_hours
    }
    pub fn hours_left(&self, stats: &Stats) -> f64 {
        self.every_hours - (stats.hours() - self.done_at_hours)
    }
}

/// The settings of the controller as they were at some time, kept with the device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FwCopy {
    /// Unix time (seconds) when the copy was made.
    pub secs: u64,
    /// What the controller called itself (`Grbl 1.1h`).
    pub firmware: String,
    /// Why: `auto` (made at connecting because something changed), `own` (after a change made here), `manual`.
    pub why: String,
    pub values: BTreeMap<u32, String>,
}

/// One setting that differs between two readings.
#[derive(Clone, Debug, PartialEq)]
pub struct Change {
    pub id: u32,
    pub old: Option<String>,
    pub new: Option<String>,
}

/// What `FwCopies::note` found.
#[derive(Clone, Debug, PartialEq)]
pub enum Noted {
    /// The first reading: a first copy was made.
    First,
    /// The same as the newest copy.
    Same,
    /// Different from the newest copy; a new copy was made.
    Changed(Vec<Change>),
    /// Something changed after a write made here; a new copy was made.
    Own(Vec<Change>),
    /// Nothing was read.
    Nothing,
}

/// The copies of the settings of the controller, the newest last.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct FwCopies(pub Vec<FwCopy>);

/// How many copies are kept; the oldest go first.
pub const MAX_COPIES: usize = 8;

/// The settings that differ: in `new` but not in `old`, changed, or gone.
pub fn diff(old: &BTreeMap<u32, String>, new: &BTreeMap<u32, String>) -> Vec<Change> {
    let mut out = vec![];
    for (id, v) in new {
        match old.get(id) {
            Some(o) if crate::grbl_settings::same_value(o, v) => {}
            o => out.push(Change { id: *id, old: o.cloned(), new: Some(v.clone()) }),
        }
    }
    for (id, o) in old {
        if !new.contains_key(id) {
            out.push(Change { id: *id, old: Some(o.clone()), new: None });
        }
    }
    out.sort_by_key(|c| c.id);
    out
}

impl FwCopies {
    pub fn newest(&self) -> Option<&FwCopy> {
        self.0.last()
    }

    fn push(&mut self, secs: u64, firmware: &str, why: &str, values: &BTreeMap<u32, String>) {
        self.0.push(FwCopy { secs, firmware: firmware.to_string(), why: why.to_string(), values: values.clone() });
        let extra = self.0.len().saturating_sub(MAX_COPIES);
        self.0.drain(..extra);
    }

    /// A copy made by hand, even when nothing changed. `false` when there is nothing to copy.
    pub fn make(&mut self, secs: u64, firmware: &str, values: &BTreeMap<u32, String>) -> bool {
        if values.is_empty() {
            return false;
        }
        self.push(secs, firmware, "manual", values);
        true
    }

    /// A reading of the settings arrived (at connecting, or after a write made here when `own`): compare it with the
    /// newest copy and make a new one when it differs.
    pub fn note(&mut self, secs: u64, firmware: &str, values: &BTreeMap<u32, String>, own: bool) -> Noted {
        if values.is_empty() {
            return Noted::Nothing;
        }
        let Some(last) = self.newest() else {
            self.push(secs, firmware, if own { "own" } else { "auto" }, values);
            return Noted::First;
        };
        let changes = diff(&last.values, values);
        if changes.is_empty() {
            return Noted::Same;
        }
        self.push(secs, firmware, if own { "own" } else { "auto" }, values);
        if own {
            Noted::Own(changes)
        } else {
            Noted::Changed(changes)
        }
    }

    pub fn remove(&mut self, i: usize) {
        if i < self.0.len() {
            self.0.remove(i);
        }
    }

    pub fn check(&self) -> Result<(), &'static str> {
        if self.0.len() > MAX_COPIES || self.0.iter().any(|c| c.values.len() > 300 || c.firmware.len() > 80 || c.why.len() > 16 || c.values.values().any(|v| v.len() > 40)) {
            return Err("the copies of the settings are not valid");
        }
        Ok(())
    }
}

/// The tool length sensor: a switch at a fixed place on the machine that the tip of every tool touches, so that a new
/// tool gets the same Z as the one before it. Positions are in machine coordinates (the machine must be homed).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Tls {
    /// The position was set (by moving the tool over the sensor).
    pub set: bool,
    pub x: f64,
    pub y: f64,
    /// Machine Z where the quick approach starts: a little above the sensor.
    pub start_z: f64,
    /// Machine Z where the tool moves freely (above everything), mm.
    pub clear_z: f64,
    pub fail_distance: f64,
    pub seek_feed: f64,
    pub slow_feed: f64,
    pub retract: f64,
    /// Work Z of the tip of the reference tool when it touched the sensor; `None` until it was measured.
    pub ref_wz: Option<f64>,
}

impl Default for Tls {
    fn default() -> Tls {
        Tls { set: false, x: 0.0, y: 0.0, start_z: -20.0, clear_z: -5.0, fail_distance: 60.0, seek_feed: 200.0, slow_feed: 40.0, retract: 3.0, ref_wz: None }
    }
}

impl Tls {
    pub fn check(&self) -> Result<(), &'static str> {
        let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
        if !ok(self.x, -20_000.0, 20_000.0) || !ok(self.y, -20_000.0, 20_000.0) || !ok(self.start_z, -5000.0, 5000.0) || !ok(self.clear_z, -5000.0, 5000.0) {
            return Err("a position of the sensor is out of range");
        }
        if !ok(self.fail_distance, 1.0, 1000.0) || !ok(self.retract, 0.1, 100.0) || self.fail_distance <= self.retract * 2.0 {
            return Err("the fail distance must be longer than twice the retract");
        }
        if !ok(self.seek_feed, 1.0, 10_000.0) || !ok(self.slow_feed, 1.0, 10_000.0) || self.slow_feed > self.seek_feed {
            return Err("the slow probe feed must be between 1 and the quick one");
        }
        if self.start_z > self.clear_z {
            return Err("the start of the approach must not be higher than the travel height");
        }
        if self.ref_wz.is_some_and(|v| !v.is_finite()) {
            return Err("the reference is not valid");
        }
        Ok(())
    }

    /// G-code that takes the tool over the sensor and probes it twice, the second time slowly. The tool ends where it
    /// touched. Two `[PRB:…]` answers follow; the second one is the measurement.
    pub fn measure_lines(&self) -> Vec<String> {
        vec![
            "G21".into(),
            format!("G53 G0 Z{}", n(self.clear_z)),
            format!("G53 G0 X{} Y{}", n(self.x), n(self.y)),
            format!("G53 G0 Z{}", n(self.start_z)),
            "G91".into(),
            format!("G38.2 Z-{} F{}", n(self.fail_distance), n(self.seek_feed)),
            format!("G0 Z{}", n(self.retract)),
            format!("G38.2 Z-{} F{}", n(self.retract * 2.0), n(self.slow_feed)),
            "G90".into(),
        ]
    }

    /// What follows a measurement of a new tool: the work Z of this place becomes the one of the reference tool, and the
    /// tool goes up.
    pub fn adopt_lines(&self, wcs: u8, ref_wz: f64) -> Vec<String> {
        vec![format!("G10 L20 P{wcs} Z{}", n(ref_wz)), format!("G53 G0 Z{}", n(self.clear_z))]
    }

    /// What follows a measurement that changes nothing: the tool goes up.
    pub fn leave_lines(&self) -> Vec<String> {
        vec![format!("G53 G0 Z{}", n(self.clear_z))]
    }
}

/// All of it, as one field of a device.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Control {
    pub jog: Jog,
    pub probe: Probe,
    pub macros: Vec<Macro>,
    pub hooks: Hooks,
    pub stats: Stats,
    pub reminders: Vec<Reminder>,
    /// Run the program in check mode (`$C`) before the real run.
    pub check_first: bool,
    /// Copies of the settings of the controller.
    pub fw_copies: FwCopies,
    /// The tool length sensor.
    pub tls: Tls,
}

impl Control {
    /// Are the numbers usable? For what is read from a file.
    pub fn check(&self) -> Result<(), String> {
        self.probe.check().map_err(|e| e.to_string())?;
        self.tls.check().map_err(|e| e.to_string())?;
        self.fw_copies.check().map_err(|e| e.to_string())?;
        let steps_ok = |v: &Vec<f64>| !v.is_empty() && v.len() <= 8 && v.iter().all(|s| s.is_finite() && *s > 0.0 && *s <= 1000.0);
        if !steps_ok(&self.jog.xy_steps) || !steps_ok(&self.jog.z_steps) || !(1.0..=100_000.0).contains(&self.jog.xy_feed) || !(1.0..=100_000.0).contains(&self.jog.z_feed) {
            return Err("the jog steps or feeds are out of range".into());
        }
        if self.macros.len() > MAX_MACROS || self.macros.iter().any(|m| m.name.len() > 60 || m.code.len() > 20_000) || self.hooks.before_job.len() > 20_000 || self.hooks.after_job.len() > 20_000 {
            return Err("too many macros or a macro is too long".into());
        }
        if self.reminders.len() > MAX_REMINDERS || self.reminders.iter().any(|r| r.name.len() > 80 || !r.every_hours.is_finite() || r.every_hours < 0.0 || !r.done_at_hours.is_finite()) {
            return Err("a maintenance reminder is not valid".into());
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_z_probe_sets_zero_to_the_plate_and_lifts() {
        let p = Probe::default();
        let g = probe_lines(&p, Axis::Z, -1, 3.0, 1);
        assert_eq!(g[2], "G38.2 Z-30 F150");
        assert_eq!(g[3], "G0 Z2");
        assert_eq!(g[4], "G38.2 Z-4 F40");
        assert_eq!(g[6], "G10 L20 P1 Z19");
        assert_eq!(g[8], "G0 Z10");
        // Relative moves are bracketed by G91 … G90.
        assert_eq!(g.iter().filter(|s| *s == "G91").count(), 2);
        assert_eq!(g.last().map(String::as_str), Some("G90"));
    }

    #[test]
    fn an_edge_probe_takes_the_tool_or_the_ball_into_account() {
        let mut p = Probe::default();
        // Plate: tool 6 mm, plate side 10 mm; moving towards +X the tool centre ends 13 mm short of the edge.
        let g = probe_lines(&p, Axis::X, 1, 6.0, 2);
        assert_eq!(g[2], "G38.2 X30 F150");
        assert_eq!(g[3], "G0 X-2");
        assert_eq!(g[6], "G10 L20 P2 X-13");
        assert_eq!(g[8], "G0 X-10");
        let g = probe_lines(&p, Axis::Y, -1, 6.0, 1);
        assert_eq!(g[2], "G38.2 Y-30 F150");
        assert_eq!(g[6], "G10 L20 P1 Y13");
        p.kind = ProbeKind::Touch3d;
        p.ball_diameter = 4.0;
        assert_eq!(probe_lines(&p, Axis::X, -1, 6.0, 1)[6], "G10 L20 P1 X2");
    }

    #[test]
    fn probe_settings_are_checked() {
        assert!(Probe::default().check().is_ok());
        let mut p = Probe::default();
        p.slow_feed = 500.0;
        assert!(p.check().is_err());
        p = Probe::default();
        p.fail_distance = 3.0;
        assert!(p.check().is_err());
        p = Probe::default();
        p.z_offset = f64::NAN;
        assert!(p.check().is_err());
    }

    #[test]
    fn macros_get_their_variables_and_lose_comments() {
        let vars: BTreeMap<String, f64> = [("posx", 1.5), ("safez", 5.0), ("zmax", f64::NAN)].into_iter().map(|(k, v)| (k.to_string(), v)).collect();
        let g = expand("; go\nG0 Z[safez]\n\n G0 X[posx] Y[ posx ]\n(MSG, hello)\n", &vars).unwrap();
        assert_eq!(g, vec!["G0 Z5", "G0 X1.5 Y1.5", "(MSG, hello)"]);
        assert!(expand("G0 X[nope]", &vars).unwrap_err().contains("nope"));
        assert!(expand("G0 Z[zmax]", &vars).unwrap_err().contains("no value"));
        assert!(expand("G0 X[posx", &vars).is_err());
        assert!(expand("$RST=*", &vars).is_err());
        assert!(expand("$H\n$X", &vars).is_ok());
        assert_eq!(expand("$J=G91 X1 F100", &vars).unwrap().len(), 1);
    }

    #[test]
    fn statistics_and_reminders_count() {
        let mut s = Stats::default();
        s.job_done(1800);
        s.job_done(5400);
        s.alarm(1);
        s.alarm(1);
        s.error(20);
        assert_eq!((s.jobs, s.run_seconds, s.alarms, s.errors), (2, 7200, 2, 1));
        assert_eq!(s.alarm_codes.get(&1), Some(&2));
        assert_eq!(s.hours(), 2.0);
        let r = Reminder { name: "Oil the rails".into(), every_hours: 1.5, done_at_hours: 0.0 };
        assert!(r.due(&s));
        let r2 = Reminder { name: "x".into(), every_hours: 10.0, done_at_hours: 0.0 };
        assert!(!r2.due(&s) && (r2.hours_left(&s) - 8.0).abs() < 1e-9);
        assert!(!Reminder { name: "off".into(), every_hours: 0.0, done_at_hours: 0.0 }.due(&s));
    }

    fn vals(l: &[(u32, &str)]) -> BTreeMap<u32, String> {
        l.iter().map(|(k, v)| (*k, v.to_string())).collect()
    }

    #[test]
    fn copies_of_the_settings_are_made_when_something_changes() {
        let mut c = FwCopies::default();
        let a = vals(&[(110, "3000.000"), (130, "300.000")]);
        assert_eq!(c.note(100, "Grbl 1.1h", &a, false), Noted::First);
        assert_eq!(c.0.len(), 1);
        // The same numbers written another way are the same.
        assert_eq!(c.note(200, "Grbl 1.1h", &vals(&[(110, "3000"), (130, "300.0")]), false), Noted::Same);
        assert_eq!(c.0.len(), 1);
        let b = vals(&[(110, "5000.000"), (130, "300.000"), (131, "200.000")]);
        match c.note(300, "Grbl 1.1h", &b, false) {
            Noted::Changed(ch) => {
                assert_eq!(ch, vec![Change { id: 110, old: Some("3000.000".into()), new: Some("5000.000".into()) }, Change { id: 131, old: None, new: Some("200.000".into()) }]);
            }
            other => panic!("{other:?}"),
        }
        assert_eq!(c.0.len(), 2);
        assert!(matches!(c.note(400, "Grbl 1.1h", &vals(&[(110, "6000")]), true), Noted::Own(_)));
        assert_eq!(c.newest().unwrap().why, "own");
        assert_eq!(c.note(500, "x", &BTreeMap::new(), false), Noted::Nothing);
        assert!(!c.make(600, "x", &BTreeMap::new()) && c.make(600, "x", &a));
        assert_eq!(c.newest().unwrap().why, "manual");
        // Only the newest few are kept.
        for i in 0..20u64 {
            c.make(700 + i, "x", &vals(&[(110, "1")]));
        }
        assert_eq!(c.0.len(), MAX_COPIES);
        assert!(c.check().is_ok());
        c.remove(0);
        assert_eq!(c.0.len(), MAX_COPIES - 1);
    }

    #[test]
    fn the_sensor_sequence_and_what_follows_are_built() {
        let t = Tls { set: true, x: -10.0, y: -20.5, start_z: -30.0, clear_z: -5.0, ..Default::default() };
        let g = t.measure_lines();
        assert_eq!(g[1], "G53 G0 Z-5");
        assert_eq!(g[2], "G53 G0 X-10 Y-20.5");
        assert_eq!(g[3], "G53 G0 Z-30");
        assert_eq!(g.iter().filter(|l| l.starts_with("G38.2")).count(), 2);
        assert_eq!(g.last().map(String::as_str), Some("G90"));
        assert_eq!(t.adopt_lines(2, 12.5), vec!["G10 L20 P2 Z12.5", "G53 G0 Z-5"]);
        assert_eq!(t.leave_lines(), vec!["G53 G0 Z-5"]);
        assert!(t.check().is_ok());
        let mut bad = t.clone();
        bad.start_z = 0.0;
        assert!(bad.check().is_err());
        bad = t.clone();
        bad.ref_wz = Some(f64::NAN);
        assert!(bad.check().is_err());
    }

    #[test]
    fn the_whole_thing_survives_json_and_old_files_without_it() {
        let mut c = Control::default();
        c.macros.push(Macro { name: "Park".into(), code: "G0 Z5".into() });
        c.stats.alarm(3);
        let j = serde_json::to_string(&c).unwrap();
        assert_eq!(serde_json::from_str::<Control>(&j).unwrap(), c);
        assert_eq!(serde_json::from_str::<Control>("{}").unwrap(), Control::default());
        assert!(c.check().is_ok());
        c.jog.xy_steps.clear();
        assert!(c.check().is_err());
        // The first buttons, never changed, become the new ones; the person's own stay.
        let old: Control = serde_json::from_str(r#"{"jog":{"xy_steps":[0.1,1.0,10.0,50.0],"z_steps":[0.05,0.5,5.0,20.0]}}"#).unwrap();
        assert_eq!(old.jog.xy_steps, XY_STEPS.to_vec());
        assert_eq!(old.jog.z_steps, Z_STEPS.to_vec());
        let own: Control = serde_json::from_str(r#"{"jog":{"xy_steps":[0.5,5.0],"z_steps":[0.2]}}"#).unwrap();
        assert_eq!((own.jog.xy_steps, own.jog.z_steps), (vec![0.5, 5.0], vec![0.2]));
        assert_eq!(set_position_lines(2, &[(Axis::X, 0.0), (Axis::Z, 1.25)]), vec!["G10 L20 P2 X0 Z1.25"]);
        assert!(set_position_lines(1, &[]).is_empty());
    }
}
