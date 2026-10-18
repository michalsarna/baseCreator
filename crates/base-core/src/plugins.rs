//! Plugins: folders in `<data>/baseCreator/plugins/` that add things to every program on baseCreator.
//!
//! A plugin is a folder with a `plugin.json` (the manifest) and the files it names. Four kinds of contributions:
//!
//! 1. **Packs, no code:** machine presets, clipart, translations and plain data files (tool libraries, materials) that
//!    the programs pick up by their `kind`.
//! 2. **Importers:** a script that reads a file format and returns shapes.
//! 3. **Generators:** a script with parameters (a form the user fills in) that returns shapes.
//! 4. **Postprocessors:** a script that rewrites the lines of a G-code program for another controller.
//!
//! The scripts are in Rhai (`plugin_script`): they have no access to files, the network, the machine or the system; they
//! only compute and return data, which the program then checks like everything else. See `docs/PLUGINS.md`.
use crate::clipart;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Component, Path, PathBuf};

pub const MANIFEST: &str = "plugin.json";
pub const FORMAT: u32 = 1;
pub const MAX_PLUGINS: usize = 200;
const MAX_MANIFEST: u64 = 1_000_000;
const MAX_FILE: u64 = 8_000_000;
const MAX_ZIP_TOTAL: u64 = 64_000_000;
/// The biggest zip file that is read.
const MAX_ZIP_FILE: u64 = 32_000_000;
const MAX_ZIP_FILES: usize = 2000;

/// The programs a plugin can name in `programs`.
/// File types the programs read themselves; an importer of a plugin cannot take them over.
pub const RESERVED_EXTENSIONS: [&str; 13] = ["svg", "pdf", "ai", "cre", "lbrn", "lbrn2", "ugsd", "png", "jpg", "jpeg", "bmp", "gif", "webp"];

pub const PROGRAMS: [&str; 3] = ["lightcreator", "millcreator", "pencreator"];

/// What kind of machine a preset is for: the programs take the ones of their class.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PresetClass {
    Laser,
    Mill,
    Pen,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PresetDef {
    pub name: String,
    pub class: PresetClass,
    /// Travel of X, Y and Z, mm (Z is 0 for a plotter or a laser).
    pub area: [f64; 3],
    /// A speed of moves, mm/min.
    #[serde(default = "default_rapid")]
    pub rapid: f64,
    /// A spindle speed range top, S units (a laser: the power range top).
    #[serde(default)]
    pub spindle_max: f64,
}

fn default_rapid() -> f64 {
    3000.0
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ClipDef {
    /// Path of the SVG file inside the plugin folder.
    pub file: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub category: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub author: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DataDef {
    /// What the file is, as the programs know it: `mill-tools`, `materials`, ...
    pub kind: String,
    pub file: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImporterDef {
    pub id: String,
    pub name: String,
    /// File extensions without the dot, lowercase.
    pub extensions: Vec<String>,
    pub script: String,
    #[serde(default)]
    pub description: String,
    /// The script gets the file as a blob of bytes (a binary format) instead of as a text.
    #[serde(default)]
    pub binary: bool,
    /// A form shown before the shapes are made; its values reach the script in `ctx.p`.
    #[serde(default)]
    pub params: Vec<ParamDef>,
    /// The script also has `fn form(data, ctx)` that returns more fields of the form, made from the contents of the file.
    #[serde(default)]
    pub form: bool,
    /// Show the import window, with a preview of the shapes, also when there is no form.
    #[serde(default)]
    pub preview: bool,
    /// The import window also shows the model in 3D (an STL file), which the person can turn; the turn reaches the script as
    /// `ctx.p.view_yaw` and `ctx.p.view_pitch` (degrees, see `Mesh::rotated`), 0 and 0 being the view from above.
    #[serde(default)]
    pub view3d: bool,
}

impl ImporterDef {
    /// Does the import go through a window (a form, a preview) instead of making the shapes at once?
    pub fn interactive(&self) -> bool {
        !self.params.is_empty() || self.form || self.preview || self.view3d
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ParamKind {
    Number,
    Bool,
    Text,
    Choice,
}

/// One field of the form of a generator.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub id: String,
    #[serde(default)]
    pub label: String,
    pub kind: ParamKind,
    /// The start value: a number, a bool or a text (for a choice, one of `options`).
    #[serde(default)]
    pub default: serde_json::Value,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub options: Vec<String>,
    /// Shown after the number: `mm`, `°`, ...
    #[serde(default)]
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GeneratorDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub script: String,
    #[serde(default)]
    pub params: Vec<ParamDef>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PostDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub script: String,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub format: u32,
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub version: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub license: String,
    #[serde(default)]
    pub description: String,
    /// The oldest baseCreator (the programs' shared version) the plugin works with, like `0.9.8`.
    #[serde(default)]
    pub min_base: String,
    /// The programs the plugin is for; empty means all.
    #[serde(default)]
    pub programs: Vec<String>,
    #[serde(default)]
    pub presets: Vec<PresetDef>,
    #[serde(default)]
    pub clipart: Vec<ClipDef>,
    /// Translations: language code -> English text -> translation.
    #[serde(default)]
    pub texts: std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>,
    #[serde(default)]
    pub data: Vec<DataDef>,
    #[serde(default)]
    pub importers: Vec<ImporterDef>,
    #[serde(default)]
    pub generators: Vec<GeneratorDef>,
    #[serde(default)]
    pub postprocessors: Vec<PostDef>,
}

fn ok_id(s: &str) -> bool {
    !s.is_empty() && s.len() <= 64 && s.chars().all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '_')
}

/// The fields of a form: at most 40, with ids that are valid and not repeated and defaults of the right kind.
pub fn check_params(owner: &str, params: &[ParamDef]) -> Result<(), String> {
    if params.len() > 40 {
        return Err(format!("{owner}: at most 40 parameters"));
    }
    let mut seen = BTreeSet::new();
    for p in params {
        if !ok_id(&p.id) || !seen.insert(&p.id) {
            return Err(format!("{owner}: parameter {:?} has a bad or repeated id", p.id));
        }
        match p.kind {
            ParamKind::Number if !(p.default.is_number() || p.default.is_null()) => return Err(format!("parameter {:?}: the default of a number must be a number", p.id)),
            ParamKind::Bool if !(p.default.is_boolean() || p.default.is_null()) => return Err(format!("parameter {:?}: the default of a bool must be true or false", p.id)),
            ParamKind::Text if !(p.default.is_string() || p.default.is_null()) => return Err(format!("parameter {:?}: the default of a text must be a text", p.id)),
            ParamKind::Choice if p.options.is_empty() || p.options.len() > 100 => return Err(format!("parameter {:?}: a choice needs 1-100 options", p.id)),
            _ => {}
        }
    }
    Ok(())
}

/// A path inside the plugin folder: relative, without `..`.
pub fn safe_rel(p: &str) -> bool {
    let path = Path::new(p);
    !p.is_empty() && p.len() <= 200 && !path.is_absolute() && path.components().all(|c| matches!(c, Component::Normal(_))) && !p.contains('\\') && !p.contains('\0')
}

impl Manifest {
    pub fn parse(text: &str) -> Result<Manifest, String> {
        let m: Manifest = serde_json::from_str(text).map_err(|e| format!("{MANIFEST}: {e}"))?;
        m.check()?;
        Ok(m)
    }

    pub fn check(&self) -> Result<(), String> {
        if self.format > FORMAT {
            return Err(format!("the plugin has format {}, this program reads up to {FORMAT}; update the program", self.format));
        }
        if !ok_id(&self.id) {
            return Err("the id must be 1-64 characters: a-z, 0-9, - and _".into());
        }
        if self.name.trim().is_empty() || self.name.len() > 120 || self.description.len() > 4000 || self.version.len() > 40 || self.author.len() > 120 || self.license.len() > 120 {
            return Err("the name, version, author, licence or description is empty or too long".into());
        }
        if !self.min_base.is_empty() && crate::registry::compare_versions(&self.min_base, crate::registry::base_version()) == std::cmp::Ordering::Greater {
            return Err(format!("the plugin needs baseCreator {} or newer (update the program)", self.min_base));
        }
        if let Some(p) = self.programs.iter().find(|p| !PROGRAMS.contains(&p.as_str())) {
            return Err(format!("unknown program {p:?}; use {}", PROGRAMS.join(", ")));
        }
        for p in &self.presets {
            let ok = |v: f64, hi: f64| v.is_finite() && (0.0..=hi).contains(&v);
            if p.name.trim().is_empty() || p.name.len() > 120 || !(p.area[0] >= 10.0 && p.area[1] >= 10.0 && p.area[0] <= 100_000.0 && p.area[1] <= 100_000.0 && ok(p.area[2], 10_000.0) && p.rapid > 0.0 && p.rapid <= 1_000_000.0 && ok(p.spindle_max, 1_000_000.0)) {
                return Err(format!("preset {:?}: a size or a speed is out of range", p.name));
            }
        }
        for c in &self.clipart {
            if !safe_rel(&c.file) {
                return Err(format!("clipart file {:?} is not a path inside the plugin", c.file));
            }
        }
        for d in &self.data {
            if !safe_rel(&d.file) || !ok_id(&d.kind) {
                return Err(format!("data {:?}: the kind or the file is not valid", d.file));
            }
        }
        let mut ids = BTreeSet::new();
        for (id, script) in self.importers.iter().map(|i| (&i.id, &i.script)).chain(self.generators.iter().map(|g| (&g.id, &g.script))).chain(self.postprocessors.iter().map(|p| (&p.id, &p.script))) {
            if !ok_id(id) || !ids.insert(id.clone()) {
                return Err(format!("{id:?}: not a valid id, or used twice in the plugin"));
            }
            if !safe_rel(script) || !script.ends_with(".rhai") {
                return Err(format!("script {script:?} must be a .rhai file inside the plugin"));
            }
        }
        for i in &self.importers {
            if let Some(e) = i.extensions.iter().find(|e| RESERVED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str())) {
                return Err(format!("importer {:?}: .{e} is read by the program itself", i.id));
            }
            check_params(&format!("importer {:?}", i.id), &i.params)?;
            if i.name.trim().is_empty() || i.extensions.is_empty() || i.extensions.iter().any(|e| e.is_empty() || e.len() > 16 || !e.chars().all(|c| c.is_ascii_alphanumeric())) {
                return Err(format!("importer {:?}: needs a name and extensions (letters and digits)", i.id));
            }
        }
        for g in &self.generators {
            if g.name.trim().is_empty() || g.params.len() > 40 {
                return Err(format!("generator {:?}: needs a name and at most 40 parameters", g.id));
            }
            check_params(&format!("generator {:?}", g.id), &g.params)?;
        }
        for p in &self.postprocessors {
            if p.name.trim().is_empty() {
                return Err(format!("postprocessor {:?}: needs a name", p.id));
            }
        }
        Ok(())
    }

    /// Is the plugin for this program (`lightcreator`, `millcreator`, `pencreator`)?
    pub fn is_for(&self, app: &str) -> bool {
        self.programs.is_empty() || self.programs.iter().any(|p| p == app)
    }
}

#[derive(Clone, Debug)]
pub struct Plugin {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub enabled: bool,
}

/// A plugin folder that could not be read, with the reason.
#[derive(Clone, Debug)]
pub struct Broken {
    pub folder: String,
    pub reason: String,
}

#[derive(Default, Serialize, Deserialize)]
struct State {
    #[serde(default)]
    disabled: Vec<String>,
    /// Look for newer plugins in the official list at the start of a program (off unless the person switches it on).
    #[serde(default)]
    auto_check: bool,
}

/// The plugins found in the folder.
#[derive(Debug, Default)]
pub struct Registry {
    dir: PathBuf,
    auto_check: bool,
    pub plugins: Vec<Plugin>,
    pub broken: Vec<Broken>,
}

/// The folder of the plugins inside the shared folder.
pub fn plugins_dir(base: &Path) -> PathBuf {
    base.join("plugins")
}

impl Registry {
    /// Read the plugins in `dir` (a missing folder is an empty registry).
    pub fn load(dir: impl Into<PathBuf>) -> Registry {
        let dir = dir.into();
        let state: State = std::fs::read_to_string(dir.join("state.json")).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default();
        let mut r = Registry { dir: dir.clone(), auto_check: state.auto_check, plugins: vec![], broken: vec![] };
        let Ok(rd) = std::fs::read_dir(&dir) else { return r };
        let mut folders: Vec<PathBuf> = rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()).collect();
        folders.sort();
        for f in folders {
            let name = f.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
            if name.starts_with('.') {
                continue;
            }
            if r.plugins.len() + r.broken.len() >= MAX_PLUGINS {
                r.broken.push(Broken { folder: name, reason: "too many plugins".into() });
                continue;
            }
            match read_manifest(&f) {
                Ok(m) if r.plugins.iter().any(|p| p.manifest.id == m.id) => r.broken.push(Broken { folder: name, reason: format!("the id {:?} is used by another plugin", m.id) }),
                Ok(m) => {
                    let enabled = !state.disabled.contains(&m.id);
                    r.plugins.push(Plugin { manifest: m, dir: f, enabled });
                }
                Err(e) => r.broken.push(Broken { folder: name, reason: e }),
            }
        }
        r
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn save_state(&self) -> Result<(), String> {
        let state = State { disabled: self.plugins.iter().filter(|p| !p.enabled).map(|p| p.manifest.id.clone()).collect(), auto_check: self.auto_check };
        std::fs::create_dir_all(&self.dir).map_err(|e| e.to_string())?;
        std::fs::write(self.dir.join("state.json"), serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
    }

    /// Is the check for newer plugins at the start of a program on? (Shared by all programs.)
    pub fn auto_check(&self) -> bool {
        self.auto_check
    }

    pub fn set_auto_check(&mut self, on: bool) -> Result<(), String> {
        self.auto_check = on;
        self.save_state()
    }

    pub fn set_enabled(&mut self, id: &str, on: bool) -> Result<(), String> {
        if let Some(p) = self.plugins.iter_mut().find(|p| p.manifest.id == id) {
            p.enabled = on;
        }
        self.save_state()
    }

    /// Delete the plugin's folder.
    pub fn remove(&mut self, id: &str) -> Result<(), String> {
        let Some(i) = self.plugins.iter().position(|p| p.manifest.id == id) else { return Ok(()) };
        let p = self.plugins.remove(i);
        // Only ever delete a folder that is directly inside the plugins folder.
        if p.dir.parent() != Some(self.dir.as_path()) {
            return Err("the plugin is not in the plugins folder".into());
        }
        std::fs::remove_dir_all(&p.dir).map_err(|e| e.to_string())?;
        self.save_state()
    }

    /// Install a plugin from a `.zip` (the manifest at the top, or inside a single top folder). A plugin of the same id
    /// is replaced. Returns the id.
    pub fn install_zip(&mut self, zip_path: &Path) -> Result<String, String> {
        let bytes = read_limited(zip_path, MAX_ZIP_FILE)?;
        self.install_zip_bytes(&bytes, None)
    }

    /// Install a zip that must hold the plugin `id` in `version` (what the official list promised). The bytes are what
    /// was checked, so nothing can be swapped between the check and the installation.
    pub fn install_zip_expecting(&mut self, bytes: &[u8], id: &str, version: &str) -> Result<String, String> {
        self.install_zip_bytes(bytes, Some((id, version)))
    }

    fn install_zip_bytes(&mut self, bytes: &[u8], expect: Option<(&str, &str)>) -> Result<String, String> {
        let file = std::io::Cursor::new(bytes);
        let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("this is not a zip file: {e}"))?;
        if zip.len() > MAX_ZIP_FILES {
            return Err("the zip has too many files".into());
        }
        // Find the root of the plugin: the folder of plugin.json.
        let mut root = None;
        let mut total = 0u64;
        for i in 0..zip.len() {
            let f = zip.by_index(i).map_err(|e| e.to_string())?;
            total += f.size();
            let name = f.name().map(|n| n.to_string()).unwrap_or_default();
            if name == MANIFEST || (name.ends_with(&format!("/{MANIFEST}")) && name.matches('/').count() == 1 && !name.starts_with("__MACOSX")) {
                root = Some(name[..name.len() - MANIFEST.len()].to_string());
            }
        }
        if total > MAX_ZIP_TOTAL {
            return Err("the zip is too big".into());
        }
        let root = root.ok_or_else(|| format!("there is no {MANIFEST} in the zip"))?;
        let mut files: Vec<(String, Vec<u8>)> = vec![];
        let mut read_total = 0u64;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            let name = f.name().map(|n| n.to_string()).unwrap_or_default();
            if f.is_dir() || !name.starts_with(&root) || name.starts_with("__MACOSX") {
                continue;
            }
            let rel = name[root.len()..].to_string();
            if !safe_rel(&rel) {
                return Err(format!("the zip has an unsafe path: {rel}"));
            }
            if f.size() > MAX_FILE {
                return Err(format!("{rel} is too big"));
            }
            let mut data = Vec::new();
            std::io::Read::read_to_end(&mut std::io::Read::take(&mut f, MAX_FILE + 1), &mut data).map_err(|e| e.to_string())?;
            // The sizes in the headers of a zip can lie: count what was really read.
            if data.len() as u64 > MAX_FILE {
                return Err(format!("{rel} is too big"));
            }
            read_total += data.len() as u64;
            if read_total > MAX_ZIP_TOTAL {
                return Err("the zip is too big".into());
            }
            files.push((rel, data));
        }
        let manifest = files.iter().find(|(n, _)| n == MANIFEST).ok_or("no manifest")?;
        if manifest.1.len() as u64 > MAX_MANIFEST {
            return Err("the manifest is too big".into());
        }
        let m = Manifest::parse(&String::from_utf8(manifest.1.clone()).map_err(|_| "the manifest is not text".to_string())?)?;
        if let Some((id, version)) = expect {
            if m.id != id || crate::registry::compare_versions(&m.version, version) != std::cmp::Ordering::Equal {
                return Err(format!("the zip holds {} {}, not {id} {version}", m.id, m.version));
            }
        }
        // The files the manifest names must be in the zip.
        for rel in m.referenced_files() {
            if !files.iter().any(|(n, _)| *n == rel) {
                return Err(format!("the manifest names {rel}, which is not in the zip"));
            }
        }
        let target = self.dir.join(&m.id);
        let tmp = self.dir.join(format!(".installing-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).map_err(|e| e.to_string())?;
        let write = || -> Result<(), String> {
            for (rel, data) in &files {
                let p = tmp.join(rel);
                if let Some(parent) = p.parent() {
                    std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                }
                std::fs::write(&p, data).map_err(|e| e.to_string())?;
            }
            let _ = std::fs::remove_dir_all(&target);
            std::fs::rename(&tmp, &target).map_err(|e| e.to_string())
        };
        if let Err(e) = write() {
            let _ = std::fs::remove_dir_all(&tmp);
            return Err(e);
        }
        let disabled: Vec<String> = self.plugins.iter().filter(|p| !p.enabled && p.manifest.id != m.id).map(|p| p.manifest.id.clone()).collect();
        let fresh = Registry::load(self.dir.clone());
        self.plugins = fresh.plugins;
        self.broken = fresh.broken;
        for p in &mut self.plugins {
            p.enabled = p.manifest.id == m.id || !disabled.contains(&p.manifest.id) && p.enabled;
        }
        self.save_state()?;
        Ok(m.id)
    }

    fn active<'a>(&'a self, app: &'a str) -> impl Iterator<Item = &'a Plugin> + 'a {
        self.plugins.iter().filter(move |p| p.enabled && p.manifest.is_for(app))
    }

    /// Presets of a class from the plugins that are on and for this program.
    pub fn presets(&self, app: &str, class: PresetClass) -> Vec<PresetDef> {
        self.active(app).flat_map(|p| p.manifest.presets.iter().filter(|d| d.class == class).cloned()).collect()
    }

    /// The text of a data file by kind, with the id of the plugin it comes from.
    pub fn data(&self, app: &str, kind: &str) -> Vec<(String, String)> {
        let mut out = vec![];
        for p in self.active(app) {
            for d in p.manifest.data.iter().filter(|d| d.kind == kind) {
                if let Ok(t) = p.read_text(&d.file) {
                    out.push((p.manifest.id.clone(), t));
                }
            }
        }
        out
    }

    /// The translation of `english` into the language `code` by the plugins, when one has it.
    pub fn translate(&self, code: &str, english: &str) -> Option<&str> {
        self.plugins.iter().filter(|p| p.enabled).find_map(|p| p.manifest.texts.get(code)?.get(english)).map(|s| s.as_str())
    }

    /// Add the clipart of the plugins that are on to the gallery (pictures already there, same name and category, are
    /// skipped). Returns how many were added.
    pub fn sync_clipart(&self, app: &str, gallery: &Path) -> usize {
        let mut lib = clipart::Library::open(gallery);
        let mut added = 0;
        for p in self.active(app) {
            for c in &p.manifest.clipart {
                let Ok(svg) = p.read_text(&c.file) else { continue };
                let name = if c.name.is_empty() { Path::new(&c.file).file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default() } else { c.name.clone() };
                let category = if c.category.is_empty() { p.manifest.name.clone() } else { c.category.clone() };
                if lib.items.iter().any(|i| i.name == name && i.category == category) {
                    continue;
                }
                let author = if c.author.is_empty() { p.manifest.author.clone() } else { c.author.clone() };
                let license = if c.license.is_empty() { p.manifest.license.clone() } else { c.license.clone() };
                if lib.add(&svg, &name, &category, &format!("plugin {}", p.manifest.id), &license, &author).is_ok() {
                    added += 1;
                }
            }
        }
        added
    }

    pub fn importers<'a>(&'a self, app: &'a str) -> Vec<(&'a Plugin, &'a ImporterDef)> {
        self.active(app).flat_map(|p| p.manifest.importers.iter().map(move |i| (p, i))).collect()
    }

    /// The importer for a file extension (lowercase, no dot).
    pub fn importer_for<'a>(&'a self, app: &'a str, ext: &str) -> Option<(&'a Plugin, &'a ImporterDef)> {
        self.importers(app).into_iter().find(|(_, i)| i.extensions.iter().any(|e| e.eq_ignore_ascii_case(ext)))
    }

    pub fn generators<'a>(&'a self, app: &'a str) -> Vec<(&'a Plugin, &'a GeneratorDef)> {
        self.active(app).flat_map(|p| p.manifest.generators.iter().map(move |g| (p, g))).collect()
    }

    pub fn postprocessors<'a>(&'a self, app: &'a str) -> Vec<(&'a Plugin, &'a PostDef)> {
        self.active(app).flat_map(|p| p.manifest.postprocessors.iter().map(move |g| (p, g))).collect()
    }

    pub fn find(&self, id: &str) -> Option<&Plugin> {
        self.plugins.iter().find(|p| p.manifest.id == id)
    }
}

impl Manifest {
    /// Every file the manifest names, relative to the plugin folder.
    pub fn referenced_files(&self) -> Vec<String> {
        let mut v: Vec<String> = self.clipart.iter().map(|c| c.file.clone()).chain(self.data.iter().map(|d| d.file.clone())).collect();
        v.extend(self.importers.iter().map(|i| i.script.clone()));
        v.extend(self.generators.iter().map(|g| g.script.clone()));
        v.extend(self.postprocessors.iter().map(|p| p.script.clone()));
        v
    }
}

impl Plugin {
    /// A text file of the plugin (a script, a data file, an SVG).
    pub fn read_text(&self, rel: &str) -> Result<String, String> {
        if !safe_rel(rel) {
            return Err("not a path inside the plugin".into());
        }
        let p = self.dir.join(rel);
        let meta = std::fs::metadata(&p).map_err(|e| format!("{rel}: {e}"))?;
        if meta.len() > MAX_FILE {
            return Err(format!("{rel} is too big"));
        }
        std::fs::read_to_string(&p).map_err(|e| format!("{rel}: {e}"))
    }
}

/// Read a file that is at most `max` bytes (a bigger one is an error, and is not read).
pub fn read_limited(path: &Path, max: u64) -> Result<Vec<u8>, String> {
    let meta = std::fs::metadata(path).map_err(|e| e.to_string())?;
    if !meta.is_file() {
        return Err("this is not a file".into());
    }
    if meta.len() > max {
        return Err(format!("the file is bigger than {} MB", max / 1_000_000));
    }
    let mut out = Vec::new();
    std::io::Read::read_to_end(&mut std::io::Read::take(std::fs::File::open(path).map_err(|e| e.to_string())?, max + 1), &mut out).map_err(|e| e.to_string())?;
    if out.len() as u64 > max {
        return Err(format!("the file is bigger than {} MB", max / 1_000_000));
    }
    Ok(out)
}

fn read_manifest(dir: &Path) -> Result<Manifest, String> {
    let p = dir.join(MANIFEST);
    let meta = std::fs::metadata(&p).map_err(|_| format!("no {MANIFEST}"))?;
    if meta.len() > MAX_MANIFEST {
        return Err("the manifest is too big".into());
    }
    let text = std::fs::read_to_string(&p).map_err(|e| e.to_string())?;
    let m = Manifest::parse(&text)?;
    // The folder keeps the id of the plugin, so a copy cannot take the place of another one.
    if dir.file_name().map(|n| n.to_string_lossy().to_string()).as_deref() != Some(m.id.as_str()) {
        return Err(format!("the folder must be named {:?}, like the id", m.id));
    }
    for rel in m.referenced_files() {
        if !dir.join(&rel).is_file() {
            return Err(format!("the file {rel} named in the manifest is missing"));
        }
    }
    Ok(m)
}
