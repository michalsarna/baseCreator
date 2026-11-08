//! The geometry of the ruler: the points it snaps to (nodes, the middles of segments, the centres of circles and ellipses) and what a
//! click on a shape measures: a straight segment, a curve (its length, and the circle it lies on when it is an arc), or a circle or an
//! ellipse (its radii).
use super::{Measure, Picked};
use base_core::bezier::Contour;
use base_core::geom::{Polyline, Pt};
use base_core::scene::{Kind, Scene, Shape};

/// What a snapped point of the ruler is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SnapKind {
    Node,
    Middle,
    Centre,
}

fn cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt, t: f64) -> Pt {
    let u = 1.0 - t;
    let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
    Pt::new(a * p0.x + b * p1.x + c * p2.x + d * p3.x, a * p0.y + b * p1.y + c * p2.y + d * p3.y)
}

/// The segments of a contour: their ends, whether they are straight, and their points (two for a straight one).
fn segments(c: &Contour) -> Vec<(bool, Vec<Pt>)> {
    let n = c.nodes.len();
    let count = if c.closed { n } else { n.saturating_sub(1) };
    (0..count)
        .map(|i| {
            let (a, b) = (c.nodes[i], c.nodes[(i + 1) % n]);
            if a.has_out() || b.has_in() {
                let mut pts = vec![a.p];
                base_core::geom::flatten_cubic(a.p, a.hout, b.hin, b.p, 0.01, &mut pts);
                (false, pts)
            } else {
                (true, vec![a.p, b.p])
            }
        })
        .collect()
}

/// The middle of a segment: halfway along a straight one, at t = 0.5 on a curve.
fn middle(c: &Contour, i: usize) -> Pt {
    let n = c.nodes.len();
    let (a, b) = (c.nodes[i], c.nodes[(i + 1) % n]);
    if a.has_out() || b.has_in() {
        cubic(a.p, a.hout, b.hin, b.p, 0.5)
    } else {
        Pt::new((a.p.x + b.p.x) / 2.0, (a.p.y + b.p.y) / 2.0)
    }
}

/// The centre and the two semi-axes (as vectors from it) of an ellipse shape on the work area.
fn ellipse_axes(s: &Shape) -> Option<(Pt, Pt, Pt)> {
    let Kind::Ellipse { w, h } = s.kind else { return None };
    let c = s.xf.apply(Pt::new(w / 2.0, h / 2.0));
    let ex = s.xf.apply(Pt::new(w, h / 2.0));
    let ey = s.xf.apply(Pt::new(w / 2.0, 0.0));
    Some((c, Pt::new(ex.x - c.x, ex.y - c.y), Pt::new(ey.x - c.x, ey.y - c.y)))
}

/// The point the ruler takes near `w` (within `tol` mm): the nearest node, middle of a segment or centre of a circle or an ellipse.
pub fn snap<'a>(shapes: impl Iterator<Item = &'a Shape>, w: Pt, tol: f64) -> Option<(Pt, SnapKind)> {
    let mut best: Option<(f64, Pt, SnapKind)> = None;
    let mut take = |p: Pt, k: SnapKind| {
        let d = p.dist(w);
        if d <= tol && best.is_none_or(|(bd, _, bk)| d < bd - 1e-9 || (d < bd + 1e-9 && k == SnapKind::Node && bk != SnapKind::Node)) {
            best = Some((d, p, k));
        }
    };
    for s in shapes {
        if let Some((c, _, _)) = ellipse_axes(s) {
            take(c, SnapKind::Centre);
        }
        for c in base_core::export::contours_of(s) {
            for n in &c.nodes {
                take(n.p, SnapKind::Node);
            }
            let n = c.nodes.len();
            let count = if c.closed { n } else { n.saturating_sub(1) };
            for i in 0..count {
                take(middle(&c, i), SnapKind::Middle);
            }
        }
    }
    best.map(|(_, p, k)| (p, k))
}

/// The nearest point of a line of points to `w`.
fn nearest_on(pts: &[Pt], w: Pt) -> Pt {
    let mut best = (f64::MAX, pts.first().copied().unwrap_or(w));
    for s in pts.windows(2) {
        let (a, b) = (s[0], s[1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len2 = dx * dx + dy * dy;
        let t = if len2 > 0.0 { (((w.x - a.x) * dx + (w.y - a.y) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
        let p = Pt::new(a.x + t * dx, a.y + t * dy);
        let d = p.dist(w);
        if d < best.0 {
            best = (d, p);
        }
    }
    best.1
}

/// The circle a curve lies on (centre, radius), when it is an arc: the circle through its ends and its middle, if every point of
/// it is within 0.4 % of the radius (and 0.02 mm) of that circle.
pub fn fit_arc(pts: &[Pt]) -> Option<(Pt, f64)> {
    if pts.len() < 3 {
        return None;
    }
    let (a, b, c) = (pts[0], pts[pts.len() / 2], pts[pts.len() - 1]);
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a.x * a.x + a.y * a.y, b.x * b.x + b.y * b.y, c.x * c.x + c.y * c.y);
    let centre = Pt::new((a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d, (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d);
    let r = centre.dist(a);
    let tol = (r * 0.004).max(0.02);
    pts.iter().all(|p| (p.dist(centre) - r).abs() <= tol).then_some((centre, r))
}

/// What a click at `w` on a shape measures (within `tol` mm), the topmost shape first: a circle or an ellipse by its radii, a
/// straight segment by its length, a curve by its length (and its circle when it is an arc).
pub fn pick(scene: &Scene, visible: &dyn Fn(&Shape) -> bool, w: Pt, tol: f64) -> Option<Measure> {
    for s in scene.shapes.iter().rev().filter(|s| visible(s)) {
        if let Some((c, ax, ay)) = ellipse_axes(s) {
            let outline: Vec<Pt> = s.polys().into_iter().flat_map(|p: Polyline| {
                let mut v = p.pts;
                if let Some(f) = v.first().copied() {
                    v.push(f);
                }
                v
            }).collect();
            if Polyline::new(outline.clone(), false).dist_to(w) <= tol {
                let at = nearest_on(&outline, w);
                return Some(Measure { picked: Some(Picked::Ellipse { c, ax, ay, at }), ..Default::default() });
            }
            continue;
        }
        for c in base_core::export::contours_of(s) {
            for (straight, pts) in segments(&c) {
                if Polyline::new(pts.clone(), false).dist_to(w) > tol {
                    continue;
                }
                if straight {
                    return Some(Measure { a: Some(pts[0]), b: Some(pts[1]), segment: true, ..Default::default() });
                }
                let len = pts.windows(2).map(|q| q[0].dist(q[1])).sum();
                let at = nearest_on(&pts, w);
                return Some(Measure { picked: Some(Picked::Curve { arc: fit_arc(&pts), pts, len, at }), ..Default::default() });
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::geom::Xf;

    #[test]
    fn the_ruler_snaps_to_nodes_middles_and_centres_and_picks_circles_and_arcs() {
        let mut sc = Scene::default();
        sc.add(0, Kind::Rect { w: 40.0, h: 20.0 }, Xf::translate(0.0, 0.0));
        sc.add(0, Kind::Ellipse { w: 30.0, h: 30.0 }, Xf::translate(100.0, 0.0));
        sc.add(0, Kind::Ellipse { w: 40.0, h: 20.0 }, Xf::translate(200.0, 0.0));
        // The middle of the top edge, a corner, the centre of the circle.
        assert_eq!(snap(sc.shapes.iter(), Pt::new(20.5, 0.4), 2.0), Some((Pt::new(20.0, 0.0), SnapKind::Middle)));
        assert_eq!(snap(sc.shapes.iter(), Pt::new(39.0, 19.0), 2.0), Some((Pt::new(40.0, 20.0), SnapKind::Node)));
        assert_eq!(snap(sc.shapes.iter(), Pt::new(115.5, 15.5), 2.0), Some((Pt::new(115.0, 15.0), SnapKind::Centre)));
        assert!(snap(sc.shapes.iter(), Pt::new(60.0, 60.0), 2.0).is_none());
        let all = |_: &Shape| true;
        // A click on the circle: its radius from the centre to the point clicked.
        let m = pick(&sc, &all, Pt::new(130.0, 15.0), 1.0).expect("the circle");
        let Some(Picked::Ellipse { c, ax, ay, at }) = m.picked else { panic!("{m:?}") };
        assert!(c.dist(Pt::new(115.0, 15.0)) < 1e-9 && (ax.x.hypot(ax.y) - 15.0).abs() < 1e-9 && (ay.x.hypot(ay.y) - 15.0).abs() < 1e-9);
        assert!((at.dist(c) - 15.0).abs() < 0.05);
        // The ellipse has two radii.
        let Some(Picked::Ellipse { ax, ay, .. }) = pick(&sc, &all, Pt::new(240.0, 10.0), 1.0).and_then(|m| m.picked) else { panic!() };
        assert_eq!(((ax.x.hypot(ax.y) * 10.0).round(), (ay.x.hypot(ay.y) * 10.0).round()), (200.0, 100.0));
        // The edge of the rectangle is a straight segment.
        let m = pick(&sc, &all, Pt::new(20.0, 0.3), 1.0).unwrap();
        assert!(m.segment && m.a.unwrap().dist(m.b.unwrap()) == 40.0);
        // A quarter circle drawn as a curve: its length and its circle.
        let mut arc = Scene::default();
        let k = 0.552_284_749_830_793_4 * 10.0;
        let n = |p: Pt, hin: Pt, hout: Pt| base_core::bezier::Node { p, hin, hout, smooth: false };
        let c = Contour { nodes: vec![n(Pt::new(10.0, 0.0), Pt::new(10.0, 0.0), Pt::new(10.0, k)), n(Pt::new(0.0, 10.0), Pt::new(k, 10.0), Pt::new(0.0, 10.0))], closed: false };
        arc.add(0, Kind::Bezier(vec![c]), Xf::IDENTITY);
        let probe = Pt::new(10.0 * std::f64::consts::FRAC_1_SQRT_2, 10.0 * std::f64::consts::FRAC_1_SQRT_2);
        let Some(Picked::Curve { len, arc: circle, .. }) = pick(&arc, &all, probe, 0.5).and_then(|m| m.picked) else { panic!() };
        assert!((len - 10.0 * std::f64::consts::FRAC_PI_2).abs() < 0.02, "{len}");
        let (cc, r) = circle.expect("an arc");
        assert!(cc.dist(Pt::new(0.0, 0.0)) < 0.02 && (r - 10.0).abs() < 0.02);
        assert!(fit_arc(&[Pt::new(0.0, 0.0), Pt::new(1.0, 1.0), Pt::new(2.0, 2.0)]).is_none(), "a straight line is no arc");
    }
}
