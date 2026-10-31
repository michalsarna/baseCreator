//! The geometry of the design commands: arrays, copies along a path, text on a path, trimming a line by a shape, duplicates,
//! simplifying, serial and CSV texts, barcodes and QR codes, and nesting by boxes. Everything works on polylines in millimetres.
use crate::geom::{Polyline, Pt, Rect, Xf};
use crate::ops;

/// The offsets of the copies of a grid array (without the original at 0, 0): `cols` x `rows`, `dx` and `dy` apart.
pub fn grid_offsets(cols: usize, rows: usize, dx: f64, dy: f64) -> Vec<(f64, f64)> {
    let mut v = vec![];
    for r in 0..rows {
        for c in 0..cols {
            if (r, c) != (0, 0) {
                v.push((c as f64 * dx, r as f64 * dy));
            }
        }
    }
    v
}

/// The transforms of the copies of a circular array (without the original): `count` places round `centre`, over `total_deg` degrees
/// (360 puts them evenly all the way round). Whether a copy is turned with its place or only moved is up to the caller (`turn` there).
pub fn circle_transforms(centre: Pt, count: usize, total_deg: f64) -> Vec<Xf> {
    if count < 2 {
        return vec![];
    }
    // All the way round: the last copy does not land on the original.
    let step = if (total_deg.abs() - 360.0).abs() < 1e-9 { total_deg / count as f64 } else { total_deg / (count - 1) as f64 };
    (1..count)
        .map(|i| Xf::rotate_about((step * i as f64).to_radians(), centre))
        .collect()
}

/// A point at the distance `s` along the polyline and the angle of the line there (radians).
pub fn point_at(path: &Polyline, s: f64) -> Option<(Pt, f64)> {
    let n = path.pts.len();
    if n < 2 {
        return None;
    }
    let segs = if path.closed { n } else { n - 1 };
    let total = path.length();
    if total <= 0.0 {
        return None;
    }
    let mut left = if path.closed { s.rem_euclid(total) } else { s.clamp(0.0, total) };
    for i in 0..segs {
        let (a, b) = (path.pts[i], path.pts[(i + 1) % n]);
        let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
        if len <= 1e-12 {
            continue;
        }
        if left <= len || i + 1 == segs {
            let t = (left / len).clamp(0.0, 1.0);
            return Some((Pt::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t), (b.y - a.y).atan2(b.x - a.x)));
        }
        left -= len;
    }
    None
}

/// The places of copies along a path: every `spacing` mm from the start, the last one at the end when it fits. At most `limit`.
pub fn along_path(path: &Polyline, spacing: f64, limit: usize) -> Vec<(Pt, f64)> {
    let total = path.length();
    if spacing <= 1e-6 || total <= 0.0 {
        return vec![];
    }
    let n = ((total / spacing) + 1e-9).floor() as usize + 1;
    let n = if path.closed { n.saturating_sub(1).max(1) } else { n };
    (0..n.min(limit)).filter_map(|i| point_at(path, i as f64 * spacing)).collect()
}

/// The outlines of a text bent along a path: the text starts `start` mm along it, its baseline follows the path and its height stands
/// on the left of the path. `polys` are the outlines with x along the line of text from `x0` and y up from the baseline.
pub fn bend_along(path: &Polyline, polys: &[Polyline], x0: f64, start: f64) -> Vec<Polyline> {
    // Long straight pieces are cut so that they bend with the path.
    let step = 0.5;
    polys
        .iter()
        .map(|p| {
            let mut pts: Vec<Pt> = vec![];
            let n = p.pts.len();
            let segs = if p.closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let (a, b) = (p.pts[i], p.pts[(i + 1) % n]);
                let len = ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
                let k = (len / step).ceil().max(1.0) as usize;
                for j in 0..k {
                    let t = j as f64 / k as f64;
                    pts.push(Pt::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
                }
            }
            if !p.closed {
                if let Some(l) = p.pts.last() {
                    pts.push(*l);
                }
            }
            let bent = pts
                .iter()
                .map(|q| match point_at(path, start + q.x - x0) {
                    Some((o, ang)) => Pt::new(o.x - ang.sin() * q.y, o.y + ang.cos() * q.y),
                    None => *q,
                })
                .collect();
            Polyline::new(bent, p.closed)
        })
        .collect()
}

fn seg_cross(a: Pt, b: Pt, c: Pt, d: Pt) -> Option<f64> {
    let (r, s) = ((b.x - a.x, b.y - a.y), (d.x - c.x, d.y - c.y));
    let den = r.0 * s.1 - r.1 * s.0;
    if den.abs() < 1e-12 {
        return None;
    }
    let (qp, ) = ((c.x - a.x, c.y - a.y), );
    let t = (qp.0 * s.1 - qp.1 * s.0) / den;
    let u = (qp.0 * r.1 - qp.1 * r.0) / den;
    ((0.0..=1.0).contains(&t) && (0.0..=1.0).contains(&u)).then_some(t)
}

/// The parts of the lines that lie inside (or outside) the closed shapes of `region` (even-odd), cut where they cross its outlines.
/// Closed lines are cut open at the crossings too.
pub fn clip_lines(lines: &[Polyline], region: &[Polyline], keep_inside: bool) -> Vec<Polyline> {
    let inside = |p: Pt| -> bool { region.iter().filter(|r| r.closed && r.contains(p)).count() % 2 == 1 };
    let mut out = vec![];
    for line in lines {
        let n = line.pts.len();
        let segs = if line.closed { n } else { n.saturating_sub(1) };
        let mut cur: Vec<Pt> = vec![];
        for i in 0..segs {
            let (a, b) = (line.pts[i], line.pts[(i + 1) % n]);
            let mut ts = vec![0.0, 1.0];
            for r in region.iter().filter(|r| r.closed) {
                let m = r.pts.len();
                for j in 0..m {
                    if let Some(t) = seg_cross(a, b, r.pts[j], r.pts[(j + 1) % m]) {
                        ts.push(t);
                    }
                }
            }
            ts.sort_by(|x, y| x.partial_cmp(y).unwrap());
            ts.dedup_by(|x, y| (*x - *y).abs() < 1e-9);
            for w in ts.windows(2) {
                let (p, q) = (Pt::new(a.x + (b.x - a.x) * w[0], a.y + (b.y - a.y) * w[0]), Pt::new(a.x + (b.x - a.x) * w[1], a.y + (b.y - a.y) * w[1]));
                let mid = Pt::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
                if inside(mid) == keep_inside {
                    if cur.last().is_none_or(|l| (l.x - p.x).abs() > 1e-9 || (l.y - p.y).abs() > 1e-9) {
                        if !cur.is_empty() {
                            out.push(Polyline::new(std::mem::take(&mut cur), false));
                        }
                        cur.push(p);
                    }
                    cur.push(q);
                } else if !cur.is_empty() {
                    out.push(Polyline::new(std::mem::take(&mut cur), false));
                }
            }
        }
        if !cur.is_empty() {
            out.push(Polyline::new(cur, false));
        }
    }
    out.retain(|p| p.pts.len() >= 2 && p.length() > 1e-9);
    out
}

/// Do two sets of outlines have the same geometry (the same points within `tol`, in any order of outlines and from any start of a closed one)?
pub fn same_geometry(a: &[Polyline], b: &[Polyline], tol: f64) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let near = |p: &Polyline, q: &Polyline| -> bool {
        if p.closed != q.closed || p.pts.len() != q.pts.len() {
            return false;
        }
        let n = p.pts.len();
        let eq = |s: usize, rev: bool| (0..n).all(|i| {
            let j = if rev { (s + n - i) % n } else { (s + i) % n };
            (p.pts[i].x - q.pts[j].x).abs() <= tol && (p.pts[i].y - q.pts[j].y).abs() <= tol
        });
        if p.closed {
            (0..n).any(|s| eq(s, false) || eq(s, true))
        } else {
            eq(0, false) || (0..n).all(|i| (p.pts[i].x - q.pts[n - 1 - i].x).abs() <= tol && (p.pts[i].y - q.pts[n - 1 - i].y).abs() <= tol)
        }
    };
    let mut used = vec![false; b.len()];
    a.iter().all(|p| match (0..b.len()).find(|&j| !used[j] && near(p, &b[j])) {
        Some(j) => {
            used[j] = true;
            true
        }
        None => false,
    })
}

/// A polyline with fewer points: Douglas-Peucker with the tolerance `tol` (mm). Closed ones keep at least three points.
pub fn simplify(p: &Polyline, tol: f64) -> Polyline {
    let n = p.pts.len();
    if n < 3 || tol <= 0.0 {
        return p.clone();
    }
    fn dist(p: Pt, a: Pt, b: Pt) -> f64 {
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let l2 = dx * dx + dy * dy;
        if l2 < 1e-18 {
            return ((p.x - a.x).powi(2) + (p.y - a.y).powi(2)).sqrt();
        }
        let t = (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0);
        ((p.x - a.x - t * dx).powi(2) + (p.y - a.y - t * dy).powi(2)).sqrt()
    }
    fn rec(pts: &[Pt], tol: f64, keep: &mut Vec<bool>, lo: usize, hi: usize) {
        if hi <= lo + 1 {
            return;
        }
        let (mut far, mut at) = (0.0, lo);
        for i in lo + 1..hi {
            let d = dist(pts[i], pts[lo], pts[hi]);
            if d > far {
                far = d;
                at = i;
            }
        }
        if far > tol {
            keep[at] = true;
            rec(pts, tol, keep, lo, at);
            rec(pts, tol, keep, at, hi);
        }
    }
    // A closed line is cut at its first point and the point farthest from it, so that both halves are simplified.
    let mut pts = p.pts.clone();
    if p.closed {
        pts.push(pts[0]);
    }
    let m = pts.len();
    let mut keep = vec![false; m];
    keep[0] = true;
    keep[m - 1] = true;
    if p.closed {
        let far = (1..m - 1).max_by(|&i, &j| (pts[i].x - pts[0].x).hypot(pts[i].y - pts[0].y).partial_cmp(&(pts[j].x - pts[0].x).hypot(pts[j].y - pts[0].y)).unwrap()).unwrap_or(1);
        keep[far] = true;
        rec(&pts, tol, &mut keep, 0, far);
        rec(&pts, tol, &mut keep, far, m - 1);
    } else {
        rec(&pts, tol, &mut keep, 0, m - 1);
    }
    let mut out: Vec<Pt> = pts.iter().zip(&keep).filter(|(_, k)| **k).map(|(q, _)| *q).collect();
    if p.closed {
        out.pop();
        if out.len() < 3 {
            return p.clone();
        }
    }
    Polyline::new(out, p.closed)
}

/// The texts of a serial run: `template` with `{n}` replaced by the number `start`, `start + step`, … written with at least `digits` digits.
pub fn serial_texts(template: &str, start: i64, step: i64, count: usize, digits: usize) -> Vec<String> {
    (0..count)
        .map(|i| {
            let v = start + step * i as i64;
            let num = if v < 0 { format!("-{:0>w$}", -v, w = digits) } else { format!("{:0>w$}", v, w = digits) };
            template.replace("{n}", &num)
        })
        .collect()
}

/// The rows of a CSV text (commas or semicolons, quotes with `""` inside); empty lines are left out.
pub fn csv_rows(text: &str) -> Vec<Vec<String>> {
    let sep = if text.lines().next().is_some_and(|l| l.matches(';').count() > l.matches(',').count()) { ';' } else { ',' };
    let mut rows = vec![];
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let (mut row, mut cur, mut quoted) = (vec![], String::new(), false);
        let mut it = line.chars().peekable();
        while let Some(c) = it.next() {
            match c {
                '"' if quoted && it.peek() == Some(&'"') => {
                    cur.push('"');
                    it.next();
                }
                '"' => quoted = !quoted,
                c if c == sep && !quoted => row.push(std::mem::take(&mut cur).trim().to_string()),
                c => cur.push(c),
            }
        }
        row.push(cur.trim().to_string());
        rows.push(row);
    }
    rows
}

/// The template with `{1}`, `{2}`… replaced by the columns of the row (an empty text for a missing column).
pub fn fill_row(template: &str, row: &[String]) -> String {
    let mut s = template.to_string();
    for i in (1..=row.len().max(9)).rev() {
        s = s.replace(&format!("{{{i}}}"), row.get(i - 1).map_or("", |x| x.as_str()));
    }
    s
}

/// The dark squares of a QR code of `data` as rows of booleans, `None` when the data does not fit.
pub fn qr_modules(data: &str) -> Option<(usize, Vec<bool>)> {
    let code = qrcode::QrCode::new(data.as_bytes()).ok()?;
    let w = code.width();
    Some((w, code.to_colors().into_iter().map(|c| c == qrcode::Color::Dark).collect()))
}

/// The dark bars of a Code 39 barcode of `data` (capitals, digits and `-. $/+%`) with a narrow bar of `module` mm and a height of `height`:
/// rectangles as (x, width) from 0. `None` for a character the code does not have.
pub fn code39_bars(data: &str, module: f64) -> Option<Vec<(f64, f64)>> {
    // 9 elements per character (bar, space, bar, …), 1 = wide; the patterns in the order of the table below.
    const CHARS: &str = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ-. $/+%*";
    const PAT: [&str; 44] = [
        "000110100", "100100001", "001100001", "101100000", "000110001", "100110000", "001110000", "000100101", "100100100", "001100100",
        "100001001", "001001001", "101001000", "000011001", "100011000", "001011000", "000001101", "100001100", "001001100", "000011100",
        "100000011", "001000011", "101000010", "000010011", "100010010", "001010010", "000000111", "100000110", "001000110", "000010110",
        "110000001", "011000001", "111000000", "010010001", "110010000", "011010000", "010000101", "110000100", "011000100", "010101000",
        "010100010", "010001010", "000101010", "010010100",
    ];
    let wide = 2.5;
    let text = format!("*{}*", data.to_ascii_uppercase());
    let mut bars = vec![];
    let mut x = 0.0;
    for (ci, ch) in text.chars().enumerate() {
        let k = CHARS.find(ch)?;
        for (i, e) in PAT[k].chars().enumerate() {
            let w = if e == '1' { wide * module } else { module };
            if i % 2 == 0 {
                bars.push((x, w));
            }
            x += w;
        }
        // The gap between characters.
        if ci + 1 < text.chars().count() {
            x += module;
        }
    }
    Some(bars)
}

/// A packing of boxes `(w, h)` on an area `w x h` in shelves, `gap` mm apart, tallest first. Returns for every box (in the order given)
/// the corner (x, y) and whether it was turned a quarter (then its sides are swapped). `None` when they do not all fit.
pub fn nest_boxes(sizes: &[(f64, f64)], area: (f64, f64), gap: f64, allow_turn: bool) -> Option<Vec<(f64, f64, bool)>> {
    let mut order: Vec<usize> = (0..sizes.len()).collect();
    // Turned so that the long side lies along the shelf when turning is allowed; then tallest first.
    let side = |i: usize| -> (f64, f64, bool) {
        let (w, h) = sizes[i];
        if allow_turn && h > w { (h, w, true) } else { (w, h, false) }
    };
    order.sort_by(|&a, &b| side(b).1.partial_cmp(&side(a).1).unwrap());
    let mut out = vec![(0.0, 0.0, false); sizes.len()];
    let (mut x, mut y, mut shelf) = (0.0, 0.0, 0.0f64);
    for &i in &order {
        let (w, h, turned) = side(i);
        if w > area.0 + 1e-9 || h > area.1 + 1e-9 {
            return None;
        }
        if x + w > area.0 + 1e-9 {
            x = 0.0;
            y += shelf + gap;
            shelf = 0.0;
        }
        if y + h > area.1 + 1e-9 {
            return None;
        }
        out[i] = (x, y, turned);
        x += w + gap;
        shelf = shelf.max(h);
    }
    Some(out)
}

/// The box of the outlines (all of them), `None` when there are none.
pub fn box_of(polys: &[Polyline]) -> Option<Rect> {
    let mut it = polys.iter().filter_map(|p| p.bounds());
    let first = it.next()?;
    Some(it.fold(first, |a, b| Rect { min: Pt::new(a.min.x.min(b.min.x), a.min.y.min(b.min.y)), max: Pt::new(a.max.x.max(b.max.x), a.max.y.max(b.max.y)) }))
}

/// Keep `ops` in the namespace for the commands that need booleans together with the design ones.
pub fn union_all(polys: &[Polyline]) -> Vec<Polyline> {
    ops::offset(polys, 0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Polyline {
        Polyline::new(vec![Pt::new(x, y), Pt::new(x + w, y), Pt::new(x + w, y + h), Pt::new(x, y + h)], true)
    }

    #[test]
    fn a_grid_has_every_place_but_the_original() {
        let g = grid_offsets(3, 2, 10.0, 5.0);
        assert_eq!(g.len(), 5);
        assert!(g.contains(&(20.0, 5.0)) && !g.contains(&(0.0, 0.0)));
        assert!(grid_offsets(1, 1, 1.0, 1.0).is_empty());
    }

    #[test]
    fn a_circle_of_six_goes_all_the_way_round_without_a_copy_on_the_original() {
        let c = Pt::new(0.0, 0.0);
        let xs = circle_transforms(c, 6, 360.0);
        assert_eq!(xs.len(), 5);
        let p = xs[0].apply(Pt::new(10.0, 0.0));
        assert!((p.x - 5.0).abs() < 1e-9 && (p.y - 8.660254037844386).abs() < 1e-9, "{p:?}");
        // A half circle with 3 places: 0°, 90°, 180°.
        let half = circle_transforms(c, 3, 180.0);
        let q = half[1].apply(Pt::new(10.0, 0.0));
        assert!((q.x + 10.0).abs() < 1e-9 && q.y.abs() < 1e-9);
        assert!(circle_transforms(c, 1, 360.0).is_empty());
    }

    #[test]
    fn copies_along_a_path_are_spaced_by_length_and_follow_its_direction() {
        let path = Polyline::new(vec![Pt::new(0.0, 0.0), Pt::new(10.0, 0.0), Pt::new(10.0, 10.0)], false);
        let v = along_path(&path, 5.0, 100);
        assert_eq!(v.len(), 5, "0, 5, 10, 15, 20 along");
        assert!((v[1].0.x - 5.0).abs() < 1e-9 && v[1].1.abs() < 1e-9);
        assert!((v[3].0.y - 5.0).abs() < 1e-9 && (v[3].1 - std::f64::consts::FRAC_PI_2).abs() < 1e-9);
        assert_eq!(along_path(&path, 5.0, 2).len(), 2);
        let ring = rect(0.0, 0.0, 10.0, 10.0);
        assert_eq!(along_path(&ring, 10.0, 100).len(), 4, "a closed path has no copy twice at its start");
    }

    #[test]
    fn a_text_bent_along_an_arc_stands_on_it() {
        // A line of text 10 mm long and 2 high along a half circle of radius 10 (length 31.4).
        let pts: Vec<Pt> = (0..=64).map(|i| { let a = std::f64::consts::PI * i as f64 / 64.0; Pt::new(10.0 * a.cos(), 10.0 * a.sin()) }).collect();
        let arc = Polyline::new(pts, false);
        let glyph = Polyline::new(vec![Pt::new(0.0, 0.0), Pt::new(10.0, 0.0), Pt::new(10.0, 2.0), Pt::new(0.0, 2.0)], true);
        let bent = bend_along(&arc, &[glyph], 0.0, 0.0);
        // The baseline starts on the arc (distance 10 from the centre) and the top is on the left of it (inside: 8 from the centre).
        let first = bent[0].pts[0];
        assert!((first.x.hypot(first.y) - 10.0).abs() < 0.02, "{first:?}");
        let top = bent[0].pts.iter().map(|p| p.x.hypot(p.y)).fold(f64::INFINITY, f64::min);
        assert!((top - 8.0).abs() < 0.05, "{top}");
        assert!(bent[0].pts.len() > 8, "cut into short pieces so that it bends");
    }

    #[test]
    fn a_line_is_cut_by_a_shape_and_the_inside_or_the_outside_is_kept() {
        let line = Polyline::new(vec![Pt::new(-10.0, 5.0), Pt::new(30.0, 5.0)], false);
        let square = rect(0.0, 0.0, 10.0, 10.0);
        let inner = clip_lines(&[line.clone()], &[square.clone()], true);
        assert_eq!(inner.len(), 1);
        assert!((inner[0].length() - 10.0).abs() < 1e-9, "{}", inner[0].length());
        let outer = clip_lines(&[line], &[square], false);
        assert_eq!(outer.len(), 2);
        assert!((outer[0].length() + outer[1].length() - 30.0).abs() < 1e-9);
    }

    #[test]
    fn the_same_shape_twice_is_found_whatever_its_start_and_direction() {
        let a = rect(0.0, 0.0, 10.0, 5.0);
        let mut b = rect(0.0, 0.0, 10.0, 5.0);
        b.pts.rotate_left(2);
        b.pts.reverse();
        assert!(same_geometry(&[a.clone()], &[b], 1e-6));
        assert!(!same_geometry(&[a.clone()], &[rect(0.0, 0.0, 10.0, 5.1)], 1e-6));
        assert!(same_geometry(&[a.clone()], &[rect(0.0, 0.0, 10.0, 5.0000001)], 1e-3));
        assert!(!same_geometry(&[a.clone()], &[a.clone(), a], 1e-6));
    }

    #[test]
    fn simplifying_drops_points_on_a_line_and_keeps_the_corners() {
        let mut pts = vec![];
        for i in 0..=10 {
            pts.push(Pt::new(i as f64, 0.0));
        }
        for i in 1..=10 {
            pts.push(Pt::new(10.0, i as f64));
        }
        let l = Polyline::new(pts, false);
        let s = simplify(&l, 0.01);
        assert_eq!(s.pts.len(), 3, "{:?}", s.pts);
        let closed = Polyline::new((0..40).map(|i| { let a = std::f64::consts::TAU * i as f64 / 40.0; Pt::new(10.0 * a.cos(), 10.0 * a.sin()) }).collect(), true);
        let sc = simplify(&closed, 0.2);
        assert!(sc.pts.len() < 40 && sc.pts.len() >= 8 && sc.closed);
        assert!((sc.area().abs() - closed.area().abs()).abs() / closed.area().abs() < 0.1);
        assert_eq!(simplify(&l, 0.0).pts.len(), l.pts.len());
    }

    #[test]
    fn serial_texts_count_with_padding_and_csv_rows_fill_a_template() {
        assert_eq!(serial_texts("SN-{n}", 8, 1, 4, 3), vec!["SN-008", "SN-009", "SN-010", "SN-011"]);
        assert_eq!(serial_texts("{n}", 10, -5, 3, 1), vec!["10", "5", "0"]);
        let rows = csv_rows("Anna,\"Smith, J.\",42\n\nBob;x;1\n");
        assert_eq!(rows[0], vec!["Anna", "Smith, J.", "42"]);
        assert_eq!(rows.len(), 2);
        assert_eq!(fill_row("{2} / {1} / {4}", &rows[0]), "Smith, J. / Anna / ");
        assert_eq!(csv_rows("a;b\nc;d")[1], vec!["c", "d"], "semicolons are the separator when the first line has more of them");
    }

    #[test]
    fn a_qr_code_and_a_code_39_barcode_are_made_and_a_bad_character_is_refused() {
        let (w, m) = qr_modules("https://example.com").unwrap();
        assert!(w >= 21 && m.len() == w * w && m.iter().any(|d| *d) && m.iter().any(|d| !*d));
        assert!(qr_modules(&"x".repeat(9000)).is_none());
        let bars = code39_bars("AB-1", 0.5).unwrap();
        // Six characters with the stars: 5 bars each, 20 bars in all... 4 characters + 2 stars = 6 characters of 5 bars.
        assert_eq!(bars.len(), 6 * 5);
        assert!(bars.windows(2).all(|w| w[1].0 > w[0].0 + w[0].1 - 1e-9), "bars do not overlap");
        assert!(code39_bars("a_b", 0.5).is_none() && code39_bars("é", 0.5).is_none());
        assert_eq!(code39_bars("ab", 0.5).unwrap(), code39_bars("AB", 0.5).unwrap(), "lower case is read as capitals");
    }

    #[test]
    fn boxes_are_packed_in_shelves_and_turned_when_allowed_and_a_too_big_set_is_refused() {
        let sizes = [(40.0, 20.0), (40.0, 20.0), (20.0, 60.0), (30.0, 30.0)];
        let p = nest_boxes(&sizes, (100.0, 100.0), 2.0, true).unwrap();
        // No two boxes overlap and all lie inside.
        let rects: Vec<(f64, f64, f64, f64)> = sizes.iter().zip(&p).map(|(s, q)| if q.2 { (q.0, q.1, s.1, s.0) } else { (q.0, q.1, s.0, s.1) }).collect();
        for (i, a) in rects.iter().enumerate() {
            assert!(a.0 >= 0.0 && a.1 >= 0.0 && a.0 + a.2 <= 100.0 + 1e-9 && a.1 + a.3 <= 100.0 + 1e-9, "{a:?}");
            for b in &rects[i + 1..] {
                let apart = a.0 + a.2 + 2.0 - 1e-9 <= b.0 || b.0 + b.2 + 2.0 - 1e-9 <= a.0 || a.1 + a.3 + 2.0 - 1e-9 <= b.1 || b.1 + b.3 + 2.0 - 1e-9 <= a.1;
                assert!(apart, "{a:?} {b:?}");
            }
        }
        assert!(p[2].2, "the tall one lies down");
        assert!(nest_boxes(&sizes, (50.0, 50.0), 2.0, true).is_none());
        assert!(nest_boxes(&[(120.0, 10.0)], (100.0, 100.0), 0.0, false).is_none());
        assert_eq!(box_of(&[rect(0.0, 0.0, 4.0, 3.0), rect(10.0, 10.0, 1.0, 1.0)]).map(|r| (r.max.x, r.max.y)), Some((11.0, 11.0)));
    }
}
