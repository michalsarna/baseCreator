//! The text of a G-code program, the same in every program: `{name}` variables in the own G-code of a device, the very start and
//! the very end of the file, and the flavour of the numbers (a decimal point after whole numbers, `%` lines, the word of a
//! dwell, line numbers, no comments) for controllers that read G-code in their own way.
//!
//! Every setting is off, or gives the text as it was, until the person changes it: the text a program makes is the same as
//! before for old settings and old projects.
use serde::{Deserialize, Serialize};

// ---------- variables ----------

/// The `{name}` words of a text, or an error for a `{` or `}` that does not form one.
pub fn variables(text: &str) -> Result<Vec<String>, String> {
    let mut out = vec![];
    let mut rest = text;
    while let Some(i) = rest.find(['{', '}']) {
        if rest.as_bytes()[i] == b'}' {
            return Err("a } without a {".into());
        }
        let after = &rest[i + 1..];
        let Some(j) = after.find('}') else { return Err("a { without a }".into()) };
        let name = &after[..j];
        if name.is_empty() || !name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
            return Err(format!("{{{}}} is not a variable", name));
        }
        out.push(name.to_string());
        rest = &after[j + 1..];
    }
    Ok(out)
}

/// What is wrong with the variables of `text`, in plain words: an unknown name, or a brace that does not close.
pub fn check_variables(text: &str, allowed: &[&str]) -> Result<(), String> {
    for v in variables(text)? {
        if !allowed.contains(&v.as_str()) {
            return Err(format!("unknown variable {{{v}}}"));
        }
    }
    Ok(())
}

/// `text` with every `{name}` replaced by what `value` says. Names that `value` does not know are left out empty
/// (a text that passed `check_variables` has none).
pub fn substitute(text: &str, value: &dyn Fn(&str) -> Option<String>) -> String {
    let mut out = String::new();
    let mut rest = text;
    while let Some(i) = rest.find('{') {
        out.push_str(&rest[..i]);
        let after = &rest[i + 1..];
        match after.find('}') {
            Some(j) => {
                out.push_str(&value(&after[..j]).unwrap_or_default());
                rest = &after[j + 1..];
            }
            None => {
                rest = "";
            }
        }
    }
    out.push_str(rest);
    out
}

/// A line without its comments (`;` to the end, and `( ... )` anywhere), trimmed.
pub fn strip_comments(line: &str) -> String {
    let mut out = String::new();
    let mut depth = 0u32;
    for c in line.chars() {
        match c {
            ';' if depth == 0 => break,
            '(' => depth += 1,
            ')' if depth > 0 => depth -= 1,
            _ if depth == 0 => out.push(c),
            _ => {}
        }
    }
    out.trim().to_string()
}

/// The variables every program knows: the project, the machine, the moment and the box of all the paths.
pub const COMMON_VARS: &[&str] = &["project_name", "project_path", "date", "time", "machine_name", "min_x", "max_x", "min_y", "max_y", "min_z", "max_z", "width", "height", "depth"];

/// The notes of the project as lines that are safe inside a G-code comment: no brackets or semicolons (they would end the comment),
/// no control characters, empty lines left out, long lines cut into pieces of at most 100 characters (at most 60 lines).
pub fn note_lines(notes: &str) -> Vec<String> {
    let mut out = vec![];
    for line in notes.lines() {
        let clean: String = line.chars().map(|c| match c { '(' | '[' | '{' => '<', ')' | ']' | '}' => '>', ';' | '%' => ',', c if c.is_control() => ' ', c => c }).collect();
        let clean = clean.trim();
        let chars: Vec<char> = clean.chars().collect();
        for piece in chars.chunks(100) {
            let t: String = piece.iter().collect();
            if !t.trim().is_empty() {
                out.push(t.trim().to_string());
            }
        }
    }
    out.truncate(60);
    out
}

/// What the common variables are made of.
#[derive(Clone, Debug, Default)]
pub struct Info {
    /// The name of the project file without its extension (empty while it has not been saved).
    pub project_name: String,
    /// The whole path of the project file (empty while it has not been saved).
    pub project_path: String,
    pub machine_name: String,
    /// `YYYY-MM-DD` and `HH:MM`, UTC.
    pub date: String,
    pub time: String,
    /// The box of all the paths of the program: the least and the most X, Y, Z in the coordinates of the machine.
    pub bounds: Option<([f64; 3], [f64; 3])>,
    /// The notes of the project (its description); they are written at the start of the program as comments.
    pub notes: String,
}

impl Info {
    /// An `Info` with the moment of now.
    pub fn now() -> Info {
        let (date, time) = utc_now();
        Info { date, time, ..Info::default() }
    }

    /// The value of a common variable (`None` for a name that is not one).
    pub fn value(&self, name: &str) -> Option<String> {
        let (lo, hi) = self.bounds.unwrap_or(([0.0; 3], [0.0; 3]));
        let n = |v: f64| {
            let s = format!("{v:.3}");
            let s = s.trim_end_matches('0').trim_end_matches('.');
            if s == "-0" || s.is_empty() { "0".to_string() } else { s.to_string() }
        };
        Some(match name {
            "project_name" => self.project_name.clone(),
            "project_path" => self.project_path.clone(),
            "date" => self.date.clone(),
            "time" => self.time.clone(),
            "machine_name" => self.machine_name.clone(),
            "min_x" => n(lo[0]),
            "max_x" => n(hi[0]),
            "min_y" => n(lo[1]),
            "max_y" => n(hi[1]),
            "min_z" => n(lo[2]),
            "max_z" => n(hi[2]),
            "width" => n(hi[0] - lo[0]),
            "height" => n(hi[1] - lo[1]),
            "depth" => n(hi[2] - lo[2]),
            _ => return None,
        })
    }
}

/// The date and the time of now, UTC, as `YYYY-MM-DD` and `HH:MM`.
pub fn utc_now() -> (String, String) {
    let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    utc_of(secs)
}

/// The date and the time of `secs` seconds after 1970-01-01 00:00 UTC.
pub fn utc_of(secs: u64) -> (String, String) {
    let days = (secs / 86_400) as i64;
    // Civil date from days since 1970-01-01 (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    (format!("{y:04}-{m:02}-{d:02}"), format!("{:02}:{:02}", secs / 3600 % 24, secs / 60 % 60))
}

/// The lines of an own G-code text with its variables put in: comments are kept (the program may want them); blank lines are
/// dropped. `Err` for a variable that is not in `allowed` or a brace that does not close.
pub fn expand(text: &str, allowed: &[&str], value: &dyn Fn(&str) -> Option<String>) -> Result<Vec<String>, String> {
    let mut out = vec![];
    for l in text.lines() {
        let l = l.trim_end();
        if l.trim().is_empty() {
            continue;
        }
        check_variables(l, allowed)?;
        out.push(substitute(l, value));
    }
    Ok(out)
}

/// The first problem in the variables of the texts (`what` names the text, for the message), or `Ok`.
pub fn check_texts(texts: &[(&str, &str)], allowed: &[&str]) -> Result<(), String> {
    for (what, text) in texts {
        check_variables(text, allowed).map_err(|e| format!("{what}: {e}"))?;
    }
    Ok(())
}

// ---------- the flavour ----------

/// The word of a dwell (a pause) and its unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum DwellWord {
    /// `G4 P` in seconds: GRBL and most others (what the programs write).
    #[default]
    PSeconds,
    /// `G4 P` in milliseconds: Fanuc.
    PMilliseconds,
    /// `G4 X` in seconds.
    XSeconds,
}

impl DwellWord {
    pub const ALL: [DwellWord; 3] = [DwellWord::PSeconds, DwellWord::PMilliseconds, DwellWord::XSeconds];

    pub fn label(self) -> &'static str {
        match self {
            DwellWord::PSeconds => "P (seconds)",
            DwellWord::PMilliseconds => "P (milliseconds)",
            DwellWord::XSeconds => "X (seconds)",
        }
    }
}

/// How a controller wants the text: a final pass over the finished program.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Flavour {
    /// `X1` becomes `X1.` (also Y, Z, F, P, I, J, K): controllers that read a number without a point as microns.
    #[serde(default)]
    pub decimal_point: bool,
    /// A `%` as the first and as the last line.
    #[serde(default)]
    pub percent: bool,
    #[serde(default)]
    pub dwell: DwellWord,
    /// Number the lines N10, N20, … (the step is `number_step`).
    #[serde(default)]
    pub line_numbers: bool,
    #[serde(default = "default_step")]
    pub number_step: u32,
    /// Take out every comment and the lines that held only a comment.
    #[serde(default)]
    pub remove_comments: bool,
}

fn default_step() -> u32 {
    10
}

impl Default for Flavour {
    fn default() -> Flavour {
        Flavour { decimal_point: false, percent: false, dwell: DwellWord::PSeconds, line_numbers: false, number_step: 10, remove_comments: false }
    }
}

impl Flavour {
    /// Does this flavour change the text at all?
    pub fn is_default(&self) -> bool {
        !self.decimal_point && !self.percent && self.dwell == DwellWord::PSeconds && !self.line_numbers && !self.remove_comments
    }
}

/// Text outside the comments of a line gets a decimal point after the whole numbers of the addresses X Y Z F P I J K.
fn point_after_whole_numbers(line: &str) -> String {
    let chars: Vec<char> = line.chars().collect();
    let mut out = String::with_capacity(line.len() + 4);
    let (mut depth, mut i) = (0u32, 0usize);
    let mut in_semicolon = false;
    while i < chars.len() {
        let c = chars[i];
        if in_semicolon {
            out.push(c);
            i += 1;
            continue;
        }
        match c {
            ';' if depth == 0 => {
                in_semicolon = true;
                out.push(c);
                i += 1;
            }
            '(' => {
                depth += 1;
                out.push(c);
                i += 1;
            }
            ')' => {
                depth = depth.saturating_sub(1);
                out.push(c);
                i += 1;
            }
            _ if depth == 0 && "XYZFPIJKxyzfpijk".contains(c) && (i == 0 || !chars[i - 1].is_ascii_alphanumeric()) => {
                out.push(c);
                i += 1;
                let start = i;
                if i < chars.len() && (chars[i] == '-' || chars[i] == '+') {
                    i += 1;
                }
                let digits = i;
                while i < chars.len() && chars[i].is_ascii_digit() {
                    i += 1;
                }
                out.extend(&chars[start..i]);
                // Whole number: digits, and no point, and not the start of a longer word.
                if i > digits && !(i < chars.len() && chars[i] == '.') {
                    out.push('.');
                }
            }
            _ => {
                out.push(c);
                i += 1;
            }
        }
    }
    out
}

/// A dwell line in the word the controller wants. The programs write `G4 P<seconds>`.
fn dwell_line(line: &str, word: DwellWord) -> String {
    if word == DwellWord::PSeconds {
        return line.to_string();
    }
    let t = line.trim_start();
    let Some(rest) = t.strip_prefix("G4 P").or_else(|| t.strip_prefix("G04 P")) else { return line.to_string() };
    let end = rest.find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-')).unwrap_or(rest.len());
    let Ok(seconds) = rest[..end].parse::<f64>() else { return line.to_string() };
    let tail = &rest[end..];
    let lead = &line[..line.len() - t.len()];
    let num = |v: f64| {
        let s = format!("{v:.3}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    match word {
        DwellWord::PMilliseconds => format!("{lead}G4 P{}{tail}", (seconds * 1000.0).round() as i64),
        DwellWord::XSeconds => format!("{lead}G4 X{}{tail}", num(seconds)),
        DwellWord::PSeconds => line.to_string(),
    }
}

/// The very start and the very end of the file: lines before everything (a `%`, a program number, the header of a controller)
/// and after everything (after `M30`). They are put in the file as they are written, after their variables are filled in.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Frame {
    #[serde(default)]
    pub very_start: String,
    #[serde(default)]
    pub very_end: String,
}

/// The finished program: `body` with the very start before it and the very end after it (given as lines, variables put in),
/// and the flavour applied. With the default flavour and no frame the text is `body`.
pub fn finish(body: &str, start: &[String], end: &[String], flavour: &Flavour) -> String {
    let mut lines: Vec<String> = Vec::with_capacity(body.lines().count() + start.len() + end.len() + 2);
    lines.extend(start.iter().cloned());
    lines.extend(body.lines().map(str::to_string));
    lines.extend(end.iter().cloned());
    if flavour.remove_comments {
        lines = lines.iter().map(|l| strip_comments(l)).filter(|l| !l.is_empty()).collect();
    }
    if flavour.dwell != DwellWord::PSeconds {
        lines = lines.iter().map(|l| dwell_line(l, flavour.dwell)).collect();
    }
    if flavour.decimal_point {
        lines = lines.iter().map(|l| point_after_whole_numbers(l)).collect();
    }
    if flavour.line_numbers {
        let step = flavour.number_step.clamp(1, 1000);
        let mut n = step;
        lines = lines
            .into_iter()
            .map(|l| {
                if l.trim().is_empty() || l.trim() == "%" {
                    return l;
                }
                let out = format!("N{n} {l}");
                n += step;
                out
            })
            .collect();
    }
    if flavour.percent {
        lines.insert(0, "%".to_string());
        lines.push("%".to_string());
    }
    let mut text = lines.join("\n");
    text.push('\n');
    text
}

#[cfg(test)]
mod tests {
    #[test]
    fn notes_become_lines_that_cannot_end_a_comment() {
        assert_eq!(note_lines("Birch (3 mm); first\n\n  glue [dry] \n"), vec!["Birch <3 mm>, first".to_string(), "glue <dry>".to_string()]);
        assert!(note_lines("").is_empty());
        let long = "a".repeat(250);
        assert_eq!(note_lines(&long).iter().map(|l| l.len()).collect::<Vec<_>>(), vec![100, 100, 50]);
    }

    use super::*;

    const PROGRAM: &str = "(MillCreator: part)\nG21 G90 G17\nG0 X10 Y20.5 Z5\nG1 X10 Y-3 Z-1 F300 ; cut\nG4 P2\nG1 X0 Y0 F100.5\nM3 S12000\nM30\n";

    #[test]
    fn the_default_flavour_and_no_frame_leave_the_text_as_it_was() {
        assert!(Flavour::default().is_default());
        assert_eq!(finish(PROGRAM, &[], &[], &Flavour::default()), PROGRAM);
        // A text without a final newline gets one: the programs always end their lines.
        assert_eq!(finish("G0 X1", &[], &[], &Flavour::default()), "G0 X1\n");
    }

    #[test]
    fn a_decimal_point_follows_whole_numbers_of_the_addresses_only() {
        let f = Flavour { decimal_point: true, ..Flavour::default() };
        let out = finish(PROGRAM, &[], &[], &f);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines[0], "(MillCreator: part)", "comments are not touched");
        assert_eq!(lines[1], "G21 G90 G17", "G and M words are not addresses of the list");
        assert_eq!(lines[2], "G0 X10. Y20.5 Z5.");
        assert_eq!(lines[3], "G1 X10. Y-3. Z-1. F300. ; cut");
        assert_eq!(lines[4], "G4 P2.");
        assert_eq!(lines[5], "G1 X0. Y0. F100.5");
        assert_eq!(lines[6], "M3 S12000", "the spindle speed has no point");
        assert_eq!(point_after_whole_numbers("G2 X1 Y2 I3 J-4 K5 (X7 Y8)"), "G2 X1. Y2. I3. J-4. K5. (X7 Y8)");
        assert_eq!(point_after_whole_numbers("G1 X10."), "G1 X10.");
        // A letter in the middle of a word is not an address.
        assert_eq!(point_after_whole_numbers("(MSG) EXIT5"), "(MSG) EXIT5");
    }

    #[test]
    fn percent_lines_wrap_the_file_and_are_not_numbered() {
        let f = Flavour { percent: true, line_numbers: true, number_step: 5, ..Flavour::default() };
        let out = finish("G0 X1\nG0 X2\n", &["O1234".to_string()], &["(end)".to_string()], &f);
        assert_eq!(out, "%\nN5 O1234\nN10 G0 X1\nN15 G0 X2\nN20 (end)\n%\n");
    }

    #[test]
    fn the_dwell_word_and_its_unit_follow_the_choice() {
        let ms = Flavour { dwell: DwellWord::PMilliseconds, ..Flavour::default() };
        assert!(finish("G4 P2\nG4 P0.5\n", &[], &[], &ms).starts_with("G4 P2000\nG4 P500\n"));
        let x = Flavour { dwell: DwellWord::XSeconds, ..Flavour::default() };
        assert_eq!(finish("G4 P2\nG1 X4 F5\n", &[], &[], &x), "G4 X2\nG1 X4 F5\n");
        assert_eq!(finish("G4 P1.25\n", &[], &[], &x), "G4 X1.25\n");
        let both = Flavour { dwell: DwellWord::PMilliseconds, decimal_point: true, ..Flavour::default() };
        assert_eq!(finish("G4 P3\n", &[], &[], &both), "G4 P3000.\n", "the point is put after the unit was changed");
    }

    #[test]
    fn comments_go_with_the_lines_that_held_only_a_comment() {
        let f = Flavour { remove_comments: true, ..Flavour::default() };
        let out = finish(PROGRAM, &[], &[], &f);
        assert_eq!(out, "G21 G90 G17\nG0 X10 Y20.5 Z5\nG1 X10 Y-3 Z-1 F300\nG4 P2\nG1 X0 Y0 F100.5\nM3 S12000\nM30\n");
    }

    #[test]
    fn the_very_start_and_the_very_end_come_before_and_after_everything() {
        let out = finish("(c)\nG0 X1\nM30\n", &["%".into(), "O1".into()], &["M99".into()], &Flavour::default());
        assert_eq!(out, "%\nO1\n(c)\nG0 X1\nM30\nM99\n");
    }

    #[test]
    fn variables_are_checked_and_put_in() {
        assert_eq!(variables("G0 X{min_x} ({project_name})").unwrap(), ["min_x", "project_name"]);
        assert!(check_variables("{nothing_here}", COMMON_VARS).is_err());
        assert!(check_variables("a } b", COMMON_VARS).is_err() && check_variables("a { b", COMMON_VARS).is_err());
        let info = Info { project_name: "part".into(), project_path: "/w/part.cre".into(), machine_name: "Mill".into(), date: "2026-11-01".into(), time: "12:30".into(), bounds: Some(([1.0, 2.0, -3.0], [11.0, 7.5, 0.0])), notes: String::new() };
        let lines = expand("(File {project_name} {date} {time})\n\nG0 X{min_x} Y{max_y} ; {width}x{height}x{depth}\n", COMMON_VARS, &|v| info.value(v)).unwrap();
        assert_eq!(lines, ["(File part 2026-11-01 12:30)", "G0 X1 Y7.5 ; 10x5.5x3"]);
        assert!(expand("{unknown}", COMMON_VARS, &|v| info.value(v)).is_err());
        assert_eq!(Info::default().value("min_x").as_deref(), Some("0"), "no paths: zero");
        assert!(check_texts(&[("Start", "{min_x}"), ("End", "{bad}")], COMMON_VARS).unwrap_err().starts_with("End: "));
    }

    #[test]
    fn the_date_and_the_time_are_utc() {
        assert_eq!(utc_of(0), ("1970-01-01".to_string(), "00:00".to_string()));
        assert_eq!(utc_of(1_000_000_000), ("2001-09-09".to_string(), "01:46".to_string()));
        assert_eq!(utc_of(1_793_534_400), ("2026-11-01".to_string(), "12:00".to_string()));
        assert_eq!(utc_of(951_782_400).0, "2000-02-29", "a leap day");
        let (d, t) = utc_now();
        assert!(d.len() == 10 && t.len() == 5);
    }

    #[test]
    fn the_flavour_is_saved_and_old_settings_read_as_the_default() {
        let f = Flavour { decimal_point: true, dwell: DwellWord::XSeconds, line_numbers: true, number_step: 2, ..Flavour::default() };
        let back: Flavour = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
        assert_eq!(back, f);
        let old: Flavour = serde_json::from_str("{}").unwrap();
        assert!(old.is_default() && old.number_step == 10);
        let frame: Frame = serde_json::from_str("{}").unwrap();
        assert_eq!(frame, Frame::default());
    }
}
