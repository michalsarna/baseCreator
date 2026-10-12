//! Clipart library: pictures you saved or downloaded (kept as SVG files with an index), and reading the answers
//! of the Iconify search service.
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A picture in the user's library (the pictures shipped with the program are not listed here).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    /// File name stem inside the library folder (`<id>.svg`).
    pub id: String,
    pub name: String,
    pub category: String,
    /// Where it came from: a web address, or "file".
    #[serde(default)]
    pub source: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub author: String,
}

/// The user's clipart: a folder with `index.json` and one SVG per picture.
#[derive(Debug)]
pub struct Library {
    dir: PathBuf,
    pub items: Vec<Clip>,
}

pub const DEFAULT_CATEGORY: &str = "My clipart";

/// Lowercase file-name friendly version of `s`.
pub fn slug(s: &str) -> String {
    let mut out = String::new();
    for c in s.chars() {
        if c.is_alphanumeric() {
            out.extend(c.to_lowercase());
        } else if !out.ends_with('-') && !out.is_empty() {
            out.push('-');
        }
    }
    let out = out.trim_end_matches('-').to_string();
    if out.is_empty() {
        "clip".into()
    } else {
        out.chars().take(48).collect()
    }
}

/// Check that `raw` is a usable SVG and make it self-contained (`currentColor` becomes black).
pub fn normalize_svg(raw: &str) -> Result<String, String> {
    if raw.len() > 2_000_000 {
        return Err("the picture is too large".into());
    }
    let text = raw.replace("currentColor", "#000000");
    if !text.contains("<svg") {
        return Err("this is not an SVG picture".into());
    }
    usvg::Tree::from_data(text.as_bytes(), &usvg::Options::default()).map_err(|e| e.to_string())?;
    Ok(text)
}

impl Library {
    /// Open (or start) the library in `dir`. A missing or damaged index gives an empty library.
    pub fn open(dir: impl Into<PathBuf>) -> Library {
        let dir = dir.into();
        let items: Vec<Clip> = std::fs::read_to_string(dir.join("index.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let items = items.into_iter().filter(|c| Path::new(&dir_join(&dir, &c.id)).exists()).collect();
        Library { dir, items }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    pub fn save(&self) -> Result<(), String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let json = serde_json::to_string_pretty(&self.items).map_err(|e| e.to_string())?;
        std::fs::write(self.dir.join("index.json"), json).map_err(|e| e.to_string())
    }

    pub fn svg(&self, id: &str) -> Option<String> {
        std::fs::read_to_string(dir_join(&self.dir, id)).ok()
    }

    /// Add a picture. The SVG is validated and stored; returns the new entry.
    pub fn add(&mut self, svg: &str, name: &str, category: &str, source: &str, license: &str, author: &str) -> Result<Clip, String> {
        let text = normalize_svg(svg)?;
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        let base = slug(name);
        let mut id = base.clone();
        let mut n = 2;
        while self.items.iter().any(|c| c.id == id) || Path::new(&dir_join(&self.dir, &id)).exists() {
            id = format!("{base}-{n}");
            n += 1;
        }
        std::fs::write(dir_join(&self.dir, &id), text).map_err(|e| e.to_string())?;
        let category = if category.trim().is_empty() { DEFAULT_CATEGORY } else { category.trim() };
        let clip = Clip { id, name: name.trim().to_string(), category: category.to_string(), source: source.to_string(), license: license.to_string(), author: author.to_string() };
        self.items.push(clip.clone());
        self.save()?;
        Ok(clip)
    }

    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let _ = std::fs::remove_file(dir_join(&self.dir, id));
        self.items.retain(|c| c.id != id);
        self.save()
    }

    pub fn set_category(&mut self, id: &str, category: &str) -> Result<(), String> {
        if let Some(c) = self.items.iter_mut().find(|c| c.id == id) {
            c.category = if category.trim().is_empty() { DEFAULT_CATEGORY.into() } else { category.trim().into() };
        }
        self.save()
    }

    pub fn rename(&mut self, id: &str, name: &str) -> Result<(), String> {
        if let Some(c) = self.items.iter_mut().find(|c| c.id == id) {
            if !name.trim().is_empty() {
                c.name = name.trim().into();
            }
        }
        self.save()
    }

    /// Categories used by the library, sorted.
    pub fn categories(&self) -> Vec<String> {
        let mut v: Vec<String> = self.items.iter().map(|c| c.category.clone()).collect();
        v.sort_by_key(|s| s.to_lowercase());
        v.dedup();
        v
    }
}

fn dir_join(dir: &Path, id: &str) -> String {
    dir.join(format!("{id}.svg")).to_string_lossy().to_string()
}

/// Answers of the Iconify search service (https://iconify.design), which indexes many open icon sets.
pub mod iconify {
    use serde::Deserialize;
    use std::collections::HashMap;

    pub const API: &str = "https://api.iconify.design";

    #[derive(Clone, Debug, PartialEq)]
    pub struct Hit {
        pub prefix: String,
        pub name: String,
        pub set_name: String,
        pub license_title: String,
        pub license_spdx: String,
        pub license_url: String,
        pub author: String,
    }

    impl Hit {
        pub fn key(&self) -> String {
            format!("{}:{}", self.prefix, self.name)
        }
        /// Page of the picture on the Iconify site.
        pub fn page_url(&self) -> String {
            format!("https://icon-sets.iconify.design/{}/{}/", self.prefix, self.name)
        }
        pub fn svg_url(&self, base: &str) -> String {
            format!("{base}/{}/{}.svg", self.prefix, self.name)
        }
        /// Licences that need no credit line in the finished product.
        pub fn needs_no_credit(&self) -> bool {
            let s = self.license_spdx.to_uppercase();
            ["CC0", "MIT", "ISC", "APACHE", "UNLICENSE", "0BSD", "PUBLIC"].iter().any(|k| s.contains(k)) && !s.contains("CC-BY")
        }
    }

    pub fn search_url(base: &str, query: &str, limit: u32) -> String {
        let enc: String = query.bytes().map(|b| if b.is_ascii_alphanumeric() || b"-_.~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") }).collect();
        format!("{base}/search?query={enc}&limit={limit}")
    }

    #[derive(Deserialize)]
    struct Reply {
        #[serde(default)]
        icons: Vec<String>,
        #[serde(default)]
        collections: HashMap<String, Coll>,
    }
    #[derive(Deserialize, Default)]
    struct Coll {
        #[serde(default)]
        name: String,
        #[serde(default)]
        author: Option<Auth>,
        #[serde(default)]
        license: Option<Lic>,
    }
    #[derive(Deserialize)]
    struct Auth {
        #[serde(default)]
        name: String,
    }
    #[derive(Deserialize)]
    struct Lic {
        #[serde(default)]
        title: String,
        #[serde(default)]
        spdx: String,
        #[serde(default)]
        url: String,
    }

    pub fn parse_search(json: &str) -> Result<Vec<Hit>, String> {
        let r: Reply = serde_json::from_str(json).map_err(|e| e.to_string())?;
        Ok(r
            .icons
            .iter()
            .filter_map(|full| {
                let (prefix, name) = full.split_once(':')?;
                let c = r.collections.get(prefix);
                Some(Hit {
                    prefix: prefix.to_string(),
                    name: name.to_string(),
                    set_name: c.map(|c| c.name.clone()).unwrap_or_default(),
                    license_title: c.and_then(|c| c.license.as_ref()).map(|l| l.title.clone()).unwrap_or_default(),
                    license_spdx: c.and_then(|c| c.license.as_ref()).map(|l| l.spdx.clone()).unwrap_or_default(),
                    license_url: c.and_then(|c| c.license.as_ref()).map(|l| l.url.clone()).unwrap_or_default(),
                    author: c.and_then(|c| c.author.as_ref()).map(|a| a.name.clone()).unwrap_or_default(),
                })
            })
            .collect())
    }
}
