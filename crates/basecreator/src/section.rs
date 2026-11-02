//! The part of the project file that belongs to BaseCreator: the page and the layers (a name, a colour, shown or not). Nothing of a
//! machine. Other programs keep it untouched, and BaseCreator keeps theirs.
pub use base_core::layers::LAYER_COUNT;
use serde::{Deserialize, Serialize};

pub const SECTION: &str = "draw";
/// The version of this part. A project with a higher one is refused.
pub const SECTION_FORMAT: u32 = 1;

/// The usual sheets, in mm with the shorter side first.
pub const SHEETS: [(&str, f64, f64); 7] = [("A5", 148.0, 210.0), ("A4", 210.0, 297.0), ("A3", 297.0, 420.0), ("A2", 420.0, 594.0), ("A1", 594.0, 841.0), ("Letter", 215.9, 279.4), ("Legal", 215.9, 355.6)];
/// The sizes a page can have, in mm.
pub const PAGE_MIN: f64 = 10.0;
pub const PAGE_MAX: f64 = 10000.0;

/// The page of a drawing, in mm.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Page {
    pub w: f64,
    pub h: f64,
}

impl Default for Page {
    /// An A4 sheet lying on its long side.
    fn default() -> Page {
        Page { w: 297.0, h: 210.0 }
    }
}

impl Page {
    pub fn landscape(self) -> bool {
        self.w >= self.h
    }

    /// The page turned so that it is `landscape` or not; the sizes stay.
    pub fn turned(self, landscape: bool) -> Page {
        if self.landscape() == landscape {
            self
        } else {
            Page { w: self.h, h: self.w }
        }
    }

    /// Sizes that are not numbers, or too small or too big, are made sensible.
    pub fn sane(self) -> Page {
        let fix = |v: f64, d: f64| if v.is_finite() { v.clamp(PAGE_MIN, PAGE_MAX) } else { d };
        Page { w: fix(self.w, Page::default().w), h: fix(self.h, Page::default().h) }
    }

    /// The page made bigger, when needed, so that it holds a drawing that reaches to `x`, `y`. It stays as it is otherwise.
    pub fn holding(self, x: f64, y: f64) -> Page {
        Page { w: self.w.max(x.ceil()), h: self.h.max(y.ceil()) }.sane()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DrawLayer {
    pub name: String,
    pub color: [u8; 3],
    /// Shown in the work area.
    pub visible: bool,
}

impl DrawLayer {
    pub fn color32(&self) -> egui::Color32 {
        egui::Color32::from_rgb(self.color[0], self.color[1], self.color[2])
    }
}

pub fn default_layers() -> Vec<DrawLayer> {
    (0..LAYER_COUNT).map(|i| DrawLayer { name: format!("Layer {}", i + 1), color: base_core::layers::color(i), visible: true }).collect()
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DrawSection {
    pub format: u32,
    pub page: Page,
    pub layers: Vec<DrawLayer>,
}

/// What a project brings in. `page` is `None` for a project without this part (a project of another program): it knows nothing of a page.
pub struct Loaded {
    pub layers: Vec<DrawLayer>,
    pub page: Option<Page>,
}

/// Read the section of a project. Missing layers get the defaults, more than there can be are left out.
pub fn load(p: &base_core::project::Project) -> Result<Loaded, String> {
    let Some(s) = p.section::<DrawSection>(SECTION)? else {
        return Ok(Loaded { layers: default_layers(), page: None });
    };
    if s.format > SECTION_FORMAT {
        return Err(format!("the drawing part of the project has format {}, this program reads up to {SECTION_FORMAT}; update the program", s.format));
    }
    let defaults = default_layers();
    let mut layers: Vec<DrawLayer> = s.layers.into_iter().take(LAYER_COUNT).collect();
    while layers.len() < LAYER_COUNT {
        layers.push(defaults[layers.len()].clone());
    }
    Ok(Loaded { layers, page: Some(s.page.sane()) })
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::project::Project;
    use base_core::scene::Scene;

    #[test]
    fn the_section_goes_into_the_project_and_comes_back() {
        let mut layers = default_layers();
        layers[1].name = "Red lines".into();
        layers[1].visible = false;
        let mut p = Project::new(Scene::default());
        p.set_section(SECTION, &DrawSection { format: SECTION_FORMAT, page: Page { w: 100.0, h: 150.0 }, layers });
        let back = load(&Project::from_json(&p.to_json()).unwrap()).unwrap();
        assert_eq!((back.layers[1].name.as_str(), back.layers[1].visible), ("Red lines", false));
        assert_eq!(back.page, Some(Page { w: 100.0, h: 150.0 }));
    }

    #[test]
    fn a_project_of_another_program_gets_the_default_layers_and_a_newer_one_is_refused() {
        let l = load(&Project::new(Scene::default())).unwrap();
        assert_eq!(l.layers.len(), LAYER_COUNT);
        assert!(l.page.is_none());
        let mut newer = Project::new(Scene::default());
        newer.set_section(SECTION, &serde_json::json!({ "format": 99, "page": { "w": 100, "h": 100 }, "layers": [] }));
        assert!(load(&newer).is_err());
    }

    #[test]
    fn missing_layers_are_added_and_a_page_with_wrong_sizes_is_made_sensible() {
        let mut p = Project::new(Scene::default());
        p.set_section(SECTION, &DrawSection { format: SECTION_FORMAT, page: Page { w: 1.0, h: 1e9 }, layers: default_layers()[..2].to_vec() });
        let l = load(&Project::from_json(&p.to_json()).unwrap()).unwrap();
        assert_eq!(l.layers.len(), LAYER_COUNT);
        assert_eq!(l.page, Some(Page { w: PAGE_MIN, h: PAGE_MAX }));
    }

    #[test]
    fn a_page_turns_and_grows_only_when_it_has_to() {
        let p = Page { w: 210.0, h: 297.0 };
        assert!(!p.landscape());
        assert_eq!(p.turned(true), Page { w: 297.0, h: 210.0 });
        assert_eq!(p.turned(false), p);
        assert_eq!(p.holding(100.0, 100.0), p);
        assert_eq!(p.holding(400.2, 100.0), Page { w: 401.0, h: 297.0 });
    }
}
