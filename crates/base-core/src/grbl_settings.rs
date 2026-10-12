//! The settings of GRBL 1.1 (`$0` ... `$132`): what they mean, which values are allowed, and the text forms used to
//! read, write, back up and restore them.
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// A number between `min` and `max`.
    Number { min: f64, max: f64 },
    /// A bit mask with this many bits (one per axis, or per option).
    Mask { bits: u32 },
    /// 0 or 1.
    Bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Setting {
    pub id: u32,
    pub name: &'static str,
    pub unit: &'static str,
    /// A typical value, as text (the defaults of GRBL or what hobby machines use).
    pub typical: &'static str,
    pub desc: &'static str,
    pub kind: Kind,
    /// A wrong value can fire the laser, stop the machine from homing or make it move the wrong distance.
    pub critical: bool,
}

const fn num(id: u32, name: &'static str, unit: &'static str, typical: &'static str, desc: &'static str, min: f64, max: f64, critical: bool) -> Setting {
    Setting { id, name, unit, typical, desc, kind: Kind::Number { min, max }, critical }
}
const fn mask(id: u32, name: &'static str, typical: &'static str, desc: &'static str, bits: u32) -> Setting {
    Setting { id, name, unit: "mask", typical, desc, kind: Kind::Mask { bits }, critical: false }
}
const fn flag(id: u32, name: &'static str, typical: &'static str, desc: &'static str, critical: bool) -> Setting {
    Setting { id, name, unit: "0 / 1", typical, desc, kind: Kind::Bool, critical }
}

pub const SETTINGS: &[Setting] = &[
    num(0, "Step pulse time", "µs", "10", "Length of the pulse sent to a stepper driver; at least what the driver needs.", 3.0, 1000.0, false),
    num(1, "Step idle delay", "ms", "25", "How long the steppers stay powered after a move (255 = always on).", 0.0, 255.0, false),
    mask(2, "Step pulse invert", "0", "Inverts the step signal of the axes: bit 0 X, bit 1 Y, bit 2 Z.", 3),
    mask(3, "Step direction invert", "0", "Reverses the direction of the axes: bit 0 X, bit 1 Y, bit 2 Z.", 3),
    flag(4, "Invert step enable pin", "0", "Inverts the stepper-enable signal.", false),
    flag(5, "Invert limit pins", "0", "Inverts the limit switch inputs (switches that are closed when not pressed).", false),
    flag(6, "Invert probe pin", "0", "Inverts the probe input.", false),
    num(10, "Status report options", "mask", "1", "What `?` reports: bit 0 machine position instead of work position, bit 1 buffer data.", 0.0, 3.0, false),
    num(11, "Junction deviation", "mm", "0.01", "How fast corners are taken: larger values go faster and risk lost steps.", 0.0, 10.0, false),
    num(12, "Arc tolerance", "mm", "0.002", "How finely arcs (G2 / G3) are split into short lines.", 0.0, 10.0, false),
    flag(13, "Report in inches", "0", "Reports positions in inches instead of millimetres.", false),
    flag(20, "Soft limits", "0", "Refuses moves outside the travel set in $130 - $132 (needs homing).", false),
    flag(21, "Hard limits", "0", "Stops the machine and raises an alarm when a limit switch closes.", false),
    flag(22, "Homing cycle", "0", "Enables homing ($H); needed for soft limits.", false),
    mask(23, "Homing direction invert", "0", "Which way each axis homes: bit 0 X, bit 1 Y, bit 2 Z.", 3),
    num(24, "Homing feed", "mm/min", "25", "Slow speed of the second, precise approach of the limit switch.", 0.0, 100_000.0, false),
    num(25, "Homing seek", "mm/min", "500", "Fast speed of the first approach of the limit switch.", 0.0, 100_000.0, false),
    num(26, "Homing debounce", "ms", "250", "Waits this long after a limit switch triggers before it is trusted.", 0.0, 10_000.0, false),
    num(27, "Homing pull-off", "mm", "1", "Distance moved back from the switch after homing.", 0.0, 1000.0, false),
    num(30, "Maximum spindle speed", "S", "1000", "The S value that means 100 % power; the G-code power is scaled to it.", 1.0, 1_000_000.0, true),
    num(31, "Minimum spindle speed", "S", "0", "The S value that means the lowest power.", 0.0, 1_000_000.0, true),
    flag(32, "Laser mode", "1", "On: power follows the speed and the laser is off during rapid moves. Off: behaves like a spindle.", true),
    num(100, "X steps per mm", "steps/mm", "80", "How many steps move the X axis by one millimetre.", 0.001, 100_000.0, false),
    num(101, "Y steps per mm", "steps/mm", "80", "How many steps move the Y axis by one millimetre.", 0.001, 100_000.0, false),
    num(102, "Z steps per mm", "steps/mm", "80", "How many steps move the Z axis by one millimetre.", 0.001, 100_000.0, false),
    num(110, "X maximum rate", "mm/min", "5000", "The fastest the X axis may move.", 0.0, 1_000_000.0, false),
    num(111, "Y maximum rate", "mm/min", "5000", "The fastest the Y axis may move.", 0.0, 1_000_000.0, false),
    num(112, "Z maximum rate", "mm/min", "500", "The fastest the Z axis may move.", 0.0, 1_000_000.0, false),
    num(120, "X acceleration", "mm/s²", "500", "How fast the X axis speeds up and slows down.", 0.0, 100_000.0, false),
    num(121, "Y acceleration", "mm/s²", "500", "How fast the Y axis speeds up and slows down.", 0.0, 100_000.0, false),
    num(122, "Z acceleration", "mm/s²", "50", "How fast the Z axis speeds up and slows down.", 0.0, 100_000.0, false),
    num(130, "X maximum travel", "mm", "400", "Length of the X axis; the work area width and the limit for soft limits.", 0.0, 100_000.0, true),
    num(131, "Y maximum travel", "mm", "400", "Length of the Y axis; the work area height and the limit for soft limits.", 0.0, 100_000.0, true),
    num(132, "Z maximum travel", "mm", "50", "Length of the Z axis.", 0.0, 100_000.0, true),
];

pub fn info(id: u32) -> Option<&'static Setting> {
    SETTINGS.iter().find(|s| s.id == id)
}

/// The settings in the lines GRBL printed for `$$` (`$110=3000.000`), by number. Anything else is ignored.
pub fn parse_report(lines: &[String]) -> BTreeMap<u32, String> {
    let mut out = BTreeMap::new();
    for l in lines {
        if let Some((k, v)) = l.trim().strip_prefix('$').and_then(|s| s.split_once('=')) {
            if let (Ok(id), true) = (k.trim().parse::<u32>(), !v.trim().is_empty() && v.trim().len() <= 24) {
                out.insert(id, v.trim().to_string());
            }
        }
    }
    out
}

/// Check what is typed for setting `id`; the text to write (a whole number where GRBL wants one, trimmed).
pub fn validate(id: u32, text: &str) -> Result<String, &'static str> {
    let t = text.trim();
    let v: f64 = t.parse().map_err(|_| "not a number")?;
    if !v.is_finite() {
        return Err("not a number");
    }
    match info(id).map(|s| s.kind) {
        Some(Kind::Number { min, max }) => {
            if v < min || v > max {
                return Err("out of range");
            }
        }
        Some(Kind::Mask { bits }) => {
            if v.fract() != 0.0 || v < 0.0 || v >= f64::from(1u32 << bits) {
                return Err("out of range");
            }
        }
        Some(Kind::Bool) => {
            if v != 0.0 && v != 1.0 {
                return Err("out of range");
            }
        }
        // An unknown setting (a build with more of them): any number, but a plain one.
        None => {
            if t.len() > 16 {
                return Err("out of range");
            }
        }
    }
    // Whole numbers are written without a decimal part, others as typed (at most three decimals are kept by GRBL).
    Ok(if v.fract() == 0.0 && v.abs() < 1e12 { format!("{}", v as i64) } else { format!("{v}") })
}

/// `$110=500`: the line that writes a setting.
pub fn write_line(id: u32, value: &str) -> String {
    format!("${id}={value}")
}

/// The text of a backup file: a header with the date and the firmware's own description, then one `$n=value` per line.
pub fn backup_text(values: &BTreeMap<u32, String>, build_info: &[String], date: &str) -> String {
    let mut s = format!("; LightCreator backup of the GRBL settings, {date}\n");
    for l in build_info {
        s.push_str(&format!("; {}\n", l.trim()));
    }
    for (id, v) in values {
        s.push_str(&format!("${id}={v}\n"));
    }
    s
}

/// Read a backup file: `$n=value` lines, with `;` comments and blank lines allowed. Anything else, a value that
/// does not pass `validate`, or a setting listed twice is an error that names the line.
pub fn parse_backup(text: &str) -> Result<Vec<(u32, String)>, String> {
    if text.len() > 20_000 {
        return Err("the file is too large".into());
    }
    let mut out: Vec<(u32, String)> = vec![];
    for (n, raw) in text.lines().enumerate() {
        let l = raw.split(';').next().unwrap_or("").trim();
        if l.is_empty() {
            continue;
        }
        let bad = || format!("line {}: not a setting", n + 1);
        let (k, v) = l.strip_prefix('$').and_then(|s| s.split_once('=')).ok_or_else(bad)?;
        let id: u32 = k.trim().parse().map_err(|_| bad())?;
        if id > 255 {
            return Err(bad());
        }
        let v = validate(id, v).map_err(|e| format!("line {}: {}", n + 1, e))?;
        if out.iter().any(|(i, _)| *i == id) {
            return Err(format!("line {}: ${} appears twice", n + 1, id));
        }
        out.push((id, v));
    }
    if out.is_empty() {
        return Err("the file holds no settings".into());
    }
    Ok(out)
}

/// Do two readings of one setting mean the same number (`500` and `500.000`)?
pub fn same_value(a: &str, b: &str) -> bool {
    match (a.trim().parse::<f64>(), b.trim().parse::<f64>()) {
        (Ok(x), Ok(y)) => (x - y).abs() < 1e-6,
        _ => a.trim() == b.trim(),
    }
}
