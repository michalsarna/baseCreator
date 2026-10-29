//! Joining paths: lines that meet end to end become one path, and a path whose ends meet is closed. Used when a drawing is read
//! (the lines of a CAD file are separate pieces) and by the editor commands Auto-join and Close path with tolerance.
use crate::bezier::{Contour, Node};
use crate::geom::Pt;
use std::collections::HashMap;

/// The contour run the other way: the nodes in reverse order and the handles of each swapped.
pub fn reversed(c: &Contour) -> Contour {
    Contour { nodes: c.nodes.iter().rev().map(|n| Node { p: n.p, hin: n.hout, hout: n.hin, smooth: n.smooth }).collect(), closed: c.closed }
}

/// `a` followed by `b`, which starts where `a` ends (the two end nodes become one).
fn append(mut a: Contour, b: Contour) -> Contour {
    if let (Some(last), Some(first)) = (a.nodes.last_mut(), b.nodes.first()) {
        last.hout = first.hout;
    }
    a.nodes.extend(b.nodes.into_iter().skip(1));
    a
}

fn cell(p: Pt, size: f64) -> (i64, i64) {
    ((p.x / size).floor() as i64, (p.y / size).floor() as i64)
}

/// Close an open path whose ends are within `tol` of each other (the last node is merged into the first). Returns whether it was
/// closed.
pub fn close_if_near(c: &mut Contour, tol: f64) -> bool {
    if c.closed || c.nodes.len() < 3 {
        return false;
    }
    let (first, last) = (c.nodes[0].p, c.nodes[c.nodes.len() - 1].p);
    if first.dist(last) > tol {
        return false;
    }
    let end = c.nodes.pop().unwrap_or(c.nodes[0]);
    c.nodes[0].hin = end.hin;
    c.closed = true;
    true
}

/// Join the open contours whose ends are within `tol` (mm) of each other into longer ones, closing those that come round; closed
/// contours stay as they are. The order of the result follows the order of the input.
pub fn join(items: Vec<Contour>, tol: f64) -> Vec<Contour> {
    let tol = tol.max(1e-9);
    let size = tol * 2.0;
    let mut out: Vec<Contour> = vec![];
    let mut open: Vec<Option<Contour>> = vec![];
    for c in items {
        if c.closed || c.nodes.len() < 2 {
            out.push(c);
        } else {
            open.push(Some(c));
        }
    }
    // Where the ends are: cell → (item, is it the tail).
    let mut grid: HashMap<(i64, i64), Vec<(usize, bool)>> = HashMap::new();
    let ends = |c: &Contour| (c.nodes[0].p, c.nodes[c.nodes.len() - 1].p);
    for (i, c) in open.iter().enumerate() {
        if let Some(c) = c {
            let (h, t) = ends(c);
            grid.entry(cell(h, size)).or_default().push((i, false));
            grid.entry(cell(t, size)).or_default().push((i, true));
        }
    }
    // The nearest end of another live contour to `p` within tol (skipping `me`).
    let find = |grid: &HashMap<(i64, i64), Vec<(usize, bool)>>, open: &[Option<Contour>], p: Pt, me: usize| -> Option<(usize, bool)> {
        let (cx, cy) = cell(p, size);
        let mut best: Option<(f64, usize, bool)> = None;
        for dx in -1..=1 {
            for dy in -1..=1 {
                for &(j, tail) in grid.get(&(cx + dx, cy + dy)).into_iter().flatten() {
                    if j == me {
                        continue;
                    }
                    let Some(c) = &open[j] else { continue };
                    let q = if tail { c.nodes[c.nodes.len() - 1].p } else { c.nodes[0].p };
                    let d = q.dist(p);
                    if d <= tol && best.is_none_or(|(bd, ..)| d < bd) {
                        best = Some((d, j, tail));
                    }
                }
            }
        }
        best.map(|(_, j, tail)| (j, tail))
    };
    for i in 0..open.len() {
        let Some(mut cur) = open[i].take() else { continue };
        loop {
            let (head, tail) = ends(&cur);
            // The tail meets another end.
            if let Some((j, other_tail)) = find(&grid, &open, tail, i) {
                let other = open[j].take().unwrap_or_default();
                let other = if other_tail { reversed(&other) } else { other };
                cur = append(cur, other);
                continue;
            }
            // The head meets another end: that one goes in front.
            if let Some((j, other_tail)) = find(&grid, &open, head, i) {
                let other = open[j].take().unwrap_or_default();
                let other = if other_tail { other } else { reversed(&other) };
                cur = append(other, cur);
                continue;
            }
            break;
        }
        close_if_near(&mut cur, tol);
        // Its ends have moved: whatever is looked for later finds them in the grid by the check above, which skips taken contours.
        out.push(cur);
        // Keep the grid right for the contours that are still waiting: only entries of taken ones are stale, and `find` skips those.
        let _ = &mut grid;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Polyline;

    fn line(a: (f64, f64), b: (f64, f64)) -> Contour {
        Contour::from_polyline(&Polyline::new(vec![Pt::new(a.0, a.1), Pt::new(b.0, b.1)], false))
    }

    #[test]
    fn lines_that_meet_become_one_path_whichever_way_they_are_drawn() {
        let joined = join(vec![line((0.0, 0.0), (10.0, 0.0)), line((10.0, 10.0), (10.0, 0.0)), line((10.0, 10.0), (0.0, 10.0)), line((50.0, 50.0), (60.0, 50.0))], 0.01);
        assert_eq!(joined.len(), 2, "{joined:?}");
        let long = joined.iter().find(|c| c.nodes.len() == 4).expect("three lines in one path");
        assert!(!long.closed);
        let ends = [long.nodes[0].p, long.nodes[3].p];
        assert!(ends.contains(&Pt::new(0.0, 0.0)) && ends.contains(&Pt::new(0.0, 10.0)));
    }

    #[test]
    fn a_path_that_comes_round_is_closed_and_a_gap_wider_than_the_tolerance_is_not_joined() {
        let square = vec![line((0.0, 0.0), (10.0, 0.0)), line((10.0, 0.0), (10.0, 10.0)), line((10.0, 10.0), (0.0, 10.0)), line((0.0, 10.0), (0.005, 0.0))];
        let joined = join(square.clone(), 0.01);
        assert_eq!(joined.len(), 1);
        assert!(joined[0].closed && joined[0].nodes.len() == 4);
        let apart = join(square, 0.001);
        assert!(apart.iter().all(|c| !c.closed), "the gap of 0.005 is wider than 0.001");
        // A path that is closed already is left alone.
        let mut ring = Contour::rect(5.0, 5.0);
        ring.closed = true;
        assert_eq!(join(vec![ring.clone()], 0.5), vec![ring]);
    }

    #[test]
    fn curves_keep_their_handles_when_they_are_joined_and_reversed() {
        let mut a = line((0.0, 0.0), (10.0, 0.0));
        a.nodes[0].hout = Pt::new(2.0, 3.0);
        a.nodes[1].hin = Pt::new(8.0, 3.0);
        let b = line((10.0, 0.0), (20.0, 0.0));
        let j = join(vec![a.clone(), b], 0.01);
        assert_eq!(j.len(), 1);
        assert_eq!(j[0].nodes[0].hout, Pt::new(2.0, 3.0));
        assert_eq!(j[0].nodes[1].hin, Pt::new(8.0, 3.0));
        let r = reversed(&a);
        assert_eq!(r.nodes[1].hin, Pt::new(2.0, 3.0));
        assert_eq!(r.nodes[0].hout, Pt::new(8.0, 3.0));
    }
}
