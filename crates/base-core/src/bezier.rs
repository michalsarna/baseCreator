//! Editable Bézier contours (the model behind node editing).
use crate::geom::*;
use serde::{Deserialize, Serialize};

/// A node with optional cubic handles. A handle equal to the node position means "no handle".
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub p: Pt,
    /// Incoming handle (control point of the segment that ends here).
    pub hin: Pt,
    /// Outgoing handle (control point of the segment that starts here).
    pub hout: Pt,
    /// Smooth nodes keep their two handles collinear while one is dragged.
    #[serde(default)]
    pub smooth: bool,
}

impl Node {
    pub fn corner(p: Pt) -> Node {
        Node { p, hin: p, hout: p, smooth: false }
    }
    pub fn has_in(&self) -> bool {
        self.hin.dist(self.p) > 1e-9
    }
    pub fn has_out(&self) -> bool {
        self.hout.dist(self.p) > 1e-9
    }
    pub fn transformed(&self, xf: &Xf) -> Node {
        Node { p: xf.apply(self.p), hin: xf.apply(self.hin), hout: xf.apply(self.hout), smooth: self.smooth }
    }
    pub fn translate(&mut self, dx: f64, dy: f64) {
        for q in [&mut self.p, &mut self.hin, &mut self.hout] {
            q.x += dx;
            q.y += dy;
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Contour {
    pub nodes: Vec<Node>,
    pub closed: bool,
}

fn lerp(a: Pt, b: Pt, t: f64) -> Pt {
    Pt::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// Control points of the cubic equivalent of a circular-ish quarter: kappa for ellipses.
const KAPPA: f64 = 0.552_284_749_830_793_4;

impl Contour {
    pub fn from_polyline(p: &Polyline) -> Contour {
        Contour { nodes: p.pts.iter().map(|q| Node::corner(*q)).collect(), closed: p.closed }
    }

    pub fn rect(w: f64, h: f64) -> Contour {
        let pts = [Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)];
        Contour { nodes: pts.iter().map(|p| Node::corner(*p)).collect(), closed: true }
    }

    pub fn ellipse(w: f64, h: f64) -> Contour {
        let (rx, ry) = (w / 2.0, h / 2.0);
        let (cx, cy) = (rx, ry);
        let node = |p: Pt, tin: Pt, tout: Pt| Node { p, hin: tin, hout: tout, smooth: true };
        let (kx, ky) = (rx * KAPPA, ry * KAPPA);
        Contour {
            nodes: vec![
                node(Pt::new(cx + rx, cy), Pt::new(cx + rx, cy - ky), Pt::new(cx + rx, cy + ky)),
                node(Pt::new(cx, cy + ry), Pt::new(cx + kx, cy + ry), Pt::new(cx - kx, cy + ry)),
                node(Pt::new(cx - rx, cy), Pt::new(cx - rx, cy + ky), Pt::new(cx - rx, cy - ky)),
                node(Pt::new(cx, cy - ry), Pt::new(cx - kx, cy - ry), Pt::new(cx + kx, cy - ry)),
            ],
            closed: true,
        }
    }

    /// Isosceles triangle filling a `w` x `h` box, apex at the top centre.
    pub fn triangle(w: f64, h: f64) -> Contour {
        let pts = [Pt::new(w / 2.0, 0.0), Pt::new(w, h), Pt::new(0.0, h)];
        Contour { nodes: pts.iter().map(|p| Node::corner(*p)).collect(), closed: true }
    }

    /// Regular polygon with `sides` corners inscribed in the ellipse of a `w` x `h` box; first corner at the top.
    pub fn polygon(sides: u32, w: f64, h: f64) -> Contour {
        let n = sides.clamp(3, 360) as usize;
        let (cx, cy, rx, ry) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
        let nodes = (0..n)
            .map(|i| {
                let a = -std::f64::consts::FRAC_PI_2 + i as f64 / n as f64 * std::f64::consts::TAU;
                Node::corner(Pt::new(cx + rx * a.cos(), cy + ry * a.sin()))
            })
            .collect();
        Contour { nodes, closed: true }
    }

    /// Open Archimedean spiral of `turns` turns that fills the ellipse of a `w` x `h` box, starting in its middle.
    pub fn spiral(turns: u32, w: f64, h: f64) -> Contour {
        let turns = turns.clamp(1, 60) as f64;
        let (cx, cy, rx, ry) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
        let total = turns * std::f64::consts::TAU;
        // About 12 curve pieces per turn keep the spiral smooth.
        let n = (turns as usize * 12).max(6);
        let step = total / n as f64;
        // Position and velocity (per radian) at angle t, with the radius growing linearly to 1.
        let at = |t: f64| {
            let r = t / total;
            let dr = 1.0 / total;
            let a = t - std::f64::consts::FRAC_PI_2;
            let p = Pt::new(cx + rx * r * a.cos(), cy + ry * r * a.sin());
            let v = Pt::new(rx * (dr * a.cos() - r * a.sin()), ry * (dr * a.sin() + r * a.cos()));
            (p, v)
        };
        let nodes = (0..=n)
            .map(|i| {
                let (p, v) = at(i as f64 * step);
                let k = step / 3.0;
                Node { p, hin: Pt::new(p.x - v.x * k, p.y - v.y * k), hout: Pt::new(p.x + v.x * k, p.y + v.y * k), smooth: true }
            })
            .collect();
        Contour { nodes, closed: false }
    }

    /// Star with `points` tips; the inner corners sit at `inner` (0..1) of the outer radius.
    pub fn star(points: u32, inner: f64, w: f64, h: f64) -> Contour {
        let n = points.clamp(3, 360) as usize;
        let (cx, cy, rx, ry) = (w / 2.0, h / 2.0, w / 2.0, h / 2.0);
        let inner = inner.clamp(0.05, 0.95);
        let nodes = (0..n * 2)
            .map(|i| {
                let a = -std::f64::consts::FRAC_PI_2 + i as f64 / (2 * n) as f64 * std::f64::consts::TAU;
                let k = if i % 2 == 0 { 1.0 } else { inner };
                Node::corner(Pt::new(cx + rx * k * a.cos(), cy + ry * k * a.sin()))
            })
            .collect();
        Contour { nodes, closed: true }
    }

    /// A heart filling a `w` x `h` box: two lobes on top, a point at the bottom.
    pub fn heart(w: f64, h: f64) -> Contour {
        let p = |x: f64, y: f64| Pt::new(x * w, y * h);
        let n = |at: Pt, hin: Pt, hout: Pt, smooth: bool| Node { p: at, hin, hout, smooth };
        let (dip, tip) = (p(0.5, 0.28), p(0.5, 1.0));
        Contour {
            nodes: vec![
                n(dip, p(0.5, 0.06), p(0.5, 0.06), false),
                n(p(0.25, 0.0), p(0.40, 0.0), p(0.10, 0.0), true),
                n(p(0.0, 0.28), p(0.0, 0.08), p(0.0, 0.55), true),
                n(tip, p(0.30, 0.80), p(0.70, 0.80), false),
                n(p(1.0, 0.28), p(1.0, 0.55), p(1.0, 0.08), true),
                n(p(0.75, 0.0), p(0.90, 0.0), p(0.60, 0.0), true),
            ],
            closed: true,
        }
    }

    pub fn transformed(&self, xf: &Xf) -> Contour {
        Contour { nodes: self.nodes.iter().map(|n| n.transformed(xf)).collect(), closed: self.closed }
    }

    pub fn seg_count(&self) -> usize {
        let n = self.nodes.len();
        if n < 2 {
            0
        } else if self.closed {
            n
        } else {
            n - 1
        }
    }

    /// Control points (p0, c1, c2, p3) of segment `i`, from node `i` to node `i + 1`.
    pub fn seg(&self, i: usize) -> [Pt; 4] {
        let (a, b) = (&self.nodes[i], &self.nodes[(i + 1) % self.nodes.len()]);
        [a.p, a.hout, b.hin, b.p]
    }

    pub fn seg_is_curve(&self, i: usize) -> bool {
        let (a, b) = (&self.nodes[i], &self.nodes[(i + 1) % self.nodes.len()]);
        a.has_out() || b.has_in()
    }

    pub fn flatten(&self, tol: f64) -> Polyline {
        let mut pts = Vec::new();
        if let Some(first) = self.nodes.first() {
            pts.push(first.p);
        }
        for i in 0..self.seg_count() {
            let [p0, c1, c2, p3] = self.seg(i);
            if self.seg_is_curve(i) {
                flatten_cubic(p0, c1, c2, p3, tol, &mut pts);
            } else {
                pts.push(p3);
            }
        }
        if self.closed && pts.len() > 2 && pts[0].dist(*pts.last().unwrap_or(&pts[0])) < 1e-9 {
            pts.pop();
        }
        Polyline::new(pts, self.closed)
    }

    pub fn point_at(&self, seg: usize, t: f64) -> Pt {
        let [p0, c1, c2, p3] = self.seg(seg);
        let (u, t2) = (1.0 - t, t * t);
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t2, t2 * t);
        Pt::new(a * p0.x + b * c1.x + c * c2.x + d * p3.x, a * p0.y + b * c1.y + c * c2.y + d * p3.y)
    }

    /// Closest point on the contour: (segment, t, distance).
    pub fn nearest(&self, q: Pt) -> Option<(usize, f64, f64)> {
        let mut best: Option<(usize, f64, f64)> = None;
        for i in 0..self.seg_count() {
            let steps = if self.seg_is_curve(i) { 24 } else { 1 };
            let mut prev = self.point_at(i, 0.0);
            for k in 1..=steps {
                let t1 = k as f64 / steps as f64;
                let cur = self.point_at(i, t1);
                let (dx, dy) = (cur.x - prev.x, cur.y - prev.y);
                let l2 = dx * dx + dy * dy;
                let s = if l2 == 0.0 { 0.0 } else { (((q.x - prev.x) * dx + (q.y - prev.y) * dy) / l2).clamp(0.0, 1.0) };
                let on = Pt::new(prev.x + s * dx, prev.y + s * dy);
                let d = on.dist(q);
                if best.map_or(true, |b| d < b.2) {
                    let t0 = (k - 1) as f64 / steps as f64;
                    best = Some((i, t0 + s * (t1 - t0), d));
                }
                prev = cur;
            }
        }
        best
    }

    /// Insert a node on segment `seg` at parameter `t` without changing the shape. Returns the new node index.
    pub fn insert_node(&mut self, seg: usize, t: f64) -> usize {
        let [p0, c1, c2, p3] = self.seg(seg);
        let curve = self.seg_is_curve(seg);
        let n = self.nodes.len();
        let next = (seg + 1) % n;
        let node = if curve {
            let (a, b, c) = (lerp(p0, c1, t), lerp(c1, c2, t), lerp(c2, p3, t));
            let (d, e) = (lerp(a, b, t), lerp(b, c, t));
            let m = lerp(d, e, t);
            self.nodes[seg].hout = a;
            self.nodes[next].hin = c;
            Node { p: m, hin: d, hout: e, smooth: true }
        } else {
            Node::corner(lerp(p0, p3, t))
        };
        let at = seg + 1;
        self.nodes.insert(at, node);
        at
    }

    /// Remove node `i`, keeping the neighbours' handles (the curve is approximated by the remaining segment).
    pub fn remove_node(&mut self, i: usize) {
        if i < self.nodes.len() {
            self.nodes.remove(i);
        }
    }

    pub fn make_corner(&mut self, i: usize) {
        self.nodes[i].smooth = false;
    }

    /// Give node `i` collinear handles along the direction between its neighbours.
    pub fn make_smooth(&mut self, i: usize) {
        let n = self.nodes.len();
        if n < 2 {
            return;
        }
        let prev = if i > 0 || self.closed { Some(self.nodes[(i + n - 1) % n].p) } else { None };
        let next = if i + 1 < n || self.closed { Some(self.nodes[(i + 1) % n].p) } else { None };
        let p = self.nodes[i].p;
        let dir = match (prev, next) {
            (Some(a), Some(b)) => Pt::new(b.x - a.x, b.y - a.y),
            (None, Some(b)) => Pt::new(b.x - p.x, b.y - p.y),
            (Some(a), None) => Pt::new(p.x - a.x, p.y - a.y),
            _ => return,
        };
        let len = (dir.x * dir.x + dir.y * dir.y).sqrt();
        if len < 1e-9 {
            return;
        }
        let u = Pt::new(dir.x / len, dir.y / len);
        let node = &mut self.nodes[i];
        node.smooth = true;
        if let Some(a) = prev {
            let l = p.dist(a) / 3.0;
            node.hin = Pt::new(p.x - u.x * l, p.y - u.y * l);
        }
        if let Some(b) = next {
            let l = p.dist(b) / 3.0;
            node.hout = Pt::new(p.x + u.x * l, p.y + u.y * l);
        }
    }

    /// Turn segment `seg` into a straight line.
    pub fn segment_to_line(&mut self, seg: usize) {
        let n = self.nodes.len();
        let next = (seg + 1) % n;
        self.nodes[seg].hout = self.nodes[seg].p;
        self.nodes[next].hin = self.nodes[next].p;
    }

    /// Turn segment `seg` into a curve with handles at a third of its length.
    pub fn segment_to_curve(&mut self, seg: usize) {
        let n = self.nodes.len();
        let next = (seg + 1) % n;
        let (a, b) = (self.nodes[seg].p, self.nodes[next].p);
        self.nodes[seg].hout = lerp(a, b, 1.0 / 3.0);
        self.nodes[next].hin = lerp(a, b, 2.0 / 3.0);
    }

    /// Move a handle of node `i`. For smooth nodes the opposite handle follows, keeping its length.
    pub fn drag_handle(&mut self, i: usize, out: bool, to: Pt) {
        let node = &mut self.nodes[i];
        let p = node.p;
        if out {
            node.hout = to;
        } else {
            node.hin = to;
        }
        if node.smooth {
            let (dx, dy) = (to.x - p.x, to.y - p.y);
            let l = (dx * dx + dy * dy).sqrt();
            if l > 1e-9 {
                let other = if out { node.hin } else { node.hout };
                let ol = other.dist(p);
                let ol = if ol < 1e-9 { l } else { ol };
                let q = Pt::new(p.x - dx / l * ol, p.y - dy / l * ol);
                if out {
                    node.hin = q;
                } else {
                    node.hout = q;
                }
            }
        }
    }
}
