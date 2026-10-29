//! DXF reading (ASCII, R12 and newer): the lines, polylines (with bulges), arcs, circles, ellipses, splines and blocks of a drawing as
//! editable Bézier contours, joined into paths, on the layers of the file, in millimetres and with Y pointing down (as the scene).
use crate::bezier::{Contour, Node};
use crate::geom::*;
use crate::svg::Outline;
use std::collections::HashMap;

/// The most entities that are read (blocks that are inserted count each time).
const MAX_ENTITIES: usize = 500_000;
/// How deep blocks may be inside blocks.
const MAX_DEPTH: usize = 8;
/// Ends that are nearer than this (mm) are one point: the lines of a drawing that meet are one path.
pub const JOIN_TOLERANCE: f64 = 0.01;
/// The biggest file that is read.
const MAX_BYTES: usize = 200 * 1024 * 1024;

/// What was read.
pub struct Dxf {
    pub outlines: Vec<Outline>,
    /// What could not be read, or was left out, in words.
    pub warnings: Vec<String>,
}

/// One entity as the file has it: its kind, layer and colour, and the group codes with their values.
struct Raw {
    kind: String,
    layer: String,
    color: Option<i32>,
    codes: Vec<(i32, String)>,
    /// The vertices of an old-style POLYLINE.
    verts: Vec<Vec<(i32, String)>>,
}

impl Raw {
    fn num(&self, code: i32) -> Option<f64> {
        self.codes.iter().find(|(c, _)| *c == code).and_then(|(_, v)| v.trim().parse().ok())
    }
    fn nums(&self, code: i32) -> Vec<f64> {
        self.codes.iter().filter(|(c, _)| *c == code).filter_map(|(_, v)| v.trim().parse().ok()).collect()
    }
    fn text(&self, code: i32) -> Option<&str> {
        self.codes.iter().find(|(c, _)| *c == code).map(|(_, v)| v.trim())
    }
}

fn pair_num(codes: &[(i32, String)], code: i32) -> Option<f64> {
    codes.iter().find(|(c, _)| *c == code).and_then(|(_, v)| v.trim().parse().ok())
}

struct Block {
    base: Pt,
    entities: Vec<Raw>,
}

fn tokens(text: &str) -> Vec<(i32, String)> {
    let mut out = vec![];
    let mut lines = text.lines();
    while let (Some(code), Some(value)) = (lines.next(), lines.next()) {
        match code.trim().parse::<i32>() {
            Ok(c) => out.push((c, value.trim_end_matches('\r').to_string())),
            Err(_) => break,
        }
    }
    out
}

/// The entities of a section (or block), from `start` on, until ENDSEC / ENDBLK.
fn entities(pairs: &[(i32, String)], start: usize) -> (Vec<Raw>, usize) {
    let mut out: Vec<Raw> = vec![];
    let mut i = start;
    while i < pairs.len() {
        let (c, v) = &pairs[i];
        if *c == 0 && (v == "ENDSEC" || v == "ENDBLK") {
            break;
        }
        if *c != 0 {
            i += 1;
            continue;
        }
        let kind = v.clone();
        let mut codes = vec![];
        i += 1;
        while i < pairs.len() && pairs[i].0 != 0 {
            codes.push(pairs[i].clone());
            i += 1;
        }
        let mut raw = Raw { kind, layer: "0".into(), color: None, codes, verts: vec![] };
        raw.layer = raw.text(8).unwrap_or("0").to_string();
        raw.color = raw.num(62).map(|c| c as i32);
        match raw.kind.as_str() {
            "POLYLINE" => {
                // The vertices follow, up to SEQEND.
                while i < pairs.len() && pairs[i].0 == 0 && pairs[i].1 == "VERTEX" {
                    i += 1;
                    let mut vc = vec![];
                    while i < pairs.len() && pairs[i].0 != 0 {
                        vc.push(pairs[i].clone());
                        i += 1;
                    }
                    raw.verts.push(vc);
                }
                if i < pairs.len() && pairs[i].0 == 0 && pairs[i].1 == "SEQEND" {
                    i += 1;
                    while i < pairs.len() && pairs[i].0 != 0 {
                        i += 1;
                    }
                }
                out.push(raw);
            }
            "SEQEND" | "VERTEX" => {}
            _ => out.push(raw),
        }
    }
    (out, i)
}

/// The size of the unit the file says it uses, in millimetres (`$INSUNITS`).
fn unit_mm(code: i32) -> Option<f64> {
    Some(match code {
        1 => 25.4,
        2 => 304.8,
        3 => 1_609_344.0,
        4 => 1.0,
        5 => 10.0,
        6 => 1000.0,
        7 => 1_000_000.0,
        8 => 25.4e-6,
        9 => 0.0254,
        10 => 914.4,
        13 => 0.001,
        14 => 100.0,
        _ => return None,
    })
}

/// The colour of an AutoCAD colour index (the basic ones; the rest are grey).
fn aci_rgb(i: i32) -> [u8; 3] {
    match i {
        1 => [255, 0, 0],
        2 => [255, 255, 0],
        3 => [0, 255, 0],
        4 => [0, 255, 255],
        5 => [0, 0, 255],
        6 => [255, 0, 255],
        7 | 0 => [0, 0, 0],
        8 => [128, 128, 128],
        9 => [192, 192, 192],
        _ => [96, 96, 96],
    }
}

// ---- building contours ----

/// A line or curve being drawn node by node.
struct Chain {
    nodes: Vec<Node>,
    closed: bool,
}

impl Chain {
    fn start(p: Pt) -> Chain {
        Chain { nodes: vec![Node::corner(p)], closed: false }
    }
    fn line_to(&mut self, p: Pt) {
        self.nodes.push(Node::corner(p));
    }
    fn cubic_to(&mut self, c1: Pt, c2: Pt, p: Pt) {
        if let Some(last) = self.nodes.last_mut() {
            last.hout = c1;
        }
        self.nodes.push(Node { p, hin: c2, hout: p, smooth: false });
    }
    fn contour(mut self) -> Contour {
        // The handles of a node that has both are smooth when they are in line.
        for n in &mut self.nodes {
            n.smooth = n.has_in() && n.has_out() && {
                let (a, b) = (Pt::new(n.p.x - n.hin.x, n.p.y - n.hin.y), Pt::new(n.hout.x - n.p.x, n.hout.y - n.p.y));
                a.x * b.x + a.y * b.y > 0.0 && (a.x * b.y - a.y * b.x).abs() <= 0.02 * ((a.x * b.x + a.y * b.y).abs() + 1e-12)
            };
        }
        Contour { nodes: self.nodes, closed: self.closed }
    }
}

/// The cubic pieces of the arc of the circle `c`, `r` from angle `a0` for the sweep `sweep` (radians, positive counter-clockwise).
fn arc_pieces(c: Pt, r: f64, a0: f64, sweep: f64) -> Vec<(Pt, Pt, Pt)> {
    let n = ((sweep.abs() / std::f64::consts::FRAC_PI_2).ceil() as usize).max(1);
    let step = sweep / n as f64;
    let k = 4.0 / 3.0 * (step / 4.0).tan();
    let at = |a: f64| Pt::new(c.x + r * a.cos(), c.y + r * a.sin());
    let tan = |a: f64| Pt::new(-r * a.sin(), r * a.cos());
    (0..n)
        .map(|i| {
            let (a, b) = (a0 + step * i as f64, a0 + step * (i + 1) as f64);
            let (p0, p1) = (at(a), at(b));
            let (t0, t1) = (tan(a), tan(b));
            (Pt::new(p0.x + k * t0.x, p0.y + k * t0.y), Pt::new(p1.x - k * t1.x, p1.y - k * t1.y), p1)
        })
        .collect()
}

/// An arc from `p0` to `p1` with a bulge (the tangent of a quarter of the angle; positive counter-clockwise), as cubic pieces.
fn bulge_pieces(p0: Pt, p1: Pt, bulge: f64) -> Vec<(Pt, Pt, Pt)> {
    let chord = p0.dist(p1);
    if chord < 1e-12 || bulge.abs() < 1e-12 {
        return vec![(p0, p1, p1)];
    }
    let sweep = 4.0 * bulge.atan();
    let r = chord / (2.0 * (sweep / 2.0).sin().abs());
    // The centre lies on the perpendicular bisector, on the left of the chord for a positive bulge and a small arc.
    let (mx, my) = ((p0.x + p1.x) / 2.0, (p0.y + p1.y) / 2.0);
    let d = (r * r - chord * chord / 4.0).max(0.0).sqrt();
    let (nx, ny) = (-(p1.y - p0.y) / chord, (p1.x - p0.x) / chord);
    let side = if (sweep.abs() < std::f64::consts::PI) == (bulge > 0.0) { 1.0 } else { -1.0 };
    let c = Pt::new(mx + nx * d * side, my + ny * d * side);
    let a0 = (p0.y - c.y).atan2(p0.x - c.x);
    arc_pieces(c, r, a0, sweep)
}

/// A polyline with bulges as a chain.
fn bulged(points: &[(Pt, f64)], closed: bool) -> Option<Chain> {
    if points.len() < 2 {
        return None;
    }
    let mut chain = Chain::start(points[0].0);
    let n = points.len();
    let last = if closed { n } else { n - 1 };
    for i in 0..last {
        let (p0, b) = points[i];
        let p1 = points[(i + 1) % n].0;
        if closed && i == n - 1 && p1.dist(chain.nodes[0].p) < 1e-12 && b == 0.0 {
            break;
        }
        for (c1, c2, p) in bulge_pieces(p0, p1, b) {
            if c1 == p0 && c2 == p1 && p == p1 {
                chain.line_to(p1);
            } else {
                chain.cubic_to(c1, c2, p);
            }
        }
    }
    if closed && chain.nodes.len() > 2 {
        // The last node is the first one again: merge, keeping the incoming handle.
        let end = chain.nodes.pop().unwrap_or_else(|| Node::corner(points[0].0));
        if end.p.dist(chain.nodes[0].p) < 1e-9 {
            chain.nodes[0].hin = end.hin;
            chain.closed = true;
        } else {
            chain.nodes.push(end);
            chain.closed = true;
        }
    }
    Some(chain)
}

/// Points of a B-spline (De Boor) of `degree` through the control points with the knots, `steps` samples.
fn spline_points(degree: usize, knots: &[f64], ctrl: &[Pt], steps: usize) -> Vec<Pt> {
    let n = ctrl.len();
    if n <= degree || knots.len() != n + degree + 1 {
        return ctrl.to_vec();
    }
    let (lo, hi) = (knots[degree], knots[n]);
    let eval = |u: f64| -> Pt {
        // The span of u.
        let mut k = degree;
        while k + 1 < n && u >= knots[k + 1] {
            k += 1;
        }
        let mut d: Vec<Pt> = (0..=degree).map(|j| ctrl[k - degree + j]).collect();
        for r in 1..=degree {
            for j in (r..=degree).rev() {
                let i = k - degree + j;
                let den = knots[i + degree + 1 - r] - knots[i];
                let a = if den.abs() < 1e-12 { 0.0 } else { (u - knots[i]) / den };
                d[j] = Pt::new((1.0 - a) * d[j - 1].x + a * d[j].x, (1.0 - a) * d[j - 1].y + a * d[j].y);
            }
        }
        d[degree]
    };
    (0..=steps).map(|i| eval(lo + (hi - lo) * (i as f64 / steps as f64)).clone()).collect()
}

/// A Catmull-Rom curve through the points as a chain.
fn through(points: &[Pt], closed: bool) -> Option<Chain> {
    let n = points.len();
    if n < 2 {
        return None;
    }
    let at = |i: i64| -> Pt {
        if closed {
            points[i.rem_euclid(n as i64) as usize]
        } else {
            points[i.clamp(0, n as i64 - 1) as usize]
        }
    };
    let mut chain = Chain::start(points[0]);
    let segs = if closed { n } else { n - 1 };
    for i in 0..segs as i64 {
        let (p0, p1, p2, p3) = (at(i - 1), at(i), at(i + 1), at(i + 2));
        let c1 = Pt::new(p1.x + (p2.x - p0.x) / 6.0, p1.y + (p2.y - p0.y) / 6.0);
        let c2 = Pt::new(p2.x - (p3.x - p1.x) / 6.0, p2.y - (p3.y - p1.y) / 6.0);
        chain.cubic_to(c1, c2, p2);
    }
    if closed {
        let end = chain.nodes.pop()?;
        chain.nodes[0].hin = end.hin;
        chain.closed = true;
    }
    Some(chain)
}

/// What a reading carries along: the transform of the block being read, the layer and colour that blocks hand down.
struct Reader<'a> {
    blocks: &'a HashMap<String, Block>,
    out: Vec<(String, Option<i32>, Contour)>,
    skipped: HashMap<String, usize>,
    count: usize,
}

impl Reader<'_> {
    fn add(&mut self, layer: &str, color: Option<i32>, c: Contour, xf: &Xf) {
        if c.nodes.len() >= 2 || (c.nodes.len() == 1 && c.closed) {
            self.out.push((layer.to_string(), color, c.transformed(xf)));
        }
    }

    fn entity(&mut self, e: &Raw, xf: &Xf, depth: usize, inherit: Option<(&str, Option<i32>)>) {
        if self.count >= MAX_ENTITIES {
            return;
        }
        self.count += 1;
        // An entity of a block on layer 0 takes the layer of the INSERT.
        let (layer, color) = match inherit {
            Some((l, c)) if e.layer == "0" => (l.to_string(), e.color.or(c)),
            _ => (e.layer.clone(), e.color),
        };
        let pt = |x: i32, y: i32| Pt::new(e.num(x).unwrap_or(0.0), e.num(y).unwrap_or(0.0));
        match e.kind.as_str() {
            "LINE" => {
                let (a, b) = (pt(10, 20), pt(11, 21));
                let mut c = Chain::start(a);
                c.line_to(b);
                self.add(&layer, color, c.contour(), xf);
            }
            "LWPOLYLINE" => {
                let xs = e.nums(10);
                let ys = e.nums(20);
                let bulges: Vec<(usize, f64)> = {
                    // A bulge (42) belongs to the vertex whose 10 / 20 came before it: walk the codes in order.
                    let mut v = vec![];
                    let mut idx: i64 = -1;
                    for (c, val) in &e.codes {
                        match c {
                            10 => idx += 1,
                            42 if idx >= 0 => v.push((idx as usize, val.trim().parse().unwrap_or(0.0))),
                            _ => {}
                        }
                    }
                    v
                };
                let points: Vec<(Pt, f64)> = xs.iter().zip(&ys).enumerate().map(|(i, (x, y))| (Pt::new(*x, *y), bulges.iter().find(|(j, _)| *j == i).map(|(_, b)| *b).unwrap_or(0.0))).collect();
                let closed = e.num(70).is_some_and(|f| (f as i64) & 1 == 1);
                if let Some(c) = bulged(&points, closed) {
                    self.add(&layer, color, c.contour(), xf);
                }
            }
            "POLYLINE" => {
                let flags = e.num(70).unwrap_or(0.0) as i64;
                if flags & (16 | 64) != 0 {
                    // A mesh: not a line drawing.
                    *self.skipped.entry("POLYLINE (3D mesh)".into()).or_default() += 1;
                    return;
                }
                let points: Vec<(Pt, f64)> = e.verts.iter().filter(|v| pair_num(v, 70).is_none_or(|f| (f as i64) & 16 == 0)).map(|v| (Pt::new(pair_num(v, 10).unwrap_or(0.0), pair_num(v, 20).unwrap_or(0.0)), pair_num(v, 42).unwrap_or(0.0))).collect();
                if let Some(c) = bulged(&points, flags & 1 == 1) {
                    self.add(&layer, color, c.contour(), xf);
                }
            }
            "ARC" => {
                let (c, r) = (pt(10, 20), e.num(40).unwrap_or(0.0));
                let (a0, mut a1) = (e.num(50).unwrap_or(0.0).to_radians(), e.num(51).unwrap_or(0.0).to_radians());
                if r <= 0.0 {
                    return;
                }
                while a1 <= a0 {
                    a1 += std::f64::consts::TAU;
                }
                let mut chain = Chain::start(Pt::new(c.x + r * a0.cos(), c.y + r * a0.sin()));
                for (c1, c2, p) in arc_pieces(c, r, a0, a1 - a0) {
                    chain.cubic_to(c1, c2, p);
                }
                self.add(&layer, color, chain.contour(), xf);
            }
            "CIRCLE" => {
                let (c, r) = (pt(10, 20), e.num(40).unwrap_or(0.0));
                if r > 0.0 {
                    let mut ring = Contour::ellipse(2.0 * r, 2.0 * r);
                    for n in &mut ring.nodes {
                        n.translate(c.x - r, c.y - r);
                    }
                    self.add(&layer, color, ring, xf);
                }
            }
            "ELLIPSE" => {
                let (c, major) = (pt(10, 20), pt(11, 21));
                let ratio = e.num(40).unwrap_or(1.0).clamp(1e-6, 1.0);
                let (t0, mut t1) = (e.num(41).unwrap_or(0.0), e.num(42).unwrap_or(std::f64::consts::TAU));
                let a = major.x.hypot(major.y);
                if a <= 0.0 {
                    return;
                }
                let rot = major.y.atan2(major.x);
                let full = (t1 - t0 - std::f64::consts::TAU).abs() < 1e-6 || (t1 - t0).abs() < 1e-9;
                if full {
                    let mut ring = Contour::ellipse(2.0 * a, 2.0 * a * ratio);
                    // Centre it on the origin, turn it, put it in place.
                    let to = Xf::translate(-a, -a * ratio).then(Xf::rotate(rot)).then(Xf::translate(c.x, c.y));
                    ring = ring.transformed(&to);
                    self.add(&layer, color, ring, xf);
                } else {
                    while t1 <= t0 {
                        t1 += std::f64::consts::TAU;
                    }
                    let steps = (((t1 - t0) / 0.05).ceil() as usize).max(8);
                    let pts: Vec<Pt> = (0..=steps)
                        .map(|i| {
                            let t = t0 + (t1 - t0) * i as f64 / steps as f64;
                            let (x, y) = (a * t.cos(), a * ratio * t.sin());
                            Pt::new(c.x + x * rot.cos() - y * rot.sin(), c.y + x * rot.sin() + y * rot.cos())
                        })
                        .collect();
                    let mut chain = Chain::start(pts[0]);
                    for p in &pts[1..] {
                        chain.line_to(*p);
                    }
                    self.add(&layer, color, chain.contour(), xf);
                }
            }
            "SPLINE" => {
                let degree = e.num(71).unwrap_or(3.0).max(1.0) as usize;
                let flags = e.num(70).unwrap_or(0.0) as i64;
                let (xs, ys) = (e.nums(10), e.nums(20));
                let ctrl: Vec<Pt> = xs.iter().zip(&ys).map(|(x, y)| Pt::new(*x, *y)).collect();
                let knots = e.nums(40);
                let (fx, fy) = (e.nums(11), e.nums(21));
                let fit: Vec<Pt> = fx.iter().zip(&fy).map(|(x, y)| Pt::new(*x, *y)).collect();
                let closed = flags & 1 == 1;
                let chain = if ctrl.len() > degree && knots.len() == ctrl.len() + degree + 1 {
                    let pts = spline_points(degree, &knots, &ctrl, (ctrl.len() * 12).clamp(24, 4000));
                    let mut chain = Chain::start(pts[0]);
                    for p in &pts[1..] {
                        chain.line_to(*p);
                    }
                    chain.closed = closed && pts.len() > 2;
                    if chain.closed && chain.nodes.last().is_some_and(|l| l.p.dist(chain.nodes[0].p) < 1e-6) {
                        chain.nodes.pop();
                    }
                    Some(chain)
                } else if fit.len() >= 2 {
                    through(&fit, closed)
                } else if ctrl.len() >= 2 {
                    through(&ctrl, closed)
                } else {
                    None
                };
                if let Some(c) = chain {
                    self.add(&layer, color, c.contour(), xf);
                }
            }
            "INSERT" => {
                if depth >= MAX_DEPTH {
                    return;
                }
                let Some(name) = e.text(2) else { return };
                let Some(block) = self.blocks.get(name) else { return };
                let (ins, sx, sy) = (pt(10, 20), e.num(41).unwrap_or(1.0), e.num(42).unwrap_or(1.0));
                let rot = e.num(50).unwrap_or(0.0).to_radians();
                let (cols, rows) = (e.num(70).unwrap_or(1.0).clamp(1.0, 1000.0) as usize, e.num(71).unwrap_or(1.0).clamp(1.0, 1000.0) as usize);
                let (dc, dr) = (e.num(44).unwrap_or(0.0), e.num(45).unwrap_or(0.0));
                for r in 0..rows {
                    for c in 0..cols {
                        let place = Xf::translate(-block.base.x, -block.base.y).then(Xf::scale(sx, sy)).then(Xf::rotate(rot)).then(Xf::translate(ins.x + c as f64 * dc, ins.y + r as f64 * dr)).then(*xf);
                        for inner in &block.entities {
                            self.entity(inner, &place, depth + 1, Some((&layer, color)));
                        }
                    }
                }
            }
            "POINT" | "VIEWPORT" | "ATTDEF" | "ATTRIB" => {}
            other => {
                *self.skipped.entry(other.to_string()).or_default() += 1;
            }
        }
    }
}

/// Read an ASCII DXF. `area_h` is the height of the work area in millimetres: the DXF's Y counts up from the bottom, the scene's
/// down from the top (a drawing that starts at the origin lies at the bottom left of the work area).
pub fn parse(data: &[u8], area_h: f64) -> Result<Dxf, String> {
    if data.len() > MAX_BYTES {
        return Err("the DXF file is too big".into());
    }
    if data.starts_with(b"AutoCAD Binary DXF") {
        return Err("this is a binary DXF: save it as an ASCII DXF".into());
    }
    let text = String::from_utf8_lossy(data);
    let pairs = tokens(&text);
    if !pairs.iter().take(40).any(|(c, v)| *c == 0 && v == "SECTION") {
        return Err("this is not a DXF file".into());
    }
    let mut warnings = vec![];
    let mut units = 0;
    let mut blocks: HashMap<String, Block> = HashMap::new();
    let mut main: Vec<Raw> = vec![];
    let mut layer_color: HashMap<String, i32> = HashMap::new();
    let mut i = 0;
    while i < pairs.len() {
        if !(pairs[i].0 == 0 && pairs[i].1 == "SECTION") {
            i += 1;
            continue;
        }
        let name = pairs.get(i + 1).filter(|(c, _)| *c == 2).map(|(_, v)| v.trim().to_string()).unwrap_or_default();
        i += 2;
        match name.as_str() {
            "HEADER" => {
                while i < pairs.len() && !(pairs[i].0 == 0 && pairs[i].1 == "ENDSEC") {
                    if pairs[i].0 == 9 && pairs[i].1 == "$INSUNITS" {
                        units = pairs.get(i + 1).and_then(|(_, v)| v.trim().parse().ok()).unwrap_or(0);
                    }
                    i += 1;
                }
            }
            "TABLES" => {
                while i < pairs.len() && !(pairs[i].0 == 0 && pairs[i].1 == "ENDSEC") {
                    if pairs[i].0 == 0 && pairs[i].1 == "LAYER" {
                        let (mut n, mut col) = (None, None);
                        let mut j = i + 1;
                        while j < pairs.len() && pairs[j].0 != 0 {
                            match pairs[j].0 {
                                2 => n = Some(pairs[j].1.trim().to_string()),
                                62 => col = pairs[j].1.trim().parse::<i32>().ok(),
                                _ => {}
                            }
                            j += 1;
                        }
                        if let (Some(n), Some(c)) = (n, col) {
                            layer_color.insert(n, c.abs());
                        }
                    }
                    i += 1;
                }
            }
            "BLOCKS" => {
                while i < pairs.len() && !(pairs[i].0 == 0 && pairs[i].1 == "ENDSEC") {
                    if pairs[i].0 == 0 && pairs[i].1 == "BLOCK" {
                        let mut j = i + 1;
                        let (mut bname, mut base) = (String::new(), Pt::default());
                        while j < pairs.len() && pairs[j].0 != 0 {
                            match pairs[j].0 {
                                2 => bname = pairs[j].1.trim().to_string(),
                                10 => base.x = pairs[j].1.trim().parse().unwrap_or(0.0),
                                20 => base.y = pairs[j].1.trim().parse().unwrap_or(0.0),
                                _ => {}
                            }
                            j += 1;
                        }
                        let (ents, next) = entities(&pairs, j);
                        blocks.insert(bname, Block { base, entities: ents });
                        i = next;
                    }
                    i += 1;
                }
            }
            "ENTITIES" => {
                let (ents, next) = entities(&pairs, i);
                main = ents;
                i = next;
            }
            _ => {}
        }
    }
    let scale = match unit_mm(units) {
        Some(s) => s,
        None => {
            if units != 0 {
                warnings.push(format!("the unit code {units} of the file is not known: millimetres are assumed"));
            } else {
                warnings.push("the file does not say which unit it uses: millimetres are assumed".into());
            }
            1.0
        }
    };
    let mut reader = Reader { blocks: &blocks, out: vec![], skipped: HashMap::new(), count: 0 };
    for e in &main {
        reader.entity(e, &Xf::IDENTITY, 0, None);
    }
    if reader.count >= MAX_ENTITIES {
        warnings.push("the file has more entities than are read; the rest was left out".into());
    }
    if !reader.skipped.is_empty() {
        let mut names: Vec<String> = reader.skipped.iter().map(|(k, v)| format!("{k} ×{v}")).collect();
        names.sort();
        warnings.push(format!("left out (not drawn as lines): {}", names.join(", ")));
    }
    // To millimetres, Y down.
    let to_scene = Xf { a: scale, b: 0.0, c: 0.0, d: -scale, e: 0.0, f: if area_h.is_finite() { area_h } else { 0.0 } };
    let found: Vec<(String, Option<i32>, Contour)> = reader.out.into_iter().map(|(l, c, k)| (l, c, k.transformed(&to_scene))).collect();
    // Lines that meet are one path (per layer).
    let mut layers: Vec<String> = vec![];
    for (l, ..) in &found {
        if !layers.contains(l) {
            layers.push(l.clone());
        }
    }
    let mut outlines = vec![];
    for layer in layers {
        let mine: Vec<(Option<i32>, Contour)> = found.iter().filter(|(l, ..)| *l == layer).map(|(_, c, k)| (*c, k.clone())).collect();
        let colour = mine.iter().find_map(|(c, _)| *c).or_else(|| layer_color.get(&layer).copied()).unwrap_or(7);
        let contours = crate::join::join(mine.into_iter().map(|(_, c)| c).collect(), JOIN_TOLERANCE);
        for c in contours {
            outlines.push(Outline { contours: vec![c], color: aci_rgb(colour), layer: Some(layer.clone()) });
        }
    }
    if outlines.is_empty() {
        warnings.push("there are no lines, arcs or curves in the file".into());
    }
    Ok(Dxf { outlines, warnings })
}
