//! The scene: shapes with their transforms, groups and ids. Knows nothing about layers, machines or jobs;
//! `Shape::layer` is just an index the application gives a meaning to.
use crate::bezier::Contour;
use crate::geom::*;
use crate::image::ImageData;
use crate::text::TextData;
use serde::{Deserialize, Serialize};

/// Default tolerance (millimetres) when curves are flattened to polylines.
pub const DEFAULT_FLATTEN_TOL: f64 = 0.05;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Kind {
    Rect { w: f64, h: f64 },
    Ellipse { w: f64, h: f64 },
    /// Pre-flattened polylines in local coordinates.
    Path(Vec<Polyline>),
    /// Editable Bézier contours in local coordinates.
    Bezier(Vec<Contour>),
    /// A bitmap engraved line by line. Local size is `w` x `h` millimetres.
    Image(ImageData),
    /// Live text, rendered to outlines on demand.
    Text(TextData),
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Shape {
    pub id: u64,
    pub layer: usize,
    pub kind: Kind,
    pub xf: Xf,
    /// Shapes with the same group id move and select together. Grouping never changes the layer.
    #[serde(default)]
    pub group: Option<u64>,
    /// A locked shape cannot be moved, edited or deleted; it can still be selected and copied.
    #[serde(default)]
    pub locked: bool,
}

impl Shape {
    pub fn local_polys(&self) -> Vec<Polyline> {
        self.local_polys_tol(DEFAULT_FLATTEN_TOL)
    }
    /// The outlines with curves flattened to within `tol` millimetres.
    pub fn local_polys_tol(&self, tol: f64) -> Vec<Polyline> {
        match &self.kind {
            Kind::Rect { w, h } => vec![Polyline::new(
                vec![Pt::new(0.0, 0.0), Pt::new(*w, 0.0), Pt::new(*w, *h), Pt::new(0.0, *h)],
                true,
            )],
            Kind::Ellipse { w, h } => {
                let n = 128;
                let pts = (0..n)
                    .map(|i| {
                        let t = i as f64 / n as f64 * std::f64::consts::TAU;
                        Pt::new(w / 2.0 + w / 2.0 * t.cos(), h / 2.0 + h / 2.0 * t.sin())
                    })
                    .collect();
                vec![Polyline::new(pts, true)]
            }
            Kind::Path(p) => p.clone(),
            Kind::Bezier(cs) => cs.iter().map(|c| c.flatten(tol)).collect(),
            Kind::Image(im) => vec![Polyline::new(vec![Pt::new(0.0, 0.0), Pt::new(im.w, 0.0), Pt::new(im.w, im.h), Pt::new(0.0, im.h)], true)],
            Kind::Text(t) => crate::text::contours(t).iter().map(|c| c.flatten(tol)).collect(),
        }
    }
    pub fn polys(&self) -> Vec<Polyline> {
        self.polys_tol(DEFAULT_FLATTEN_TOL)
    }
    pub fn polys_tol(&self, tol: f64) -> Vec<Polyline> {
        self.local_polys_tol(tol).iter().map(|p| p.transformed(&self.xf)).collect()
    }
    pub fn bounds(&self) -> Option<Rect> {
        self.polys().iter().filter_map(|p| p.bounds()).reduce(Rect::union)
    }
    /// Does the shape touch the rectangle (crossing selection)?
    pub fn intersects_rect(&self, r: &Rect) -> bool {
        self.polys().iter().any(|p| p.intersects_rect(r))
    }
    /// Distance from a point to this shape (0 if inside a closed shape).
    pub fn hit_dist(&self, p: Pt) -> f64 {
        let polys = self.polys();
        let inside = polys.iter().filter(|q| q.closed).fold(false, |acc, q| acc ^ q.contains(p));
        if inside {
            return 0.0;
        }
        polys.iter().map(|q| q.dist_to(p)).fold(f64::INFINITY, f64::min)
    }
    /// Contours of this shape in local coordinates, as editable Bézier data.
    pub fn local_contours(&self) -> Vec<Contour> {
        match &self.kind {
            Kind::Rect { w, h } => vec![Contour::rect(*w, *h)],
            Kind::Ellipse { w, h } => vec![Contour::ellipse(*w, *h)],
            Kind::Path(p) => p.iter().map(Contour::from_polyline).collect(),
            Kind::Bezier(c) => c.clone(),
            Kind::Image(im) => vec![Contour::rect(im.w, im.h)],
            Kind::Text(t) => crate::text::contours(t).as_ref().clone(),
        }
    }
    pub fn is_image(&self) -> bool {
        matches!(self.kind, Kind::Image(_))
    }
    /// Convert into editable Bézier contours, baking the transform into the nodes.
    pub fn to_bezier(&mut self) {
        let xf = self.xf;
        let cs: Vec<Contour> = self.local_contours().iter().map(|c| c.transformed(&xf)).collect();
        self.kind = Kind::Bezier(cs);
        self.xf = Xf::IDENTITY;
    }
    /// Convert any shape into a generic path (baking the transform in).
    pub fn bake(&mut self) {
        if self.is_image() {
            return;
        }
        let polys = self.polys();
        self.kind = Kind::Path(polys);
        self.xf = Xf::IDENTITY;
    }
}

/// All shapes of a drawing and the counters that give new shapes and groups their ids.
#[derive(Clone, Debug)]
pub struct Scene {
    pub shapes: Vec<Shape>,
    next_id: u64,
    next_group: u64,
}

impl Default for Scene {
    fn default() -> Self {
        Scene { shapes: vec![], next_id: 1, next_group: 1 }
    }
}

impl Scene {
    /// A scene from saved parts. A group counter of 0 (old files) is read as 1.
    pub fn from_parts(shapes: Vec<Shape>, next_id: u64, next_group: u64) -> Scene {
        Scene { shapes, next_id, next_group }
    }
    /// The counters as saved: next shape id, next group id.
    pub fn counters(&self) -> (u64, u64) {
        (self.next_id, self.next_group)
    }
    pub fn add(&mut self, layer: usize, kind: Kind, xf: Xf) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        self.shapes.push(Shape { id, layer, kind, xf, group: None, locked: false });
        id
    }
    pub fn add_shape(&mut self, mut s: Shape) -> u64 {
        s.id = self.next_id;
        self.next_id += 1;
        let id = s.id;
        self.shapes.push(s);
        id
    }
    pub fn new_group_id(&mut self) -> u64 {
        self.next_group = self.next_group.max(1);
        let g = self.next_group;
        self.next_group += 1;
        g
    }
    /// All shape ids in the same group as `id` (just `id` when ungrouped).
    pub fn group_of(&self, id: u64) -> Vec<u64> {
        match self.shape(id).and_then(|s| s.group) {
            Some(g) => self.shapes.iter().filter(|s| s.group == Some(g)).map(|s| s.id).collect(),
            None => vec![id],
        }
    }
    pub fn shape(&self, id: u64) -> Option<&Shape> {
        self.shapes.iter().find(|s| s.id == id)
    }
    pub fn shape_mut(&mut self, id: u64) -> Option<&mut Shape> {
        self.shapes.iter_mut().find(|s| s.id == id)
    }
    /// Mutable access for editing: `None` when the shape does not exist or is locked.
    pub fn unlocked_mut(&mut self, id: u64) -> Option<&mut Shape> {
        self.shape_mut(id).filter(|s| !s.locked)
    }
    /// The ids in `ids` that may be edited.
    pub fn unlocked_ids(&self, ids: &[u64]) -> Vec<u64> {
        ids.iter().copied().filter(|id| self.shape(*id).is_some_and(|s| !s.locked)).collect()
    }
    pub fn bounds_of(&self, ids: &[u64]) -> Option<Rect> {
        ids.iter().filter_map(|i| self.shape(*i)).filter_map(|s| s.bounds()).reduce(Rect::union)
    }
}
