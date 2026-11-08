//! A drag knife: a blade that turns freely, its tip `offset` millimetres behind the axis it turns about, so the tip trails the move
//! of the machine as a caster does. Driving the outline itself would round the corners, cut the start crooked and leave the end of a
//! closed outline open. [`compensate`] turns an outline (the path the tip is to cut) into the path of the axis:
//!
//! * every point is moved `offset` ahead along the direction of the cut, so the tip trailing behind lands on the outline;
//! * at a corner sharper than `swivel_deg` the axis goes round the corner point on an arc of radius `offset` (a swivel): the blade
//!   turns on the spot and the corner stays sharp;
//! * a cut starts with a swivel from the way the blade points after the last cut (`heading`) to the first direction of the outline,
//!   so its first millimetres are straight;
//! * a closed outline is cut on past its start by `overcut`, so it closes.
//!
//! [`simulate`] drags a tip behind a path of the axis, the way the blade follows: the preview draws it and the tests check with it
//! that the tip cuts the outline. [`inside_first`] orders outlines so that holes are cut before the part round them.
use crate::geom::Pt;

/// How the knife is set up.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct Knife {
    /// The tip behind the axis (mm): about 0.25 for a 45° blade, 0.5 for a 60° one.
    pub offset: f64,
    /// A turn sharper than this (degrees) is made by a swivel round the corner; a gentler one is cut through.
    pub swivel_deg: f64,
    /// How far a closed outline is cut on past its start (mm).
    pub overcut: f64,
}

impl Default for Knife {
    fn default() -> Knife {
        Knife { offset: 0.25, swivel_deg: 15.0, overcut: 0.5 }
    }
}

fn sub(a: Pt, b: Pt) -> Pt {
    Pt::new(a.x - b.x, a.y - b.y)
}

fn len(v: Pt) -> f64 {
    v.x.hypot(v.y)
}

fn unit(v: Pt) -> Option<Pt> {
    let l = len(v);
    (l > 1e-9).then(|| Pt::new(v.x / l, v.y / l))
}

fn along(p: Pt, d: Pt, t: f64) -> Pt {
    Pt::new(p.x + d.x * t, p.y + d.y * t)
}

/// The points without repeats (two after each other closer than a micron).
fn clean(pts: &[Pt]) -> Vec<Pt> {
    let mut out: Vec<Pt> = Vec::with_capacity(pts.len());
    for p in pts {
        if out.last().is_none_or(|q| len(sub(*p, *q)) > 1e-6) {
            out.push(*p);
        }
    }
    out
}

/// The axis going round `corner` on the circle of radius `r` from the direction `from` to `to`, the short way, in steps of 10° or
/// less (without the first point, with the last).
fn swivel(corner: Pt, from: Pt, to: Pt, r: f64) -> Vec<Pt> {
    let a0 = from.y.atan2(from.x);
    let mut da = to.y.atan2(to.x) - a0;
    while da > std::f64::consts::PI {
        da -= std::f64::consts::TAU;
    }
    while da < -std::f64::consts::PI {
        da += std::f64::consts::TAU;
    }
    let steps = ((da.abs() / 10f64.to_radians()).ceil() as usize).max(1);
    (1..=steps).map(|k| {
        let a = a0 + da * k as f64 / steps as f64;
        Pt::new(corner.x + r * a.cos(), corner.y + r * a.sin())
    }).collect()
}

/// The angle between two directions (degrees, 0 = straight on).
fn turn_deg(a: Pt, b: Pt) -> f64 {
    (a.x * b.x + a.y * b.y).clamp(-1.0, 1.0).acos().to_degrees()
}

/// The path of the axis for an outline (the path the tip is to cut), and the way the blade points at its end. `heading` is the way
/// it points before (after the last cut; the blade keeps it while it is lifted). A closed outline is given with its first point at
/// the end too, or without it: both are closed by `closed`.
pub fn compensate(outline: &[Pt], closed: bool, heading: Pt, k: &Knife) -> (Vec<Pt>, Pt) {
    let mut pts = clean(outline);
    if closed && pts.len() > 2 && len(sub(pts[0], *pts.last().expect("points"))) <= 1e-6 {
        pts.pop();
    }
    if pts.len() < 2 {
        return (pts, heading);
    }
    // The tip's path: for a closed outline round once and on past the start by the overcut.
    let mut tip: Vec<Pt> = pts.clone();
    if closed {
        tip.push(pts[0]);
        let mut left = k.overcut.max(0.0);
        let mut i = 0;
        while left > 1e-9 && i < pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            let l = len(sub(b, a));
            if l <= 1e-9 {
                i += 1;
                continue;
            }
            let t = left.min(l);
            tip.push(along(a, unit(sub(b, a)).expect("a segment"), t));
            left -= t;
            i += 1;
        }
    }
    let tip = clean(&tip);
    let dirs: Vec<Pt> = tip.windows(2).filter_map(|w| unit(sub(w[1], w[0]))).collect();
    if dirs.is_empty() {
        return (tip, heading);
    }
    let r = k.offset.max(0.0);
    let start_heading = unit(heading).unwrap_or(Pt::new(1.0, 0.0));
    // The blade is put down where its tip touches the start, pointing as it did, and turned to the first direction.
    let mut axis = vec![along(tip[0], start_heading, r)];
    if turn_deg(start_heading, dirs[0]) > 1.0 {
        axis.extend(swivel(tip[0], start_heading, dirs[0], r));
    } else {
        axis.push(along(tip[0], dirs[0], r));
    }
    let mut seg = 0;
    for w in tip.windows(2) {
        let Some(d) = unit(sub(w[1], w[0])) else { continue };
        // The end of this segment, with the axis ahead of the tip.
        axis.push(along(w[1], d, r));
        if let Some(next) = dirs.get(seg + 1) {
            if turn_deg(d, *next) > k.swivel_deg {
                axis.extend(swivel(w[1], d, *next, r));
            } else {
                axis.push(along(w[1], *next, r));
            }
        }
        seg += 1;
    }
    (clean(&axis), *dirs.last().expect("a direction"))
}

/// The path the tip makes when the axis follows `axis`: it trails `offset` behind, pulled along the line from itself to the axis.
/// `start` is where the tip is when the axis is at the first point.
pub fn simulate(axis: &[Pt], start: Pt, offset: f64) -> Vec<Pt> {
    let mut tip = start;
    let mut out = vec![tip];
    for w in axis.windows(2) {
        // Small steps, so the tip turns as the blade does.
        let l = len(sub(w[1], w[0]));
        let steps = ((l / (offset.max(0.01) / 8.0)).ceil() as usize).clamp(1, 10_000);
        for s in 1..=steps {
            let a = along(w[0], sub(w[1], w[0]), s as f64 / steps as f64);
            let d = sub(tip, a);
            tip = match unit(d) {
                Some(u) => along(a, u, offset),
                None => tip,
            };
        }
        out.push(tip);
    }
    out
}

fn contains(poly: &[Pt], p: Pt) -> bool {
    let mut inside = false;
    let n = poly.len();
    for i in 0..n {
        let (a, b) = (poly[i], poly[(i + 1) % n]);
        if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
            inside = !inside;
        }
    }
    inside
}

/// How deep each outline lies: how many closed outlines are round its first point. Cutting the deepest first (holes before the parts
/// round them, the parts before the sheet) keeps a part held by the material until it is cut out.
pub fn nesting(outlines: &[(Vec<Pt>, bool)]) -> Vec<usize> {
    outlines
        .iter()
        .enumerate()
        .map(|(i, (pts, _))| {
            let Some(p) = pts.first() else { return 0 };
            outlines.iter().enumerate().filter(|(j, (o, closed))| *j != i && *closed && o.len() > 2 && contains(o, *p)).count()
        })
        .collect()
}

/// The order of the outlines for cutting: the deepest first, the order given kept among the same depth.
pub fn inside_first(outlines: &[(Vec<Pt>, bool)]) -> Vec<usize> {
    let depth = nesting(outlines);
    let mut idx: Vec<usize> = (0..outlines.len()).collect();
    idx.sort_by_key(|i| std::cmp::Reverse(depth[*i]));
    idx
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist_to_outline(p: Pt, poly: &[Pt], closed: bool) -> f64 {
        let n = poly.len();
        let segs = if closed { n } else { n - 1 };
        (0..segs).map(|i| crate::geom::Polyline::new(vec![poly[i], poly[(i + 1) % n]], false).dist_to(p)).fold(f64::MAX, f64::min)
    }

    #[test]
    fn the_tip_cuts_the_outline_with_sharp_corners_and_a_closed_end() {
        let k = Knife { offset: 0.5, swivel_deg: 15.0, overcut: 1.0 };
        let square = vec![Pt::new(0.0, 0.0), Pt::new(20.0, 0.0), Pt::new(20.0, 20.0), Pt::new(0.0, 20.0)];
        // The blade points backwards before the cut: it is turned at the start, not dragged crooked.
        let (axis, end) = compensate(&square, true, Pt::new(-1.0, 0.0), &k);
        // The blade is put down with its tip on the start of the outline.
        assert_eq!(axis[0], Pt::new(-0.5, 0.0));
        let tip = simulate(&axis, square[0], k.offset);
        // Every point the tip goes through is on the square (the blade turns on the corners, it does not cut them round).
        let worst = tip.iter().map(|p| dist_to_outline(*p, &square, true)).fold(0.0, f64::max);
        assert!(worst < 0.02, "the tip leaves the outline by {worst} mm");
        // The corners are reached: the tip comes to within a hundredth of each.
        for c in &square {
            let near = tip.iter().map(|p| len(sub(*p, *c))).fold(f64::MAX, f64::min);
            assert!(near < 0.02, "the corner {c:?} is cut round: {near} mm");
        }
        // The cut goes on past the start along the first side: the outline is closed.
        assert!(tip.iter().any(|p| p.y.abs() < 0.02 && p.x > 0.8 && p.x < 1.2), "the overcut");
        assert!((end.x - 1.0).abs() < 1e-9 && end.y.abs() < 1e-9, "it ends going along the first side");
        // Without the compensation the corners would be cut round by about the offset.
        let raw = simulate(&clean(&[square.clone(), vec![square[0]]].concat()), Pt::new(-0.5, 0.0), k.offset);
        let rounded = raw.iter().map(|p| len(sub(*p, Pt::new(20.0, 0.0)))).fold(f64::MAX, f64::min);
        assert!(rounded > 0.1, "the plain path rounds the corner: {rounded}");
    }

    #[test]
    fn a_gentle_curve_is_cut_through_and_holes_come_before_the_part() {
        let k = Knife::default();
        // A circle of 40 points: every turn is 9°, gentler than the swivel angle, so no swivels are made.
        let circle: Vec<Pt> = (0..40).map(|i| {
            let a = i as f64 / 40.0 * std::f64::consts::TAU;
            Pt::new(10.0 * a.cos(), 10.0 * a.sin())
        }).collect();
        let (axis, _) = compensate(&circle, true, Pt::new(0.0, 1.0), &k);
        assert!(axis.len() < circle.len() * 2 + 10, "no swivel on every point: {}", axis.len());
        let sheet = vec![Pt::new(-50.0, -50.0), Pt::new(50.0, -50.0), Pt::new(50.0, 50.0), Pt::new(-50.0, 50.0)];
        let hole = vec![Pt::new(-2.0, -2.0), Pt::new(2.0, -2.0), Pt::new(2.0, 2.0), Pt::new(-2.0, 2.0)];
        let order = inside_first(&[(sheet, true), (circle, true), (hole, true)]);
        assert_eq!(order, vec![2, 1, 0], "the hole, the part round it, the sheet");
        // An open line keeps its ends and leaves the blade in its last direction.
        let (axis, end) = compensate(&[Pt::new(0.0, 0.0), Pt::new(10.0, 0.0)], false, Pt::new(1.0, 0.0), &k);
        assert_eq!(axis.first().copied(), Some(Pt::new(0.25, 0.0)));
        assert_eq!(axis.last().copied(), Some(Pt::new(10.25, 0.0)));
        assert_eq!(end, Pt::new(1.0, 0.0));
    }
}
