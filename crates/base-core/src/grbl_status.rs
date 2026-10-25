//! The status report of GRBL 1.1 (what `?` answers): `<Idle|MPos:1.000,2.000,3.000|FS:0,0|WCO:0,0,0|Pn:XYZ>`.
//! Every part is optional; the offset (`WCO`) and the overrides come only every few reports.

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    /// `Idle`, `Run`, `Hold:0`, `Jog`, `Alarm`, `Door:1`, `Check`, `Home`, `Sleep`.
    pub state: String,
    /// Machine position, when the controller reports it (`$10` bit 0).
    pub mpos: Option<[f64; 3]>,
    /// Work position, when the controller reports it instead.
    pub wpos: Option<[f64; 3]>,
    /// Work coordinate offset: work = machine - offset.
    pub wco: Option<[f64; 3]>,
    /// The input pins that are active: `X`, `Y`, `Z` (limit switches), `P` (probe), `D` (door), `H` (hold), `R` (reset), `S` (start).
    pub pins: Option<String>,
    /// Free blocks of the planner and of the serial buffer.
    pub buffer: Option<(u32, u32)>,
    /// Current feed rate and spindle speed.
    pub feed_spindle: Option<(f64, f64)>,
    /// Overrides in percent: feed, rapids, spindle.
    pub overrides: Option<(u32, u32, u32)>,
}

fn triple(v: &str) -> Option<[f64; 3]> {
    let n: Vec<f64> = v.split(',').filter_map(|s| s.trim().parse().ok()).collect();
    (n.len() >= 3).then(|| [n[0], n[1], n[2]])
}

/// Read a GRBL status line; `None` for anything else (Marlin and Smoothieware reports are not handled here).
pub fn parse(line: &str) -> Option<Report> {
    let inner = line.trim().strip_prefix('<')?.strip_suffix('>')?;
    if !inner.contains('|') && inner.contains(',') {
        return None;
    }
    let mut parts = inner.split('|');
    let state = parts.next()?.to_string();
    if state.is_empty() {
        return None;
    }
    let mut r = Report { state, ..Default::default() };
    for p in parts {
        let Some((k, v)) = p.split_once(':') else { continue };
        match k {
            "MPos" => r.mpos = triple(v),
            "WPos" => r.wpos = triple(v),
            "WCO" => r.wco = triple(v),
            "Pn" => r.pins = Some(v.to_string()),
            "Bf" => {
                let n: Vec<u32> = v.split(',').filter_map(|s| s.parse().ok()).collect();
                if n.len() == 2 {
                    r.buffer = Some((n[0], n[1]));
                }
            }
            "FS" | "F" => {
                let n: Vec<f64> = v.split(',').filter_map(|s| s.parse().ok()).collect();
                if !n.is_empty() {
                    r.feed_spindle = Some((n[0], n.get(1).copied().unwrap_or(0.0)));
                }
            }
            "Ov" => {
                let n: Vec<u32> = v.split(',').filter_map(|s| s.parse().ok()).collect();
                if n.len() == 3 {
                    r.overrides = Some((n[0], n[1], n[2]));
                }
            }
            _ => {}
        }
    }
    Some(r)
}

/// The status report of GRBL 0.9: `<Idle,MPos:0.000,0.000,0.000,WPos:0.000,0.000,0.000>`, with the buffer
/// (`,Buf:0,RX:0`) and the limit pins (`,Lim:000`) when `$10` asks for them. `None` for anything else.
pub fn parse_legacy(line: &str) -> Option<Report> {
    let inner = line.trim().strip_prefix('<')?.strip_suffix('>')?;
    if inner.contains('|') {
        return None;
    }
    let parts: Vec<&str> = inner.split(',').collect();
    let state = parts.first()?.trim().to_string();
    if state.is_empty() || state.contains(':') {
        return None;
    }
    let mut r = Report { state, ..Default::default() };
    // The three numbers of a position are separate parts: `MPos:1.000` `2.000` `3.000`.
    let triple_at = |i: usize| -> Option<[f64; 3]> {
        let first = parts[i].split_once(':')?.1.trim().parse().ok()?;
        Some([first, parts.get(i + 1)?.trim().parse().ok()?, parts.get(i + 2)?.trim().parse().ok()?])
    };
    for (i, p) in parts.iter().enumerate() {
        match p.split_once(':').map(|(k, _)| k) {
            Some("MPos") => r.mpos = triple_at(i),
            Some("WPos") => r.wpos = triple_at(i),
            Some("Lim") => r.pins = Some(p["Lim:".len()..].trim().to_string()).filter(|v| v.chars().any(|c| c == '1')),
            _ => {}
        }
    }
    (r.mpos.is_some() || r.wpos.is_some()).then_some(r)
}

/// Work position from a report and the last known offset.
pub fn work_position(r: &Report, last_wco: Option<[f64; 3]>) -> Option<[f64; 3]> {
    if let Some(w) = r.wpos {
        return Some(w);
    }
    let (m, o) = (r.mpos?, r.wco.or(last_wco)?);
    Some([m[0] - o[0], m[1] - o[1], m[2] - o[2]])
}

/// Machine position from a report and the last known offset.
pub fn machine_position(r: &Report, last_wco: Option<[f64; 3]>) -> Option<[f64; 3]> {
    if let Some(m) = r.mpos {
        return Some(m);
    }
    let (w, o) = (r.wpos?, r.wco.or(last_wco)?);
    Some([w[0] + o[0], w[1] + o[1], w[2] + o[2]])
}

/// The work coordinate system of a `$G` reply (`[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]`): 1 for G54 … 6 for G59.
pub fn parser_wcs(line: &str) -> Option<u8> {
    let body = line.trim().strip_prefix("[GC:")?;
    body.split_whitespace().find_map(|w| match w.trim_end_matches(']') {
        "G54" => Some(1),
        "G55" => Some(2),
        "G56" => Some(3),
        "G57" => Some(4),
        "G58" => Some(5),
        "G59" => Some(6),
        _ => None,
    })
}

/// `[PRB:1.000,2.000,-3.000:1]`: where a probe cycle stopped and whether it touched.
pub fn parse_probe(line: &str) -> Option<([f64; 3], bool)> {
    let body = line.trim().strip_prefix("[PRB:")?.strip_suffix(']')?;
    let (pos, ok) = body.rsplit_once(':')?;
    Some((triple(pos)?, ok.trim() == "1"))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_status_of_grbl_09_is_read() {
        let r = super::parse_legacy("<Idle,MPos:5.529,0.560,7.000,WPos:1.529,-5.440,-0.000>").unwrap();
        assert_eq!(r.state, "Idle");
        assert_eq!(r.mpos, Some([5.529, 0.56, 7.0]));
        assert_eq!(r.wpos, Some([1.529, -5.44, 0.0]));
        let b = super::parse_legacy("<Run,MPos:1.000,2.000,3.000,WPos:1.000,2.000,3.000,Buf:0,RX:3,Lim:010>").unwrap();
        assert_eq!((b.state.as_str(), b.pins.as_deref()), ("Run", Some("010")));
        assert_eq!(super::parse_legacy("<Idle,MPos:1.000,2.000,3.000,WPos:0.000,0.000,0.000,Lim:000>").unwrap().pins, None);
        assert!(super::parse_legacy("<Idle|MPos:1.000,2.000,3.000|FS:0,0>").is_none());
        assert!(super::parse_legacy("<Idle>").is_none());
        assert!(super::parse_legacy("ok").is_none());
        assert!(super::parse_legacy("<Idle,MPos:1.0,x,3.0>").is_none_or(|r| r.mpos.is_none()));
    }

    use super::*;

    #[test]
    fn a_full_report_is_read() {
        let r = parse("<Run|MPos:-10.000,20.500,-3.000|Bf:15,127|FS:500,12000|WCO:-5.000,5.000,0.000|Ov:100,110,90|Pn:XP>").unwrap();
        assert_eq!(r.state, "Run");
        assert_eq!(r.mpos, Some([-10.0, 20.5, -3.0]));
        assert_eq!(r.buffer, Some((15, 127)));
        assert_eq!(r.feed_spindle, Some((500.0, 12000.0)));
        assert_eq!(r.wco, Some([-5.0, 5.0, 0.0]));
        assert_eq!(r.overrides, Some((100, 110, 90)));
        assert_eq!(r.pins.as_deref(), Some("XP"));
        assert_eq!(work_position(&r, None), Some([-5.0, 15.5, -3.0]));
    }

    #[test]
    fn the_offset_of_an_earlier_report_is_used_when_this_one_has_none() {
        let r = parse("<Idle|MPos:1.000,2.000,3.000|FS:0,0>").unwrap();
        assert_eq!(r.wco, None);
        assert_eq!(work_position(&r, None), None);
        assert_eq!(work_position(&r, Some([1.0, 1.0, 1.0])), Some([0.0, 1.0, 2.0]));
        let w = parse("<Idle|WPos:0.000,5.000,1.000|F:100>").unwrap();
        assert_eq!(machine_position(&w, Some([10.0, 0.0, -2.0])), Some([10.0, 5.0, -1.0]));
        assert_eq!(w.feed_spindle, Some((100.0, 0.0)));
    }

    #[test]
    fn other_lines_are_not_reports() {
        assert!(parse("ok").is_none());
        assert!(parse("<>").is_none());
        assert!(parse("<Idle,MPos:1,2,3,WPos:1,2,3>").is_none(), "Smoothieware has its own reader");
        assert_eq!(parse("<Hold:0|MPos:0,0,0>").unwrap().state, "Hold:0");
    }

    #[test]
    fn parser_state_and_probe_results() {
        assert_eq!(parser_wcs("[GC:G0 G55 G17 G21 G90 G94 M5 M9 T0 F0 S0]"), Some(2));
        assert_eq!(parser_wcs("[GC:G0 G17 G21]"), None);
        assert_eq!(parse_probe("[PRB:1.000,2.000,-3.250:1]"), Some(([1.0, 2.0, -3.25], true)));
        assert_eq!(parse_probe("[PRB:0.000,0.000,0.000:0]").map(|p| p.1), Some(false));
        assert!(parse_probe("[GC:G0]").is_none());
    }
}
