//! G-code for a machine that works in inches. The programs write their G-code in millimetres (`G21`); for a device set to inches the
//! text is turned over word by word: `G21` becomes `G20`, the lengths (X Y Z, U V W, the centres I J K, the radius R, the peck Q) and
//! the feed F are divided by 25.4. Angles (A B C), the power or speed of the spindle (S), the dwell (P), tools (T), line numbers (N)
//! and everything in comments stay as they are, so a machine in inches does the same job.
use crate::units::Units;

const MM_PER_INCH: f64 = 25.4;

fn is_length(c: char) -> bool {
    matches!(c.to_ascii_uppercase(), 'X' | 'Y' | 'Z' | 'U' | 'V' | 'W' | 'I' | 'J' | 'K' | 'R' | 'Q' | 'F')
}

fn number(v: f64) -> String {
    let s = format!("{v:.5}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.to_string() }
}

/// One line in inches (see the module).
pub fn line_to_inches(line: &str) -> String {
    // Settings of GRBL (`$110=...`) and other lines for the controller itself are not G-code; a jog (`$J=`) is.
    let t = line.trim_start();
    if t.starts_with('$') && !t.to_ascii_uppercase().starts_with("$J=") {
        return line.to_string();
    }
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len() + 8);
    let mut i = 0;
    let mut in_paren = false;
    while i < chars.len() {
        let c = chars[i];
        if in_paren {
            out.push(c);
            in_paren = c != ')';
            i += 1;
            continue;
        }
        match c {
            '(' => {
                in_paren = true;
                out.push(c);
                i += 1;
            }
            ';' => {
                out.extend(&chars[i..]);
                break;
            }
            _ if c.is_ascii_alphabetic() => {
                // The number of the word, with its sign and spaces before it.
                let mut j = i + 1;
                while j < chars.len() && chars[j] == ' ' {
                    j += 1;
                }
                let start = j;
                if j < chars.len() && (chars[j] == '-' || chars[j] == '+') {
                    j += 1;
                }
                while j < chars.len() && (chars[j].is_ascii_digit() || chars[j] == '.') {
                    j += 1;
                }
                let text: String = chars[start..j].iter().collect();
                let value = text.parse::<f64>().ok().filter(|_| j > start);
                let prev_letter = i > 0 && chars[i - 1].is_ascii_alphabetic();
                match value {
                    Some(v) if !prev_letter && c.eq_ignore_ascii_case(&'G') && (v - 21.0).abs() < 1e-9 => {
                        out.push(c);
                        out.push_str("20");
                    }
                    Some(v) if !prev_letter && is_length(c) => {
                        out.push(c);
                        out.push_str(&number(v / MM_PER_INCH));
                    }
                    _ => out.extend(&chars[i..j.max(i + 1)]),
                }
                i = j.max(i + 1);
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// A whole program in inches.
pub fn to_inches(text: &str) -> String {
    let mut out: String = text.lines().map(line_to_inches).collect::<Vec<_>>().join("\n");
    if text.ends_with('\n') {
        out.push('\n');
    }
    out
}

/// The program for a machine of these units: in inches for `Units::Inch`, as it is (millimetres) for every metric unit.
pub fn for_machine(text: String, units: Units) -> String {
    if units.is_metric() { text } else { to_inches(&text) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_in_millimetres_becomes_one_in_inches_with_the_same_moves() {
        let mm = "(MillCreator: part X10)\nG21 G90 G17\nG0 X25.4 Y-50.8 Z5\nG1 Z-1.27 F254 ; cut X1\nG2 X50.8 Y0 I12.7 J0\nG4 P2\nM3 S12000\nG0 A90\nT2 M6\n$J=G21G91X2.54F1000\n$110=6000\nN10 G1 X0\n";
        let inch = to_inches(mm);
        let lines: Vec<&str> = inch.lines().collect();
        assert_eq!(lines[0], "(MillCreator: part X10)", "comments stay");
        assert_eq!(lines[1], "G20 G90 G17");
        assert_eq!(lines[2], "G0 X1 Y-2 Z0.19685");
        assert_eq!(lines[3], "G1 Z-0.05 F10 ; cut X1");
        assert_eq!(lines[4], "G2 X2 Y0 I0.5 J0");
        assert_eq!(lines[5], "G4 P2", "a dwell is time");
        assert_eq!(lines[6], "M3 S12000", "the speed of the spindle");
        assert_eq!(lines[7], "G0 A90", "an angle");
        assert_eq!(lines[8], "T2 M6");
        assert_eq!(lines[9], "$J=G20G91X0.1F39.37008", "a jog is G-code");
        assert_eq!(lines[10], "$110=6000", "a setting of the controller is not");
        assert_eq!(lines[11], "N10 G1 X0");
        assert!(inch.ends_with('\n'));
        assert_eq!(for_machine("G21\nG0 X10\n".into(), Units::Mm), "G21\nG0 X10\n");
        assert_eq!(for_machine("G21\nG0 X25.4\n".into(), Units::Inch), "G20\nG0 X1\n");
    }
}
