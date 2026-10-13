//! The project file (`.cre`) shared by the programs built on baseCreator.
//!
//! One file holds the drawing (the `Scene`, common to every program), a free text, and one **section** per kind of
//! machine program (`laser`, `mill`, `pen`, ...). Each program reads and writes only its own section and hands the
//! others back untouched when it saves, so a file made for a laser can be opened and saved in the mill program (and
//! the other way round) without losing anything. A section is plain JSON with its own `format` number, owned by the
//! program that writes it; sections should hold what the job needs, never connection details or other secrets of a
//! computer.
use crate::scene::{Scene, Shape};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

/// The file name extension of a project, without the dot.
pub const EXTENSION: &str = "cre";
/// The version of the container written now.
pub const FORMAT: u32 = 1;
/// The biggest file read, in bytes.
pub const MAX_BYTES: usize = 256 * 1024 * 1024;

/// A project: the drawing, a description and the sections of the programs.
#[derive(Clone, Debug, Default)]
pub struct Project {
    pub scene: Scene,
    /// A free text about the project (what it is, the settings that worked...).
    pub description: String,
    sections: BTreeMap<String, Value>,
    /// Top-level entries this version does not know, kept for the next save.
    extra: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
struct SceneWire {
    shapes: Vec<Shape>,
    next_id: u64,
    #[serde(default)]
    next_group: u64,
}

/// Borrowed twin of `Wire` for saving without copying the shapes.
#[derive(Serialize)]
struct WireRef<'a> {
    format: u32,
    description: &'a str,
    scene: SceneRef<'a>,
    sections: &'a BTreeMap<String, Value>,
    #[serde(flatten)]
    extra: &'a BTreeMap<String, Value>,
}

#[derive(Serialize)]
struct SceneRef<'a> {
    shapes: &'a [Shape],
    next_id: u64,
    next_group: u64,
}

#[derive(Deserialize)]
struct Wire {
    format: u32,
    #[serde(default)]
    description: String,
    scene: SceneWire,
    #[serde(default)]
    sections: BTreeMap<String, Value>,
    #[serde(flatten)]
    extra: BTreeMap<String, Value>,
}

impl Project {
    pub fn new(scene: Scene) -> Project {
        Project { scene, ..Project::default() }
    }

    /// The text of the file.
    pub fn to_json(&self) -> String {
        let (next_id, next_group) = self.scene.counters();
        let wire = WireRef { format: FORMAT, description: &self.description, scene: SceneRef { shapes: &self.scene.shapes, next_id, next_group }, sections: &self.sections, extra: &self.extra };
        serde_json::to_string_pretty(&wire).unwrap_or_default()
    }

    /// Read the text of a file. A file from a newer container version is refused instead of half read.
    pub fn from_json(text: &str) -> Result<Project, String> {
        if text.len() > MAX_BYTES {
            return Err("the project file is too big".into());
        }
        let w: Wire = serde_json::from_str(text).map_err(|e| format!("this is not a project file: {e}"))?;
        if w.format == 0 || w.format > FORMAT {
            return Err(format!("the project file has format {}, this program reads up to {FORMAT}; update the program", w.format));
        }
        Ok(Project { scene: Scene::from_parts(w.scene.shapes, w.scene.next_id, w.scene.next_group), description: w.description, sections: w.sections, extra: w.extra })
    }

    /// The names of the sections in the file.
    pub fn section_names(&self) -> Vec<&str> {
        self.sections.keys().map(|s| s.as_str()).collect()
    }

    /// The section `name` as `T`: `Ok(None)` when the file has none, an error when it cannot be read.
    pub fn section<T: DeserializeOwned>(&self, name: &str) -> Result<Option<T>, String> {
        match self.sections.get(name) {
            None => Ok(None),
            Some(v) => serde_json::from_value(v.clone()).map(Some).map_err(|e| format!("the \"{name}\" part of the project cannot be read: {e}")),
        }
    }

    /// Put (or replace) the section `name`. The other sections stay as they are.
    pub fn set_section<T: Serialize>(&mut self, name: &str, value: &T) {
        if let Ok(v) = serde_json::to_value(value) {
            self.sections.insert(name.to_string(), v);
        }
    }

    /// Remove the section `name`.
    pub fn remove_section(&mut self, name: &str) {
        self.sections.remove(name);
    }

    /// Everything but the section `name`, to be kept by a program that holds only its own part in memory.
    pub fn other_sections(&self, name: &str) -> BTreeMap<String, Value> {
        self.sections.iter().filter(|(k, _)| k.as_str() != name).map(|(k, v)| (k.clone(), v.clone())).collect()
    }

    /// Put sections kept earlier (see `other_sections`) back, without touching the ones already here.
    pub fn restore_sections(&mut self, kept: &BTreeMap<String, Value>) {
        for (k, v) in kept {
            self.sections.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }

    /// The top-level entries this version does not know (kept for saving).
    pub fn extra(&self) -> &BTreeMap<String, Value> {
        &self.extra
    }

    /// Put top-level entries kept earlier back.
    pub fn restore_extra(&mut self, kept: &BTreeMap<String, Value>) {
        for (k, v) in kept {
            self.extra.entry(k.clone()).or_insert_with(|| v.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Xf;
    use crate::scene::Kind;

    fn sample() -> Project {
        let mut p = Project::default();
        p.scene.add(0, Kind::Rect { w: 10.0, h: 5.0 }, Xf::translate(1.0, 2.0));
        p.description = "a test".into();
        p.set_section("laser", &serde_json::json!({ "format": 1, "material": "Plywood" }));
        p.set_section("mill", &serde_json::json!({ "format": 3, "tools": [1, 2, 3] }));
        p
    }

    #[test]
    fn a_project_is_saved_again_unchanged() {
        let text = sample().to_json();
        let back = Project::from_json(&text).unwrap();
        assert_eq!(back.scene.shapes.len(), 1);
        assert_eq!(back.description, "a test");
        assert_eq!(back.to_json(), text);
    }

    #[test]
    fn the_sections_of_other_programs_survive_a_save() {
        let text = sample().to_json();
        let mut p = Project::from_json(&text).unwrap();
        // A laser program changes only its own section.
        p.set_section("laser", &serde_json::json!({ "format": 1, "material": "Acrylic" }));
        let again = Project::from_json(&p.to_json()).unwrap();
        assert_eq!(again.section::<Value>("mill").unwrap().unwrap()["format"], 3);
        assert_eq!(again.section::<Value>("laser").unwrap().unwrap()["material"], "Acrylic");
        assert_eq!(again.section_names(), vec!["laser", "mill"]);
        // A program that keeps only its own part in memory hands the rest back.
        let kept = again.other_sections("laser");
        let mut fresh = Project::new(again.scene.clone());
        fresh.set_section("laser", &serde_json::json!({ "format": 1 }));
        fresh.restore_sections(&kept);
        assert!(fresh.section::<Value>("mill").unwrap().is_some());
    }

    #[test]
    fn unknown_top_level_entries_are_kept() {
        let mut v: Value = serde_json::from_str(&sample().to_json()).unwrap();
        v["future"] = serde_json::json!({ "a": 1 });
        let p = Project::from_json(&v.to_string()).unwrap();
        let v2: Value = serde_json::from_str(&p.to_json()).unwrap();
        assert_eq!(v2["future"]["a"], 1);
    }

    #[test]
    fn nonsense_and_newer_files_are_errors() {
        assert!(Project::from_json("").is_err());
        assert!(Project::from_json("{}").is_err());
        assert!(Project::from_json("not json").is_err());
        let mut v: Value = serde_json::from_str(&sample().to_json()).unwrap();
        v["format"] = serde_json::json!(FORMAT + 1);
        assert!(Project::from_json(&v.to_string()).unwrap_err().contains("update the program"));
        v["format"] = serde_json::json!(0);
        assert!(Project::from_json(&v.to_string()).is_err());
    }

    #[test]
    fn a_section_that_does_not_fit_is_an_error_not_a_panic() {
        let p = sample();
        assert!(p.section::<Vec<u8>>("laser").is_err());
        assert!(p.section::<Value>("pen").unwrap().is_none());
    }
}
