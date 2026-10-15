//! The online list of official plugins: a JSON index (`index.json`) built by the plugins repository
//! (`michalsarna/baseCreator-plugins`). This module only reads and compares; the program fetches the files.
use crate::plugins::Registry;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::cmp::Ordering;

/// Where the list is published (GitHub Pages of the plugins repository).
pub const INDEX_URL: &str = "https://michalsarna.github.io/baseCreator-plugins/index.json";
/// Files of plugins are only fetched from here.
pub const TRUSTED_PREFIX: &str = "https://michalsarna.github.io/baseCreator-plugins/";
pub const REPO_URL: &str = "https://github.com/michalsarna/baseCreator-plugins";
pub const MAX_INDEX: usize = 2_000_000;
pub const MAX_ZIP: u64 = 16_000_000;
pub const FORMAT: u32 = 1;

/// The version of baseCreator these crates are.
pub fn base_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    /// The programs it is for; empty means all.
    #[serde(default)]
    pub programs: Vec<String>,
    /// What it brings: `presets`, `clipart`, `importers`, ...
    #[serde(default)]
    pub provides: Vec<String>,
    pub version: String,
    /// The oldest baseCreator (program) it works with.
    #[serde(default)]
    pub min_base: String,
    pub url: String,
    pub sha256: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub homepage: String,
    #[serde(default)]
    pub changelog: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Index {
    #[serde(default)]
    pub format: u32,
    #[serde(default)]
    pub updated: String,
    pub plugins: Vec<Entry>,
}

/// What the person can do with an entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Status {
    NotInstalled,
    UpToDate,
    /// The installed version is older.
    Update { installed: String },
    /// The installed one is newer than the list's (a plugin of your own, or the list is old).
    Newer,
    /// The plugin needs a newer program than this.
    NeedsNewerProgram { min: String },
}

/// Compare dotted versions by their numbers (`1.10.0` is newer than `1.9.0`); missing parts count as 0.
pub fn compare_versions(a: &str, b: &str) -> Ordering {
    let parts = |s: &str| -> Vec<u64> { s.trim().trim_start_matches('v').split(['.', '-', '+']).map(|p| p.chars().take_while(|c| c.is_ascii_digit()).collect::<String>().parse().unwrap_or(0)).collect() };
    let (x, y) = (parts(a), parts(b));
    for i in 0..x.len().max(y.len()) {
        match x.get(i).unwrap_or(&0).cmp(y.get(i).unwrap_or(&0)) {
            Ordering::Equal => {}
            o => return o,
        }
    }
    Ordering::Equal
}

pub fn parse_index(text: &str) -> Result<Index, String> {
    if text.len() > MAX_INDEX {
        return Err("the list is too big".into());
    }
    let idx: Index = serde_json::from_str(text).map_err(|e| format!("the list of plugins cannot be read: {e}"))?;
    if idx.format > FORMAT {
        return Err("the list is from a newer version of the program; update the program".into());
    }
    let mut seen = std::collections::BTreeSet::new();
    let mut ok = vec![];
    for e in idx.plugins {
        // An entry that is not valid is skipped, so one mistake in the list does not hide the others.
        if e.id.is_empty() || e.name.is_empty() || e.version.is_empty() || !is_sha256(&e.sha256) || !url_allowed(&e.url) || !seen.insert(e.id.clone()) {
            continue;
        }
        ok.push(e);
    }
    Ok(Index { format: idx.format, updated: idx.updated, plugins: ok })
}

fn is_sha256(s: &str) -> bool {
    s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// Files are only taken from the official site.
pub fn url_allowed(url: &str) -> bool {
    url.starts_with(TRUSTED_PREFIX) && !url.contains("..") && url.len() < 300 && url.ends_with(".zip")
}

/// Does `bytes` have this SHA-256 (hex, either case)?
pub fn verify(bytes: &[u8], sha256: &str) -> bool {
    let mut h = Sha256::new();
    h.update(bytes);
    let got: String = h.finalize().iter().map(|b| format!("{b:02x}")).collect();
    got.eq_ignore_ascii_case(sha256)
}

impl Entry {
    /// Is the plugin for this program (`lightcreator`, ...)?
    pub fn is_for(&self, app: &str) -> bool {
        self.programs.is_empty() || self.programs.iter().any(|p| p == app)
    }

    pub fn status(&self, installed: &Registry) -> Status {
        if !self.min_base.is_empty() && compare_versions(&self.min_base, base_version()) == Ordering::Greater {
            return Status::NeedsNewerProgram { min: self.min_base.clone() };
        }
        match installed.find(&self.id) {
            None => Status::NotInstalled,
            Some(p) => match compare_versions(&p.manifest.version, &self.version) {
                Ordering::Less => Status::Update { installed: p.manifest.version.clone() },
                Ordering::Equal => Status::UpToDate,
                Ordering::Greater => Status::Newer,
            },
        }
    }
}

impl Index {
    /// How many installed plugins have a newer version in the list.
    pub fn updates(&self, installed: &Registry) -> Vec<&Entry> {
        self.plugins.iter().filter(|e| matches!(e.status(installed), Status::Update { .. })).collect()
    }
}

/// Install a downloaded zip of an entry: the size and the checksum must match, and the plugin inside must be the
/// plugin the list says (same id and version).
pub fn install(reg: &mut Registry, e: &Entry, bytes: &[u8]) -> Result<String, String> {
    if bytes.len() as u64 > MAX_ZIP {
        return Err("the download is too big".into());
    }
    if !verify(bytes, &e.sha256) {
        return Err("the downloaded file does not match its checksum".into());
    }
    reg.install_zip_expecting(bytes, &e.id, &e.version)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_compare_by_number() {
        assert_eq!(compare_versions("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(compare_versions("1.0", "1.0.0"), Ordering::Equal);
        assert_eq!(compare_versions("v0.9.8", "0.10.0"), Ordering::Less);
        assert_eq!(compare_versions("2.0.0-beta", "2.0.0"), Ordering::Equal);
    }

    #[test]
    fn only_the_official_site_is_trusted() {
        assert!(url_allowed("https://michalsarna.github.io/baseCreator-plugins/zips/a-1.0.0.zip"));
        assert!(!url_allowed("https://evil.example/baseCreator-plugins/zips/a.zip"));
        assert!(!url_allowed("https://michalsarna.github.io/baseCreator-plugins/../x.zip"));
        assert!(!url_allowed("http://michalsarna.github.io/baseCreator-plugins/a.zip"));
        assert!(!url_allowed("https://michalsarna.github.io/baseCreator-plugins/a.exe"));
    }

    #[test]
    fn a_bad_entry_does_not_hide_the_others() {
        let good = format!(r#"{{"id":"a","name":"A","version":"1.0.0","url":"{TRUSTED_PREFIX}zips/a-1.0.0.zip","sha256":"{}"}}"#, "a".repeat(64));
        let bad = r#"{"id":"b","name":"B","version":"1.0.0","url":"https://evil/x.zip","sha256":"zz"}"#;
        let idx = parse_index(&format!(r#"{{"format":1,"plugins":[{good},{bad},{good}]}}"#)).unwrap();
        assert_eq!(idx.plugins.len(), 1);
        assert!(parse_index(r#"{"format":99,"plugins":[]}"#).is_err());
        assert!(parse_index("nope").is_err());
    }

    #[test]
    fn the_checksum_is_checked() {
        let sum = "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824";
        assert!(verify(b"hello", sum));
        assert!(verify(b"hello", &sum.to_uppercase()));
        assert!(!verify(b"hellp", sum));
    }
}
