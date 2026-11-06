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

/// What dropping a node onto another node of the same path did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Dropped {
    /// An end of an open path was dropped on its other end: the path is closed.
    Closed,
    /// A node was dropped on its neighbour: the two are one node now.
    Merged,
}

/// Node `ai` of `c` dropped onto node `bi` of the same path. The node dropped on keeps its place; nothing is done for nodes that are
/// not ends of an open path or neighbours.
pub fn drop_on_same(c: &mut Contour, ai: usize, bi: usize) -> Option<Dropped> {
    let n = c.nodes.len();
    if ai == bi || ai >= n || bi >= n {
        return None;
    }
    let last = n - 1;
    if !c.closed && n >= 3 && ((ai == 0 && bi == last) || (ai == last && bi == 0)) {
        if ai == last {
            // The first node takes the curve that came into the last one.
            c.nodes[0].hin = c.nodes[last].hin;
            c.nodes.pop();
        } else {
            c.nodes[last].hout = c.nodes[0].hout;
            c.nodes.remove(0);
        }
        c.closed = true;
        return Some(Dropped::Closed);
    }
    let next = |i: usize| if i + 1 < n { Some(i + 1) } else if c.closed { Some(0) } else { None };
    let prev = |i: usize| if i > 0 { Some(i - 1) } else if c.closed { Some(last) } else { None };
    let keep_min = if c.closed { 3 } else { 2 };
    if n <= keep_min {
        return None;
    }
    if next(ai) == Some(bi) {
        // The segment that came into the dropped node now comes into the one it was dropped on.
        c.nodes[bi].hin = c.nodes[ai].hin;
    } else if prev(ai) == Some(bi) {
        c.nodes[bi].hout = c.nodes[ai].hout;
    } else {
        return None;
    }
    c.nodes.remove(ai);
    Some(Dropped::Merged)
}

/// End `ai` of open path `a` dropped onto end `bi` of another open path `b`: one path, `a` then `b`, the node dropped on keeping its
/// place. `None` when one of the nodes is not an end.
pub fn drop_on_other(a: &Contour, ai: usize, b: &Contour, bi: usize) -> Option<Contour> {
    let end = |c: &Contour, i: usize| !c.closed && c.nodes.len() >= 2 && (i == 0 || i + 1 == c.nodes.len());
    if !end(a, ai) || !end(b, bi) {
        return None;
    }
    // `a` runs into the dropped node, `b` runs out of the node dropped on.
    let a = if ai == 0 { reversed(a) } else { a.clone() };
    let b = if bi == 0 { b.clone() } else { reversed(b) };
    let mut joined = a;
    let first = b.nodes[0];
    if let Some(last) = joined.nodes.last_mut() {
        last.p = first.p;
        last.hout = first.hout;
        last.smooth = false;
    }
    joined.nodes.extend(b.nodes.into_iter().skip(1));
    joined.closed = false;
    Some(joined)
}

/// Break a path at node `i`: a closed path opens there (the node becomes both ends); an open one is cut in two there, and the part
/// after the node is returned (both parts keep a copy of the node). `None` for an end of an open path, which has nothing to cut.
pub fn break_at(c: &mut Contour, i: usize) -> Option<Contour> {
    let n = c.nodes.len();
    if i >= n {
        return None;
    }
    if c.closed {
        c.nodes.rotate_left(i);
        let mut end = c.nodes[0];
        end.hout = end.p;
        c.nodes[0].hin = c.nodes[0].p;
        c.nodes.push(end);
        c.closed = false;
        return None;
    }
    if i == 0 || i + 1 >= n {
        return None;
    }
    let mut tail = Contour { nodes: c.nodes[i..].to_vec(), closed: false };
    tail.nodes[0].hin = tail.nodes[0].p;
    c.nodes.truncate(i + 1);
    let last = c.nodes.len() - 1;
    c.nodes[last].hout = c.nodes[last].p;
    Some(tail)
}

#[cfg(test)]
mod drop_tests {
    use super::*;

    fn open(pts: &[(f64, f64)]) -> Contour {
        Contour { nodes: pts.iter().map(|(x, y)| Node::corner(Pt::new(*x, *y))).collect(), closed: false }
    }

    #[test]
    fn an_end_dropped_on_the_other_end_closes_the_path_and_a_neighbour_merges() {
        let mut c = open(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.1, 0.1)]);
        assert_eq!(drop_on_same(&mut c, 3, 0), Some(Dropped::Closed));
        assert!(c.closed && c.nodes.len() == 3 && c.nodes[0].p == Pt::new(0.0, 0.0), "the node dropped on keeps its place");
        let mut c = open(&[(0.0, 0.0), (10.0, 0.0), (10.2, 0.0), (20.0, 0.0)]);
        assert_eq!(drop_on_same(&mut c, 1, 2), Some(Dropped::Merged));
        assert_eq!(c.nodes.iter().map(|n| n.p.x).collect::<Vec<_>>(), vec![0.0, 10.2, 20.0]);
        let mut c = open(&[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0)]);
        assert_eq!(drop_on_same(&mut c, 0, 2), None, "not neighbours, not the two ends");
        let mut two = open(&[(0.0, 0.0), (1.0, 0.0)]);
        assert_eq!(drop_on_same(&mut two, 0, 1), None, "a path of two nodes stays");
    }

    #[test]
    fn ends_of_two_paths_dropped_on_each_other_make_one_path() {
        let a = open(&[(0.0, 0.0), (10.0, 0.0)]);
        let b = open(&[(20.0, 5.0), (10.1, 0.1)]);
        // The end of `a` dropped on the end of `b`: a, then b the other way.
        let j = drop_on_other(&a, 1, &b, 1).unwrap();
        assert_eq!(j.nodes.iter().map(|n| (n.p.x, n.p.y)).collect::<Vec<_>>(), vec![(0.0, 0.0), (10.1, 0.1), (20.0, 5.0)]);
        // The start of `a` dropped on the start of `b`.
        let j = drop_on_other(&a, 0, &open(&[(0.1, 0.0), (-10.0, 0.0)]), 0).unwrap();
        assert_eq!(j.nodes.iter().map(|n| n.p.x).collect::<Vec<_>>(), vec![10.0, 0.1, -10.0]);
        assert!(drop_on_other(&open(&[(0.0, 0.0), (5.0, 0.0), (9.0, 0.0)]), 1, &b, 0).is_none(), "not an end");
    }

    #[test]
    fn breaking_opens_a_closed_path_and_cuts_an_open_one_in_two() {
        let mut c = open(&[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0)]);
        c.closed = true;
        assert!(break_at(&mut c, 1).is_none());
        assert!(!c.closed && c.nodes.len() == 4 && c.nodes[0].p == c.nodes[3].p && c.nodes[0].p == Pt::new(10.0, 0.0));
        let mut o = open(&[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0)]);
        let tail = break_at(&mut o, 2).unwrap();
        assert_eq!((o.nodes.len(), tail.nodes.len(), tail.nodes[0].p), (3, 2, Pt::new(20.0, 0.0)));
        assert!(break_at(&mut o, 0).is_none(), "an end has nothing to cut");
    }
}
