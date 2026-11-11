//! Leads of a closed cut: the tool comes onto the line from the waste side and leaves it the same way, so that the mark of the
//! start (a plunge, the pierce of a laser) is not on the part. A lead is a straight line at an angle or a quarter of a circle that
//! touches the line; an overlap goes on past the start before the lead out. A lead that would cross the line or reach into another
//! ring is made smaller (half, a quarter) or left out.
use crate::geom::{Polyline, Pt};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum LeadKind {
    #[default]
    None,
    /// A straight line at `Lead::angle` to the cut.
    Line,
    /// A quarter of a circle that touches the cut, of the radius `Lead::length`.
    Arc,
}

impl LeadKind {
    pub const ALL: [LeadKind; 3] = [LeadKind::None, LeadKind::Line, LeadKind::Arc];
    pub fn label(self) -> &'static str {
        match self {
            LeadKind::None => "None",
            LeadKind::Line => "Line",
            LeadKind::Arc => "Arc",
        }
    }
}

/// The leads of a layer.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lead {
    #[serde(default)]
    pub lead_in: LeadKind,
    #[serde(default)]
    pub lead_out: LeadKind,
    /// The length of a line, the radius of an arc (mm).
    #[serde(default = "three")]
    pub length: f64,
    /// The angle of a line to the cut (degrees: 90 comes straight at it, a small angle almost along it).
    #[serde(default = "forty_five")]
    pub angle: f64,
    /// How far the cut goes on past its start before the lead out (mm).
    #[serde(default)]
    pub overlap: f64,
}

fn three() -> f64 {
    3.0
}
fn forty_five() -> f64 {
    45.0
}

impl Default for Lead {
    fn default() -> Lead {
        Lead { lead_in: LeadKind::None, lead_out: LeadKind::None, length: 3.0, angle: 45.0, overlap: 0.0 }
    }
}

impl Lead {
    /// Nothing changes the cut.
    pub fn is_off(&self) -> bool {
        self.lead_in == LeadKind::None && self.lead_out == LeadKind::None && self.overlap <= 0.0
    }

    pub fn is_default(&self) -> bool {
        *self == Lead::default()
    }

    pub fn check(&self) -> Result<(), String> {
        let ok = |v: f64, lo: f64, hi: f64| v.is_finite() && (lo..=hi).contains(&v);
        if !ok(self.length, 0.1, 1000.0) || !ok(self.angle, 5.0, 90.0) || !ok(self.overlap, 0.0, 1000.0) {
            return Err("a number of the leads is out of range".into());
        }
        Ok(())
    }
}

/// The leads of one ring, in the order they are cut: `lead_in` ends where the ring starts (its first point is where the tool goes
/// down), the ring is gone round once and on for `overlap` mm, and `lead_out` starts there.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Leads {
    /// From the start of the lead to the start of the ring (that point included).
    pub lead_in: Vec<Pt>,
    /// The overlap that fits (no more than the ring is long).
    pub overlap: f64,
    /// From the end of the overlap (that point included) to the end of the lead.
    pub lead_out: Vec<Pt>,
    /// A lead was made smaller to fit, or left out.
    pub shrunk: bool,
    pub dropped: bool,
}

/// The leads of a closed `ring` that starts at its first point and goes the way its points go. `away_inside`: the waste is inside
/// the ring (a hole, an inside profile), otherwise outside. `rings` are all the rings of the cut (this one among them): a lead
/// stays in the same space between them as the waste next to the start.
pub fn plan(ring: &Polyline, away_inside: bool, lead: &Lead, rings: &[Polyline]) -> Leads {
    let mut out = Leads::default();
    let pts = &ring.pts;
    if pts.len() < 3 || lead.is_off() {
        return out;
    }
    let len = ring.length();
    if !(len > 1e-6) {
        return out;
    }
    out.overlap = lead.overlap.clamp(0.0, len);
    let p0 = pts[0];
    let Some(t_in) = direction(p0, next_distinct(pts, 0)) else { return out };
    let side = away_normal(ring, away_inside);
    let m = midpoint(pts);
    let away = turn(side, t_in);
    let reference = region(rings, Pt::new(m.x + away.x * 1e-3, m.y + away.y * 1e-3));
    let fits = |path: &[Pt]| path_fits(path, ring, rings, &reference);
    if lead.lead_in != LeadKind::None {
        let n = turn(side, t_in);
        match shrink(lead, |l| {
            let path = lead_in_path(p0, t_in, n, lead.lead_in, l, lead.angle);
            fits(&path).then_some(path)
        }) {
            Some((path, smaller)) => {
                out.lead_in = path;
                out.shrunk |= smaller;
            }
            None => out.dropped = true,
        }
    }
    if lead.lead_out != LeadKind::None {
        let (o, t_out) = walk(ring, out.overlap);
        if let Some(t_out) = t_out {
            let n = turn(side, t_out);
            match shrink(lead, |l| {
                let path = lead_out_path(o, t_out, n, lead.lead_out, l, lead.angle);
                fits(&path).then_some(path)
            }) {
                Some((path, smaller)) => {
                    out.lead_out = path;
                    out.shrunk |= smaller;
                }
                None => out.dropped = true,
            }
        }
    }
    out
}

/// The ring started where a lead can touch it: a start at a corner (where the line turns by more than 10°) moves to the middle of
/// the longest piece, so that a lead along the line does not come from behind the corner. The path is the same.
pub fn good_start(ring: &Polyline) -> Polyline {
    let pts = &ring.pts;
    let n = pts.len();
    if n < 3 {
        return ring.clone();
    }
    let prev = pts.iter().rev().find(|p| p.dist(pts[0]) > 1e-9).copied().unwrap_or(pts[0]);
    if let (Some(a), Some(b)) = (direction(prev, pts[0]), direction(pts[0], next_distinct(pts, 0))) {
        if a.x * b.x + a.y * b.y > 10f64.to_radians().cos() {
            return ring.clone();
        }
    }
    // The longest piece, the first of equal ones.
    let piece = |i: usize| pts[i].dist(pts[(i + 1) % n]);
    let i = (1..n).fold(0, |best, i| if piece(i) > piece(best) + 1e-9 { i } else { best });
    let (a, b) = (pts[i], pts[(i + 1) % n]);
    let mut out = vec![Pt::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)];
    out.extend((1..=n).map(|k| pts[(i + k) % n]));
    Polyline::new(out, true)
}

/// The full size, half, a quarter: the first that `make` accepts, and whether it was smaller than asked.
fn shrink(lead: &Lead, make: impl Fn(f64) -> Option<Vec<Pt>>) -> Option<(Vec<Pt>, bool)> {
    [1.0, 0.5, 0.25].iter().enumerate().find_map(|(i, f)| make(lead.length * f).map(|p| (p, i > 0)))
}

/// Which side of the ring the waste is on, as a sign for the left normal of the direction of travel: +1 left, -1 right.
fn away_normal(ring: &Polyline, away_inside: bool) -> f64 {
    let pts = &ring.pts;
    let a = pts[0];
    let b = next_distinct(pts, 0);
    let Some(t) = direction(a, b) else { return 1.0 };
    let probe = Pt::new((a.x + b.x) / 2.0 - t.y * 1e-4, (a.y + b.y) / 2.0 + t.x * 1e-4);
    if ring.contains(probe) == away_inside {
        1.0
    } else {
        -1.0
    }
}

fn turn(side: f64, t: Pt) -> Pt {
    Pt::new(-t.y * side, t.x * side)
}

fn next_distinct(pts: &[Pt], i: usize) -> Pt {
    pts.iter().skip(i + 1).chain(pts.iter()).find(|p| p.dist(pts[i]) > 1e-9).copied().unwrap_or(pts[i])
}

fn direction(a: Pt, b: Pt) -> Option<Pt> {
    let d = a.dist(b);
    (d > 1e-9).then(|| Pt::new((b.x - a.x) / d, (b.y - a.y) / d))
}

fn midpoint(pts: &[Pt]) -> Pt {
    let b = next_distinct(pts, 0);
    Pt::new((pts[0].x + b.x) / 2.0, (pts[0].y + b.y) / 2.0)
}

/// Which rings hold a point.
fn region(rings: &[Polyline], p: Pt) -> Vec<bool> {
    rings.iter().map(|r| r.contains(p)).collect()
}

/// Does the lead stay in the same space between the rings as the waste by the start? Looked at every 0.1 mm, leaving out where it
/// touches its own ring.
fn path_fits(path: &[Pt], own: &Polyline, rings: &[Polyline], reference: &[bool]) -> bool {
    for w in path.windows(2) {
        let l = w[0].dist(w[1]);
        let n = (l / 0.1).ceil().max(1.0) as usize;
        for k in 0..=n {
            let f = k as f64 / n as f64;
            let p = Pt::new(w[0].x + (w[1].x - w[0].x) * f, w[0].y + (w[1].y - w[0].y) * f);
            if own.dist_to(p) < 0.01 {
                continue;
            }
            if region(rings, p) != reference {
                return false;
            }
        }
    }
    true
}

/// The way in to `p0`, along which the tool cuts in the direction `t`, from the waste side `n`.
fn lead_in_path(p0: Pt, t: Pt, n: Pt, kind: LeadKind, l: f64, angle: f64) -> Vec<Pt> {
    match kind {
        LeadKind::None => vec![],
        LeadKind::Line => {
            let (s, c) = angle.to_radians().sin_cos();
            vec![Pt::new(p0.x + (-t.x * c + n.x * s) * l, p0.y + (-t.y * c + n.y * s) * l), p0]
        }
        LeadKind::Arc => {
            // The centre is `l` away on the waste side; the arc starts a quarter turn back and comes onto the line along it.
            let c = Pt::new(p0.x + n.x * l, p0.y + n.y * l);
            (0..=8)
                .rev()
                .map(|k| {
                    let phi = k as f64 / 8.0 * std::f64::consts::FRAC_PI_2;
                    Pt::new(c.x + l * (-phi.cos() * n.x - phi.sin() * t.x), c.y + l * (-phi.cos() * n.y - phi.sin() * t.y))
                })
                .collect()
        }
    }
}

/// The way out from `p` (where the tool moves in the direction `t`), to the waste side `n`.
fn lead_out_path(p: Pt, t: Pt, n: Pt, kind: LeadKind, l: f64, angle: f64) -> Vec<Pt> {
    match kind {
        LeadKind::None => vec![],
        LeadKind::Line => {
            let (s, c) = angle.to_radians().sin_cos();
            vec![p, Pt::new(p.x + (t.x * c + n.x * s) * l, p.y + (t.y * c + n.y * s) * l)]
        }
        LeadKind::Arc => {
            let c = Pt::new(p.x + n.x * l, p.y + n.y * l);
            (0..=8)
                .map(|k| {
                    let phi = k as f64 / 8.0 * std::f64::consts::FRAC_PI_2;
                    Pt::new(c.x + l * (-phi.cos() * n.x + phi.sin() * t.x), c.y + l * (-phi.cos() * n.y + phi.sin() * t.y))
                })
                .collect()
        }
    }
}

/// The point `d` mm along the closed ring from its start, and the direction there.
pub fn walk(ring: &Polyline, d: f64) -> (Pt, Option<Pt>) {
    let pts = &ring.pts;
    let n = pts.len();
    let mut left = d.max(0.0);
    for i in 0..n {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let l = a.dist(b);
        if l < 1e-12 {
            continue;
        }
        if left <= l {
            let f = left / l;
            return (Pt::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f), direction(a, b));
        }
        left -= l;
    }
    (pts[0], direction(pts[0], next_distinct(pts, 0)))
}

/// The points of the ring from its start, round once and on for `overlap` mm, ending at the point of the lead out.
pub fn ring_with_overlap(ring: &Polyline, overlap: f64) -> Vec<Pt> {
    let mut out = ring.pts.clone();
    if let Some(&first) = ring.pts.first() {
        out.push(first);
    }
    if overlap > 0.0 {
        let pts = &ring.pts;
        let n = pts.len();
        let mut left = overlap;
        for i in 0..n {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let l = a.dist(b);
            if left <= l {
                if l > 1e-12 {
                    let f = left / l;
                    out.push(Pt::new(a.x + (b.x - a.x) * f, a.y + (b.y - a.y) * f));
                }
                break;
            }
            out.push(b);
            left -= l;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64) -> Polyline {
        Polyline::new(vec![Pt::new(x, y), Pt::new(x + s, y), Pt::new(x + s, y + s), Pt::new(x, y + s)], true)
    }

    fn lead(kind: LeadKind, length: f64) -> Lead {
        Lead { lead_in: kind, lead_out: kind, length, ..Lead::default() }
    }

    #[test]
    fn nothing_is_added_when_the_leads_are_off() {
        let r = square(0.0, 0.0, 10.0);
        assert!(Lead::default().is_off());
        assert_eq!(plan(&r, false, &Lead::default(), &[r.clone()]), Leads::default());
    }

    #[test]
    fn a_lead_comes_from_the_waste_side() {
        let r = good_start(&square(0.0, 0.0, 10.0));
        for kind in [LeadKind::Line, LeadKind::Arc] {
            // Outside the part: the lead starts outside the square, below its first edge (which runs along +x at y = 0).
            let l = plan(&r, false, &lead(kind, 3.0), &[r.clone()]);
            let s = l.lead_in[0];
            assert!(!r.contains(s) && s.y < -1.0, "{kind:?} {s:?}");
            assert_eq!(*l.lead_in.last().unwrap(), Pt::new(5.0, 0.0));
            assert!(!r.contains(*l.lead_out.last().unwrap()));
            assert!(!l.shrunk && !l.dropped);
            // A hole: from inside.
            let h = plan(&r, true, &lead(kind, 3.0), &[r.clone()]);
            assert!(r.contains(h.lead_in[0]), "{kind:?}");
            assert!(r.contains(*h.lead_out.last().unwrap()));
        }
        // The same for a ring that goes the other way round.
        let mut back = r.clone();
        back.pts.reverse();
        back.pts.rotate_right(1);
        let l = plan(&back, false, &lead(LeadKind::Arc, 3.0), &[back.clone()]);
        assert!(!back.contains(l.lead_in[0]));
    }

    #[test]
    fn an_arc_touches_the_line_and_a_line_has_its_angle() {
        let r = good_start(&square(0.0, 0.0, 20.0));
        let a = plan(&r, false, &lead(LeadKind::Arc, 4.0), &[r.clone()]);
        // It ends going along +x, and starts 4 mm back and 4 mm out.
        let n = a.lead_in.len();
        let (p, q) = (a.lead_in[n - 2], a.lead_in[n - 1]);
        assert!((q.y - p.y).abs() < 0.2 * (q.x - p.x), "{p:?} {q:?}");
        assert!(a.lead_in[0].dist(Pt::new(6.0, -4.0)) < 1e-9, "{:?}", a.lead_in[0]);
        let mut ld = lead(LeadKind::Line, 5.0);
        ld.angle = 90.0;
        let l = plan(&r, false, &ld, &[r.clone()]);
        assert!(l.lead_in[0].dist(Pt::new(10.0, -5.0)) < 1e-9);
        // A start in the middle of a piece stays; the path is the same.
        assert_eq!(good_start(&r), r);
        assert_eq!(r.pts.len(), 5);
        assert!((r.length() - 80.0).abs() < 1e-9 && (r.area().abs() - 400.0).abs() < 1e-9);
    }

    #[test]
    fn a_lead_that_does_not_fit_is_made_smaller_or_left_out() {
        // A hole of 4 mm: an arc of 3 mm reaches across it, 1.5 mm fits.
        let r = good_start(&square(0.0, 0.0, 4.0));
        let l = plan(&r, true, &lead(LeadKind::Arc, 3.0), &[r.clone()]);
        assert!(l.shrunk && !l.dropped, "{l:?}");
        assert!(l.lead_in.iter().all(|p| p.x >= -1e-9 && p.y >= -1e-9));
        // A hole of 0.5 mm: nothing fits.
        let tiny = good_start(&square(0.0, 0.0, 0.5));
        let l = plan(&tiny, true, &lead(LeadKind::Arc, 3.0), &[tiny.clone()]);
        assert!(l.dropped && l.lead_in.is_empty());
        // Another part close by: the lead must not reach into it.
        let a = good_start(&square(0.0, 0.0, 10.0));
        let b = square(0.0, -3.0, 10.0);
        let b = Polyline::new(b.pts.iter().map(|p| Pt::new(p.x, p.y - 9.0)).collect(), true);
        let l = plan(&a, false, &lead(LeadKind::Line, 6.0), &[a.clone(), b.clone()]);
        assert!(l.shrunk && !b.contains(l.lead_in[0]), "{l:?}");
    }

    #[test]
    fn the_overlap_goes_on_past_the_start() {
        let r = square(0.0, 0.0, 10.0);
        let mut ld = lead(LeadKind::Line, 2.0);
        ld.overlap = 3.0;
        let l = plan(&r, false, &ld, &[r.clone()]);
        assert_eq!(l.overlap, 3.0);
        assert!(l.lead_out[0].dist(Pt::new(3.0, 0.0)) < 1e-9);
        let way = ring_with_overlap(&r, 3.0);
        assert_eq!(way.first(), Some(&Pt::new(0.0, 0.0)));
        assert!(way.last().unwrap().dist(Pt::new(3.0, 0.0)) < 1e-9);
        assert_eq!(way.len(), 6);
        // Past a corner.
        let (p, t) = walk(&r, 12.0);
        assert!(p.dist(Pt::new(10.0, 2.0)) < 1e-9 && t == Some(Pt::new(0.0, 1.0)));
    }

    #[test]
    fn the_settings_are_checked_and_old_files_read() {
        assert!(Lead::default().check().is_ok());
        assert!(Lead { angle: 2.0, ..Lead::default() }.check().is_err());
        assert!(Lead { length: f64::NAN, ..Lead::default() }.check().is_err());
        let l: Lead = serde_json::from_str("{}").unwrap();
        assert!(l.is_default());
    }
}
