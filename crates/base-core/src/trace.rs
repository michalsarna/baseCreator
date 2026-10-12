//! Bitmap tracing: turn the dark areas of an image into Bézier outlines.
use crate::bezier::{Contour, Node};
use crate::geom::*;
use crate::image::ImageData;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TraceParams {
    /// Pixels darker than this (0..=255) are traced.
    pub threshold: u8,
    /// Ignore regions smaller than this many pixels.
    pub min_area: f64,
    /// Corner simplification tolerance in pixels (0 keeps every step).
    pub tolerance: f64,
    /// Fit smooth curves through the outline instead of straight segments.
    pub smooth: bool,
    /// Trace the light areas instead of the dark ones.
    pub invert: bool,
}

impl Default for TraceParams {
    fn default() -> Self {
        TraceParams { threshold: 128, min_area: 8.0, tolerance: 1.0, smooth: true, invert: false }
    }
}

type V = (i32, i32);

/// Closed boundary loops of the foreground mask, as grid vertices (foreground on the right-hand side).
fn loops(mask: &[bool], w: usize, h: usize) -> Vec<Vec<V>> {
    let fg = |x: i32, y: i32| x >= 0 && y >= 0 && (x as usize) < w && (y as usize) < h && mask[y as usize * w + x as usize];
    let mut edges: Vec<(V, V)> = vec![];
    for y in 0..h as i32 {
        for x in 0..w as i32 {
            if !fg(x, y) {
                continue;
            }
            if !fg(x, y - 1) {
                edges.push(((x, y), (x + 1, y)));
            }
            if !fg(x + 1, y) {
                edges.push(((x + 1, y), (x + 1, y + 1)));
            }
            if !fg(x, y + 1) {
                edges.push(((x + 1, y + 1), (x, y + 1)));
            }
            if !fg(x - 1, y) {
                edges.push(((x, y + 1), (x, y)));
            }
        }
    }
    let mut from: HashMap<V, Vec<usize>> = HashMap::new();
    for (i, e) in edges.iter().enumerate() {
        from.entry(e.0).or_default().push(i);
    }
    let mut used = vec![false; edges.len()];
    let mut out = vec![];
    for i in 0..edges.len() {
        if used[i] {
            continue;
        }
        let mut lp = vec![edges[i].0];
        let mut cur = i;
        loop {
            used[cur] = true;
            let (a, b) = edges[cur];
            lp.push(b);
            let dir = (b.0 - a.0, b.1 - a.1);
            // Prefer a left turn at saddle points so diagonal pixels stay separate.
            let next = from.get(&b).and_then(|c| {
                let free: Vec<usize> = c.iter().copied().filter(|j| !used[*j]).collect();
                free.iter().copied().min_by_key(|j| {
                    let d = (edges[*j].1 .0 - edges[*j].0 .0, edges[*j].1 .1 - edges[*j].0 .1);
                    // cross > 0: turn towards +y; rank left turn first (cross < 0 in y-down terms).
                    let cross = dir.0 * d.1 - dir.1 * d.0;
                    match cross.signum() {
                        -1 => 0,
                        0 => 1,
                        _ => 2,
                    }
                })
            });
            match next {
                Some(n) if edges[n].0 == b && n != i => cur = n,
                _ => break,
            }
            if edges[cur].1 == lp[0] && used[cur] {
                break;
            }
        }
        if lp.len() > 3 && lp.first() == lp.last() {
            lp.pop();
            out.push(lp);
        }
    }
    out
}

fn dedupe_collinear(p: &[V]) -> Vec<Pt> {
    let n = p.len();
    let mut out = vec![];
    for i in 0..n {
        let (a, b, c) = (p[(i + n - 1) % n], p[i], p[(i + 1) % n]);
        let cross = (b.0 - a.0) * (c.1 - b.1) - (b.1 - a.1) * (c.0 - b.0);
        if cross != 0 {
            out.push(Pt::new(b.0 as f64, b.1 as f64));
        }
    }
    out
}

fn area(p: &[Pt]) -> f64 {
    let n = p.len();
    (0..n).map(|i| p[i].x * p[(i + 1) % n].y - p[(i + 1) % n].x * p[i].y).sum::<f64>() / 2.0
}

fn dp(pts: &[Pt], tol: f64, out: &mut Vec<Pt>) {
    // Open polyline simplification; the end points are kept by the caller.
    if pts.len() < 3 {
        return;
    }
    let (a, b) = (pts[0], pts[pts.len() - 1]);
    let (mut best, mut idx) = (0.0, 0);
    for (i, p) in pts.iter().enumerate().skip(1).take(pts.len() - 2) {
        let d = seg_dist(a, b, *p);
        if d > best {
            best = d;
            idx = i;
        }
    }
    if best > tol {
        dp(&pts[..=idx], tol, out);
        out.push(pts[idx]);
        dp(&pts[idx..], tol, out);
    }
}

fn simplify_closed(p: &[Pt], tol: f64) -> Vec<Pt> {
    if tol <= 0.0 || p.len() < 5 {
        return p.to_vec();
    }
    // Split at the point farthest from the first one, simplify both halves.
    let far = (1..p.len()).max_by(|&i, &j| p[0].dist(p[i]).partial_cmp(&p[0].dist(p[j])).unwrap_or(std::cmp::Ordering::Equal)).unwrap_or(1);
    let first: Vec<Pt> = p[..=far].to_vec();
    let mut second: Vec<Pt> = p[far..].to_vec();
    second.push(p[0]);
    let mut out = vec![p[0]];
    dp(&first, tol, &mut out);
    out.push(p[far]);
    dp(&second, tol, &mut out);
    out
}

fn to_contour(pts: &[Pt], smooth: bool, scale: (f64, f64)) -> Contour {
    let sc = |p: Pt| Pt::new(p.x * scale.0, p.y * scale.1);
    let mut c = Contour { nodes: pts.iter().map(|p| Node::corner(sc(*p))).collect(), closed: true };
    if smooth {
        let n = c.nodes.len();
        for i in 0..n {
            let (a, b, d) = (pts[(i + n - 1) % n], pts[i], pts[(i + 1) % n]);
            let (v1, v2) = (Pt::new(b.x - a.x, b.y - a.y), Pt::new(d.x - b.x, d.y - b.y));
            let (l1, l2) = ((v1.x * v1.x + v1.y * v1.y).sqrt(), (v2.x * v2.x + v2.y * v2.y).sqrt());
            if l1 < 1e-9 || l2 < 1e-9 {
                continue;
            }
            let cos = (v1.x * v2.x + v1.y * v2.y) / (l1 * l2);
            // Keep sharp corners (turn of more than about 55 degrees) and very short segments.
            if cos < 0.57 {
                continue;
            }
            c.make_smooth(i);
        }
    }
    c
}

/// Trace the image. Contours are in the image's local millimetre coordinates (y down).
pub fn trace(im: &ImageData, p: &TraceParams) -> Vec<Contour> {
    // Work on a reduced copy for very large pictures; the outline is scaled back.
    let work = im.downscaled(2500);
    let (w, h) = (work.px_w as usize, work.px_h as usize);
    let flip = im.invert ^ p.invert;
    let dark = |g: u8| (g < p.threshold) ^ flip;
    let mask: Vec<bool> = work.gray.iter().map(|g| dark(*g)).collect();
    let scale = (work.w / work.px_w as f64, work.h / work.px_h as f64);
    let mut out = vec![];
    for lp in loops(&mask, w, h) {
        let pts = dedupe_collinear(&lp);
        if pts.len() < 3 || area(&pts).abs() < p.min_area {
            continue;
        }
        let pts = simplify_closed(&pts, p.tolerance);
        if pts.len() < 3 {
            continue;
        }
        out.push(to_contour(&pts, p.smooth, scale));
    }
    out
}
