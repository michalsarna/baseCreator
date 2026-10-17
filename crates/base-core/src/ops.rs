//! Polygon boolean operations and offsetting (backed by the `i_overlay` crate).
use crate::geom::*;
use i_overlay::core::fill_rule::FillRule;
use i_overlay::core::overlay_rule::OverlayRule;
use i_overlay::float::single::SingleFloatOverlay;
use i_overlay::mesh::float::outline::offset::OutlineOffset;
use i_overlay::mesh::float::style::{LineJoin, OutlineStyle};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoolOp {
    Union,
    Intersect,
    /// First operand minus the second.
    Difference,
    Xor,
}

impl BoolOp {
    fn rule(self) -> OverlayRule {
        match self {
            BoolOp::Union => OverlayRule::Union,
            BoolOp::Intersect => OverlayRule::Intersect,
            BoolOp::Difference => OverlayRule::Difference,
            BoolOp::Xor => OverlayRule::Xor,
        }
    }
}

type Ring = Vec<[f64; 2]>;

/// Closed polylines with at least three points, as rings.
fn rings(polys: &[Polyline]) -> Vec<Ring> {
    polys.iter().filter(|p| p.closed && p.pts.len() >= 3).map(|p| p.pts.iter().map(|q| [q.x, q.y]).collect()).collect()
}

fn to_polys(shapes: Vec<Vec<Ring>>) -> Vec<Polyline> {
    shapes.into_iter().flatten().map(|r| Polyline::new(r.into_iter().map(|q| Pt::new(q[0], q[1])).collect(), true)).collect()
}

/// Combine two sets of closed polylines (each set uses the even-odd rule). Open polylines are ignored.
pub fn boolean(a: &[Polyline], b: &[Polyline], op: BoolOp) -> Vec<Polyline> {
    let (ra, rb) = (rings(a), rings(b));
    if ra.is_empty() && rb.is_empty() {
        return vec![];
    }
    to_polys(ra.overlay(&rb, op.rule(), FillRule::EvenOdd))
}

/// Merge the contours of a set (even-odd) into clean, non-overlapping outlines.
pub fn normalize(a: &[Polyline]) -> Vec<Polyline> {
    boolean(a, &[], BoolOp::Union)
}

/// The union of any number of closed polylines that may overlap (a mesh projected on a plane, for example): the outlines of
/// the area they cover together, outer rings counter-clockwise and holes clockwise. Unlike `normalize` the rings are not
/// taken as even-odd, so shapes lying over each other add up instead of cancelling.
pub fn union_all(polys: &[Polyline]) -> Vec<Polyline> {
    let mut rs = rings(polys);
    for r in &mut rs {
        // Every ring counter-clockwise: with the non-zero rule they all add.
        let area: f64 = (0..r.len()).map(|i| r[i][0] * r[(i + 1) % r.len()][1] - r[(i + 1) % r.len()][0] * r[i][1]).sum();
        if area < 0.0 {
            r.reverse();
        }
    }
    if rs.is_empty() {
        return vec![];
    }
    to_polys(rs.overlay(&Vec::<Ring>::new(), OverlayRule::Subject, FillRule::NonZero))
}

/// Offset closed polylines by `d` millimetres: positive grows, negative shrinks.
/// Returns closed outlines; holes come out as separate rings (even-odd).
pub fn offset(polys: &[Polyline], d: f64) -> Vec<Polyline> {
    let clean = rings(&normalize(polys));
    if clean.is_empty() {
        return vec![];
    }
    // Normalised output has outer rings counter-clockwise and holes clockwise, which is what the outliner wants.
    // Group each shape (outer + holes) again through a union so the hierarchy is explicit.
    let shapes = clean.overlay(&Vec::<Ring>::new(), OverlayRule::Subject, FillRule::EvenOdd);
    let style = OutlineStyle::new(d).line_join(LineJoin::Round(0.2));
    to_polys(shapes.outline(&style))
}

/// Concentric inward rings of the closed `polys`, `step` millimetres apart, outermost first.
/// Stops when nothing is left or after `max_rings`.
pub fn inset_rings(polys: &[Polyline], step: f64, max_rings: usize) -> Vec<Polyline> {
    let step = step.max(0.01);
    let mut out = vec![];
    let mut cur = normalize(polys);
    // First ring sits half a step inside the outline so edges are not burnt twice with Fill + Line.
    for _ in 0..max_rings {
        if cur.is_empty() {
            break;
        }
        out.extend(cur.iter().cloned());
        cur = offset(&cur, -step);
        if cur.iter().all(|p| p.area().abs() < 1e-4) {
            break;
        }
    }
    out
}

#[cfg(test)]
mod union_tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64, ccw: bool) -> Polyline {
        let mut pts = vec![Pt::new(x, y), Pt::new(x + s, y), Pt::new(x + s, y + s), Pt::new(x, y + s)];
        if !ccw {
            pts.reverse();
        }
        Polyline::new(pts, true)
    }

    fn area(v: &[Polyline]) -> f64 {
        v.iter().map(|p| p.area()).sum::<f64>().abs()
    }

    #[test]
    fn overlapping_polygons_add_up_whichever_way_they_are_turned() {
        // Two 10 x 10 squares overlapping by 5 x 10: 150, not 100 (even-odd) and not 200.
        let a = [square(0.0, 0.0, 10.0, true), square(5.0, 0.0, 10.0, false)];
        let u = union_all(&a);
        assert_eq!(u.len(), 1);
        assert!((area(&u) - 150.0).abs() < 1e-6, "{}", area(&u));
        // The old even-odd merge would have cancelled the overlap.
        assert!((area(&normalize(&a)) - 100.0).abs() < 1e-6);
        // A square inside another disappears into it; separate squares stay separate; nothing gives nothing.
        assert!((area(&union_all(&[square(0.0, 0.0, 10.0, true), square(2.0, 2.0, 3.0, true)])) - 100.0).abs() < 1e-6);
        assert_eq!(union_all(&[square(0.0, 0.0, 1.0, true), square(5.0, 0.0, 1.0, true)]).len(), 2);
        assert!(union_all(&[]).is_empty());
    }

    #[test]
    fn many_small_triangles_make_one_outline() {
        // A 100 x 100 sheet cut into 20 000 triangles (a grid of 100 x 100 squares, two each).
        let mut tris = vec![];
        for i in 0..100 {
            for j in 0..100 {
                let (x, y) = (i as f64, j as f64);
                tris.push(Polyline::new(vec![Pt::new(x, y), Pt::new(x + 1.0, y), Pt::new(x + 1.0, y + 1.0)], true));
                tris.push(Polyline::new(vec![Pt::new(x, y), Pt::new(x + 1.0, y + 1.0), Pt::new(x, y + 1.0)], true));
            }
        }
        let u = union_all(&tris);
        assert_eq!(u.len(), 1, "{} rings", u.len());
        assert!((area(&u) - 10_000.0).abs() < 1e-3);
    }
}
