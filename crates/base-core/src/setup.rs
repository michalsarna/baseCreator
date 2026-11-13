//! The setup of a project, the same in every program that works on a piece of material: where the zero of the program is on it,
//! and a two-sided job. In a two-sided job side A is drawn as the material lies first, side B as it lies after it is turned over
//! round one of its middle lines; side B is kept in the project section `two_sided` and is cut as it is drawn. Each side shows the
//! shapes of the other one turned over (see [`turn`]) as ghosts, to place its own shapes against them.
use crate::geom::Xf;
use crate::scene::{Scene, Shape};
use serde::{Deserialize, Serialize};

/// A place on the piece of material that is the zero of X and Y.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZeroPoint {
    FrontLeft,
    FrontRight,
    BackLeft,
    BackRight,
    Center,
}

impl ZeroPoint {
    pub const ALL: [ZeroPoint; 5] = [ZeroPoint::FrontLeft, ZeroPoint::FrontRight, ZeroPoint::BackLeft, ZeroPoint::BackRight, ZeroPoint::Center];
    pub fn label(self) -> &'static str {
        match self {
            ZeroPoint::FrontLeft => "Front left",
            ZeroPoint::FrontRight => "Front right",
            ZeroPoint::BackLeft => "Back left",
            ZeroPoint::BackRight => "Back right",
            ZeroPoint::Center => "Centre",
        }
    }
}

/// The line the material is turned over round for its other side.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FlipAxis {
    /// Round the vertical middle line (from the back to the front): left and right change places.
    #[default]
    SideToSide,
    /// Round the horizontal middle line (from left to right): the back and the front change places.
    EndToEnd,
}

impl FlipAxis {
    pub const ALL: [FlipAxis; 2] = [FlipAxis::SideToSide, FlipAxis::EndToEnd];
    pub fn label(self) -> &'static str {
        match self {
            FlipAxis::SideToSide => "Side to side (left and right change places)",
            FlipAxis::EndToEnd => "End to end (back and front change places)",
        }
    }
    /// The short name of the axis: V (the vertical line) or H (the horizontal one).
    pub fn short(self) -> &'static str {
        match self {
            FlipAxis::SideToSide => "V",
            FlipAxis::EndToEnd => "H",
        }
    }
    pub fn is_side_to_side(&self) -> bool {
        *self == FlipAxis::SideToSide
    }
}

/// What a shape at a place of one side lies under on the other side, for a piece `w` × `h` at the top left of the page turned over
/// round `flip`. It is its own inverse.
pub fn turn(flip: FlipAxis, w: f64, h: f64) -> Xf {
    match flip {
        FlipAxis::SideToSide => Xf { a: -1.0, b: 0.0, c: 0.0, d: 1.0, e: w, f: 0.0 },
        FlipAxis::EndToEnd => Xf { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: 0.0, f: h },
    }
}

/// The name of the project section of a two-sided job.
pub const SECTION: &str = "two_sided";

/// A two-sided job: how the material is turned over, whether the ghosts of the other side are shown, and the shapes of side B.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TwoSided {
    pub flip: FlipAxis,
    #[serde(default = "yes")]
    pub ghosts: bool,
    #[serde(default)]
    side_b: Vec<Shape>,
    #[serde(default)]
    next_id: u64,
    #[serde(default)]
    next_group: u64,
    /// The size of the piece (width, height, thickness, mm) for a program that keeps it nowhere else.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub piece: Option<[f64; 3]>,
    /// Alignment marks round the piece, burned as a job of their own before the piece is laid down (LightCreator).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub marks: bool,
}

fn yes() -> bool {
    true
}

impl TwoSided {
    pub fn new(flip: FlipAxis) -> TwoSided {
        TwoSided { flip, ghosts: true, side_b: vec![], next_id: 1, next_group: 1, piece: None, marks: false }
    }

    /// The section with the shapes of side B taken from `scene`.
    pub fn with_side_b(flip: FlipAxis, ghosts: bool, scene: &Scene) -> TwoSided {
        let (next_id, next_group) = scene.counters();
        TwoSided { flip, ghosts, side_b: scene.shapes.clone(), next_id, next_group, piece: None, marks: false }
    }

    /// The shapes of side B as a scene.
    pub fn side_b(&self) -> Scene {
        Scene::from_parts(self.side_b.clone(), self.next_id.max(1), self.next_group.max(1))
    }
}

/// The shapes of `scene` as they lie on the other side: turned over, for ghosts or to cut them from there.
pub fn turned(scene: &Scene, flip: FlipAxis, w: f64, h: f64) -> Vec<Shape> {
    let t = turn(flip, w, h);
    scene.shapes.iter().map(|s| Shape { xf: s.xf.then(t), ..s.clone() }).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Pt;
    use crate::scene::Kind;

    #[test]
    fn turning_over_twice_gives_the_same_place_and_the_axes_are_the_middle_lines() {
        for flip in FlipAxis::ALL {
            let t = turn(flip, 300.0, 200.0);
            let p = Pt::new(20.0, 30.0);
            let back = t.then(t).apply(p);
            assert!((back.x - p.x).abs() < 1e-9 && (back.y - p.y).abs() < 1e-9, "{flip:?}");
        }
        assert_eq!(turn(FlipAxis::SideToSide, 300.0, 200.0).apply(Pt::new(20.0, 30.0)), Pt::new(280.0, 30.0), "left and right change places");
        assert_eq!(turn(FlipAxis::EndToEnd, 300.0, 200.0).apply(Pt::new(20.0, 30.0)), Pt::new(20.0, 170.0), "back and front change places");
        assert_eq!((FlipAxis::SideToSide.short(), FlipAxis::EndToEnd.short()), ("V", "H"));
    }

    #[test]
    fn the_section_keeps_side_b_and_reads_old_files() {
        let mut b = Scene::default();
        let id = b.add(0, Kind::Rect { w: 10.0, h: 5.0 }, Xf::translate(3.0, 4.0));
        let s = TwoSided::with_side_b(FlipAxis::EndToEnd, false, &b);
        let back: TwoSided = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        let scene = back.side_b();
        assert!(scene.shape(id).is_some() && !back.ghosts && back.flip == FlipAxis::EndToEnd);
        // A new shape of side B does not take the id of an old one.
        let mut scene = scene;
        assert_ne!(scene.add(0, Kind::Rect { w: 1.0, h: 1.0 }, Xf::IDENTITY), id);
        let bare: TwoSided = serde_json::from_str("{\"flip\":\"SideToSide\"}").unwrap();
        assert!(bare.ghosts && bare.side_b().shapes.is_empty());
        let turned_shapes = turned(&b, FlipAxis::SideToSide, 100.0, 50.0);
        let bb = turned_shapes[0].bounds().unwrap();
        assert!((bb.min.x - 87.0).abs() < 1e-9 && (bb.max.x - 97.0).abs() < 1e-9);
    }
}
