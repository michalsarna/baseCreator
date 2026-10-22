//! Reads a G-code file into a toolpath for viewing: straight segments in millimetres, split into the ones that cut
//! (the laser burns, the tool is in the material) and the rapid or idle moves. Arcs (G2/G3, with centre offsets or a
//! radius, in any of the three planes) are flattened. Nothing is run or checked against a machine: unknown words are
//! skipped, so a file from any CAM program shows what it can.

/// One straight piece of the path.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Seg {
    pub a: [f32; 3],
    pub b: [f32; 3],
    /// Cutting (burning, milling, drawing) or a move through the air.
    pub cut: bool,
    /// The spindle / laser power S of the move as a share of the biggest S in the file (1 when the file has no S).
    pub power: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Toolpath {
    pub segs: Vec<Seg>,
    pub min: [f32; 3],
    pub max: [f32; 3],
    /// Lines of the file that were read.
    pub lines: usize,
    /// Z changes along the path: a mill or a plotter with a lifting pen; a laser file stays flat.
    pub is_3d: bool,
    pub cut_len: f64,
    pub rapid_len: f64,
    /// Rough time in seconds: the cuts at their feed rate, the rapids at 3000 mm/min.
    pub est_seconds: f64,
    pub max_s: f64,
    /// Words that were not understood (counted, not listed).
    pub skipped: usize,
}

const ARC_TOL: f64 = 0.02;

struct State {
    pos: [f64; 3],
    abs: bool,
    inch: bool,
    /// 0 rapid, 1 line, 2 clockwise arc, 3 counter-clockwise arc.
    motion: u8,
    plane: u8,
    spindle: bool,
    s: f64,
    feed: f64,
}

/// Splits a line into words (a letter and a number), without comments: `(...)` and `; ...`.
fn words(line: &str) -> Vec<(char, f64)> {
    let mut out = vec![];
    let mut depth = 0;
    let b: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < b.len() {
        let c = b[i];
        if c == '(' {
            depth += 1;
        } else if c == ')' {
            if depth > 0 {
                depth -= 1;
            }
        } else if depth == 0 {
            if c == ';' {
                break;
            }
            if c.is_ascii_alphabetic() {
                let mut j = i + 1;
                let mut num = String::new();
                while j < b.len() && (b[j].is_ascii_digit() || matches!(b[j], '.' | '-' | '+') || b[j] == ' ') {
                    if b[j] != ' ' {
                        num.push(b[j]);
                    } else if !num.is_empty() {
                        break;
                    }
                    j += 1;
                }
                if let Ok(v) = num.parse::<f64>() {
                    out.push((c.to_ascii_uppercase(), v));
                }
                i = j;
                continue;
            }
        }
        i += 1;
    }
    out
}

pub fn parse(text: &str) -> Toolpath {
    let mut st = State { pos: [0.0; 3], abs: true, inch: false, motion: 0, plane: 17, spindle: false, s: 0.0, feed: 1000.0 };
    let mut tp = Toolpath::default();
    // The moves are first kept with what the spindle was doing; whether a G1 is a cut is decided at the end.
    struct Raw {
        seg: Seg,
        motion: u8,
        on: bool,
        s: f64,
        feed: f64,
    }
    let mut raw: Vec<Raw> = vec![];
    let mut any_spindle = false;
    for line in text.lines() {
        tp.lines += 1;
        let ws = words(line);
        if ws.is_empty() {
            continue;
        }
        let mut target = st.pos;
        let mut has_move = false;
        let (mut i, mut j, mut k, mut r) = (None, None, None, None);
        let k_unit = if st.inch { 25.4 } else { 1.0 };
        for &(c, v) in &ws {
            match c {
                'G' => match v.round() as i32 {
                    0 => st.motion = 0,
                    1 => st.motion = 1,
                    2 => st.motion = 2,
                    3 => st.motion = 3,
                    17 => st.plane = 17,
                    18 => st.plane = 18,
                    19 => st.plane = 19,
                    20 => st.inch = true,
                    21 => st.inch = false,
                    90 => st.abs = true,
                    91 => st.abs = false,
                    4 | 28 | 30 | 40 | 49 | 54..=59 | 61 | 64 | 80 | 92 | 93 | 94 | 95 => {}
                    _ => tp.skipped += 1,
                },
                'M' => match v.round() as i32 {
                    3 | 4 => {
                        st.spindle = true;
                        any_spindle = true;
                    }
                    5 => st.spindle = false,
                    _ => {}
                },
                'S' => {
                    st.s = v;
                    any_spindle = true;
                    if v > tp.max_s {
                        tp.max_s = v;
                    }
                }
                'F' => st.feed = v * k_unit,
                'X' | 'Y' | 'Z' => {
                    let ax = (c as u8 - b'X') as usize;
                    let mm = v * k_unit;
                    target[ax] = if st.abs { mm } else { st.pos[ax] + mm };
                    has_move = true;
                }
                'I' => i = Some(v * k_unit),
                'J' => j = Some(v * k_unit),
                'K' => k = Some(v * k_unit),
                'R' => r = Some(v * k_unit),
                _ => {}
            }
        }
        if !has_move {
            continue;
        }
        let from = st.pos;
        let push = |raw: &mut Vec<Raw>, a: [f64; 3], b: [f64; 3], st: &State| {
            raw.push(Raw {
                seg: Seg { a: [a[0] as f32, a[1] as f32, a[2] as f32], b: [b[0] as f32, b[1] as f32, b[2] as f32], cut: false, power: 0.0 },
                motion: st.motion,
                on: st.spindle,
                s: st.s,
                feed: st.feed,
            });
        };
        if st.motion >= 2 {
            // The plane picks the two axes of the circle and the axis the arc climbs along.
            let (ax, ay, az, io, jo) = match st.plane {
                18 => (2, 0, 1, k, i),
                19 => (1, 2, 0, j, k),
                _ => (0, 1, 2, i, j),
            };
            let (sx, sy) = (from[ax], from[ay]);
            let (ex, ey) = (target[ax], target[ay]);
            let centre = match (io, jo, r) {
                (Some(a), Some(b), _) => Some((sx + a, sy + b)),
                (None, None, Some(rad)) => {
                    let (dx, dy) = (ex - sx, ey - sy);
                    let d = (dx * dx + dy * dy).sqrt();
                    if d < 1e-9 || rad.abs() < d / 2.0 - 1e-9 {
                        None
                    } else {
                        let h = (rad * rad - d * d / 4.0).max(0.0).sqrt();
                        // The sign of R and the direction pick which of the two centres.
                        let sign = if (st.motion == 3) == (rad > 0.0) { 1.0 } else { -1.0 };
                        Some((sx + dx / 2.0 - sign * h * dy / d, sy + dy / 2.0 + sign * h * dx / d))
                    }
                }
                _ => None,
            };
            if let Some((cx, cy)) = centre {
                let rad = ((sx - cx).powi(2) + (sy - cy).powi(2)).sqrt();
                let a0 = (sy - cy).atan2(sx - cx);
                let a1 = (ey - cy).atan2(ex - cx);
                let tau = std::f64::consts::TAU;
                let mut sweep = if st.motion == 3 { a1 - a0 } else { a0 - a1 };
                while sweep < 0.0 {
                    sweep += tau;
                }
                // The same point twice is a whole circle.
                if sweep < 1e-9 && (sx - ex).abs() < 1e-9 && (sy - ey).abs() < 1e-9 {
                    sweep = tau;
                }
                let step = if rad > ARC_TOL { 2.0 * (1.0 - ARC_TOL / rad).acos() } else { 0.5 };
                let n = ((sweep / step.clamp(0.02, 0.2)).ceil() as usize).clamp(2, 4000);
                let mut prev = from;
                for q in 1..=n {
                    let f = q as f64 / n as f64;
                    let ang = if st.motion == 3 { a0 + sweep * f } else { a0 - sweep * f };
                    let mut p = [0.0; 3];
                    p[ax] = cx + rad * ang.cos();
                    p[ay] = cy + rad * ang.sin();
                    p[az] = from[az] + (target[az] - from[az]) * f;
                    if q == n {
                        p = target;
                    }
                    push(&mut raw, prev, p, &st);
                    prev = p;
                }
            } else {
                tp.skipped += 1;
                push(&mut raw, from, target, &st);
            }
        } else {
            push(&mut raw, from, target, &st);
        }
        st.pos = target;
    }
    let max_s = if tp.max_s > 0.0 { tp.max_s } else { 1.0 };
    let mut min = [f32::MAX; 3];
    let mut max = [f32::MIN; 3];
    let (mut z_lo, mut z_hi) = (f32::MAX, f32::MIN);
    for r in &raw {
        let mut seg = r.seg;
        // Where the file drives the spindle or the laser, a cut is a line move with it on; otherwise every line move is a cut.
        seg.cut = r.motion >= 1 && (!any_spindle || (r.on && r.s > 0.0));
        seg.power = if any_spindle { (r.s / max_s).clamp(0.0, 1.0) as f32 } else { 1.0 };
        let len = (0..3).map(|d| ((seg.b[d] - seg.a[d]) as f64).powi(2)).sum::<f64>().sqrt();
        if seg.cut {
            tp.cut_len += len;
            tp.est_seconds += len / (r.feed.max(1.0) / 60.0);
        } else {
            tp.rapid_len += len;
            tp.est_seconds += len / (3000.0 / 60.0);
        }
        for p in [seg.a, seg.b] {
            for d in 0..3 {
                min[d] = min[d].min(p[d]);
                max[d] = max[d].max(p[d]);
            }
            z_lo = z_lo.min(p[2]);
            z_hi = z_hi.max(p[2]);
        }
        tp.segs.push(seg);
    }
    if tp.segs.is_empty() {
        min = [0.0; 3];
        max = [0.0; 3];
        z_lo = 0.0;
        z_hi = 0.0;
    }
    tp.min = min;
    tp.max = max;
    tp.is_3d = z_hi - z_lo > 0.01;
    tp
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_square_of_a_laser_is_flat_and_the_travel_is_not_a_cut() {
        let t = parse("G21 G90\nG0 X10 Y10\nM4 S800\nG1 X20 Y10 F1000\nG1 X20 Y20\nG1 X10 Y20\nG1 X10 Y10\nM5 S0\nG0 X0 Y0\n");
        assert!(!t.is_3d);
        assert_eq!(t.segs.len(), 6);
        assert_eq!(t.segs.iter().filter(|s| s.cut).count(), 4);
        assert!((t.cut_len - 40.0).abs() < 1e-6);
        assert_eq!((t.min[0], t.max[0], t.min[1], t.max[1]), (0.0, 20.0, 0.0, 20.0));
        assert!((t.segs[1].power - 1.0).abs() < 1e-6);
    }

    #[test]
    fn a_mill_file_without_a_spindle_command_cuts_with_every_line_move_and_goes_3d() {
        let t = parse("G0 Z5\nG0 X1 Y1\nG1 Z-1 F100\nG1 X5\nG0 Z5\n");
        assert!(t.is_3d);
        assert_eq!(t.segs.iter().filter(|s| s.cut).count(), 2);
    }

    #[test]
    fn arcs_are_flattened_and_end_where_they_should() {
        // A half circle, counter-clockwise, round (5, 0).
        let t = parse("G0 X0 Y0\nG3 X10 Y0 I5 J0\n");
        let last = t.segs.last().unwrap();
        assert_eq!(last.b, [10.0, 0.0, 0.0]);
        assert!(t.segs.len() > 8);
        assert!(t.max[1].abs() > 4.9 || t.min[1].abs() > 4.9);
        // A radius gives the same half circle.
        let r = parse("G0 X0 Y0\nG3 X10 Y0 R5\n");
        assert!((r.cut_len - t.cut_len).abs() < 0.05);
        // The whole circle.
        let c = parse("G0 X5 Y0\nG2 X5 Y0 I-5 J0\n");
        assert!((c.cut_len - std::f64::consts::TAU * 5.0).abs() < 0.1);
    }

    #[test]
    fn inches_relative_moves_and_comments_are_understood() {
        let t = parse("G20 G91 (a comment G1 X999)\nG1 X1 ; X500\nG1 Y1\n");
        assert_eq!(t.segs.last().unwrap().b, [25.4, 25.4, 0.0]);
    }

    #[test]
    fn words_without_spaces_and_line_numbers_are_read() {
        let t = parse("N10 G1X5Y5F300\n");
        assert_eq!(t.segs[0].b, [5.0, 5.0, 0.0]);
    }
}
