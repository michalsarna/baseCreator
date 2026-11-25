//! Who made a project and who worked on it, kept in the project file (`authorship` at the top of a `.cre`).
//!
//! The first save gives the project its **creator**; a save by someone else adds that person once to the **contributors**; a save by
//! hand adds an **edit**, at most one a minute for the same person and program (an automatic save adds none). Each entry has the
//! name, the e-mail and a free text about the person, the time (UTC) and the program with its version. The person is what was set in
//! the program options; a save without it records the time and the program but no person.
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The version of the block written now.
pub const FORMAT: u32 = 1;
/// The most edits kept: the oldest go first.
pub const MAX_EDITS: usize = 200;
/// A save this soon (seconds) after the last edit of the same person and program adds no edit: at most one a minute.
pub const JOIN_SECS: u64 = 60;

/// One person at one time, with the program they used.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    /// Empty when no name was set.
    #[serde(default)]
    pub name: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub email: String,
    /// Anything else the person wrote about themselves (a company, a website...).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub other: String,
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
    pub email: &'a str,
    pub other: &'a str,
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

/// The same text, without the spaces around it and the case of the letters; an empty text is nobody.
pub fn same_name(a: &str, b: &str) -> bool {
    let (a, b) = (a.trim(), b.trim());
    !a.is_empty() && a.to_lowercase() == b.to_lowercase()
}

impl Entry {
    /// Somebody: a name or an e-mail was set.
    pub fn is_somebody(&self) -> bool {
        !self.name.trim().is_empty() || !self.email.trim().is_empty()
    }

    /// The same person: by the name when both have one, otherwise by the e-mail.
    pub fn same_person(&self, o: &Entry) -> bool {
        if !self.name.trim().is_empty() && !o.name.trim().is_empty() {
            same_name(&self.name, &o.name)
        } else {
            same_name(&self.email, &o.email)
        }
    }
}

impl Authorship {
    /// Nothing recorded: a project from before, or never saved.
    pub fn is_empty(&self) -> bool {
        self.creator.is_none() && self.contributors.is_empty() && self.edits.is_empty() && self.extra.is_empty()
    }

    /// What a contributor or an edit keeps of a person: the name and the e-mail, without the other field; only the name when the
    /// e-mail is the creator's (it is there already). Without a name the e-mail stays, as it tells the person apart.
    fn slim(&self, e: &Entry) -> Entry {
        let mut e = e.clone();
        e.other.clear();
        let creator_email = self.creator.as_ref().map(|c| c.email.as_str()).unwrap_or("");
        if !e.name.is_empty() && !e.email.is_empty() && same_name(&e.email, creator_email) {
            e.email.clear();
        }
        e
    }

    /// A save by `who` at the Unix time `secs`. `auto` is an automatic save: it can make the creator or a contributor, but adds no
    /// edit. A save by hand adds none either when the last edit is of the same person, program and version and less than
    /// `JOIN_SECS` old.
    pub fn stamp(&mut self, who: Signer, secs: u64, auto: bool) {
        self.format = FORMAT;
        let t = |s: &str| s.trim().to_string();
        let full = Entry { name: t(who.name), email: t(who.email), other: t(who.other), at: iso(secs), program: t(who.program), version: t(who.version) };
        match &self.creator {
            None => self.creator = Some(full.clone()),
            Some(c) => {
                let known = full.same_person(c) || self.contributors.iter().any(|p| full.same_person(p));
                if full.is_somebody() && !known {
                    self.contributors.push(self.slim(&full));
                }
            }
        }
        // The creator keeps everything; a contributor and an edit keep the name and the e-mail.
        let entry = self.slim(&full);
        if auto {
            return;
        }
        if let Some(last) = self.edits.last() {
            let same = (last.same_person(&entry) || (!last.is_somebody() && !entry.is_somebody())) && last.program == entry.program && last.version == entry.version;
            let recent = parse_iso(&last.at).is_some_and(|t| secs >= t && secs - t < JOIN_SECS);
            if same && recent {
                return;
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
        Signer { name, email: "", other: "", program: "LightCreator", version: "v0.9.9" }
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
        a.stamp(who(" anna "), T0 + 60, false);
        assert!(a.contributors.is_empty(), "the creator is not their own contributor");
        assert_eq!(a.edits.len(), 2);
        assert_eq!(a.creator.unwrap().at, iso(T0), "the time of creation stays");
    }

    #[test]
    fn someone_else_is_added_once_as_a_contributor() {
        let mut a = Authorship::default();
        a.stamp(who("Anna"), T0, false);
        a.stamp(Signer { name: "Jan", email: "jan@example.org", other: "Workshop", program: "MillCreator", version: "v0.9.10" }, T0 + 100, false);
        a.stamp(who("JAN"), T0 + 200, false);
        assert_eq!(a.contributors.len(), 1);
        let j = &a.contributors[0];
        assert_eq!((j.name.as_str(), j.email.as_str(), j.other.as_str(), j.program.as_str(), j.version.as_str(), j.at.as_str()), ("Jan", "jan@example.org", "", "MillCreator", "v0.9.10", iso(T0 + 100).as_str()), "a contributor keeps no other field");
        assert_eq!(a.edits.len(), 3);
    }

    #[test]
    fn the_creator_keeps_everything_and_a_contributor_with_the_creators_email_only_the_name() {
        let mut a = Authorship::default();
        a.stamp(Signer { name: "Anna", email: "anna@example.org", other: "Studio A", program: "LightCreator", version: "v1" }, T0, false);
        let c = a.creator.clone().unwrap();
        assert_eq!((c.name.as_str(), c.email.as_str(), c.other.as_str()), ("Anna", "anna@example.org", "Studio A"));
        // The same e-mail under another name (a colleague on a shared address): a contributor with the name only.
        a.stamp(Signer { name: "Jan", email: "Anna@Example.org", other: "Studio A", program: "LightCreator", version: "v1" }, T0 + 100, false);
        let j = &a.contributors[0];
        assert_eq!((j.name.as_str(), j.email.as_str(), j.other.as_str()), ("Jan", "", ""));
        // The edits of the creator do not repeat what the creator has.
        assert_eq!((a.edits[0].email.as_str(), a.edits[0].other.as_str()), ("", ""));
        assert_eq!(a.edits[1].name, "Jan");
    }

    #[test]
    fn without_a_name_the_times_are_kept_but_no_person() {
        let mut a = Authorship::default();
        a.stamp(who(""), T0, false);
        assert_eq!(a.creator.as_ref().unwrap().name, "");
        a.stamp(who("  "), T0 + 61, false);
        assert!(a.contributors.is_empty(), "a save without a name adds nobody");
        // A named person after an unnamed creator is a contributor: the program cannot tell it was them.
        a.stamp(who("Anna"), T0 + 122, false);
        assert_eq!(a.contributors.len(), 1);
        // Only an e-mail is somebody too, told apart by it.
        a.stamp(Signer { name: "", email: "x@example.org", other: "", program: "PenCreator", version: "v1" }, T0 + 200, false);
        a.stamp(Signer { name: "", email: "X@example.org ", other: "", program: "PenCreator", version: "v1" }, T0 + 300, false);
        assert_eq!(a.contributors.len(), 2);
        assert!(!same_name("", ""));
    }

    #[test]
    fn at_most_one_edit_a_minute_and_none_for_an_automatic_save() {
        let mut a = Authorship::default();
        a.stamp(who("Anna"), T0, false);
        for k in 1..6 {
            a.stamp(who("Anna"), T0 + k * 10, false);
        }
        assert_eq!(a.edits.len(), 1, "many saves within a minute are one edit");
        assert_eq!(a.edits[0].at, iso(T0), "the first time stays");
        a.stamp(who("Anna"), T0 + JOIN_SECS, false);
        assert_eq!(a.edits.len(), 2, "a minute later is a new edit");
        a.stamp(who("Jan"), T0 + JOIN_SECS + 1, false);
        assert_eq!(a.edits.len(), 3, "someone else is a new edit");
        a.stamp(who("Ewa"), T0 + 5000, true);
        assert_eq!(a.edits.len(), 3, "an automatic save adds no edit");
        assert_eq!(a.contributors.len(), 2, "but its person is a contributor");
        // A project first written by an automatic save has its creator, and no edit.
        let mut b = Authorship::default();
        b.stamp(who("Anna"), T0, true);
        assert_eq!((b.creator.is_some(), b.edits.len()), (true, 0));
    }

    #[test]
    fn the_edits_are_capped_and_the_oldest_go() {
        let mut a = Authorship::default();
        for i in 0..(MAX_EDITS as u64 + 5) {
            a.stamp(who("Anna"), T0 + i * JOIN_SECS, false);
        }
        assert_eq!(a.edits.len(), MAX_EDITS);
        assert_eq!(a.edits[0].at, iso(T0 + 5 * JOIN_SECS));
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
