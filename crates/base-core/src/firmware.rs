//! What the controller says about itself (`$I`) and what its settings (`$$`) tell about the machine, so that a device
//! can be filled in from the machine instead of typed.
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct FirmwareInfo {
    /// `Grbl`, `grblHAL`, `FluidNC`, `Grbl_ESP32` or the word the controller used.
    pub name: String,
    pub version: String,
    /// The option letters of `[OPT:…]`.
    pub options: String,
    /// The `[VER:…]` and `[OPT:…]` lines, as received (and grblHAL's `[NEWOPT:…]`, `[FIRMWARE:…]`, `[AXS:…]`, `[BOARD:…]`,
    /// `[DRIVER:…]`).
    pub raw: Vec<String>,
    /// What grblHAL says it can do besides GRBL 1.1 (`[NEWOPT:ENUMS,RT+,HOME,TC,SD,ETH]`).
    pub extensions: Vec<String>,
}

impl FirmwareInfo {
    pub fn is_known(&self) -> bool {
        !self.name.is_empty()
    }
    /// grblHAL, with its extra real-time commands (`0x87`: a complete status report) and its extra report fields.
    pub fn is_grblhal(&self) -> bool {
        self.name == "grblHAL"
    }
    /// Did grblHAL say it has `what` (`SD`, `TC`, `ETH`…)?
    pub fn has_extension(&self, what: &str) -> bool {
        self.extensions.iter().any(|e| e.eq_ignore_ascii_case(what))
    }
    /// `grblHAL 1.1f` for a line in a window.
    pub fn label(&self) -> String {
        if self.version.is_empty() {
            self.name.clone()
        } else {
            format!("{} {}", self.name, self.version)
        }
    }
}

/// Read the lines of `$I` (`[VER:1.1h.20190825:]`, `[OPT:V,15,128]`) or a start-up banner (`Grbl 1.1h ['$' for help]`).
pub fn parse_info(lines: &[String]) -> FirmwareInfo {
    let mut info = FirmwareInfo::default();
    for l in lines {
        let l = l.trim();
        if let Some(v) = l.strip_prefix("[VER:") {
            let body = v.trim_end_matches(']');
            let mut it = body.splitn(2, ':');
            let ver = it.next().unwrap_or("").trim().to_string();
            let tail = it.next().unwrap_or("").trim().to_string();
            info.raw.push(l.to_string());
            // FluidNC: `[VER:3.7.8 FluidNC v3.7.8:…]`; grblHAL: `[VER:1.1f.20240101:grblHAL …]` or the name in a banner.
            let low = format!("{ver} {tail}").to_lowercase();
            let (name, version) = if low.contains("fluidnc") {
                ("FluidNC", ver.split_whitespace().next().unwrap_or("").to_string())
            } else if low.contains("grblhal") {
                ("grblHAL", ver.split(['.']).take(2).collect::<Vec<_>>().join("."))
            } else if low.contains("esp32") {
                ("Grbl_ESP32", ver.clone())
            } else {
                ("Grbl", ver.split('.').take(2).collect::<Vec<_>>().join("."))
            };
            if info.name.is_empty() || name != "Grbl" {
                info.name = name.to_string();
                // `1.1h.20190825` -> `1.1h`
                info.version = if name == "Grbl" { ver.split('.').take(2).collect::<Vec<_>>().join(".") } else { version };
            }
        } else if l.starts_with("[OPT:") {
            info.raw.push(l.to_string());
            info.options = crate::machine::grbl_options(&[l.to_string()]).unwrap_or_default();
        } else if let Some(v) = l.strip_prefix("[NEWOPT:") {
            info.raw.push(l.to_string());
            info.extensions = v.trim_end_matches(']').split(',').map(|e| e.trim().to_string()).filter(|e| !e.is_empty()).collect();
            // Only grblHAL says this.
            if info.name.is_empty() || info.name == "Grbl" {
                info.name = "grblHAL".into();
            }
        } else if let Some(v) = l.strip_prefix("[FIRMWARE:") {
            info.raw.push(l.to_string());
            if v.to_ascii_lowercase().starts_with("grblhal") {
                info.name = "grblHAL".into();
            }
        } else if l.starts_with("[AXS:") || l.starts_with("[BOARD:") || l.starts_with("[DRIVER:") {
            info.raw.push(l.to_string());
        } else if info.name.is_empty() {
            // `Grbl 1.1h ['$' for help]`, `GrblHAL 1.1f ['$' or '$HELP' for help]`
            if let Some((w, rest)) = l.split_once(' ') {
                let lw = w.to_lowercase();
                if lw == "grbl" || lw == "grblhal" || lw == "fluidnc" {
                    info.name = if lw == "grblhal" { "grblHAL".into() } else if lw == "fluidnc" { "FluidNC".into() } else { "Grbl".into() };
                    info.version = rest.split_whitespace().next().unwrap_or("").to_string();
                }
            }
        }
    }
    info
}

/// What the settings say about the machine.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Suggestion {
    /// Travel of X, Y and Z (`$130` – `$132`), mm.
    pub area: Option<[f64; 3]>,
    /// Lowest and highest spindle speed (`$31`, `$30`).
    pub spindle: Option<(f64, f64)>,
    /// The speed of rapid moves: the lowest of the maximum rates of X and Y (`$110`, `$111`), mm/min.
    pub rapid: Option<f64>,
    /// The maximum rate of Z (`$112`), mm/min.
    pub z_rate: Option<f64>,
    pub homing: bool,
    pub soft_limits: bool,
    pub laser_mode: bool,
    /// Does the controller report positions in inches (`$13`)?
    pub inches: bool,
    /// The acceleration of X, Y and Z (`$120` – `$122`), mm/s².
    pub accel: Option<[f64; 3]>,
    /// The junction deviation (`$11`), mm.
    pub junction: Option<f64>,
}

impl Suggestion {
    pub fn is_empty(&self) -> bool {
        *self == Suggestion::default()
    }
}

pub fn suggest(values: &BTreeMap<u32, String>) -> Suggestion {
    let num = |id: u32| values.get(&id).and_then(|v| v.trim().parse::<f64>().ok());
    let flag = |id: u32| num(id).is_some_and(|v| v != 0.0);
    let area = match (num(130), num(131), num(132)) {
        (Some(x), Some(y), Some(z)) if x > 0.0 && y > 0.0 && z > 0.0 => Some([x, y, z]),
        _ => None,
    };
    let spindle = num(30).filter(|m| *m > 0.0).map(|max| (num(31).unwrap_or(0.0).min(max), max));
    let rapid = match (num(110), num(111)) {
        (Some(x), Some(y)) if x > 0.0 && y > 0.0 => Some(x.min(y)),
        _ => None,
    };
    let accel = match (num(120), num(121), num(122)) {
        (Some(x), Some(y), z) if x > 0.0 && y > 0.0 => Some([x, y, z.filter(|v| *v > 0.0).unwrap_or(0.0)]),
        _ => None,
    };
    let junction = num(11).filter(|v| *v > 0.0);
    Suggestion { area, spindle, rapid, z_rate: num(112).filter(|v| *v > 0.0), homing: flag(22), soft_limits: flag(20), laser_mode: flag(32), inches: flag(13), accel, junction }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lines(l: &[&str]) -> Vec<String> {
        l.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn grbl_grblhal_and_fluidnc_are_told_apart() {
        let g = parse_info(&lines(&["[VER:1.1h.20190825:]", "[OPT:V,15,128]"]));
        assert_eq!((g.name.as_str(), g.version.as_str(), g.options.as_str()), ("Grbl", "1.1h", "V"));
        assert_eq!(g.label(), "Grbl 1.1h");
        let h = parse_info(&lines(&["[VER:1.1f.20240101:grblHAL on Teensy]", "[OPT:VMPHN,35,1024]"]));
        assert_eq!((h.name.as_str(), h.version.as_str()), ("grblHAL", "1.1f"));
        let f = parse_info(&lines(&["[VER:3.7.8 FluidNC v3.7.8:]"]));
        assert_eq!((f.name.as_str(), f.version.as_str()), ("FluidNC", "3.7.8"));
        let b = parse_info(&lines(&["Grbl 1.1h ['$' for help]"]));
        assert_eq!((b.name.as_str(), b.version.as_str()), ("Grbl", "1.1h"));
        assert!(!parse_info(&lines(&["ok", "garbage"])).is_known());
    }

    #[test]
    fn grblhal_is_known_by_its_extra_lines_even_when_the_version_line_says_nothing() {
        // `$I` of grblHAL: the name is not in [VER:…], but [NEWOPT:…] and [FIRMWARE:…] tell.
        let h = parse_info(&lines(&["[VER:1.1f.20240624:]", "[OPT:VNMSL,35,1024,3,0]", "[AXS:3:XYZ]", "[NEWOPT:ENUMS,RT+,HOME,TC,SED,ETH,SD]", "[FIRMWARE:grblHAL]", "[DRIVER:STM32F407]", "[BOARD:BTT SKR-2]", "ok"]));
        assert!(h.is_grblhal(), "{h:?}");
        assert_eq!((h.version.as_str(), h.options.as_str()), ("1.1f", "VNMSL"));
        assert!(h.has_extension("sd") && h.has_extension("TC") && !h.has_extension("ATC"));
        assert_eq!(h.raw.len(), 7, "every line is shown in the firmware window");
        assert!(!parse_info(&lines(&["[VER:1.1h.20190825:]", "[OPT:V,15,128]"])).is_grblhal());
        assert!(parse_info(&lines(&["[VER:1.1f.20240101:]", "[FIRMWARE:grblHAL]"])).is_grblhal());
    }

    #[test]
    fn the_settings_suggest_the_device() {
        let v: BTreeMap<u32, String> = [(30, "24000.000"), (31, "1000.000"), (110, "3000.000"), (111, "2500.000"), (112, "500.000"), (130, "300.000"), (131, "180.000"), (132, "45.000"), (22, "1"), (20, "0"), (32, "0")].into_iter().map(|(k, v)| (k, v.to_string())).collect();
        let s = suggest(&v);
        assert_eq!(s.area, Some([300.0, 180.0, 45.0]));
        assert_eq!(s.spindle, Some((1000.0, 24000.0)));
        assert_eq!(s.rapid, Some(2500.0));
        assert_eq!(s.z_rate, Some(500.0));
        assert!(s.homing && !s.soft_limits && !s.laser_mode && !s.inches);
        assert_eq!((s.accel, s.junction), (None, None));
        assert!(suggest(&BTreeMap::new()).is_empty());
        let v: BTreeMap<u32, String> = [(11, "0.020"), (120, "800.000"), (121, "600.000"), (122, "50.000")].into_iter().map(|(k, v)| (k, v.to_string())).collect();
        let s = suggest(&v);
        assert_eq!((s.accel, s.junction), (Some([800.0, 600.0, 50.0]), Some(0.02)));
        let mut m = crate::motion::Motion::default();
        m.take(&s);
        assert!(m.known() && m.junction == 0.02);
    }
}
