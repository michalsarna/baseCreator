//! Basic 2D geometry in millimetres. Y grows downwards (screen convention).
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Pt {
    pub x: f64,
    pub y: f64,
}

impl Pt {
    pub const fn new(x: f64, y: f64) -> Self {
        Self { x, y }
    }
    pub fn dist(self, o: Pt) -> f64 {
        ((self.x - o.x).powi(2) + (self.y - o.y).powi(2)).sqrt()
    }
}

/// Affine transform: x' = a*x + c*y + e ; y' = b*x + d*y + f
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Xf {
    pub a: f64,
    pub b: f64,
    pub c: f64,
    pub d: f64,
    pub e: f64,
    pub f: f64,
}

impl Default for Xf {
    fn default() -> Self {
        Self::IDENTITY
    }
}

impl Xf {
    pub const IDENTITY: Xf = Xf { a: 1.0, b: 0.0, c: 0.0, d: 1.0, e: 0.0, f: 0.0 };

    pub fn translate(x: f64, y: f64) -> Self {
        Xf { e: x, f: y, ..Self::IDENTITY }
    }
    pub fn scale(sx: f64, sy: f64) -> Self {
        Xf { a: sx, d: sy, ..Self::IDENTITY }
    }
    pub fn rotate(rad: f64) -> Self {
        let (s, c) = rad.sin_cos();
        Xf { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }
    }
    /// Apply `self` first, then `next`.
    pub fn then(self, n: Xf) -> Xf {
        Xf {
            a: n.a * self.a + n.c * self.b,
            b: n.b * self.a + n.d * self.b,
            c: n.a * self.c + n.c * self.d,
            d: n.b * self.c + n.d * self.d,
            e: n.a * self.e + n.c * self.f + n.e,
            f: n.b * self.e + n.d * self.f + n.f,
        }
    }
    pub fn apply(&self, p: Pt) -> Pt {
        Pt::new(self.a * p.x + self.c * p.y + self.e, self.b * p.x + self.d * p.y + self.f)
    }
    pub fn det(&self) -> f64 {
        self.a * self.d - self.b * self.c
    }
    pub fn inverse(&self) -> Option<Xf> {
        let det = self.det();
        if det.abs() < 1e-12 {
            return None;
        }
        let (a, b, c, d) = (self.d / det, -self.b / det, -self.c / det, self.a / det);
        Some(Xf { a, b, c, d, e: -(a * self.e + c * self.f), f: -(b * self.e + d * self.f) })
    }
    /// Rotation about a point.
    pub fn rotate_about(rad: f64, o: Pt) -> Xf {
        Xf::translate(-o.x, -o.y).then(Xf::rotate(rad)).then(Xf::translate(o.x, o.y))
    }
    /// Shear about a point: `kx` shifts x by `kx` per unit of distance in y, `ky` shifts y per unit of distance in x.
    pub fn skew_about(kx: f64, ky: f64, o: Pt) -> Xf {
        Xf::translate(-o.x, -o.y).then(Xf { a: 1.0, b: ky, c: kx, d: 1.0, e: 0.0, f: 0.0 }).then(Xf::translate(o.x, o.y))
    }
    /// Scale about a point.
    pub fn scale_about(sx: f64, sy: f64, o: Pt) -> Xf {
        Xf::translate(-o.x, -o.y).then(Xf::scale(sx, sy)).then(Xf::translate(o.x, o.y))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub min: Pt,
    pub max: Pt,
}

impl Rect {
    pub fn from_pts(pts: impl IntoIterator<Item = Pt>) -> Option<Rect> {
        let mut it = pts.into_iter();
        let first = it.next()?;
        let mut r = Rect { min: first, max: first };
        for p in it {
            r.min.x = r.min.x.min(p.x);
            r.min.y = r.min.y.min(p.y);
            r.max.x = r.max.x.max(p.x);
            r.max.y = r.max.y.max(p.y);
        }
        Some(r)
    }
    pub fn union(self, o: Rect) -> Rect {
        Rect {
            min: Pt::new(self.min.x.min(o.min.x), self.min.y.min(o.min.y)),
            max: Pt::new(self.max.x.max(o.max.x), self.max.y.max(o.max.y)),
        }
    }
    pub fn width(&self) -> f64 {
        self.max.x - self.min.x
    }
    pub fn height(&self) -> f64 {
        self.max.y - self.min.y
    }
    pub fn center(&self) -> Pt {
        Pt::new((self.min.x + self.max.x) / 2.0, (self.min.y + self.max.y) / 2.0)
    }
    pub fn contains(&self, p: Pt) -> bool {
        p.x >= self.min.x && p.x <= self.max.x && p.y >= self.min.y && p.y <= self.max.y
    }
    pub fn intersects(&self, o: &Rect) -> bool {
        self.min.x <= o.max.x && self.max.x >= o.min.x && self.min.y <= o.max.y && self.max.y >= o.min.y
    }
    pub fn inflate(&self, d: f64) -> Rect {
        Rect { min: Pt::new(self.min.x - d, self.min.y - d), max: Pt::new(self.max.x + d, self.max.y + d) }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Polyline {
    pub pts: Vec<Pt>,
    pub closed: bool,
}

impl Polyline {
    pub fn new(pts: Vec<Pt>, closed: bool) -> Self {
        Self { pts, closed }
    }
    pub fn transformed(&self, xf: &Xf) -> Polyline {
        Polyline { pts: self.pts.iter().map(|p| xf.apply(*p)).collect(), closed: self.closed }
    }
    pub fn bounds(&self) -> Option<Rect> {
        Rect::from_pts(self.pts.iter().copied())
    }
    pub fn length(&self) -> f64 {
        let mut l: f64 = self.pts.windows(2).map(|w| w[0].dist(w[1])).sum();
        if self.closed && self.pts.len() > 2 {
            l += self.pts[self.pts.len() - 1].dist(self.pts[0]);
        }
        l
    }
    /// Signed area (positive = clockwise on a y-down screen).
    pub fn area(&self) -> f64 {
        let n = self.pts.len();
        if n < 3 {
            return 0.0;
        }
        let mut s = 0.0;
        for i in 0..n {
            let (p, q) = (self.pts[i], self.pts[(i + 1) % n]);
            s += p.x * q.y - q.x * p.y;
        }
        s / 2.0
    }
    /// Even-odd point containment (only meaningful for closed polylines).
    pub fn contains(&self, p: Pt) -> bool {
        let n = self.pts.len();
        let mut inside = false;
        let mut j = n.wrapping_sub(1);
        for i in 0..n {
            let (a, b) = (self.pts[i], self.pts[j]);
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            j = i;
        }
        inside
    }
    /// Does any part of this outline (or, for a closed one, its inside) touch the rectangle?
    pub fn intersects_rect(&self, r: &Rect) -> bool {
        if self.pts.iter().any(|p| r.contains(*p)) {
            return true;
        }
        let n = self.pts.len();
        if n < 2 {
            return false;
        }
        let corners = [r.min, Pt::new(r.max.x, r.min.y), r.max, Pt::new(r.min.x, r.max.y)];
        let segs = if self.closed { n } else { n - 1 };
        for i in 0..segs {
            let (a, b) = (self.pts[i], self.pts[(i + 1) % n]);
            for k in 0..4 {
                if segments_cross(a, b, corners[k], corners[(k + 1) % 4]) {
                    return true;
                }
            }
        }
        // The rectangle may lie entirely inside a closed outline.
        self.closed && n >= 3 && self.contains(r.min)
    }
    /// Minimum distance from `p` to the outline.
    pub fn dist_to(&self, p: Pt) -> f64 {
        let n = self.pts.len();
        if n == 0 {
            return f64::INFINITY;
        }
        if n == 1 {
            return self.pts[0].dist(p);
        }
        let segs = if self.closed { n } else { n - 1 };
        (0..segs).map(|i| seg_dist(self.pts[i], self.pts[(i + 1) % n], p)).fold(f64::INFINITY, f64::min)
    }
}

pub fn seg_dist(a: Pt, b: Pt, p: Pt) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 == 0.0 { 0.0 } else { (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0) };
    p.dist(Pt::new(a.x + t * dx, a.y + t * dy))
}

pub fn flatten_quad(p0: Pt, p1: Pt, p2: Pt, tol: f64, out: &mut Vec<Pt>) {
    let len = p0.dist(p1) + p1.dist(p2);
    let n = ((len / tol.max(1e-3)).sqrt().ceil() as usize).clamp(2, 64);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let u = 1.0 - t;
        out.push(Pt::new(
            u * u * p0.x + 2.0 * u * t * p1.x + t * t * p2.x,
            u * u * p0.y + 2.0 * u * t * p1.y + t * t * p2.y,
        ));
    }
}

pub fn flatten_cubic(p0: Pt, p1: Pt, p2: Pt, p3: Pt, tol: f64, out: &mut Vec<Pt>) {
    let len = p0.dist(p1) + p1.dist(p2) + p2.dist(p3);
    let n = ((len / tol.max(1e-3)).sqrt().ceil() as usize).clamp(2, 96);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let u = 1.0 - t;
        let (a, b, c, d) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push(Pt::new(
            a * p0.x + b * p1.x + c * p2.x + d * p3.x,
            a * p0.y + b * p1.y + c * p2.y + d * p3.y,
        ));
    }
}


/// Do the closed segments a-b and c-d share a point?
pub fn segments_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> bool {
    let o = |p: Pt, q: Pt, r: Pt| (q.x - p.x) * (r.y - p.y) - (q.y - p.y) * (r.x - p.x);
    let on = |p: Pt, q: Pt, r: Pt| r.x >= p.x.min(q.x) - 1e-12 && r.x <= p.x.max(q.x) + 1e-12 && r.y >= p.y.min(q.y) - 1e-12 && r.y <= p.y.max(q.y) + 1e-12;
    let (d1, d2, d3, d4) = (o(c, d, a), o(c, d, b), o(a, b, c), o(a, b, d));
    if ((d1 > 0.0 && d2 < 0.0) || (d1 < 0.0 && d2 > 0.0)) && ((d3 > 0.0 && d4 < 0.0) || (d3 < 0.0 && d4 > 0.0)) {
        return true;
    }
    (d1 == 0.0 && on(c, d, a)) || (d2 == 0.0 && on(c, d, b)) || (d3 == 0.0 && on(a, b, c)) || (d4 == 0.0 && on(a, b, d))
}
