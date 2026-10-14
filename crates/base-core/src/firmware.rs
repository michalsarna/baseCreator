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
    /// The `[VER:…]` and `[OPT:…]` lines, as received.
    pub raw: Vec<String>,
}

impl FirmwareInfo {
    pub fn is_known(&self) -> bool {
        !self.name.is_empty()
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
    Suggestion { area, spindle, rapid, z_rate: num(112).filter(|v| *v > 0.0), homing: flag(22), soft_limits: flag(20), laser_mode: flag(32), inches: flag(13) }
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
    fn the_settings_suggest_the_device() {
        let v: BTreeMap<u32, String> = [(30, "24000.000"), (31, "1000.000"), (110, "3000.000"), (111, "2500.000"), (112, "500.000"), (130, "300.000"), (131, "180.000"), (132, "45.000"), (22, "1"), (20, "0"), (32, "0")].into_iter().map(|(k, v)| (k, v.to_string())).collect();
        let s = suggest(&v);
        assert_eq!(s.area, Some([300.0, 180.0, 45.0]));
        assert_eq!(s.spindle, Some((1000.0, 24000.0)));
        assert_eq!(s.rapid, Some(2500.0));
        assert_eq!(s.z_rate, Some(500.0));
        assert!(s.homing && !s.soft_limits && !s.laser_mode && !s.inches);
        assert!(suggest(&BTreeMap::new()).is_empty());
    }
}
