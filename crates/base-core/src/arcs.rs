//! Fitting circular arcs to polylines, for G-code that uses `G2` / `G3` instead of many short lines.
use crate::geom::Pt;

/// A piece of a fitted path: the polyline points `..=end` are covered from the previous piece's end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Piece {
    /// One straight segment to point `end`.
    Line { end: usize },
    /// An arc to point `end` around `center`; the points in between lie within the tolerance of it.
    Arc { end: usize, center: Pt },
}

/// Largest radius that becomes an arc (a flatter curve is a line for all practical purposes) and the fewest segments
/// an arc must replace.
const MAX_RADIUS: f64 = 5000.0;
const MIN_SEGMENTS: usize = 3;

fn circumcentre(a: Pt, b: Pt, c: Pt) -> Option<Pt> {
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-12 {
        return None;
    }
    let (a2, b2, c2) = (a.x * a.x + a.y * a.y, b.x * b.x + b.y * b.y, c.x * c.x + c.y * c.y);
    Some(Pt::new((a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d, (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d))
}

fn cross(o: Pt, a: Pt, b: Pt) -> f64 {
    (a.x - o.x) * (b.y - o.y) - (a.y - o.y) * (b.x - o.x)
}

/// The centre of the circle that `pts` follow within `tol`, if they follow one: all the points and the middles of
/// all the segments are close to it, the path always turns the same way and goes round less than once.
fn arc_centre(pts: &[Pt], tol: f64) -> Option<Pt> {
    let (a, b, c) = (pts[0], pts[pts.len() / 2], pts[pts.len() - 1]);
    let centre = circumcentre(a, b, c)?;
    let r = centre.dist(a);
    if !(3.0 * tol..=MAX_RADIUS).contains(&r) {
        return None;
    }
    let turn = cross(a, b, c).signum();
    let mut sweep = 0.0;
    let mut last: Option<Pt> = None;
    for w in pts.windows(2) {
        if w[0].dist(w[1]) < 1e-9 {
            continue;
        }
        if (centre.dist(w[0]) - r).abs() > tol || (centre.dist(w[1]) - r).abs() > tol {
            return None;
        }
        let mid = Pt::new((w[0].x + w[1].x) / 2.0, (w[0].y + w[1].y) / 2.0);
        if (centre.dist(mid) - r).abs() > tol {
            return None;
        }
        // Turning the other way, or doubling back, is not an arc.
        let step = cross(centre, w[0], w[1]);
        if step * turn <= 0.0 {
            return None;
        }
        sweep += (step / (r * r)).clamp(-1.0, 1.0).asin().abs();
        if let Some(p) = last {
            if cross(p, w[0], w[1]) * turn < -1e-9 {
                return None;
            }
        }
        last = Some(w[0]);
    }
    (sweep < std::f64::consts::TAU * 0.97).then_some(centre)
}

/// Cover the polyline `pts` (at least two points) by lines and arcs that stay within `tol` millimetres of it.
pub fn fit(pts: &[Pt], tol: f64) -> Vec<Piece> {
    let n = pts.len();
    let mut out = vec![];
    let mut i = 0;
    while i + 1 < n {
        let mut best: Option<(usize, Pt)> = None;
        let mut j = i + MIN_SEGMENTS;
        while j < n {
            match arc_centre(&pts[i..=j], tol) {
                Some(c) => best = Some((j, c)),
                None => break,
            }
            j += 1;
        }
        match best {
            Some((end, center)) => {
                out.push(Piece::Arc { end, center });
                i = end;
            }
            None => {
                out.push(Piece::Line { end: i + 1 });
                i += 1;
            }
        }
    }
    out
}
