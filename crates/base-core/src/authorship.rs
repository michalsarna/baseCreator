//! Who made a project and who worked on it, kept in the project file (`authorship` at the top of a `.cre`).
//!
//! The first save gives the project its **creator**; a save by someone else adds that person once to the **contributors**; every save
//! adds an **edit** (an automatic save is joined with the edit just before it when that one is by the same person and program and
//! recent). Each entry has the name, the time (UTC) and the program with its version. The name is what the person set in the
//! program options; a save without a name records the time and the program but no name.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The version of the block written now.
pub const FORMAT: u32 = 1;
/// The most edits kept: the oldest go first.
pub const MAX_EDITS: usize = 200;
/// An automatic save joins the edit before it when that edit is at most this old (seconds).
pub const JOIN_SECS: u64 = 30 * 60;

/// One person at one time, with the program they used.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Empty when no name was set.
    #[serde(default)]
    pub name: String,
    /// The time in UTC, `2026-11-25T10:31:05Z`.
    #[serde(default)]
    pub at: String,
    #[serde(default)]
    pub program: String,
    #[serde(default)]
    pub version: String,
}

/// The person saving now.
#[derive(Clone, Copy, Debug)]
pub struct Signer<'a> {
    pub name: &'a str,
    pub program: &'a str,
    pub version: &'a str,
}

/// The block of a project file.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Authorship {
    #[serde(default = "format_now")]
    pub format: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub creator: Option<Entry>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub contributors: Vec<Entry>,
    /// The saves, oldest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub edits: Vec<Entry>,
    /// Entries of a later version of the block, kept for the next save.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

fn format_now() -> u32 {
    FORMAT
}

/// The same person: the names compared without the spaces around them and the case of the letters.
pub fn same_name(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim(), b.trim());
    !a.is_empty() && a.to_lowercase() == b.to_lowercase()
}

impl Authorship {
    /// Nothing recorded: a project from before, or never saved.
    pub fn is_empty(&self) -> bool {
        self.creator.is_none() && self.contributors.is_empty() && self.edits.is_empty() && self.extra.is_empty()
    }

    /// A save by `who` at the Unix time `secs`. `auto` is an automatic save, which joins the edit before it when that one is by
    /// the same person, program and version and at most `JOIN_SECS` old.
    pub fn stamp(&mut self, who: Signer, secs: u64, auto: bool) {
        self.format = FORMAT;
        let entry = Entry { name: who.name.trim().to_string(), at: iso(secs), program: who.program.to_string(), version: who.version.to_string() };
        match &self.creator {
            None => self.creator = Some(entry.clone()),
            Some(c) => {
                let known = same_name(&entry.name, &c.name) || self.contributors.iter().any(|p| same_name(&entry.name, &p.name));
                if !entry.name.is_empty() && !known {
                    self.contributors.push(entry.clone());
                }
            }
        }
        if auto {
            if let Some(last) = self.edits.last_mut() {
                let mine = last.name.trim().to_lowercase() == entry.name.to_lowercase() && last.program == entry.program && last.version == entry.version;
                let recent = parse_iso(&last.at).is_some_and(|t| secs >= t && secs - t <= JOIN_SECS);
                if mine && recent {
                    last.at = entry.at;
                    return;
                }
            }
        }
        self.edits.push(entry);
        if self.edits.len() > MAX_EDITS {
            let over = self.edits.len() - MAX_EDITS;
            self.edits.drain(..over);
        }
    }
}

/// The Unix time now.
pub fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

/// Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + if m <= 2 { 1 } else { 0 }, m, d)
}

/// A calendar date to days since 1970-01-01.
fn days_from_civil(y: i64, m: u32, d: u32) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y.rem_euclid(400);
    let m = m as i64;
    let doy = (153 * (if m > 2 { m - 3 } else { m + 9 }) + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// `2026-11-25T10:31:05Z` of a Unix time.
pub fn iso(secs: u64) -> String {
    let (y, m, d) = civil((secs / 86_400) as i64);
    let s = secs % 86_400;
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", s / 3600, s / 60 % 60, s % 60)
}

/// The Unix time of `2026-11-25T10:31:05Z`; `None` for anything else.
pub fn parse_iso(t: &str) -> Option<u64> {
    let t = t.strip_suffix('Z')?;
    let (date, time) = t.split_once('T')?;
    let mut dp = date.split('-');
    let (y, m, d): (i64, u32, u32) = (dp.next()?.parse().ok()?, dp.next()?.parse().ok()?, dp.next()?.parse().ok()?);
    let mut tp = time.split(':');
    let (h, mi, s): (u64, u64, u64) = (tp.next()?.parse().ok()?, tp.next()?.parse().ok()?, tp.next()?.parse().ok()?);
    if dp.next().is_some() || tp.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) || h > 23 || mi > 59 || s > 60 {
        return None;
    }
    let days = days_from_civil(y, m, d);
    (days >= 0).then(|| days as u64 * 86_400 + h * 3600 + mi * 60 + s)
}

/// `2026-11-25 10:31 UTC`, for showing; the text as it is when it is not such a time.
pub fn shown(at: &str) -> String {
    match parse_iso(at) {
        Some(secs) => {
            let t = iso(secs);
            format!("{} {} UTC", &t[..10], &t[11..16])
        }
        None => at.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: u64 = 1_795_000_000;

    fn who<'a>(name: &'a str) -> Signer<'a> {
        Signer { name, program: "LightCreator", version: "v0.9.9" }
    }

    #[test]
    fn times_go_to_text_and_back() {
        assert_eq!(iso(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso(951_782_400), "2000-02-29T00:00:00Z");
        for secs in [0, 951_782_400, T0, T0 + 86_399, 4_102_444_800] {
            assert_eq!(parse_iso(&iso(secs)), Some(secs));
        }
        assert_eq!(parse_iso("yesterday"), None);
        assert_eq!(parse_iso("2026-13-01T00:00:00Z"), None);
        assert_eq!(shown(&iso(951_782_400 + 3_660)), "2000-02-29 01:01 UTC");
    }

    #[test]
    fn the_first_save_makes_the_creator_and_their_saves_are_edits() {
        let mut a = Authorship::default();
        assert!(a.is_empty());
        a.stamp(who("Anna"), T0, false);
        let c = a.creator.clone().unwrap();
        assert_eq!((c.name.as_str(), c.program.as_str(), c.version.as_str(), c.at.as_str()), ("Anna", "LightCreator", "v0.9.9", iso(T0).as_str()));
        a.stamp(who(" anna "), T0 + 10, false);
        assert!(a.contributors.is_empty(), "the creator is not their own contributor");
        assert_eq!(a.edits.len(), 2);
        assert_eq!(a.creator.unwrap().at, iso(T0), "the time of creation stays");
    }

    #[test]
    fn someone_else_is_added_once_as_a_contributor() {
        let mut a = Authorship::default();
        a.stamp(who("Anna"), T0, false);
        a.stamp(Signer { name: "Jan", program: "MillCreator", version: "v0.9.10" }, T0 + 100, false);
        a.stamp(who("JAN"), T0 + 200, false);
        assert_eq!(a.contributors.len(), 1);
        let j = &a.contributors[0];
        assert_eq!((j.name.as_str(), j.program.as_str(), j.version.as_str(), j.at.as_str()), ("Jan", "MillCreator", "v0.9.10", iso(T0 + 100).as_str()));
        assert_eq!(a.edits.len(), 3);
    }

    #[test]
    fn without_a_name_the_times_are_kept_but_no_person() {
        let mut a = Authorship::default();
        a.stamp(who(""), T0, false);
        assert_eq!(a.creator.as_ref().unwrap().name, "");
        a.stamp(who("  "), T0 + 1, false);
        assert!(a.contributors.is_empty(), "a save without a name adds nobody");
        // A named person after an unnamed creator is a contributor: the program cannot tell it was them.
        a.stamp(who("Anna"), T0 + 2, false);
        assert_eq!(a.contributors.len(), 1);
        assert!(!same_name("", ""));
    }

    #[test]
    fn automatic_saves_join_the_recent_edit_of_the_same_person() {
        let mut a = Authorship::default();
        a.stamp(who("Anna"), T0, false);
        a.stamp(who("Anna"), T0 + 60, true);
        a.stamp(who("Anna"), T0 + 120, true);
        assert_eq!(a.edits.len(), 1);
        assert_eq!(a.edits[0].at, iso(T0 + 120));
        a.stamp(who("Anna"), T0 + 120 + JOIN_SECS + 1, true);
        assert_eq!(a.edits.len(), 2, "a long time after is a new edit");
        a.stamp(who("Jan"), T0 + 120 + JOIN_SECS + 2, true);
        assert_eq!(a.edits.len(), 3, "someone else is a new edit");
        a.stamp(who("Jan"), T0 + 120 + JOIN_SECS + 3, false);
        assert_eq!(a.edits.len(), 4, "a save by hand is always a new edit");
    }

    #[test]
    fn the_edits_are_capped_and_the_oldest_go() {
        let mut a = Authorship::default();
        for i in 0..(MAX_EDITS as u64 + 5) {
            a.stamp(who("Anna"), T0 + i, false);
        }
        assert_eq!(a.edits.len(), MAX_EDITS);
        assert_eq!(a.edits[0].at, iso(T0 + 5));
        assert_eq!(a.creator.unwrap().at, iso(T0));
    }

    #[test]
    fn unknown_entries_of_the_block_are_kept() {
        let a: Authorship = serde_json::from_str(r#"{"format":1,"creator":{"name":"Anna","at":"2026-11-25T10:00:00Z","program":"LightCreator","version":"v0.9.9"},"licence":"CC-BY"}"#).unwrap();
        let back: Value = serde_json::to_value(&a).unwrap();
        assert_eq!(back["licence"], "CC-BY");
        assert_eq!(back["creator"]["name"], "Anna");
    }
}
