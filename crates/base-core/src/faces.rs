//! Jobs on more than one face of one object, the same in every program: a sheet worked on both sides (side A on top, side B under
//! it after it is turned over), a post worked on its four long faces (it lies along X and is rolled a quarter turn back between
//! them) and a block worked on all six faces.
//!
//! Every face is drawn as it lies on the machine when it is the one worked on: its own work area (its width and height), the
//! material under it as thick as the object is deep there. The object has its own space: X from left to right, Y from the back to
//! the front (as the drawing goes down the page) and Z up, with the object in the box 0..X, 0..Y, 0..Z as it lies for face A. A
//! face knows where it lies in that box ([`Face`]), and from that every program gets the ghosts (the other faces where they lie
//! beside or under the shown one), the words of the G-code (which face, how to turn the object to it) and the pictures in 3D.
use crate::geom::{Pt, Xf};
use crate::project::Project;
use crate::scene::{Scene, Shape};
use crate::setup::{FlipAxis, TwoSided};
use serde::{Deserialize, Serialize};

/// A point or a direction in the space of the object.
pub type V3 = [f64; 3];

/// What is worked on from more than one side.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Body {
    /// A sheet turned over round one of its middle lines: two sides.
    Sheet(FlipAxis),
    /// A post with a rectangular base, lying along X: its four long faces.
    Post,
    /// A block: its six faces.
    Block,
}

/// The object and its size.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Solid {
    pub body: Body,
    /// A sheet: width, height, thickness. A post: the sides of its base `a` and `b`, and its height `H`. A block: `a`, `b`, `c`
    /// (from left to right, from the back to the front, up). Millimetres.
    pub size: [f64; 3],
}

/// Where a face lies on the object (as it lies for face A).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Place {
    Top,
    /// The front, after a quarter turn back.
    Front,
    /// The bottom reached by turning end to end (or by two quarter turns back).
    Bottom,
    /// The back, after three quarter turns back.
    Back,
    /// The left end of a block, turned up with the top to the right.
    Left,
    /// The right end of a block, turned up with the top to the left.
    Right,
    /// The bottom of a sheet turned over side to side.
    BottomSideToSide,
}

impl Place {
    /// Its name (in English: a key of the translations and the words of the G-code).
    pub fn name(self) -> &'static str {
        match self {
            Place::Top => "top",
            Place::Front => "front",
            Place::Bottom | Place::BottomSideToSide => "bottom",
            Place::Back => "back",
            Place::Left => "left end",
            Place::Right => "right end",
        }
    }
}

/// A face of the object: its size on the work area, how deep the object is under it, and where it lies in the space of the object.
/// A point (x, y) of its drawing lies at `o + x·u + y·v`; `out` points out of the object.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Face {
    pub place: Place,
    pub w: f64,
    pub h: f64,
    pub depth: f64,
    pub o: V3,
    pub u: V3,
    pub v: V3,
    pub out: V3,
}

fn add(a: V3, b: V3) -> V3 {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn sub(a: V3, b: V3) -> V3 {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn mul(a: V3, k: f64) -> V3 {
    [a[0] * k, a[1] * k, a[2] * k]
}
/// The dot product.
pub fn dot(a: V3, b: V3) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
/// The cross product.
pub fn cross(a: V3, b: V3) -> V3 {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

impl Face {
    /// The point of the object under (x, y) of the drawing of this face, `depth` mm in from the face.
    pub fn at(&self, p: Pt, depth: f64) -> V3 {
        add(add(add(self.o, mul(self.u, p.x)), mul(self.v, p.y)), mul(self.out, -depth))
    }
    /// A point of the object as this face sees it: (x, y) of its drawing and how deep it is under the face.
    pub fn local(&self, q: V3) -> (Pt, f64) {
        let d = sub(q, self.o);
        (Pt::new(dot(d, self.u), dot(d, self.v)), -dot(d, self.out))
    }
    /// The four corners of the face in its drawing, from the top left round.
    pub fn corners(&self) -> [Pt; 4] {
        [Pt::new(0.0, 0.0), Pt::new(self.w, 0.0), Pt::new(self.w, self.h), Pt::new(0.0, self.h)]
    }
}

/// How another face lies to the shown one.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relation {
    Same,
    /// On the other side of the object: seen through it.
    Opposite,
    /// Beside the shown face, over its edge: left, right, at the back (the top of the drawing) or at the front (its bottom).
    Left,
    Right,
    Back,
    Front,
}

impl Solid {
    pub fn sheet(w: f64, h: f64, t: f64, flip: FlipAxis) -> Solid {
        Solid { body: Body::Sheet(flip), size: [w, h, t] }
    }
    pub fn post(a: f64, b: f64, h: f64) -> Solid {
        Solid { body: Body::Post, size: [a, b, h] }
    }
    pub fn block(a: f64, b: f64, c: f64) -> Solid {
        Solid { body: Body::Block, size: [a, b, c] }
    }

    /// The box of the object as it lies for face A: X, Y (from the back to the front) and Z.
    pub fn bbox(&self) -> V3 {
        let [p, q, r] = self.size.map(|v| v.max(0.01));
        match self.body {
            Body::Sheet(_) | Body::Block => [p, q, r],
            // The post lies along X: its height from left to right, the side `a` of its base from the back to the front, `b` up.
            Body::Post => [r, p, q],
        }
    }

    pub fn count(&self) -> usize {
        match self.body {
            Body::Sheet(_) => 2,
            Body::Post => 4,
            Body::Block => 6,
        }
    }

    /// The places of the faces, A first.
    pub fn places(&self) -> Vec<Place> {
        match self.body {
            Body::Sheet(FlipAxis::SideToSide) => vec![Place::Top, Place::BottomSideToSide],
            Body::Sheet(FlipAxis::EndToEnd) => vec![Place::Top, Place::Bottom],
            Body::Post => vec![Place::Top, Place::Front, Place::Bottom, Place::Back],
            Body::Block => vec![Place::Top, Place::Front, Place::Bottom, Place::Back, Place::Left, Place::Right],
        }
    }

    /// Face `i` (A is 0); the last face for a bigger `i`.
    pub fn face(&self, i: usize) -> Face {
        let places = self.places();
        let place = places[i.min(places.len() - 1)];
        let [x, y, z] = self.bbox();
        let (o, u, v, out, w, h, depth) = match place {
            Place::Top => ([0.0, 0.0, z], [1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0], x, y, z),
            Place::Front => ([0.0, y, z], [1.0, 0.0, 0.0], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0], x, z, y),
            Place::Bottom => ([0.0, y, 0.0], [1.0, 0.0, 0.0], [0.0, -1.0, 0.0], [0.0, 0.0, -1.0], x, y, z),
            Place::Back => ([0.0, 0.0, 0.0], [1.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, -1.0, 0.0], x, z, y),
            Place::Left => ([0.0, 0.0, 0.0], [0.0, 0.0, 1.0], [0.0, 1.0, 0.0], [-1.0, 0.0, 0.0], z, y, x),
            Place::Right => ([x, 0.0, z], [0.0, 0.0, -1.0], [0.0, 1.0, 0.0], [1.0, 0.0, 0.0], z, y, x),
            Place::BottomSideToSide => ([x, 0.0, 0.0], [-1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, -1.0], x, y, z),
        };
        Face { place, w, h, depth, o, u, v, out }
    }

    /// The object with face `i` made `w` × `h` and the object `depth` deep under it (the other faces follow: they share its edges).
    /// A sheet changes its width, height and thickness.
    pub fn with_face_size(&self, i: usize, w: f64, h: f64, depth: f64) -> Solid {
        let f = self.face(i);
        let mut b = self.bbox();
        let axis = |d: V3| (0..3).max_by(|x, y| d[*x].abs().total_cmp(&d[*y].abs())).unwrap_or(0);
        b[axis(f.u)] = w;
        b[axis(f.v)] = h;
        b[axis(f.out)] = depth;
        let size = match self.body {
            Body::Sheet(_) | Body::Block => b,
            Body::Post => [b[1], b[2], b[0]],
        };
        Solid { body: self.body, size }
    }

    pub fn faces(&self) -> Vec<Face> {
        (0..self.count()).map(|i| self.face(i)).collect()
    }

    /// The letter of face `i`: A, B, C...
    pub fn letter(i: usize) -> char {
        (b'A' + i.min(25) as u8) as char
    }

    /// How face `other` lies to face `shown`.
    pub fn relation(&self, other: usize, shown: usize) -> Relation {
        let (f, s) = (self.face(other), self.face(shown));
        let k = dot(f.out, s.out);
        if k > 0.5 {
            Relation::Same
        } else if k < -0.5 {
            Relation::Opposite
        } else if dot(f.out, s.u) < -0.5 {
            Relation::Left
        } else if dot(f.out, s.u) > 0.5 {
            Relation::Right
        } else if dot(f.out, s.v) < -0.5 {
            Relation::Back
        } else {
            Relation::Front
        }
    }

    /// Where a point of the drawing of face `other` shows on the drawing of face `shown`: a face on the other side of the object
    /// is seen through it (where the point lies under the shown face); a face beside it is unfolded over the shared edge, as on
    /// the net of a box. The same face stays as it is.
    pub fn show_on(&self, other: usize, shown: usize, p: Pt) -> Pt {
        let (f, s) = (self.face(other), self.face(shown));
        let (q, depth) = s.local(f.at(p, 0.0));
        match self.relation(other, shown) {
            Relation::Same | Relation::Opposite => q,
            Relation::Left => Pt::new(-depth, q.y),
            Relation::Right => Pt::new(s.w + depth, q.y),
            Relation::Back => Pt::new(q.x, -depth),
            Relation::Front => Pt::new(q.x, s.h + depth),
        }
    }

    /// [`Solid::show_on`] as a transform of the drawing.
    pub fn ghost_xf(&self, other: usize, shown: usize) -> Xf {
        let p0 = self.show_on(other, shown, Pt::new(0.0, 0.0));
        let p1 = self.show_on(other, shown, Pt::new(1.0, 0.0));
        let p2 = self.show_on(other, shown, Pt::new(0.0, 1.0));
        Xf { a: p1.x - p0.x, b: p1.y - p0.y, c: p2.x - p0.x, d: p2.y - p0.y, e: p0.x, f: p0.y }
    }

    /// The shapes of face `other` where they show on face `shown` (see [`Solid::show_on`]).
    pub fn ghosts_of(&self, scene: &Scene, other: usize, shown: usize) -> Vec<Shape> {
        let t = self.ghost_xf(other, shown);
        scene.shapes.iter().map(|s| Shape { xf: s.xf.then(t), ..s.clone() }).collect()
    }

    /// The outline of face `other` unfolded beside face `shown` (its corners on the drawing of the shown face), for the faces that
    /// are beside it.
    pub fn neighbour(&self, other: usize, shown: usize) -> Option<[Pt; 4]> {
        match self.relation(other, shown) {
            Relation::Same | Relation::Opposite => None,
            _ => Some(self.face(other).corners().map(|c| self.show_on(other, shown, c))),
        }
    }

    /// The face unfolded beside face `shown` at `p` (millimetres of its drawing, from its top left corner), if there is one.
    pub fn face_at(&self, shown: usize, p: Pt) -> Option<usize> {
        (0..self.count()).find(|j| {
            self.neighbour(*j, shown).is_some_and(|c| {
                let (lx, hx) = (c.iter().map(|q| q.x).fold(f64::MAX, f64::min), c.iter().map(|q| q.x).fold(f64::MIN, f64::max));
                let (ly, hy) = (c.iter().map(|q| q.y).fold(f64::MAX, f64::min), c.iter().map(|q| q.y).fold(f64::MIN, f64::max));
                p.x >= lx && p.x <= hx && p.y >= ly && p.y <= hy
            })
        })
    }

    /// The comment at the head of the G-code of face `i` (ASCII only: GRBL takes bytes above 127 for commands of its own).
    pub fn head_comment(&self, i: usize) -> String {
        let f = self.face(i);
        match self.body {
            Body::Sheet(flip) => {
                if i == 0 {
                    "Side A".to_string()
                } else {
                    format!("Side B: the material is turned over {}", if flip.is_side_to_side() { "side to side" } else { "end to end" })
                }
            }
            Body::Post | Body::Block => {
                let what = if self.body == Body::Post { "post" } else { "block" };
                format!("Face {} of {}, {} of the {what}: {} x {} mm, {} mm high", Solid::letter(i), self.count(), f.place.name(), mm(f.w), mm(f.h), mm(f.depth))
            }
        }
    }

    /// How to lay the object for face `i` (ASCII, short enough for a line of G-code), the comment of the stop before it.
    pub fn turn_comment(&self, i: usize) -> String {
        let f = self.face(i);
        let l = Solid::letter(i);
        match (self.body, f.place) {
            (Body::Sheet(flip), _) => format!("Turn the material over {} and resume to do side B", if flip.is_side_to_side() { "side to side" } else { "end to end" }),
            (_, Place::Front | Place::Bottom | Place::Back) => format!("Roll it a quarter turn back: face {l} up, face {} at the back", Solid::letter(i - 1)),
            (_, Place::Left) => format!("Lay it with face {l} up, face A to the right, its back at the back"),
            (_, Place::Right) => format!("Lay it with face {l} up, face A to the left, its back at the back"),
            _ => format!("Lay it with face {l} up"),
        }
    }
}

/// A number of millimetres in the words of the G-code: whole, or with up to two decimals.
fn mm(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// The name of the project section of a post or a block (a sheet worked on two sides keeps `setup::SECTION`).
pub const SECTION: &str = "faces";

/// The drawing of one face in the project file.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FaceDrawing {
    #[serde(default)]
    shapes: Vec<Shape>,
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    next_group: u64,
}

impl FaceDrawing {
    pub fn of(scene: &Scene) -> FaceDrawing {
        let (next_id, next_group) = scene.counters();
        FaceDrawing { shapes: scene.shapes.clone(), next_id, next_group }
    }
    pub fn scene(&self) -> Scene {
        Scene::from_parts(self.shapes.clone(), self.next_id.max(1), self.next_group.max(1))
    }
}

/// The section of a post or a block: the object, whether the ghosts are shown, and the drawings of the faces after A (A is the
/// drawing of the project).
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct FacesSection {
    pub solid: Solid,
    #[serde(default = "yes")]
    pub ghosts: bool,
    #[serde(default)]
    pub faces: Vec<FaceDrawing>,
}

fn yes() -> bool {
    true
}

/// A job on more than one face as a program keeps it: the object, the ghosts, the drawing of every face (A first) and the face that
/// is shown.
#[derive(Clone, Debug)]
pub struct Faces {
    pub solid: Solid,
    pub ghosts: bool,
    pub scenes: Vec<Scene>,
    /// A sheet with alignment marks round it, burned as a job of their own.
    pub marks: bool,
}

impl Faces {
    pub fn new(solid: Solid) -> Faces {
        Faces { solid, ghosts: true, scenes: vec![Scene::default(); solid.count()], marks: false }
    }

    /// Put the faces into a project that is saved: face A is its drawing; side B of a sheet goes into the section of a two-sided job
    /// (as before there were posts and blocks), the faces of a post or a block into their own section.
    pub fn write(&self, p: &mut Project) {
        p.remove_section(crate::setup::SECTION);
        p.remove_section(SECTION);
        if let Some(a) = self.scenes.first() {
            p.scene = a.clone();
        }
        match self.solid.body {
            Body::Sheet(flip) => {
                let empty = Scene::default();
                let mut t = TwoSided::with_side_b(flip, self.ghosts, self.scenes.get(1).unwrap_or(&empty));
                t.piece = Some(self.solid.size);
                t.marks = self.marks;
                p.set_section(crate::setup::SECTION, &t);
            }
            Body::Post | Body::Block => {
                p.set_section(SECTION, &FacesSection { solid: self.solid, ghosts: self.ghosts, faces: self.scenes.iter().skip(1).map(FaceDrawing::of).collect() });
            }
        }
    }

    /// The faces of a project that was read, and the sections taken out of it. A two-sided job takes the size of the sheet from
    /// `sheet` (what the program keeps of its material) when its section has none.
    pub fn read(p: &mut Project, sheet: [f64; 3]) -> Option<Faces> {
        let faces = p.section::<FacesSection>(SECTION).ok().flatten();
        let two = p.section::<TwoSided>(crate::setup::SECTION).ok().flatten();
        p.remove_section(SECTION);
        p.remove_section(crate::setup::SECTION);
        if let Some(f) = faces {
            let mut scenes = vec![p.scene.clone()];
            scenes.extend(f.faces.iter().map(FaceDrawing::scene));
            scenes.resize(f.solid.count(), Scene::default());
            return Some(Faces { solid: f.solid, ghosts: f.ghosts, scenes, marks: false });
        }
        let t = two?;
        let size = t.piece.filter(|s| s.iter().all(|v| v.is_finite() && *v > 0.0)).unwrap_or(sheet);
        Some(Faces { solid: Solid::sheet(size[0], size[1], size[2], t.flip), ghosts: t.ghosts, scenes: vec![p.scene.clone(), t.side_b()], marks: t.marks })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Kind;

    fn all() -> Vec<Solid> {
        vec![Solid::sheet(300.0, 200.0, 18.0, FlipAxis::SideToSide), Solid::sheet(300.0, 200.0, 18.0, FlipAxis::EndToEnd), Solid::post(40.0, 30.0, 250.0), Solid::block(120.0, 80.0, 50.0)]
    }

    fn close(a: Pt, b: Pt) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    /// Every face is drawn as it is seen from outside (none is a mirror image), its corners are corners of the box, and the face on
    /// the other side of it is as big and the object as deep under it.
    #[test]
    fn the_faces_lie_on_the_box_seen_from_outside() {
        for s in all() {
            let b = s.bbox();
            for (i, f) in s.faces().iter().enumerate() {
                assert_eq!(cross(f.u, f.v), f.out, "{s:?} face {i} is not a mirror image");
                for c in f.corners() {
                    let q = f.at(c, 0.0);
                    for k in 0..3 {
                        assert!(q[k].abs() < 1e-9 || (q[k] - b[k]).abs() < 1e-9, "{s:?} face {i}: corner {q:?} is not on the box {b:?}");
                    }
                    // The far side of the object under the face is on the box too.
                    let far = f.at(c, f.depth);
                    assert!((0..3).all(|k| far[k] > -1e-9 && far[k] < b[k] + 1e-9));
                }
                // Back and forth.
                let p = Pt::new(f.w * 0.3, f.h * 0.6);
                let (back, d) = f.local(f.at(p, 2.5));
                assert!(close(back, p) && (d - 2.5).abs() < 1e-9);
                for j in 0..s.count() {
                    if s.relation(j, i) == Relation::Opposite {
                        let g = s.face(j);
                        assert!((g.w - f.w).abs() < 1e-9 && (g.h - f.h).abs() < 1e-9 && (g.depth - f.depth).abs() < 1e-9);
                    }
                }
            }
        }
    }

    /// Side B of a sheet lies under side A as the material is turned over (`setup::turn`), both ways round.
    #[test]
    fn a_sheet_turns_over_as_before() {
        for flip in FlipAxis::ALL {
            let s = Solid::sheet(300.0, 200.0, 18.0, flip);
            let t = crate::setup::turn(flip, 300.0, 200.0);
            for p in [Pt::new(20.0, 30.0), Pt::new(0.0, 0.0), Pt::new(300.0, 17.0)] {
                assert!(close(s.show_on(1, 0, p), t.apply(p)) && close(s.show_on(0, 1, p), t.apply(p)), "{flip:?}");
                assert!(close(s.ghost_xf(1, 0).apply(p), t.apply(p)));
            }
            assert_eq!(s.relation(1, 0), Relation::Opposite);
        }
    }

    /// The faces of a post: a quarter turn back brings the front up, with face A at its back; the faces beside the shown one unfold
    /// over its edges, the one on the other side is seen through.
    #[test]
    fn the_faces_of_a_post_and_their_ghosts() {
        let s = Solid::post(40.0, 30.0, 250.0);
        let sizes: Vec<(f64, f64, f64)> = s.faces().iter().map(|f| (f.w, f.h, f.depth)).collect();
        assert_eq!(sizes, vec![(250.0, 40.0, 30.0), (250.0, 30.0, 40.0), (250.0, 40.0, 30.0), (250.0, 30.0, 40.0)]);
        assert_eq!((s.relation(1, 0), s.relation(3, 0), s.relation(2, 0)), (Relation::Front, Relation::Back, Relation::Opposite));
        assert_eq!(s.relation(0, 1), Relation::Back, "on face B, face A is at the back");
        // The edge face B shares with face A: on B it is at the top of the drawing; on A it is the bottom.
        assert!(close(s.show_on(1, 0, Pt::new(10.0, 0.0)), Pt::new(10.0, 40.0)));
        // 5 mm down face B is 5 mm beyond the front edge of A, unfolded.
        assert!(close(s.show_on(1, 0, Pt::new(10.0, 5.0)), Pt::new(10.0, 45.0)));
        // Face C under A: a point at the back left of C (after two turns) is at the front left of A.
        assert!(close(s.show_on(2, 0, Pt::new(0.0, 0.0)), Pt::new(0.0, 40.0)));
        // The outline of face D unfolded above A.
        let d = s.neighbour(3, 0).unwrap();
        let (lo, hi) = d.iter().fold((f64::MAX, f64::MIN), |(lo, hi), p| (lo.min(p.y), hi.max(p.y)));
        assert!((lo + 30.0).abs() < 1e-9 && hi.abs() < 1e-9, "{d:?}");
        assert!(s.neighbour(2, 0).is_none());
        // A click above face A is on face D, below it on face B, on A itself on none of those beside it.
        assert_eq!((s.face_at(0, Pt::new(100.0, -10.0)), s.face_at(0, Pt::new(100.0, 50.0)), s.face_at(0, Pt::new(100.0, 20.0))), (Some(3), Some(1), None));
        // The ghosts are the shapes moved there.
        let mut sc = Scene::default();
        sc.add(0, Kind::Rect { w: 10.0, h: 5.0 }, Xf::translate(20.0, 2.0));
        let g = s.ghosts_of(&sc, 1, 0);
        let b = g[0].bounds().unwrap();
        assert!((b.min.y - 42.0).abs() < 1e-9 && (b.max.y - 47.0).abs() < 1e-9, "{b:?}");
    }

    #[test]
    fn a_face_made_bigger_changes_the_object() {
        let p = Solid::post(40.0, 30.0, 250.0);
        // Face B is H × b with a under it.
        assert_eq!(p.with_face_size(1, 260.0, 35.0, 45.0), Solid::post(45.0, 35.0, 260.0));
        let b = Solid::block(120.0, 80.0, 50.0);
        assert_eq!(b.with_face_size(4, 55.0, 85.0, 125.0), Solid::block(125.0, 85.0, 55.0));
        for s in all() {
            for i in 0..s.count() {
                let f = s.face(i);
                assert_eq!(s.with_face_size(i, f.w, f.h, f.depth), s, "{s:?} {i}");
            }
        }
    }

    #[test]
    fn the_ends_of_a_block_unfold_left_and_right() {
        let s = Solid::block(120.0, 80.0, 50.0);
        assert_eq!((s.relation(4, 0), s.relation(5, 0), s.relation(4, 5)), (Relation::Left, Relation::Right, Relation::Opposite));
        assert_eq!((s.relation(0, 4), s.relation(0, 5)), (Relation::Right, Relation::Left), "face A is to the right of E and to the left of F");
        let e = s.face(4);
        assert_eq!((e.w, e.h, e.depth), (50.0, 80.0, 120.0));
        // The top right corner of face E (the top of the block at the back) unfolds onto the left edge of A.
        assert!(close(s.show_on(4, 0, Pt::new(50.0, 0.0)), Pt::new(0.0, 0.0)));
        assert!(close(s.show_on(4, 0, Pt::new(40.0, 10.0)), Pt::new(-10.0, 10.0)));
        assert!(close(s.show_on(5, 0, Pt::new(10.0, 10.0)), Pt::new(130.0, 10.0)));
        // Every pair of faces of a block lies one way or the other.
        for i in 0..6 {
            let n = (0..6).filter(|j| s.relation(*j, i) == Relation::Opposite).count();
            assert_eq!(n, 1);
            assert_eq!((0..6).filter(|j| s.neighbour(*j, i).is_some()).count(), 4);
        }
    }

    #[test]
    fn the_words_of_the_g_code_are_ascii_and_name_the_face() {
        for s in all() {
            for i in 0..s.count() {
                assert!(s.head_comment(i).is_ascii() && s.turn_comment(i).is_ascii());
                // A line of G-code is short: M0 and the brackets round it.
                assert!(s.head_comment(i).len() <= 72 && s.turn_comment(i).len() <= 72, "{}", s.turn_comment(i));
            }
        }
        let p = Solid::post(40.0, 30.0, 250.0);
        assert_eq!(p.head_comment(1), "Face B of 4, front of the post: 250 x 30 mm, 40 mm high");
        // No brackets: a comment of G-code is in brackets itself.
        assert!((0..6).all(|i| !Solid::block(1.0, 2.0, 3.0).head_comment(i).contains(['(', ')']) && !Solid::block(1.0, 2.0, 3.0).turn_comment(i).contains(['(', ')'])));
        assert!(p.turn_comment(2).starts_with("Roll it a quarter turn back: face C up, face B at the back"));
        assert_eq!(Solid::sheet(1.0, 1.0, 1.0, FlipAxis::SideToSide).head_comment(1), "Side B: the material is turned over side to side");
    }

    /// A post or a block is saved in its own section, a sheet as a two-sided job; both come back with every drawing.
    #[test]
    fn the_faces_are_saved_and_read_back() {
        for s in all() {
            let mut f = Faces::new(s);
            for (i, sc) in f.scenes.iter_mut().enumerate() {
                for _ in 0..=i {
                    sc.add(0, Kind::Rect { w: 5.0, h: 5.0 }, Xf::IDENTITY);
                }
            }
            f.ghosts = false;
            f.marks = matches!(s.body, Body::Sheet(_));
            let mut p = Project::new(Scene::default());
            f.write(&mut p);
            let mut back = Project::from_json(&p.to_json()).unwrap();
            let r = Faces::read(&mut back, [1.0, 1.0, 1.0]).expect("faces");
            assert_eq!(r.solid, s);
            assert!(!r.ghosts && r.marks == matches!(s.body, Body::Sheet(_)));
            assert_eq!(r.scenes.iter().map(|s| s.shapes.len()).collect::<Vec<_>>(), (1..=s.count()).collect::<Vec<_>>());
            assert!(back.section_names().is_empty(), "the sections are taken out");
        }
        // An old two-sided file without the size of the sheet takes the one of the program.
        let mut p = Project::new(Scene::default());
        p.set_section(crate::setup::SECTION, &TwoSided::new(FlipAxis::EndToEnd));
        let r = Faces::read(&mut p, [100.0, 50.0, 3.0]).unwrap();
        assert_eq!(r.solid, Solid::sheet(100.0, 50.0, 3.0, FlipAxis::EndToEnd));
        assert!(Faces::read(&mut Project::new(Scene::default()), [1.0; 3]).is_none());
    }
}
