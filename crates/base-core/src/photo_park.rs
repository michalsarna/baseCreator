//! Photos of a time-lapse with the head out of the way: the job stops at a line where it can be taken up again, the head
//! goes to a place of the machine (the park position), the camera takes the picture, the head comes back and the job goes on.
//!
//! Where a job may stop depends on the machine: a laser at any line (it is switched off for the trip); a mill and a pen only
//! where the tool or the pen is already up, before a rapid move (a mill waits for its next lift).
use crate::resume::{can_resume, preamble, Modal};
use serde::{Deserialize, Serialize};

/// The settings of the parked photos, a part of the settings of a device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ParkSettings {
    /// The pictures of a time-lapse are taken with the head moved aside.
    pub enabled: bool,
    /// The place, in machine coordinates, mm.
    pub x: f64,
    pub y: f64,
    /// Seconds to wait at the place before the picture (the machine stops shaking, the stream of the camera catches up).
    pub settle: f64,
    /// A mill keeps its spindle running on the way (no new start, no mark of a new start on the wall).
    pub keep_spindle: bool,
}

impl Default for ParkSettings {
    fn default() -> Self {
        ParkSettings { enabled: false, x: 0.0, y: 0.0, settle: 1.0, keep_spindle: false }
    }
}

pub const MAX_SETTLE: f64 = 60.0;

impl ParkSettings {
    pub fn check(&self) -> Result<(), &'static str> {
        if !self.x.is_finite() || !self.y.is_finite() || self.x.abs() > 100_000.0 || self.y.abs() > 100_000.0 {
            return Err("the park position is not valid");
        }
        if !self.settle.is_finite() || !(0.0..=MAX_SETTLE).contains(&self.settle) {
            return Err("the wait before the photo is not valid");
        }
        Ok(())
    }
}

/// Where a job may stop for a photo.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StopKind {
    /// At any line (a laser: the beam goes off for the trip).
    AnyLine,
    /// Before the first rapid move after a lift: the tool (or the pen) is above the work zero, or the program has no Z.
    Lift,
}

/// The words of a line without comments, upper case: `(G, 1.0)`.
fn words(line: &str) -> Vec<(char, f64)> {
    crate::resume::line_words(line)
}

/// For every line of a job, whether it may stop just before that line. One pass over the job. `safe_z` is in millimetres.
pub fn stop_points(lines: &[String], kind: StopKind, safe_z: f64) -> Vec<bool> {
    let mut out = vec![false; lines.len()];
    let mut m = Modal::default();
    // The line before was a rapid move after a lift: the next one is not the first of its lift.
    let mut in_rapids = false;
    for (i, line) in lines.iter().enumerate() {
        let w = words(line);
        let ok = i > 0 && can_resume(&m, line, safe_z).is_ok();
        match kind {
            StopKind::AnyLine => out[i] = ok,
            StopKind::Lift => {
                // The motion of this line: its own G0/G1/G2/G3, or the one in force.
                let g_motion = w.iter().filter(|(l, _)| *l == 'G').map(|(_, v)| (v * 10.0).round() as i64).find(|n| matches!(n, 0 | 10 | 20 | 30)).map(|n| (n / 10) as u8);
                let motion = g_motion.unwrap_or(m.motion);
                let moves_xy = w.iter().any(|(l, _)| *l == 'X' || *l == 'Y');
                let machine = w.iter().any(|(l, v)| *l == 'G' && (v * 10.0).round() as i64 == 530);
                let rapid = motion == 0 && moves_xy && !machine;
                let up = !m.pos[2].is_finite() || m.pos[2] > 0.0;
                if rapid && up {
                    out[i] = ok && !in_rapids;
                    in_rapids = true;
                } else if moves_xy || w.iter().any(|(l, _)| *l == 'Z') {
                    in_rapids = false;
                }
            }
        }
        m.step(line);
    }
    out
}

/// The way of the head to the park position and back for a stop before line `at` of a job.
#[derive(Clone, Debug, PartialEq)]
pub struct Trip {
    /// To the place: up to the safe height (when the job moves Z), the spindle or the beam off (`spindle_off`), to the park position.
    pub away: Vec<String>,
    /// Back: the state of the job before the line, the place, the spindle, down to the depth.
    pub back: Vec<String>,
}

/// The trip for a stop before line `at`. `spindle_off`: the spindle (or the beam) is stopped for the trip and started again with
/// `spin_up` seconds to come to speed; a pen and a mill that keeps its spindle running pass false. `safe_z` in millimetres.
pub fn trip(lines: &[String], at: usize, park: &ParkSettings, safe_z: f64, spin_up: f64, spindle_off: bool) -> Result<Trip, &'static str> {
    if at == 0 || at > lines.len() {
        return Err("the job cannot stop at this line");
    }
    let mut m = Modal::default();
    for l in &lines[..at] {
        m.step(l);
    }
    let next = lines.get(at).map(String::as_str).unwrap_or("");
    let back = preamble(&m, next, safe_z, spin_up, spindle_off)?;
    Ok(Trip { away: away_lines(&m, park, safe_z, spindle_off), back })
}

fn away_lines(m: &Modal, park: &ParkSettings, safe_z: f64, spindle_off: bool) -> Vec<String> {
    // Millimetres from here on: the way back sets the units of the job again.
    let mut out = vec!["G21".to_string(), "G90".to_string()];
    let z_mm = if m.inches { m.pos[2] * 25.4 } else { m.pos[2] };
    if z_mm.is_finite() && z_mm < safe_z {
        out.push(format!("G0 Z{}", num(safe_z)));
    }
    if spindle_off {
        out.push("M5".into());
    }
    if m.mist || m.flood {
        out.push("M9".into());
    }
    out.push(format!("G53 G0 X{} Y{}", num(park.x), num(park.y)));
    out
}

fn num(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" { "0".into() } else { s.to_string() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(text: &str) -> Vec<String> {
        text.lines().map(|s| s.to_string()).collect()
    }

    const MILL: &str = "G21 G90 G54\nM5\nG0 Z5\nG0 X10 Y10\nS12000 M3\nF300\nG1 Z-1\nG1 X50 Y10\nG0 Z5\nG0 X60 Y10\nG0 X60 Y20\nG1 Z-1\nG1 X70 Y20\nG0 Z5\nM5\nM30";

    #[test]
    fn a_mill_stops_only_at_the_first_rapid_move_after_a_lift() {
        let l = prog(MILL);
        let s = stop_points(&l, StopKind::Lift, 5.0);
        let at: Vec<usize> = (0..l.len()).filter(|&i| s[i]).collect();
        // Line 3 has no X and Y before it; line 9 is the first rapid after the lift of line 8, line 10 the second.
        assert_eq!(at, vec![9]);
    }

    #[test]
    fn a_laser_stops_at_any_line_where_it_can_go_on_but_not_inside_arcs() {
        let l = prog("G21 G90\nM4 S0\nG0 X0 Y0\nG1 X10 S500 F3000\nG2 X20 Y0 I5 J0\nX30 Y0 I5 J0\nG1 X40\nM5");
        let s = stop_points(&l, StopKind::AnyLine, 5.0);
        assert!(!s[0] && !s[1] && !s[2], "nothing before the first X and Y: {s:?}");
        assert!(s[3] && s[4] && s[6] && s[7]);
        assert!(!s[5], "an arc that goes on without a G word");
    }

    #[test]
    fn a_pen_without_z_stops_before_its_travel_moves() {
        let l = prog("G21 G90\nM3 S30\nG0 X0 Y0\nM3 S90\nG1 X10 Y0 F2000\nM3 S30\nG0 X20 Y20\nM3 S90\nG1 X30 Y20");
        let s = stop_points(&l, StopKind::Lift, 5.0);
        let at: Vec<usize> = (0..l.len()).filter(|&i| s[i]).collect();
        assert_eq!(at, vec![6]);
    }

    #[test]
    fn the_trip_of_a_mill_lifts_parks_in_machine_coordinates_and_comes_back_down() {
        let l = prog(MILL);
        let park = ParkSettings { enabled: true, x: -5.0, y: -300.0, settle: 1.0, keep_spindle: false };
        // At a lift the tool is up already: only the spindle stops on the way out.
        let t = trip(&l, 9, &park, 5.0, 2.0, true).unwrap();
        assert_eq!(t.away, ["G21", "G90", "M5", "G53 G0 X-5 Y-300"]);
        assert!(t.back.contains(&"G0 X50 Y10".to_string()) && t.back.contains(&"S12000 M3".to_string()) && t.back.contains(&"G4 P2".to_string()), "{:?}", t.back);
        // Kept running: no M5, no wait for the speed.
        let t = trip(&l, 9, &park, 5.0, 2.0, false).unwrap();
        assert!(!t.away.contains(&"M5".to_string()) && !t.back.contains(&"M5".to_string()) && !t.back.iter().any(|s| s.starts_with("G4")), "{t:?}");
        // In the middle of a cut (a laser-like stop) the tool goes up first and down again at the feed.
        let t = trip(&l, 7, &park, 5.0, 0.0, true).unwrap();
        assert_eq!(t.away[2], "G0 Z5");
        assert_eq!(t.back[t.back.len() - 1], "G1 Z-1", "{:?}", t.back);
    }

    #[test]
    fn inches_are_lifted_in_millimetres_and_the_job_gets_its_units_back() {
        let l = prog("G20 G90\nG0 X1 Y1 Z-0.04\nF10\nG1 X2");
        let park = ParkSettings { enabled: true, x: 1.0, y: 2.0, ..Default::default() };
        let t = trip(&l, 3, &park, 5.0, 0.0, true).unwrap();
        assert_eq!(t.away, ["G21", "G90", "G0 Z5", "M5", "G53 G0 X1 Y2"]);
        assert_eq!(t.back[0], "G20");
        assert!(trip(&l, 0, &park, 5.0, 0.0, true).is_err());
    }

    #[test]
    fn bad_settings_are_refused() {
        assert!(ParkSettings::default().check().is_ok());
        assert!(ParkSettings { x: f64::NAN, ..Default::default() }.check().is_err());
        assert!(ParkSettings { settle: 61.0, ..Default::default() }.check().is_err());
        let p: ParkSettings = serde_json::from_str("{}").unwrap();
        assert_eq!(p, ParkSettings::default());
    }
}
