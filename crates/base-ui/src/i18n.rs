//! Tiny translation layer. English is the source language: `tr("English text")` returns the
//! translation for the active language, or the English text itself when none exists.
//!
//! The texts live in `Texts` sets. An application registers its own set with `register`; `tr` asks the registered
//! sets in order and then the set of this crate.
use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{OnceLock, RwLock};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Lang {
    En,
    Pl,
    De,
    It,
    Fi,
    Zh,
    Hi,
    // European languages whose texts live in `assets/lang/<code>.txt` (see `extra_map`).
    Bg,
    Ca,
    Hr,
    Cs,
    Da,
    Nl,
    Et,
    Fr,
    El,
    Hu,
    Is,
    Ga,
    Lv,
    Lt,
    Mt,
    Nb,
    Pt,
    Ro,
    Sk,
    Sl,
    Es,
    Sv,
    Uk,
}

impl Lang {
    pub const ALL: [Lang; 30] = [Lang::En, Lang::Pl, Lang::De, Lang::It, Lang::Fi, Lang::Zh, Lang::Hi, Lang::Bg, Lang::Ca, Lang::Hr, Lang::Cs, Lang::Da, Lang::Nl, Lang::Et, Lang::Fr, Lang::El, Lang::Hu, Lang::Is, Lang::Ga, Lang::Lv, Lang::Lt, Lang::Mt, Lang::Nb, Lang::Pt, Lang::Ro, Lang::Sk, Lang::Sl, Lang::Es, Lang::Sv, Lang::Uk];

    /// Name shown in the language menu (always in the language itself, plus English for scripts that
    /// might lack a font on a bare system).
    pub fn name(self) -> &'static str {
        match self {
            Lang::En => "English",
            Lang::Pl => "Polski",
            Lang::De => "Deutsch",
            Lang::It => "Italiano",
            Lang::Fi => "Suomi",
            Lang::Zh => "中文 (Chinese)",
            Lang::Hi => "हिन्दी (Hindi)",
            Lang::Bg => "Български",
            Lang::Ca => "Català",
            Lang::Hr => "Hrvatski",
            Lang::Cs => "Čeština",
            Lang::Da => "Dansk",
            Lang::Nl => "Nederlands",
            Lang::Et => "Eesti",
            Lang::Fr => "Français",
            Lang::El => "Ελληνικά",
            Lang::Hu => "Magyar",
            Lang::Is => "Íslenska",
            Lang::Ga => "Gaeilge",
            Lang::Lv => "Latviešu",
            Lang::Lt => "Lietuvių",
            Lang::Mt => "Malti",
            Lang::Nb => "Norsk bokmål",
            Lang::Pt => "Português",
            Lang::Ro => "Română",
            Lang::Sk => "Slovenčina",
            Lang::Sl => "Slovenščina",
            Lang::Es => "Español",
            Lang::Sv => "Svenska",
            Lang::Uk => "Українська",
        }
    }
    pub fn code(self) -> &'static str {
        match self {
            Lang::En => "en",
            Lang::Pl => "pl",
            Lang::De => "de",
            Lang::It => "it",
            Lang::Fi => "fi",
            Lang::Zh => "zh",
            Lang::Hi => "hi",
            Lang::Bg => "bg",
            Lang::Ca => "ca",
            Lang::Hr => "hr",
            Lang::Cs => "cs",
            Lang::Da => "da",
            Lang::Nl => "nl",
            Lang::Et => "et",
            Lang::Fr => "fr",
            Lang::El => "el",
            Lang::Hu => "hu",
            Lang::Is => "is",
            Lang::Ga => "ga",
            Lang::Lv => "lv",
            Lang::Lt => "lt",
            Lang::Mt => "mt",
            Lang::Nb => "nb",
            Lang::Pt => "pt",
            Lang::Ro => "ro",
            Lang::Sk => "sk",
            Lang::Sl => "sl",
            Lang::Es => "es",
            Lang::Sv => "sv",
            Lang::Uk => "uk",
        }
    }
    pub fn from_code(s: &str) -> Option<Lang> {
        Lang::ALL.into_iter().find(|l| l.code() == s)
    }
    pub fn index(self) -> u8 {
        Lang::ALL.iter().position(|l| *l == self).unwrap_or(0) as u8
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_lang(l: Lang) {
    CURRENT.store(l.index(), Ordering::Relaxed);
}

pub fn lang() -> Lang {
    Lang::ALL[CURRENT.load(Ordering::Relaxed) as usize % Lang::ALL.len()]
}

/// Translate an English UI string.
pub fn tr(s: &'static str) -> &'static str {
    let i = CURRENT.load(Ordering::Relaxed) as usize;
    if i == 0 {
        return s;
    }
    if let Ok(sets) = SETS.read() {
        if let Some(t) = sets.iter().find_map(|t| t.lookup(i, s)) {
            return t;
        }
    }
    BASE.lookup(i, s).unwrap_or(s)
}

static SETS: RwLock<Vec<&'static Texts>> = RwLock::new(Vec::new());

use crate::texts::TEXTS as BASE;

/// Add the texts of an application. They are asked before the shared ones, in the order they were registered.
/// Registering the same set again does nothing.
pub fn register(texts: &'static Texts) {
    if let Ok(mut sets) = SETS.write() {
        if !sets.iter().any(|t| std::ptr::eq(*t, texts)) {
            sets.push(texts);
        }
    }
}

/// Number of languages that come from the table (English plus six).
pub const TABLE_LANGS: usize = 7;

/// A set of texts: a table with English and six languages, and the other 23 languages as `id|translation` lines,
/// where the id is the line of the English text in the `keys` text (`id<TAB>English`).
pub struct Texts {
    table: &'static [[&'static str; TABLE_LANGS]],
    keys: &'static str,
    files: [&'static str; 23],
    rows: OnceLock<HashMap<&'static str, &'static [&'static str; TABLE_LANGS]>>,
    english: OnceLock<HashMap<usize, &'static str>>,
    maps: [OnceLock<HashMap<&'static str, &'static str>>; 23],
}

impl Texts {
    /// `files` are in the order of `Lang::ALL` after the first seven.
    pub const fn new(table: &'static [[&'static str; TABLE_LANGS]], keys: &'static str, files: [&'static str; 23]) -> Texts {
        Texts { table, keys, files, rows: OnceLock::new(), english: OnceLock::new(), maps: [const { OnceLock::new() }; 23] }
    }
    pub fn table(&self) -> &'static [[&'static str; TABLE_LANGS]] {
        self.table
    }
    pub fn keys(&self) -> &'static str {
        self.keys
    }
    pub fn files(&self) -> &[&'static str; 23] {
        &self.files
    }

    /// English text → translation for the language with column `i` (7 or more).
    fn extra_map(&self, i: usize) -> &HashMap<&'static str, &'static str> {
        let english = self
            .english
            .get_or_init(|| self.keys.lines().filter_map(|l| l.split_once('\t')).filter_map(|(id, text)| Some((id.parse().ok()?, text))).collect());
        self.maps[i - TABLE_LANGS].get_or_init(|| {
            self.files[i - TABLE_LANGS]
                .lines()
                .filter_map(|l| l.split_once('|'))
                .filter_map(|(id, text)| Some((*english.get(&id.parse::<usize>().ok()?)?, text)))
                .collect()
        })
    }

    /// The translation of `s` into the language with table column `i` (0 = English), or `None` when this set has none.
    pub fn lookup(&self, i: usize, s: &'static str) -> Option<&'static str> {
        if i == 0 {
            return None;
        }
        if i >= TABLE_LANGS {
            return self.extra_map(i).get(s).copied();
        }
        let rows = self.rows.get_or_init(|| self.table.iter().map(|row| (row[0], row)).collect());
        rows.get(s).map(|row| row[i]).filter(|t| !t.is_empty())
    }
}

/// Substitute each `{}` in `text` with the next argument.
pub fn fill(text: &str, args: &[&dyn Display]) -> String {
    let mut out = text.to_string();
    for a in args {
        out = out.replacen("{}", &a.to_string(), 1);
    }
    out
}

/// Translate and substitute each `{}` with the next argument.
pub fn trf(s: &'static str, args: &[&dyn Display]) -> String {
    fill(tr(s), args)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: &[[&str; TABLE_LANGS]] = &[["Save", "Zapisz", "Speichern", "Salva", "Tallenna", "保存", "सहेजें"]];
    static T: Texts = Texts::new(ROWS, "0\tSave\n", {
        let mut f = [""; 23];
        f[7] = "0|Enregistrer\n";
        f
    });

    #[test]
    fn a_set_answers_for_its_texts_and_for_every_column() {
        assert_eq!(T.lookup(2, "Save"), Some("Speichern"));
        assert_eq!(T.lookup(0, "Save"), None, "English needs no lookup");
        assert_eq!(T.lookup(TABLE_LANGS + 7, "Save"), Some("Enregistrer"));
        assert_eq!(T.lookup(2, "not in table"), None);
        assert_eq!(T.lookup(TABLE_LANGS, "Save"), None, "a language file without the text gives none");
    }

    #[test]
    fn languages_have_unique_codes_and_are_found_by_code() {
        assert_eq!(Lang::from_code("sv"), Some(Lang::Sv));
        assert_eq!(Lang::ALL.len(), 30);
        let mut codes: Vec<_> = Lang::ALL.iter().map(|l| l.code()).collect();
        codes.sort();
        codes.dedup();
        assert_eq!(codes.len(), 30, "language codes are unique");
        assert_eq!(Lang::Fr.index() as usize, TABLE_LANGS + 7);
    }

    #[test]
    fn placeholders_are_filled_in_order() {
        assert_eq!(fill("{} of {}", &[&1, &"x"]), "1 of x");
    }
}
