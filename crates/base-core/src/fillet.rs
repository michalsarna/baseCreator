//! Rounding corners: between two straight segments a circular arc of a given radius replaces the sharp corner.
use crate::bezier::{Contour, Node};
use crate::geom::*;
use std::collections::HashSet;

fn sub(a: Pt, b: Pt) -> Pt {
    Pt::new(a.x - b.x, a.y - b.y)
}
fn len(a: Pt) -> f64 {
    (a.x * a.x + a.y * a.y).sqrt()
}
fn unit(a: Pt) -> Option<Pt> {
    let l = len(a);
    (l > 1e-12).then(|| Pt::new(a.x / l, a.y / l))
}

/// Nodes replacing the corner at `p` between `prev` and `next` (all original positions), or `None` when the
/// corner is (almost) straight. `max_d` limits how far the arc may start from the corner.
/// Returns the nodes along the arc: the first and last sit on the two segments.
fn round_corner(prev: Pt, p: Pt, next: Pt, radius: f64, max_d: f64) -> Option<Vec<Node>> {
    let (u, v) = (unit(sub(prev, p))?, unit(sub(next, p))?);
    let cos = (u.x * v.x + u.y * v.y).clamp(-1.0, 1.0);
    let theta = cos.acos();
    // Straight through (theta ~ pi) or folded back (theta ~ 0): nothing to round.
    if theta > std::f64::consts::PI - 1e-3 || theta < 1e-3 || radius <= 0.0 {
        return None;
    }
    let half = theta / 2.0;
    let mut d = radius / half.tan();
    let mut r = radius;
    if d > max_d {
        d = max_d;
        r = d * half.tan();
    }
    if d < 1e-9 || r < 1e-9 {
        return None;
    }
    let (t1, t2) = (Pt::new(p.x + u.x * d, p.y + u.y * d), Pt::new(p.x + v.x * d, p.y + v.y * d));
    let bis = unit(Pt::new(u.x + v.x, u.y + v.y))?;
    let dc = r / half.sin();
    let c = Pt::new(p.x + bis.x * dc, p.y + bis.y * dc);
    let a1 = (t1.y - c.y).atan2(t1.x - c.x);
    let a2 = (t2.y - c.y).atan2(t2.x - c.x);
    // Shortest way round, with sign.
    let mut sweep = a2 - a1;
    while sweep > std::f64::consts::PI {
        sweep -= std::f64::consts::TAU;
    }
    while sweep < -std::f64::consts::PI {
        sweep += std::f64::consts::TAU;
    }
    let n = ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan() * r;
    let on = |a: f64| Pt::new(c.x + r * a.cos(), c.y + r * a.sin());
    let tan = |a: f64| Pt::new(-a.sin(), a.cos());
    let mut nodes: Vec<Node> = Vec::with_capacity(n + 1);
    for i in 0..=n {
        let a = a1 + step * i as f64;
        let pt = if i == 0 { t1 } else if i == n { t2 } else { on(a) };
        let mut node = Node::corner(pt);
        if i < n {
            let t = tan(a);
            node.hout = Pt::new(pt.x + t.x * k, pt.y + t.y * k);
        }
        if i > 0 {
            let t = tan(a);
            node.hin = Pt::new(pt.x - t.x * k, pt.y - t.y * k);
        }
        node.smooth = i > 0 && i < n;
        nodes.push(node);
    }
    Some(nodes)
}

/// Round the straight corners of `c`. `only` limits it to those node indices. Returns how many corners changed.
pub fn round_contour(c: &Contour, radius: f64, only: Option<&HashSet<usize>>) -> (Contour, usize) {
    let n = c.nodes.len();
    if n < 3 || radius <= 0.0 {
        return (c.clone(), 0);
    }
    let straight = |a: usize, b: usize| !c.nodes[a].has_out() && !c.nodes[b].has_in();
    let is_end = |i: usize| !c.closed && (i == 0 || i == n - 1);
    let mut out: Vec<Node> = Vec::with_capacity(n * 2);
    let mut changed = 0;
    for i in 0..n {
        let (pi, ni) = ((i + n - 1) % n, (i + 1) % n);
        let eligible = !is_end(i) && only.map_or(true, |s| s.contains(&i)) && straight(pi, i) && straight(i, ni) && !c.nodes[i].smooth;
        if eligible {
            let (prev, p, next) = (c.nodes[pi].p, c.nodes[i].p, c.nodes[ni].p);
            // Neighbouring rounded corners may each use half of a shared segment; an open end may use all of it.
            let share = |other: usize, l: f64| if is_end(other) { l } else { l / 2.0 };
            let max_d = share(pi, p.dist(prev)).min(share(ni, p.dist(next)));
            if let Some(arc) = round_corner(prev, p, next, radius, max_d) {
                out.extend(arc);
                changed += 1;
                continue;
            }
        }
        out.push(c.nodes[i]);
    }
    (Contour { nodes: out, closed: c.closed }, changed)
}

/// Join two straight lines with a rounded corner. Each line is `(a, b)`. The lines are extended or trimmed to
/// meet; on each line the longer part beyond the meeting point is kept. `None` for parallel lines or a radius
/// that does not fit.
pub fn fillet_lines(l1: (Pt, Pt), l2: (Pt, Pt), radius: f64) -> Option<Contour> {
    let (d1, d2) = (sub(l1.1, l1.0), sub(l2.1, l2.0));
    let det = d1.x * d2.y - d1.y * d2.x;
    if det.abs() < 1e-12 {
        return None;
    }
    // Intersection of the infinite lines: l1.0 + t d1 = l2.0 + s d2.
    let w = sub(l2.0, l1.0);
    let t = (w.x * d2.y - w.y * d2.x) / det;
    let p = Pt::new(l1.0.x + d1.x * t, l1.0.y + d1.y * t);
    let far = |l: (Pt, Pt)| if l.0.dist(p) >= l.1.dist(p) { l.0 } else { l.1 };
    let (fa, fb) = (far(l1), far(l2));
    if fa.dist(p) < 1e-9 || fb.dist(p) < 1e-9 {
        return None;
    }
    let max_d = fa.dist(p).min(fb.dist(p));
    let arc = round_corner(fa, p, fb, radius, max_d)?;
    let mut nodes = vec![Node::corner(fa)];
    nodes.extend(arc);
    nodes.push(Node::corner(fb));
    Some(Contour { nodes, closed: false })
}

/// Is the shape a single straight segment? Returns its end points.
pub fn as_segment(contours: &[Contour]) -> Option<(Pt, Pt)> {
    match contours {
        [c] if !c.closed && c.nodes.len() == 2 && !c.nodes[0].has_out() && !c.nodes[1].has_in() => Some((c.nodes[0].p, c.nodes[1].p)),
        _ => None,
    }
}
