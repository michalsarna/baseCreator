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

/// Relief notches in the sharp inside corners of closed shapes that are slots or holes (what a round bit cannot cut square): a disc of
/// radius `r` is added at every corner whose angle is under 150 degrees. A **dogbone** puts the disc where the edge of a bit of that radius,
/// coming along both walls, reaches the corner (the centre is `r` from the corner, on the bisector, into the shape); a **T-bone** puts it
/// on the line of the longer wall, going on past the corner, so that only one wall is overcut. Returns the shapes with the notches; a
/// shape without a corner is returned as it is.
pub fn dogbone(polys: &[Polyline], r: f64, tbone: bool) -> Vec<Polyline> {
    let mut discs: Vec<Polyline> = vec![];
    for poly in polys.iter().filter(|p| p.closed && p.pts.len() >= 3) {
        let n = poly.pts.len();
        let ccw = poly.area() > 0.0;
        for i in 0..n {
            let (a, q, b) = (poly.pts[(i + n - 1) % n], poly.pts[i], poly.pts[(i + 1) % n]);
            let (v1, v2) = ((q.x - a.x, q.y - a.y), (b.x - q.x, b.y - q.y));
            let (l1, l2) = (v1.0.hypot(v1.1), v2.0.hypot(v2.1));
            if l1 < 1e-9 || l2 < 1e-9 {
                continue;
            }
            let cross = v1.0 * v2.1 - v1.1 * v2.0;
            // A corner of the outline that points into the shape's inside (convex for the shape): the outline turns the way it goes round.
            if (cross > 0.0) != ccw || cross.abs() < 1e-12 {
                continue;
            }
            let interior = std::f64::consts::PI - cross.atan2(v1.0 * v2.0 + v1.1 * v2.1).abs();
            if interior >= 150f64.to_radians() {
                continue;
            }
            // Unit directions from the corner along the two walls.
            let (e1, e2) = ((-v1.0 / l1, -v1.1 / l1), (v2.0 / l2, v2.1 / l2));
            let (sx, sy) = (e1.0 + e2.0, e1.1 + e2.1);
            let sl = sx.hypot(sy).max(1e-12);
            let inward = (sx / sl, sy / sl);
            let centre = if tbone {
                // Along the line of the longer wall, away from the corner into the wall's own extension past the corner.
                let along = if l1 >= l2 { e1 } else { e2 };
                Pt::new(q.x - along.0 * r, q.y - along.1 * r)
            } else {
                Pt::new(q.x + inward.0 * r, q.y + inward.1 * r)
            };
            let steps = 32;
            discs.push(Polyline::new((0..steps).map(|k| { let t = k as f64 / steps as f64 * std::f64::consts::TAU; Pt::new(centre.x + r * t.cos(), centre.y + r * t.sin()) }).collect(), true));
        }
    }
    if discs.is_empty() {
        return polys.to_vec();
    }
    let mut all: Vec<Polyline> = polys.to_vec();
    all.extend(discs);
    union_all(&all)
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

#[cfg(test)]
mod dogbone_tests {
    use super::*;

    fn rect(w: f64, h: f64) -> Polyline {
        Polyline::new(vec![Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)], true)
    }

    fn area(v: &[Polyline]) -> f64 {
        v.iter().map(|p| p.area()).sum::<f64>().abs()
    }

    #[test]
    fn a_dogbone_adds_a_notch_at_every_corner_so_that_a_round_bit_reaches_it() {
        let slot = [rect(30.0, 10.0)];
        let bone = dogbone(&slot, 1.5, false);
        // Four corners, each notch a part of a disc of radius 1.5 outside the walls: a bit more area, never less.
        assert!(area(&bone) > 300.0 && area(&bone) < 300.0 + 4.0 * std::f64::consts::PI * 1.5 * 1.5, "{}", area(&bone));
        // The disc is centred 1.5 mm from the corner along the diagonal: it pokes out of the walls by 0.44 mm sideways, and the corner
        // itself is on its edge.
        let near = Pt::new(-0.3, 1.0);
        let far = Pt::new(-3.0, 1.0);
        assert!(bone.iter().filter(|p| p.closed).any(|p| p.contains(near)));
        assert!(!bone.iter().filter(|p| p.closed).any(|p| p.contains(far)));
    }

    #[test]
    fn a_t_bone_overcuts_one_wall_only_and_a_round_shape_gets_nothing() {
        let slot = [rect(30.0, 10.0)];
        let t = dogbone(&slot, 1.5, true);
        // The longer wall (the 30 mm one) is extended past the corner: a point beyond the end of it, along the line of the wall, is inside.
        assert!(t.iter().any(|p| p.contains(Pt::new(-1.0, 0.5))) || t.iter().any(|p| p.contains(Pt::new(0.5, -1.0))));
        // A circle has no corner.
        let round = Polyline::new((0..64).map(|i| { let a = i as f64 / 64.0 * std::f64::consts::TAU; Pt::new(5.0 * a.cos(), 5.0 * a.sin()) }).collect(), true);
        assert_eq!(dogbone(&[round.clone()], 1.0, false), vec![round]);
    }
}

