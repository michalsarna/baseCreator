//! Long straight moves cut into shorter ones before a job is sent: the same path, more lines. A job can then stop (for a
//! photo, at a pause) sooner, and the progress moves more evenly. Only straight cuts (`G1`) are cut; rapid moves, arcs and
//! lines with anything unusual (machine coordinates, other axes, line numbers) stay as they are.
use serde::{Deserialize, Serialize};

pub const MIN_PARTS: u32 = 2;
pub const MAX_PARTS: u32 = 256;

/// The setting, a part of the settings of a device.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SplitSettings {
    pub enabled: bool,
    /// Into how many lines a long move is cut, 2 to 256.
    pub parts: u32,
    /// The shortest part, mm: a move is cut into fewer parts when they would be shorter, and not at all when it is shorter
    /// than two of them.
    pub min_part: f64,
}

impl Default for SplitSettings {
    fn default() -> Self {
        SplitSettings { enabled: false, parts: 4, min_part: 5.0 }
    }
}

impl SplitSettings {
    pub fn check(&self) -> Result<(), &'static str> {
        if !(MIN_PARTS..=MAX_PARTS).contains(&self.parts) || !self.min_part.is_finite() || !(0.1..=10_000.0).contains(&self.min_part) {
            return Err("the splitting of long lines is not valid");
        }
        Ok(())
    }
}

/// One word of a line, as written (`X10.5`), with its letter and value.
struct Word<'a> {
    letter: char,
    value: f64,
    text: &'a str,
}

/// The code of a line and its comment (from the first `(` or `;`).
fn split_comment(line: &str) -> (&str, &str) {
    match line.find(['(', ';']) {
        Some(i) => (&line[..i], &line[i..]),
        None => (line, ""),
    }
}

/// The words of the code of a line; `None` when it holds something that is not a word (a `$` command, an expression).
fn parse(code: &str) -> Option<Vec<Word<'_>>> {
    let b = code.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        if b[i].is_ascii_whitespace() {
            i += 1;
            continue;
        }
        if !b[i].is_ascii_alphabetic() {
            return None;
        }
        let start = i;
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let num_start = i;
        while i < b.len() && (b[i].is_ascii_digit() || matches!(b[i], b'.' | b'-' | b'+')) {
            i += 1;
        }
        let value: f64 = code[num_start..i].parse().ok()?;
        out.push(Word { letter: (b[start] as char).to_ascii_uppercase(), value, text: code[start..i].trim() });
    }
    Some(out)
}

fn num(v: f64, inches: bool) -> String {
    let s = if inches { format!("{:.5}", v) } else { format!("{:.4}", v) };
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" { "0".into() } else { s.to_string() }
}

/// The lines of a job with the long straight cuts cut into parts. The lines come back as they were when the setting is off.
pub fn split_long_lines(lines: &[String], s: &SplitSettings) -> Vec<String> {
    if !s.enabled || s.check().is_err() {
        return lines.to_vec();
    }
    let mut out = Vec::with_capacity(lines.len());
    let (mut inches, mut relative, mut motion) = (false, false, 0u8);
    let mut pos = [f64::NAN; 3];
    for line in lines {
        let (code, comment) = split_comment(line);
        let Some(w) = parse(code) else {
            // Not a line of words: it may still change the state, and nothing is known any more.
            pos = [f64::NAN; 3];
            out.push(line.clone());
            continue;
        };
        let mut machine = false;
        let mut odd = false;
        for word in w.iter().filter(|w| w.letter == 'G') {
            match (word.value * 10.0).round() as i64 {
                200 => inches = true,
                210 => inches = false,
                900 => relative = false,
                910 => relative = true,
                0 => motion = 0,
                10 => motion = 1,
                20 | 30 => motion = 2,
                530 => machine = true,
                40 | 170 | 180 | 190 | 540 | 550 | 560 | 570 | 580 | 590 | 940 => {}
                _ => odd = true,
            }
        }
        let start = pos;
        for word in &w {
            if let Some(k) = "XYZ".find(word.letter) {
                if machine {
                    pos[k] = f64::NAN;
                } else if relative {
                    pos[k] += word.value;
                } else {
                    pos[k] = word.value;
                }
            }
        }
        if odd {
            // G28, G38, G92, G10…: where the machine is afterwards is not known here.
            pos = [f64::NAN; 3];
            out.push(line.clone());
            continue;
        }
        let axes: Vec<usize> = w.iter().filter_map(|word| "XYZ".find(word.letter)).collect();
        let other_axes = w.iter().any(|word| matches!(word.letter, 'A' | 'B' | 'C' | 'U' | 'V' | 'W' | 'I' | 'J' | 'K' | 'R' | 'P' | 'L' | 'N'));
        let can = motion == 1 && !machine && !other_axes && !axes.is_empty() && axes.iter().all(|&k| start[k].is_finite() || relative);
        if !can {
            out.push(line.clone());
            continue;
        }
        let delta: Vec<f64> = (0..3).map(|k| if axes.contains(&k) { if relative { w.iter().filter(|x| "XYZ".find(x.letter) == Some(k)).map(|x| x.value).sum() } else { pos[k] - start[k] } } else { 0.0 }).collect();
        let len = (delta[0].powi(2) + delta[1].powi(2) + delta[2].powi(2)).sqrt() * if inches { 25.4 } else { 1.0 };
        let n = (s.parts as f64).min((len / s.min_part).floor()) as u32;
        if n < 2 || !len.is_finite() {
            out.push(line.clone());
            continue;
        }
        // The first part keeps the other words (G1, F, S, M) and the comment; the others are only the axes.
        let rest: Vec<&str> = w.iter().filter(|x| "XYZ".find(x.letter).is_none()).map(|x| x.text).collect();
        let mut done = [0.0f64; 3];
        let mut ks: Vec<usize> = axes.clone();
        ks.sort_unstable();
        ks.dedup();
        for part in 1..=n {
            let f = part as f64 / n as f64;
            let mut moves = vec![];
            for &k in &ks {
                let letter = ['X', 'Y', 'Z'][k];
                let v = if relative {
                    // The rounded parts add up to the whole move.
                    let to = round(delta[k] * f, inches);
                    let d = to - done[k];
                    done[k] = to;
                    d
                } else if part == n {
                    pos[k]
                } else {
                    start[k] + delta[k] * f
                };
                moves.push(format!("{letter}{}", num(v, inches)));
            }
            let mut text = if part == 1 { rest.iter().map(|s| s.to_string()).chain(moves).collect::<Vec<_>>().join(" ") } else { moves.join(" ") };
            if part == 1 && !comment.is_empty() {
                text.push(' ');
                text.push_str(comment);
            }
            out.push(text);
        }
    }
    out
}

fn round(v: f64, inches: bool) -> f64 {
    let k = if inches { 1e5 } else { 1e4 };
    (v * k).round() / k
}

#[cfg(test)]
mod tests {
    use super::*;

    fn on(parts: u32, min_part: f64) -> SplitSettings {
        SplitSettings { enabled: true, parts, min_part }
    }

    fn prog(text: &str) -> Vec<String> {
        text.lines().map(|s| s.to_string()).collect()
    }

    #[test]
    fn a_long_cut_becomes_equal_parts_that_end_where_it_ended() {
        let l = prog("G21 G90\nG0 X0 Y0\nG1 X100 Y0 F3000 S500 (edge)\nG0 X0 Y10");
        let o = split_long_lines(&l, &on(4, 1.0));
        assert_eq!(o[2], "G1 F3000 S500 X25 Y0 (edge)");
        assert_eq!(&o[3..6], ["X50 Y0", "X75 Y0", "X100 Y0"]);
        assert_eq!(o[6], "G0 X0 Y10", "a rapid move stays whole");
    }

    #[test]
    fn short_moves_rapids_arcs_and_odd_lines_stay_whole_and_off_changes_nothing() {
        let l = prog("G21 G90\nG0 X0 Y0\nG1 X8 F100\nG2 X20 Y0 I6 J0\nG53 G1 X100\nG1 X100 A5\nN10 G1 X200");
        let o = split_long_lines(&l, &on(4, 5.0));
        assert_eq!(o, l, "8 mm with parts of at least 5 mm is one line");
        assert_eq!(split_long_lines(&l, &SplitSettings { enabled: false, ..on(4, 1.0) }), l);
        // A move of 12 mm with parts of at least 5 mm: two parts, not four.
        let o = split_long_lines(&prog("G0 X0 Y0\nG1 X12 F100"), &on(4, 5.0));
        assert_eq!(o, ["G0 X0 Y0", "G1 F100 X6", "X12"]);
    }

    #[test]
    fn relative_and_inch_moves_add_up_to_the_whole_move() {
        let l = prog("G20 G91\nG1 X1 Y0.5 Z-0.1 F10");
        let o = split_long_lines(&l, &on(3, 1.0));
        assert_eq!(o.len(), 4);
        let sum = |c: char| -> f64 { o[1..].iter().map(|line| line.split(' ').filter(|w| w.starts_with(c)).map(|w| w[1..].parse::<f64>().unwrap()).sum::<f64>()).sum() };
        assert!((sum('X') - 1.0).abs() < 1e-9 && (sum('Y') - 0.5).abs() < 1e-9 && (sum('Z') + 0.1).abs() < 1e-9, "{o:?}");
    }

    #[test]
    fn a_modal_cut_without_g1_and_an_unknown_start_are_handled() {
        let l = prog("G1 X50 F100\nG0 X0 Y0\nG1 X40 F100\nX0");
        let o = split_long_lines(&l, &on(2, 1.0));
        // The first line: Y and X were not known before it.
        assert_eq!(o[0], "G1 X50 F100");
        assert_eq!(&o[2..], ["G1 F100 X20", "X40", "X20", "X0"]);
    }

    #[test]
    fn the_numbers_of_parts_are_limited() {
        assert!(on(1, 1.0).check().is_err());
        assert!(on(257, 1.0).check().is_err());
        assert!(on(256, 1.0).check().is_ok());
        let o = split_long_lines(&prog("G0 X0 Y0\nG1 X1000 F100"), &on(256, 0.1));
        assert_eq!(o.len(), 1 + 256);
    }
}
