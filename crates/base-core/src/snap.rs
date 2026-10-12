//! Snapping a moved selection to the other objects: its left, middle and right (top, middle, bottom) line
//! lands on the same line of a nearby object when it comes within the tolerance.
use crate::geom::{Pt, Rect};

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct Snap {
    /// What to add to the move.
    pub dx: f64,
    pub dy: f64,
    /// The vertical / horizontal line the selection now sits on, to draw as a guide.
    pub guide_x: Option<f64>,
    pub guide_y: Option<f64>,
}

fn lines(lo: f64, hi: f64) -> [f64; 3] {
    [lo, (lo + hi) / 2.0, hi]
}

/// The closest pair (moving line, target line) of all of them within `tol`: the shift and the target line.
fn best(moving: [f64; 3], targets: impl Iterator<Item = [f64; 3]>, tol: f64) -> Option<(f64, f64)> {
    let mut found: Option<(f64, f64)> = None;
    for t in targets {
        for m in moving {
            for l in t {
                let d = l - m;
                if d.abs() <= tol && found.is_none_or(|(b, _)| d.abs() < b.abs()) {
                    found = Some((d, l));
                }
            }
        }
    }
    found
}

/// `moving` is the box of the selection where it would be, `others` the boxes of the objects it can snap to.
/// Each axis is snapped on its own, to the closest line within `tol` (millimetres).
pub fn snap_to_objects(moving: Rect, others: &[Rect], tol: f64) -> Snap {
    if !(moving.min.x.is_finite() && moving.min.y.is_finite() && moving.max.x.is_finite() && moving.max.y.is_finite()) || !tol.is_finite() || tol <= 0.0 {
        return Snap::default();
    }
    let ok = |r: &&Rect| r.min.x.is_finite() && r.min.y.is_finite() && r.max.x.is_finite() && r.max.y.is_finite();
    let x = best(lines(moving.min.x, moving.max.x), others.iter().filter(ok).map(|r| lines(r.min.x, r.max.x)), tol);
    let y = best(lines(moving.min.y, moving.max.y), others.iter().filter(ok).map(|r| lines(r.min.y, r.max.y)), tol);
    Snap { dx: x.map_or(0.0, |v| v.0), dy: y.map_or(0.0, |v| v.0), guide_x: x.map(|v| v.1), guide_y: y.map(|v| v.1) }
}

/// A box from two corners, for tests and callers that have only points.
pub fn rect(a: Pt, b: Pt) -> Rect {
    Rect { min: Pt::new(a.x.min(b.x), a.y.min(b.y)), max: Pt::new(a.x.max(b.x), a.y.max(b.y)) }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(x0: f64, y0: f64, x1: f64, y1: f64) -> Rect {
        rect(Pt::new(x0, y0), Pt::new(x1, y1))
    }

    #[test]
    fn edges_centres_and_far_things() {
        let other = r(100.0, 100.0, 140.0, 120.0);
        // The left edge 2 mm short of the other's right edge: it lands on it.
        let s = snap_to_objects(r(138.0, 0.0, 150.0, 10.0), &[other], 3.0);
        assert_eq!((s.dx, s.guide_x), (2.0, Some(140.0)));
        assert_eq!((s.dy, s.guide_y), (0.0, None), "nothing near in y");
        // Centres line up: the middle of the moving box is 108, the other's is 110.
        let s = snap_to_objects(r(0.0, 103.0, 10.0, 113.0), &[other], 2.5);
        assert_eq!((s.dy, s.guide_y), (2.0, Some(110.0)), "centre to centre: {s:?}");
        // Out of reach: nothing happens.
        assert_eq!(snap_to_objects(r(0.0, 0.0, 10.0, 10.0), &[other], 3.0), Snap::default());
        // The closest of several lines wins.
        let a = r(50.0, 0.0, 60.0, 5.0);
        let b = r(52.5, 0.0, 70.0, 5.0);
        let s = snap_to_objects(r(48.0, 0.0, 49.0, 5.0), &[a, b], 5.0);
        assert_eq!(s.guide_x, Some(50.0));
        assert_eq!(s.dx, 1.0, "the right edge 49 is the closest to 50");
    }

    #[test]
    fn both_axes_snap_on_their_own_and_bad_numbers_do_nothing() {
        let other = r(0.0, 0.0, 10.0, 10.0);
        let s = snap_to_objects(r(10.5, 10.5, 20.0, 20.0), &[other], 1.0);
        assert_eq!((s.dx, s.dy), (-0.5, -0.5));
        assert_eq!((s.guide_x, s.guide_y), (Some(10.0), Some(10.0)));
        let nan = Rect { min: Pt::new(f64::NAN, 0.0), max: Pt::new(1.0, 1.0) };
        assert_eq!(snap_to_objects(nan, &[other], 1.0), Snap::default());
        assert_eq!(snap_to_objects(other, &[nan], 1.0), Snap::default());
        assert_eq!(snap_to_objects(other, &[other], 0.0), Snap::default());
        assert_eq!(snap_to_objects(other, &[], 5.0), Snap::default());
    }
}
