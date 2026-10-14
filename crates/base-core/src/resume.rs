//! Start a job from a line in the middle: the state the program is in at that line (units, distance mode, spindle,
//! feed, position …) is worked out from the lines before it and set up again, then the machine goes to the place and
//! the rest of the program follows.

#[derive(Clone, Debug, PartialEq)]
pub struct Modal {
    pub inches: bool,
    pub relative: bool,
    /// 1 for G54 … 6 for G59.
    pub wcs: u8,
    /// 17, 18 or 19.
    pub plane: u8,
    /// 0, 1, 2 or 3 (G0 … G3).
    pub motion: u8,
    /// `3` or `4` while the spindle runs (M3, M4).
    pub spindle: Option<u8>,
    pub rpm: f64,
    pub feed: f64,
    pub mist: bool,
    pub flood: bool,
    /// Absolute work position after the lines, in the units of the file; `NAN` where no line has set it.
    pub pos: [f64; 3],
    /// A G53 (machine coordinates) line was seen: the position is not reliable.
    pub used_machine_coordinates: bool,
    /// G92 moved the origin.
    pub used_g92: bool,
}

impl Default for Modal {
    fn default() -> Modal {
        Modal { inches: false, relative: false, wcs: 1, plane: 17, motion: 0, spindle: None, rpm: 0.0, feed: 0.0, mist: false, flood: false, pos: [f64::NAN; 3], used_machine_coordinates: false, used_g92: false }
    }
}

/// The words of a line without comments: `(G, 1.0)`, `(X, -5.5)`.
fn words(line: &str) -> Vec<(char, f64)> {
    let mut clean = String::new();
    let mut depth = 0;
    for c in line.chars() {
        match c {
            ';' if depth == 0 => break,
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            _ if depth == 0 => clean.push(c.to_ascii_uppercase()),
            _ => {}
        }
    }
    let chars: Vec<char> = clean.chars().filter(|c| !c.is_whitespace()).collect();
    let mut out = vec![];
    let mut i = 0;
    while i < chars.len() {
        if chars[i].is_ascii_alphabetic() {
            let letter = chars[i];
            let mut j = i + 1;
            while j < chars.len() && (chars[j].is_ascii_digit() || matches!(chars[j], '.' | '-' | '+')) {
                j += 1;
            }
            if let Ok(v) = chars[i + 1..j].iter().collect::<String>().parse::<f64>() {
                out.push((letter, v));
            }
            i = j.max(i + 1);
        } else {
            i += 1;
        }
    }
    out
}

/// Does the line stop the program for the person: M0, or a tool change (M6)?
pub fn is_pause(line: &str) -> bool {
    words(line).iter().any(|&(l, v)| l == 'M' && (v == 0.0 || v == 6.0))
}

/// Does the line (without comments) hold a G word?
pub fn has_g_word(line: &str) -> bool {
    words(line).iter().any(|(l, _)| *l == 'G')
}

/// The state of the program after the first `upto` lines.
pub fn modal_at(lines: &[String], upto: usize) -> Modal {
    let mut m = Modal::default();
    for line in lines.iter().take(upto) {
        let w = words(line);
        let mut machine_coords = false;
        for &(l, v) in &w {
            if l != 'G' && l != 'M' {
                continue;
            }
            let n = (v * 10.0).round() as i64;
            match (l, n) {
                ('G', 200) => m.inches = true,
                ('G', 210) => m.inches = false,
                ('G', 900) => m.relative = false,
                ('G', 910) => m.relative = true,
                ('G', 170) => m.plane = 17,
                ('G', 180) => m.plane = 18,
                ('G', 190) => m.plane = 19,
                ('G', 540..=590) if n % 10 == 0 => m.wcs = ((n - 540) / 10 + 1) as u8,
                ('G', 0 | 10 | 20 | 30) => m.motion = (n / 10) as u8,
                ('G', 530) => machine_coords = true,
                ('G', 920 | 921 | 922 | 923) => m.used_g92 = true,
                ('M', 30) => m.spindle = Some(3),
                ('M', 40) => m.spindle = Some(4),
                ('M', 50) => m.spindle = None,
                ('M', 70) => m.mist = true,
                ('M', 80) => m.flood = true,
                ('M', 90) => {
                    m.mist = false;
                    m.flood = false;
                }
                _ => {}
            }
        }
        for &(l, v) in &w {
            match l {
                'S' => m.rpm = v,
                'F' => m.feed = v,
                'X' | 'Y' | 'Z' => {
                    if machine_coords {
                        m.used_machine_coordinates = true;
                        continue;
                    }
                    let k = (l as u8 - b'X') as usize;
                    if m.relative {
                        m.pos[k] += v;
                    } else {
                        m.pos[k] = v;
                    }
                }
                _ => {}
            }
        }
    }
    m
}

fn num(v: f64) -> String {
    let s = format!("{:.4}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" { "0".into() } else { s.to_string() }
}

/// The lines to send to start at line `from` (0-based): a preamble that brings the machine to the state of the program
/// at that line, then the lines from `from` on. `safe_z` is in millimetres (above the work zero), `spin_up` in seconds.
pub fn plan(lines: &[String], from: usize, safe_z: f64, spin_up: f64) -> Result<Vec<String>, &'static str> {
    if from > lines.len() {
        return Err("the line is past the end of the program");
    }
    if from == 0 {
        return Ok(lines.to_vec());
    }
    let m = modal_at(lines, from);
    if !m.pos[0].is_finite() || !m.pos[1].is_finite() {
        return Err("the program has not moved in X and Y before this line");
    }
    if m.used_machine_coordinates || m.used_g92 {
        return Err("the lines before use G53 or G92, so the position cannot be worked out");
    }
    let next = lines.get(from).map(String::as_str).unwrap_or("");
    if (m.motion == 2 || m.motion == 3) && !has_g_word(next) {
        return Err("the line continues an arc (G2/G3) without a G word");
    }
    let safe = if m.inches { safe_z / 25.4 } else { safe_z };
    let mut out: Vec<String> = vec![if m.inches { "G20" } else { "G21" }.into(), "G90".into(), format!("G{}", m.plane), format!("G{}", 53 + m.wcs), "M5".into()];
    // A program that never moves Z (a pen plotter with a servo) gets no Z moves either.
    let uses_z = m.pos[2].is_finite();
    if uses_z {
        out.push(format!("G0 Z{}", num(safe)));
    }
    out.push(format!("G0 X{} Y{}", num(m.pos[0]), num(m.pos[1])));
    if let Some(dir) = m.spindle {
        out.push(format!("S{} M{}", num(m.rpm), dir));
        if spin_up > 0.0 {
            out.push(format!("G4 P{}", num(spin_up)));
        }
    }
    if m.mist {
        out.push("M7".into());
    }
    if m.flood {
        out.push("M8".into());
    }
    let z = m.pos[2];
    if uses_z && z < safe {
        if m.feed <= 0.0 {
            return Err("no feed rate was set before this line");
        }
        out.push(format!("F{}", num(m.feed)));
        out.push(format!("G1 Z{}", num(z)));
    } else if m.feed > 0.0 {
        out.push(format!("F{}", num(m.feed)));
    }
    if m.motion == 0 {
        out.push("G0".into());
    }
    if m.relative {
        out.push("G91".into());
    }
    out.extend(lines[from..].iter().cloned());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(text: &str) -> Vec<String> {
        text.lines().map(|s| s.to_string()).collect()
    }

    const JOB: &str = "G21 G90 G54\nM5\nG0 Z5\nG0 X10 Y10\nS12000 M3\nG4 P2\nF300\nG1 Z-1\nG1 X50 Y10\nG1 X50 Y40\nG1 X10 Y40\nG0 Z5\nM5\nM30";

    #[test]
    fn the_state_is_followed_line_by_line() {
        let l = prog(JOB);
        let m = modal_at(&l, 10);
        assert_eq!((m.pos[0], m.pos[1], m.pos[2]), (50.0, 40.0, -1.0));
        assert_eq!((m.spindle, m.rpm, m.feed, m.motion), (Some(3), 12000.0, 300.0, 1));
        assert_eq!(modal_at(&l, 12).spindle, Some(3));
        assert_eq!(modal_at(&l, 13).spindle, None);
    }

    #[test]
    fn a_job_started_in_the_middle_gets_a_preamble() {
        let l = prog(JOB);
        let p = plan(&l, 10, 5.0, 2.0).unwrap();
        assert_eq!(&p[..5], ["G21", "G90", "G17", "G54", "M5"]);
        assert!(p.contains(&"G0 X50 Y40".to_string()), "{p:?}");
        assert!(p.iter().any(|s| s == "S12000 M3") && p.iter().any(|s| s == "G4 P2"), "{p:?}");
        let at = p.iter().position(|s| s == "G1 Z-1").expect("plunge");
        assert!(p[..at].contains(&"F300".to_string()));
        // The program itself follows from its line 10 on.
        assert_eq!(p[p.len() - 4..], l[10..]);
        // The Z move to the safe height comes before the move in X and Y.
        let z = p.iter().position(|s| s == "G0 Z5").unwrap();
        let xy = p.iter().position(|s| s == "G0 X50 Y40").unwrap();
        assert!(z < xy);
    }

    #[test]
    fn start_at_the_top_sends_everything_unchanged_and_bad_cases_are_refused() {
        let l = prog(JOB);
        assert_eq!(plan(&l, 0, 5.0, 0.0).unwrap(), l);
        assert!(plan(&l, 99, 5.0, 0.0).is_err());
        assert!(plan(&prog("G21\nG90\nM3 S1000\nG1 X1"), 3, 5.0, 0.0).is_err(), "no XY known yet");
        assert!(plan(&prog("G21 G90\nG0 X1 Y1\nG53 G0 Z0\nG1 X5 Y5"), 3, 5.0, 0.0).is_err(), "G53 in front");
        let arc = prog("G21 G90 G17\nG0 X0 Y0\nG1 Z-1 F100\nG2 X10 Y0 I5 J0\nX20 Y0 I5 J0");
        assert!(plan(&arc, 4, 5.0, 0.0).is_err(), "an arc continues without a G word");
        assert!(plan(&prog("G21 G90\nG0 X1 Y1\nG0 Z-2\nG1 X5"), 3, 5.0, 0.0).is_err(), "no feed before a cut below the safe height");
    }

    #[test]
    fn pauses_are_found() {
        assert!(is_pause("M0"));
        assert!(is_pause("M00 (Change the bit)"));
        assert!(is_pause("T2 M6"));
        assert!(is_pause("g0 x1 m0 ; stop"));
        assert!(!is_pause("M03 S1000"));
        assert!(!is_pause("(M0 in a comment)"));
        assert!(!is_pause("G0 X10 ; M6 later"));
        assert!(!is_pause("M30"));
    }

    #[test]
    fn a_program_without_z_gets_no_z_moves() {
        let l = prog("G21 G90\nM5\nG0 X5 Y5\nS500 M3\nG1 X10 Y10 F2000\nM5\nG0 X20 Y20");
        let p = plan(&l, 5, 5.0, 0.0).unwrap();
        assert!(!p.iter().any(|s| s.contains('Z')), "{p:?}");
        assert!(p.contains(&"G0 X10 Y10".to_string()));
    }

    #[test]
    fn inches_relative_mode_and_comments_are_handled() {
        let l = prog("G20 G90 (units)\nG0 X1 Y1 ; start\nG91\nG0 X1\nG0 Y1\nG1 X1 F10");
        let m = modal_at(&l, 5);
        assert!(m.inches && m.relative);
        assert_eq!((m.pos[0], m.pos[1]), (2.0, 2.0));
        let p = plan(&l, 5, 25.4, 0.0).unwrap();
        assert!(!p.iter().any(|s| s.contains('Z')), "no Z in this program: {p:?}");
        assert!(p.contains(&"G0 X2 Y2".to_string()) && p.contains(&"G91".to_string()), "{p:?}");
        assert_eq!(words("G1 X-5.5 (c) Y2 ; z9").len(), 3);
    }
}
