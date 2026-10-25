//! The machine: which controller it speaks, how the program reaches it, a rotary attachment, and the small differences
//! between the controllers (GRBL, Marlin, Smoothieware) that the serial worker and the buttons need. Shared by every
//! program on baseCreator.
use serde::{Deserialize, Serialize};

/// Which controller family a device speaks.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Controller {
    /// GRBL 1.1 over a serial port (streamed live).
    #[default]
    Grbl,
    /// Marlin firmware with laser support over a serial port (streamed live).
    Marlin,
    /// Ruida DSP controllers: jobs are exported as `.rd` files.
    Ruida,
    /// Trocen DSP controllers: jobs are exported as HPGL `.plt` files.
    Trocen,
    /// Smoothieware firmware over a serial port (streamed live like GRBL).
    Smoothieware,
    /// GRBL 0.8 to 1.0 (for example 0.9i, 1.0c): the status report has commas instead of bars and there is no jog
    /// command. Chosen through `GrblVersion`, not offered in `Controller::ALL`.
    GrblOld,
    /// GRBL 0.7 and older: no real-time commands at all (no status report, no pause, no reset) and no homing.
    /// Chosen through `GrblVersion`, not offered in `Controller::ALL`.
    Grbl07,
}

/// Which GRBL a device has; the apps that offer the choice keep it in the connection.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum GrblVersion {
    /// GRBL 1.1 and newer (grblHAL, FluidNC).
    #[default]
    Grbl11,
    /// GRBL 1.0 (1.0c): comma status report, no jog command, text errors.
    Grbl10,
    /// GRBL 0.9 (0.9a to 0.9j), as found on many plotters and small CNC machines.
    Grbl09,
    /// GRBL 0.8 (0.8c): the first with real-time commands and a status report.
    Grbl08,
    /// GRBL 0.7 and older: lines and `ok` only, nothing is real-time.
    Grbl07,
}

impl GrblVersion {
    pub const ALL: [GrblVersion; 5] = [GrblVersion::Grbl11, GrblVersion::Grbl10, GrblVersion::Grbl09, GrblVersion::Grbl08, GrblVersion::Grbl07];
    pub fn label(self) -> &'static str {
        match self {
            GrblVersion::Grbl11 => "GRBL 1.1 or newer",
            GrblVersion::Grbl10 => "GRBL 1.0",
            GrblVersion::Grbl09 => "GRBL 0.9",
            GrblVersion::Grbl08 => "GRBL 0.8",
            GrblVersion::Grbl07 => "GRBL 0.7 or older",
        }
    }
    pub fn controller(self) -> Controller {
        match self {
            GrblVersion::Grbl11 => Controller::Grbl,
            GrblVersion::Grbl10 | GrblVersion::Grbl09 | GrblVersion::Grbl08 => Controller::GrblOld,
            GrblVersion::Grbl07 => Controller::Grbl07,
        }
    }
}

impl Controller {
    pub const ALL: [Controller; 5] = [Controller::Grbl, Controller::Marlin, Controller::Smoothieware, Controller::Ruida, Controller::Trocen];
    pub fn label(self) -> &'static str {
        match self {
            Controller::Grbl => "GRBL",
            Controller::Marlin => "Marlin",
            Controller::Ruida => "Ruida",
            Controller::Trocen => "Trocen",
            Controller::Smoothieware => "Smoothieware",
            Controller::GrblOld => "GRBL 0.8 to 1.0",
            Controller::Grbl07 => "GRBL 0.7",
        }
    }
    /// Can the program talk to this controller over a serial port?
    pub fn is_serial(self) -> bool {
        matches!(self, Controller::Grbl | Controller::GrblOld | Controller::Grbl07 | Controller::Marlin | Controller::Smoothieware)
    }
}

/// How the program reaches the controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkKind {
    /// A serial (USB) port.
    #[default]
    Serial,
    /// A TCP socket, for example a Raspberry Pi running `ser2net`.
    Tcp,
}

impl LinkKind {
    pub const ALL: [LinkKind; 2] = [LinkKind::Serial, LinkKind::Tcp];
    pub fn label(self) -> &'static str {
        match self {
            LinkKind::Serial => "Serial port",
            LinkKind::Tcp => "Network (TCP, e.g. ser2net)",
        }
    }
}

fn d_tcp_port() -> u16 {
    3333
}

/// How a job is fed to the controller.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Streaming {
    /// Several lines are in the controller's buffer at once; the program counts the bytes (fast, the default).
    #[default]
    Counting,
    /// One line at a time: send, wait for `ok`, send the next. Slow, but for firmware that answers wrongly.
    Simple,
}

impl Streaming {
    pub const ALL: [Streaming; 2] = [Streaming::Counting, Streaming::Simple];
    pub fn label(self) -> &'static str {
        match self {
            Streaming::Counting => "Count the buffer",
            Streaming::Simple => "Simple (wait for each ok)",
        }
    }
}

/// How the program reaches the machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Connection {
    /// Serial port or TCP socket.
    #[serde(default)]
    pub link: LinkKind,
    /// TCP host (name or address) when `link` is `Tcp`.
    #[serde(default)]
    pub host: String,
    #[serde(default = "d_tcp_port")]
    pub tcp_port: u16,
    /// Serial port used for this device (empty = none chosen yet).
    #[serde(default)]
    pub port: String,
    pub baud: u32,
    #[serde(default)]
    pub streaming: Streaming,
    /// USB vendor and product id of the controller (`0403:6001`); when set the port is found by it, even after its
    /// name changed (empty = bound to the port name only).
    #[serde(default)]
    pub usb_id: String,
    /// Connect again by itself, every few seconds, when the link was lost without the user asking for it.
    #[serde(default)]
    pub auto_reconnect: bool,
    /// Run the homing cycle (`$H`, `G28`) once the link is up.
    #[serde(default)]
    pub home_on_connect: bool,
    /// Clear an alarm (`$X`, `M999`) once the link is up. This skips homing: only for machines whose head is known
    /// to be in a safe place.
    #[serde(default)]
    pub unlock_on_connect: bool,
    /// The GRBL of the machine (1.1 or the older 0.9).
    #[serde(default)]
    pub grbl_version: GrblVersion,
}

impl Default for Connection {
    fn default() -> Self {
        Connection { link: LinkKind::Serial, host: String::new(), tcp_port: 3333, port: String::new(), baud: 115_200, streaming: Streaming::Counting, usb_id: String::new(), auto_reconnect: false, home_on_connect: false, unlock_on_connect: false, grbl_version: GrblVersion::Grbl11 }
    }
}

/// `0403:6001` (hexadecimal vendor and product id) as numbers.
pub fn parse_usb_id(text: &str) -> Option<(u16, u16)> {
    let (v, p) = text.trim().split_once(':')?;
    let hex = |s: &str| (!s.is_empty() && s.len() <= 4 && s.chars().all(|c| c.is_ascii_hexdigit())).then(|| u16::from_str_radix(s, 16).ok()).flatten();
    Some((hex(v.trim())?, hex(p.trim())?))
}

/// How the rotation is written into the G-code.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotaryMode {
    /// The Y motor drives the rotary: Y stays a distance on the surface, the controller's steps per mm (`$101`) make
    /// it right.
    #[default]
    ReplaceY,
    /// A fourth axis A in degrees takes the place of Y (needs firmware with a fourth axis; GRBL 1.1 has three).
    TrueA,
}

/// The kind of rotary attachment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum RotaryKind {
    /// The work is clamped and turned by the motor: the angle follows the workpiece diameter.
    #[default]
    Chuck,
    /// The work lies on two rollers and is turned by friction: the surface moves like the surface of a roller.
    Roller,
}

/// The settings of a rotary attachment.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rotary {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub mode: RotaryMode,
    #[serde(default)]
    pub kind: RotaryKind,
    /// Diameter of the workpiece, millimetres.
    #[serde(default = "d_rot_diameter")]
    pub diameter: f64,
    /// Diameter of the rollers (for the roller kind), millimetres.
    #[serde(default = "d_rot_roller")]
    pub roller_diameter: f64,
    /// Microsteps of the motor for one full turn of what it drives (the roller, or the chuck).
    #[serde(default = "d_rot_micro")]
    pub microsteps_per_rev: f64,
}

fn d_rot_diameter() -> f64 {
    50.0
}
fn d_rot_roller() -> f64 {
    30.0
}
fn d_rot_micro() -> f64 {
    6400.0
}

impl Default for Rotary {
    fn default() -> Self {
        Rotary { enabled: false, mode: RotaryMode::ReplaceY, kind: RotaryKind::Chuck, diameter: 50.0, roller_diameter: 30.0, microsteps_per_rev: 6400.0 }
    }
}

impl Rotary {
    /// Distance round the workpiece, millimetres: the most the design may be high.
    pub fn circumference(&self) -> f64 {
        std::f64::consts::PI * self.diameter.max(0.0)
    }

    /// The diameter that sets how far the surface moves for a turn of the motor: the workpiece on a chuck, a roller
    /// when the work is turned by rollers (the surface of the work moves like the surface of a roller).
    pub fn drive_diameter(&self) -> f64 {
        match self.kind {
            RotaryKind::Chuck => self.diameter,
            RotaryKind::Roller => self.roller_diameter,
        }
    }

    /// Steps per millimetre of surface the controller's Y axis needs (`$101`) in the `ReplaceY` mode.
    pub fn steps_per_mm(&self) -> f64 {
        let c = std::f64::consts::PI * self.drive_diameter();
        if c > 0.0 { self.microsteps_per_rev / c } else { 0.0 }
    }

    /// Degrees of the driven part for one millimetre of surface (`A` in the `TrueA` mode).
    pub fn degrees_per_mm(&self) -> f64 {
        let c = std::f64::consts::PI * self.drive_diameter();
        if c > 0.0 { 360.0 / c } else { 0.0 }
    }
}


// ---- the dialects of the controllers ----


/// How the periodic status poll is done.
pub enum StatusPoll {
    /// A single realtime byte (GRBL `?`).
    Byte(u8),
    /// A normal queued line (Marlin `M114`).
    Line(&'static str),
    /// Not supported.
    None,
}

/// Realtime-style actions the Device tab offers.
pub enum Action {
    /// One or more lines queued like any other command.
    Lines(Vec<String>),
    /// A single realtime byte, sent immediately.
    Byte(u8),
    None,
}

impl Controller {
    pub fn status_poll(self) -> StatusPoll {
        match self {
            Controller::Grbl | Controller::GrblOld | Controller::Smoothieware => StatusPoll::Byte(b'?'),
            Controller::Marlin => StatusPoll::Line("M114"),
            _ => StatusPoll::None,
        }
    }
    /// Milliseconds between status polls.
    pub fn poll_interval_ms(self) -> u64 {
        match self {
            Controller::Marlin => 1000,
            _ => 250,
        }
    }
    pub fn home(self) -> Action {
        match self {
            Controller::Grbl | Controller::GrblOld | Controller::Smoothieware => Action::Lines(vec!["$H".into()]),
            Controller::Marlin => Action::Lines(vec!["G28 X Y".into()]),
            _ => Action::None,
        }
    }
    /// Clear an alarm / halted state.
    pub fn unlock(self) -> Action {
        match self {
            Controller::Grbl | Controller::GrblOld => Action::Lines(vec!["$X".into()]),
            Controller::Marlin | Controller::Smoothieware => Action::Lines(vec!["M999".into()]),
            _ => Action::None,
        }
    }
    pub fn pause(self) -> Action {
        match self {
            Controller::Grbl | Controller::GrblOld | Controller::Smoothieware => Action::Byte(b'!'),
            Controller::Marlin => Action::Lines(vec!["M25".into()]),
            _ => Action::None,
        }
    }
    pub fn resume(self) -> Action {
        match self {
            Controller::Grbl | Controller::GrblOld | Controller::Smoothieware => Action::Byte(b'~'),
            Controller::Marlin => Action::Lines(vec!["M24".into()]),
            _ => Action::None,
        }
    }
    /// GRBL before 1.1 has no jog command and no jog cancel: its moves are plain `G1` moves that run to the end.
    pub fn cancel_jog(self) -> Action {
        match self {
            Controller::Grbl => Action::Byte(0x85),
            Controller::Marlin => Action::Lines(vec!["M410".into()]),
            _ => Action::None,
        }
    }
    /// Jog by (dx, dy) millimetres at `feed` mm/min.
    pub fn jog(self, dx: f64, dy: f64, feed: f64) -> Action {
        match self {
            Controller::GrblOld | Controller::Grbl07 => Action::Lines(vec!["G91 G21".into(), format!("G1 X{dx} Y{dy} F{feed}"), "G90".into()]),
            Controller::Grbl => Action::Lines(vec![format!("$J=G91 G21 X{dx} Y{dy} F{feed}")]),
            Controller::Marlin | Controller::Smoothieware => Action::Lines(vec!["G91".into(), format!("G0 X{dx} Y{dy} F{feed}"), "G90".into()]),
            _ => Action::None,
        }
    }
    /// Jog the Z axis by `dz` millimetres at `feed` mm/min.
    pub fn jog_z(self, dz: f64, feed: f64) -> Action {
        match self {
            Controller::GrblOld | Controller::Grbl07 => Action::Lines(vec!["G91 G21".into(), format!("G1 Z{dz} F{feed}"), "G90".into()]),
            Controller::Grbl => Action::Lines(vec![format!("$J=G91 G21 Z{dz} F{feed}")]),
            Controller::Marlin | Controller::Smoothieware => Action::Lines(vec!["G91".into(), format!("G0 Z{dz} F{feed}"), "G90".into()]),
            _ => Action::None,
        }
    }
    /// Byte(s) sent right after connecting.
    pub fn on_connect(self) -> Option<u8> {
        match self {
            Controller::Grbl | Controller::GrblOld => Some(0x18),
            _ => None,
        }
    }
    /// Emergency stop / abort byte, if the controller has a realtime one.
    pub fn abort_byte(self) -> Option<u8> {
        match self {
            Controller::Grbl | Controller::GrblOld | Controller::Smoothieware => Some(0x18),
            _ => None,
        }
    }
    /// Line queued after an abort for controllers without a realtime abort.
    pub fn abort_line(self) -> Option<&'static str> {
        match self {
            Controller::Marlin => Some("M410"),
            _ => None,
        }
    }
}

/// The Z position of a status report, when it has one (`MPos:1,2,3` of GRBL and Smoothieware, `Z:3.00` of Marlin).
pub fn parse_status_z(line: &str) -> Option<f64> {
    if let Some(inner) = line.strip_prefix('<').and_then(|l| l.strip_suffix('>')) {
        let sep = if inner.contains('|') { '|' } else { ',' };
        let parts: Vec<&str> = inner.split(sep).collect();
        let at = parts.iter().position(|p| p.starts_with("MPos:") || p.starts_with("WPos:"))?;
        let first = parts[at].split(':').nth(1)?;
        // GRBL: "MPos:x,y,z" in one part; Smoothieware: the values are separate parts.
        let nums: Vec<f64> = if sep == '|' { first.split(',').filter_map(|s| s.parse().ok()).collect() } else { std::iter::once(first).chain(parts[at + 1..].iter().copied()).take(3).filter_map(|s| s.parse().ok()).collect() };
        return nums.get(2).copied();
    }
    if line.starts_with("X:") {
        return line.split_whitespace().find_map(|t| t.strip_prefix("Z:")).and_then(|v| v.parse::<f64>().ok());
    }
    None
}

/// The option letters GRBL reports in its build info (`$I`): `[OPT:VH,15,128]` gives `"VH"`.
pub fn grbl_options(lines: &[String]) -> Option<String> {
    lines.iter().find_map(|l| l.trim().strip_prefix("[OPT:")).map(|o| o.split([',', ']']).next().unwrap_or("").to_string())
}

/// Was this GRBL built with `$HX` / `$HY` / `$HZ` (single-axis homing, option letter `H`)?
pub fn single_axis_homing(options: &str) -> bool {
    options.contains('H')
}

/// Parse a status report into (state, x, y). GRBL: `<Idle|MPos:1,2,0|...>`; Marlin: `X:1.00 Y:2.00 Z:0.00 ...`.
pub fn parse_status(line: &str) -> Option<(String, f64, f64)> {
    // Smoothieware: `<Idle,MPos:1.0000,2.0000,3.0000,WPos:...>` (commas everywhere).
    if let Some(inner) = line.strip_prefix('<').and_then(|l| l.strip_suffix('>')).filter(|l| !l.contains('|') && l.contains(',')) {
        let parts: Vec<&str> = inner.split(',').collect();
        let state = parts.first()?.to_string();
        let at = parts.iter().position(|p| p.starts_with("MPos:") || p.starts_with("WPos:"))?;
        let x = parts[at].split(':').nth(1)?.parse::<f64>().ok()?;
        let y = parts.get(at + 1)?.parse::<f64>().ok()?;
        return Some((state, x, y));
    }
    if let Some(inner) = line.strip_prefix('<').and_then(|l| l.strip_suffix('>')) {
        let mut parts = inner.split('|');
        let state = parts.next()?.to_string();
        for p in parts {
            if let Some(v) = p.strip_prefix("MPos:").or_else(|| p.strip_prefix("WPos:")) {
                let n: Vec<f64> = v.split(',').filter_map(|s| s.parse().ok()).collect();
                if n.len() >= 2 {
                    return Some((state, n[0], n[1]));
                }
            }
        }
        return Some((state, f64::NAN, f64::NAN));
    }
    if line.starts_with("X:") {
        let num = |key: &str| line.split_whitespace().find_map(|t| t.strip_prefix(key)).and_then(|v| v.parse::<f64>().ok());
        if let (Some(x), Some(y)) = (num("X:"), num("Y:")) {
            return Some(("Idle".into(), x, y));
        }
    }
    None
}

#[cfg(test)]
mod dialect_tests {
    use super::*;

    #[test]
    fn the_older_grbls_jog_with_plain_moves_and_cannot_cancel() {
        let Action::Lines(l) = Controller::GrblOld.jog(1.5, -2.0, 3000.0) else { panic!("lines") };
        assert_eq!(l, ["G91 G21", "G1 X1.5 Y-2 F3000", "G90"]);
        let Action::Lines(z) = Controller::GrblOld.jog_z(-1.0, 500.0) else { panic!("lines") };
        assert_eq!(z[1], "G1 Z-1 F500");
        assert!(matches!(Controller::GrblOld.cancel_jog(), Action::None));
        assert!(matches!(Controller::Grbl.cancel_jog(), Action::Byte(0x85)));
        assert!(Controller::GrblOld.is_serial() && Controller::GrblOld.on_connect() == Some(0x18));
        assert!(!Controller::ALL.contains(&Controller::GrblOld));
        let c = |d: GrblVersion| d.controller();
        assert_eq!((c(GrblVersion::Grbl11), c(GrblVersion::Grbl10), c(GrblVersion::Grbl09), c(GrblVersion::Grbl08), c(GrblVersion::Grbl07)), (Controller::Grbl, Controller::GrblOld, Controller::GrblOld, Controller::GrblOld, Controller::Grbl07));
        assert!(!Controller::ALL.contains(&Controller::Grbl07));
        // GRBL 0.7 has nothing real-time: no status poll, no pause, no reset byte, no homing.
        let g7 = Controller::Grbl07;
        assert!(matches!(g7.status_poll(), StatusPoll::None) && matches!(g7.pause(), Action::None) && matches!(g7.home(), Action::None));
        assert!(g7.on_connect().is_none() && g7.abort_byte().is_none() && g7.is_serial());
        assert!(matches!(g7.jog(1.0, 0.0, 100.0), Action::Lines(_)));
        assert_eq!(Connection::default().grbl_version, GrblVersion::Grbl11);
    }

    #[test]
    fn an_old_connection_without_a_dialect_still_loads() {
        let c: Connection = serde_json::from_str(r#"{"baud":115200}"#).unwrap();
        assert_eq!(c.grbl_version, GrblVersion::Grbl11);
    }
}
