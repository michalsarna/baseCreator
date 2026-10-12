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
