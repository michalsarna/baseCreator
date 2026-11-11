//! No-go zones of a machine: parts of the work area the tool must never enter (a clamp, a fixture, a mirror), as rectangles and
//! circles in the coordinates of the design. A zone may stand only between two heights (a box on the table): what moves above or
//! below it is free. The programs route their moves through the air around the zones ([`route`]) and report a cut that touches one.
use crate::geom::{Pt, Rect};
use serde::{Deserialize, Serialize};

/// The shape of a no-go zone, in design millimetres.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ZoneShape {
    /// A rectangle: top-left corner and size.
    Rect { x: f64, y: f64, w: f64, h: f64 },
    /// A circle: centre and radius.
    Circle { x: f64, y: f64, r: f64 },
}

/// A no-go zone of a device.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub name: String,
    #[serde(default = "d_true")]
    pub enabled: bool,
    pub shape: ZoneShape,
    /// With a Z axis: the zone only stops what happens at a height inside this range (both ends included), like a box or a
    /// cylinder standing on the table. `None` = at every height.
    #[serde(default)]
    pub z_range: Option<(f64, f64)>,
}

fn d_true() -> bool {
    true
}

impl Zone {
    /// Is the zone in the way of something between the heights `lo` and `hi`?
    pub fn at_heights(&self, lo: f64, hi: f64) -> bool {
        match self.z_range {
            Some((a, b)) => lo.min(hi) <= a.max(b) + 1e-9 && hi.max(lo) >= a.min(b) - 1e-9,
            None => true,
        }
    }

    /// Are the numbers usable (from a file)?
    pub fn check(&self) -> Result<(), String> {
        let ok = |v: f64| v.is_finite() && (-100_000.0..=100_000.0).contains(&v);
        let shape = match self.shape {
            ZoneShape::Rect { x, y, w, h } => [x, y, w, h].iter().all(|v| ok(*v)),
            ZoneShape::Circle { x, y, r } => [x, y, r].iter().all(|v| ok(*v)),
        };
        let z = self.z_range.is_none_or(|(a, b)| ok(a) && ok(b));
        if !shape || !z || self.name.chars().count() > 100 {
            return Err("a no-go zone is out of range".into());
        }
        Ok(())
    }
}

/// Are the zones of a device usable?
pub fn check_all(zones: &[Zone]) -> Result<(), String> {
    if zones.len() > 100 {
        return Err("too many no-go zones".into());
    }
    zones.iter().try_for_each(Zone::check)
}

impl ZoneShape {
    /// The smallest rectangle that holds the zone.
    pub fn bounds(&self) -> Rect {
        match *self {
            ZoneShape::Rect { x, y, w, h } => Rect { min: Pt::new(x.min(x + w), y.min(y + h)), max: Pt::new(x.max(x + w), y.max(y + h)) },
            ZoneShape::Circle { x, y, r } => Rect { min: Pt::new(x - r.abs(), y - r.abs()), max: Pt::new(x + r.abs(), y + r.abs()) },
        }
    }

    /// The zone grown by `margin` as a convex polygon (anticlockwise with y up), around the circle for a round zone.
    pub fn polygon(&self, margin: f64) -> Vec<Pt> {
        let m = margin.max(0.0);
        match *self {
            ZoneShape::Rect { .. } => {
                let b = self.bounds();
                // The corners are cut a little, as the edge of a round tool would go round them.
                let (x0, y0, x1, y1) = (b.min.x - m, b.min.y - m, b.max.x + m, b.max.y + m);
                vec![Pt::new(x0, y0), Pt::new(x1, y0), Pt::new(x1, y1), Pt::new(x0, y1)]
            }
            ZoneShape::Circle { x, y, r } => {
                // A polygon round the circle (its edges touch the circle grown by the margin).
                let n = 16;
                let rr = (r.abs() + m) / (std::f64::consts::PI / n as f64).cos();
                (0..n).map(|i| {
                    let a = (i as f64 + 0.5) / n as f64 * std::f64::consts::TAU;
                    Pt::new(x + rr * a.cos(), y + rr * a.sin())
                }).collect()
            }
        }
    }
}

/// Does the straight move `a` to `b` touch the zone (its inside or its edge)?
pub fn segment_hits(shape: &ZoneShape, a: Pt, b: Pt) -> bool {
    match *shape {
        ZoneShape::Rect { x, y, w, h } => {
            // Clip the segment to the rectangle (Liang-Barsky); what is left means it touches.
            let (x0, x1) = (x.min(x + w), x.max(x + w));
            let (y0, y1) = (y.min(y + h), y.max(y + h));
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let (mut t0, mut t1) = (0.0f64, 1.0f64);
            for (p, q) in [(-dx, a.x - x0), (dx, x1 - a.x), (-dy, a.y - y0), (dy, y1 - a.y)] {
                if p == 0.0 {
                    if q < 0.0 {
                        return false;
                    }
                } else {
                    let r = q / p;
                    if p < 0.0 {
                        if r > t1 {
                            return false;
                        }
                        t0 = t0.max(r);
                    } else {
                        if r < t0 {
                            return false;
                        }
                        t1 = t1.min(r);
                    }
                }
            }
            t0 <= t1
        }
        ZoneShape::Circle { x, y, r } => dist_to_segment(Pt::new(x, y), a, b) <= r.abs(),
    }
}

/// Does the straight move `a` to `b` touch the zone grown by `margin`?
pub fn segment_hits_with(shape: &ZoneShape, a: Pt, b: Pt, margin: f64) -> bool {
    match *shape {
        ZoneShape::Rect { .. } => {
            let b0 = shape.bounds();
            let m = margin.max(0.0);
            segment_hits(&ZoneShape::Rect { x: b0.min.x - m, y: b0.min.y - m, w: b0.width() + 2.0 * m, h: b0.height() + 2.0 * m }, a, b)
        }
        ZoneShape::Circle { x, y, r } => dist_to_segment(Pt::new(x, y), a, b) <= r.abs() + margin.max(0.0),
    }
}

fn dist_to_segment(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len2 = dx * dx + dy * dy;
    if len2 < 1e-18 {
        return p.dist(a);
    }
    let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0);
    p.dist(Pt::new(a.x + t * dx, a.y + t * dy))
}

/// Does the segment go through the inside of the convex polygon (more than touching its edge or a corner)?
fn crosses(poly: &[Pt], a: Pt, b: Pt) -> bool {
    // Cyrus-Beck against the polygon shrunk by a hair, so that a way along an edge or through a corner is free.
    let n = poly.len();
    let area: f64 = (0..n).map(|i| poly[i].x * poly[(i + 1) % n].y - poly[(i + 1) % n].x * poly[i].y).sum();
    let s = if area >= 0.0 { 1.0 } else { -1.0 };
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let d = Pt::new(b.x - a.x, b.y - a.y);
    for i in 0..n {
        let (p, q) = (poly[i], poly[(i + 1) % n]);
        // The inward normal of the edge.
        let nx = -(q.y - p.y) * s;
        let ny = (q.x - p.x) * s;
        let len = (nx * nx + ny * ny).sqrt();
        if len < 1e-12 {
            continue;
        }
        let (nx, ny) = (nx / len, ny / len);
        // Inside when n · (x - p) > eps.
        let num = nx * (a.x - p.x) + ny * (a.y - p.y) - 1e-6;
        let den = nx * d.x + ny * d.y;
        if den.abs() < 1e-15 {
            if num < 0.0 {
                return false;
            }
        } else {
            let t = -num / den;
            if den > 0.0 {
                t0 = t0.max(t);
            } else {
                t1 = t1.min(t);
            }
        }
        if t0 > t1 {
            return false;
        }
    }
    t1 - t0 > 1e-9
}

fn inside(poly: &[Pt], p: Pt) -> bool {
    crosses(poly, p, p) || {
        let n = poly.len();
        let mut c = false;
        let mut j = n - 1;
        for i in 0..n {
            let (a, b) = (poly[i], poly[j]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                c = !c;
            }
            j = i;
        }
        c
    }
}

/// The way from `a` to `b` around the zones (each grown by `margin`): the points to go through in between (none when the straight
/// way is free). `None` when `a` or `b` lies in a zone, or no way round is found.
pub fn route(zones: &[ZoneShape], a: Pt, b: Pt, margin: f64) -> Option<Vec<Pt>> {
    let polys: Vec<Vec<Pt>> = zones.iter().map(|z| z.polygon(margin)).collect();
    if polys.iter().any(|p| inside(p, a) || inside(p, b)) {
        return None;
    }
    let free = |p: Pt, q: Pt| !polys.iter().any(|poly| crosses(poly, p, q));
    if free(a, b) {
        return Some(vec![]);
    }
    // The corners of the grown zones, pushed out a hair, that are not inside another zone.
    let mut nodes = vec![a, b];
    for (i, poly) in polys.iter().enumerate() {
        let c = Pt::new(poly.iter().map(|p| p.x).sum::<f64>() / poly.len() as f64, poly.iter().map(|p| p.y).sum::<f64>() / poly.len() as f64);
        for p in poly {
            let d = p.dist(c).max(1e-9);
            let q = Pt::new(p.x + (p.x - c.x) / d * 1e-4, p.y + (p.y - c.y) / d * 1e-4);
            if !polys.iter().enumerate().any(|(j, o)| j != i && inside(o, q)) {
                nodes.push(q);
            }
        }
    }
    // Dijkstra over the ways between them that cross no zone.
    let n = nodes.len();
    let mut dist = vec![f64::INFINITY; n];
    let mut prev = vec![usize::MAX; n];
    let mut done = vec![false; n];
    dist[0] = 0.0;
    loop {
        let Some(u) = (0..n).filter(|&i| !done[i] && dist[i].is_finite()).min_by(|&i, &j| dist[i].total_cmp(&dist[j])) else { break };
        if u == 1 {
            break;
        }
        done[u] = true;
        for v in 0..n {
            if done[v] || v == u {
                continue;
            }
            let w = dist[u] + nodes[u].dist(nodes[v]);
            if w < dist[v] && free(nodes[u], nodes[v]) {
                dist[v] = w;
                prev[v] = u;
            }
        }
    }
    if !dist[1].is_finite() {
        return None;
    }
    let mut way = vec![];
    let mut at = prev[1];
    while at != 0 && at != usize::MAX {
        way.push(nodes[at]);
        at = prev[at];
    }
    way.reverse();
    Some(way)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect_zone() -> ZoneShape {
        ZoneShape::Rect { x: 10.0, y: 10.0, w: 20.0, h: 10.0 } // (10,10) to (30,20)
    }

    #[test]
    fn segments_against_a_rectangular_zone_including_the_edge_cases() {
        let z = rect_zone();
        let hit = |a: (f64, f64), b: (f64, f64)| segment_hits(&z, Pt::new(a.0, a.1), Pt::new(b.0, b.1));
        assert!(!hit((0.0, 0.0), (5.0, 5.0)), "far away");
        assert!(!hit((0.0, 25.0), (40.0, 25.0)), "passes below");
        assert!(hit((0.0, 15.0), (40.0, 15.0)), "straight through");
        assert!(hit((15.0, 12.0), (25.0, 18.0)), "completely inside");
        assert!(hit((0.0, 10.0), (40.0, 10.0)), "runs along the edge");
        assert!(!hit((0.0, 0.0), (9.9, 9.9)), "stops short of the corner");
        assert!(segment_hits(&ZoneShape::Rect { x: 30.0, y: 20.0, w: -20.0, h: -10.0 }, Pt::new(0.0, 15.0), Pt::new(40.0, 15.0)));
        assert!(segment_hits_with(&z, Pt::new(0.0, 22.0), Pt::new(40.0, 22.0), 3.0));
        assert!(!segment_hits_with(&z, Pt::new(0.0, 24.0), Pt::new(40.0, 24.0), 3.0));
    }

    #[test]
    fn a_way_goes_round_a_zone_and_stays_out_of_it() {
        let z = [rect_zone()];
        let (a, b) = (Pt::new(0.0, 15.0), Pt::new(40.0, 15.0));
        let way = route(&z, a, b, 2.0).unwrap();
        assert_eq!(way.len(), 2, "{way:?}");
        let mut pts = vec![a];
        pts.extend(&way);
        pts.push(b);
        for w in pts.windows(2) {
            assert!(!segment_hits_with(&z[0], w[0], w[1], 1.99), "{w:?}");
        }
        // The short way: over the side nearest to the line (the zone is 10 high, 20 long).
        let len: f64 = pts.windows(2).map(|w| w[0].dist(w[1])).sum();
        assert!(len < 40.0 + 2.0 * 9.0, "{len}");
        // Free: no points; from inside: none.
        assert_eq!(route(&z, Pt::new(0.0, 0.0), Pt::new(40.0, 0.0), 2.0), Some(vec![]));
        assert_eq!(route(&z, Pt::new(20.0, 15.0), Pt::new(40.0, 0.0), 2.0), None);
    }

    #[test]
    fn a_way_goes_between_several_zones_and_round_a_circle() {
        let z = [ZoneShape::Rect { x: 20.0, y: -50.0, w: 10.0, h: 60.0 }, ZoneShape::Rect { x: 50.0, y: -10.0, w: 10.0, h: 60.0 }, ZoneShape::Circle { x: 80.0, y: 0.0, r: 5.0 }];
        let (a, b) = (Pt::new(0.0, 0.0), Pt::new(100.0, 0.0));
        let way = route(&z, a, b, 1.0).unwrap();
        let mut pts = vec![a];
        pts.extend(&way);
        pts.push(b);
        for w in pts.windows(2) {
            for zone in &z {
                assert!(!segment_hits_with(zone, w[0], w[1], 0.99), "{w:?} {zone:?}");
            }
        }
        // Walled in: no way.
        let walls = [ZoneShape::Rect { x: -20.0, y: -20.0, w: 40.0, h: 5.0 }, ZoneShape::Rect { x: -20.0, y: 15.0, w: 40.0, h: 5.0 }, ZoneShape::Rect { x: -20.0, y: -20.0, w: 5.0, h: 40.0 }, ZoneShape::Rect { x: 15.0, y: -20.0, w: 5.0, h: 40.0 }];
        assert_eq!(route(&walls, Pt::new(0.0, 0.0), Pt::new(100.0, 0.0), 0.5), None);
    }

    #[test]
    fn heights_and_the_check() {
        let z = Zone { name: "Clamp".into(), enabled: true, shape: rect_zone(), z_range: Some((0.0, 15.0)) };
        assert!(z.at_heights(5.0, 5.0) && z.at_heights(-3.0, f64::INFINITY) && !z.at_heights(-3.0, -1.0) && !z.at_heights(16.0, 20.0));
        assert!(Zone { z_range: None, ..z.clone() }.at_heights(100.0, 100.0));
        assert!(check_all(&[z.clone()]).is_ok());
        assert!(Zone { shape: ZoneShape::Circle { x: f64::NAN, y: 0.0, r: 1.0 }, ..z }.check().is_err());
        assert!(ZoneShape::Circle { x: 50.0, y: 50.0, r: 10.0 }.polygon(0.0).iter().all(|p| p.dist(Pt::new(50.0, 50.0)) >= 10.0));
    }
}
