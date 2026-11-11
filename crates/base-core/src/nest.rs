//! Nesting by the real outlines of the parts (true shape): the sheet is a grid of small cells, every part (grown by half the gap,
//! and a little more so that the grid never lets two parts touch) is a mask of the cells it covers, holes left open, and the parts
//! are put one after the other, the biggest first, at the first place from the top left where they fit, turned the way that
//! fits highest. A small part can so go into the hole of a big one, and an L into the corner of another.
use crate::geom::{Polyline, Pt, Rect, Xf};
use crate::ops;

/// Cells of the sheet at most (the size of a cell follows from it, never finer than 0.25 mm).
const MAX_CELLS: f64 = 400_000.0;

/// The cells a part covers, row by row: for every row from the top the runs `[from, to)` of covered cells.
struct Mask {
    rows: Vec<Vec<(usize, usize)>>,
    width: usize,
    /// The corner of the grown outline (where the cell 0, 0 begins) in the part's turned coordinates.
    corner: Pt,
}

/// The covered runs of one row between the heights `y0` and `y1` (even-odd over all the rings): the union of the runs at the top,
/// the middle and the bottom of the row.
fn row_runs(rings: &[Polyline], y0: f64, y1: f64, x0: f64, cell: f64, width: usize) -> Vec<(usize, usize)> {
    let mut spans: Vec<(f64, f64)> = vec![];
    for y in [y0 + 1e-7, (y0 + y1) / 2.0, y1 - 1e-7] {
        let mut xs: Vec<f64> = vec![];
        for r in rings {
            let n = r.pts.len();
            for i in 0..n {
                let (a, b) = (r.pts[i], r.pts[(i + 1) % n]);
                if (a.y > y) != (b.y > y) {
                    xs.push(a.x + (y - a.y) * (b.x - a.x) / (b.y - a.y));
                }
            }
        }
        xs.sort_by(f64::total_cmp);
        for p in xs.chunks(2) {
            if let [a, b] = p {
                spans.push((*a, *b));
            }
        }
    }
    let mut cells: Vec<(usize, usize)> = spans
        .into_iter()
        .map(|(a, b)| (((a - x0) / cell).floor().max(0.0) as usize, (((b - x0) / cell).ceil().max(0.0) as usize).min(width)))
        .filter(|(a, b)| b > a)
        .collect();
    cells.sort();
    let mut out: Vec<(usize, usize)> = vec![];
    for (a, b) in cells {
        match out.last_mut() {
            Some(last) if a <= last.1 => last.1 = last.1.max(b),
            _ => out.push((a, b)),
        }
    }
    out
}

fn mask_of(polys: &[Polyline], grow: f64, cell: f64) -> Option<Mask> {
    let grown = ops::offset(polys, grow);
    let b = crate::design::box_of(&grown)?;
    let width = ((b.max.x - b.min.x) / cell).ceil().max(1.0) as usize;
    let height = ((b.max.y - b.min.y) / cell).ceil().max(1.0) as usize;
    let rows = (0..height).map(|j| row_runs(&grown, b.min.y + j as f64 * cell, b.min.y + (j + 1) as f64 * cell, b.min.x, cell, width)).collect();
    Some(Mask { rows, width, corner: b.min })
}

/// The sheet: which cells are taken, and for every cell how many free cells start there in its row.
struct Sheet {
    w: usize,
    h: usize,
    taken: Vec<Vec<bool>>,
    free: Vec<Vec<u32>>,
}

impl Sheet {
    fn new(w: usize, h: usize) -> Sheet {
        let mut s = Sheet { w, h, taken: vec![vec![false; w]; h], free: vec![vec![0; w + 1]; h] };
        for y in 0..h {
            s.recount(y);
        }
        s
    }
    fn recount(&mut self, y: usize) {
        let mut run = 0u32;
        for x in (0..self.w).rev() {
            run = if self.taken[y][x] { 0 } else { run + 1 };
            self.free[y][x] = run;
        }
    }
    fn fits(&self, m: &Mask, cx: usize, cy: usize) -> bool {
        if cx + m.width > self.w || cy + m.rows.len() > self.h {
            return false;
        }
        m.rows.iter().enumerate().all(|(r, runs)| runs.iter().all(|&(a, b)| self.free[cy + r][cx + a] as usize >= b - a))
    }
    /// The first place from the top left, rows first, no lower than `limit`.
    fn first_fit(&self, m: &Mask, limit: usize) -> Option<(usize, usize)> {
        if m.width > self.w || m.rows.len() > self.h {
            return None;
        }
        (0..=(self.h - m.rows.len()).min(limit)).find_map(|cy| (0..=self.w - m.width).find(|&cx| self.fits(m, cx, cy)).map(|cx| (cx, cy)))
    }
    fn take(&mut self, m: &Mask, cx: usize, cy: usize) {
        for (r, runs) in m.rows.iter().enumerate() {
            for &(a, b) in runs {
                for x in a..b {
                    self.taken[cy + r][cx + x] = true;
                }
            }
            self.recount(cy + r);
        }
    }
}

/// Where every part goes on the sheet `area`, at least `gap` apart, turned in `turns` equal steps of a full turn (1 = not
/// turned, 2 = also upside down, 4 = by quarters, 8 = by eighths): the transform to apply to the part, or `None` for a part that does
/// not fit (any more). The parts are in the coordinates of the sheet as they are now.
pub fn nest(parts: &[Vec<Polyline>], area: Rect, gap: f64, turns: u32) -> Vec<Option<Xf>> {
    let (aw, ah) = (area.max.x - area.min.x, area.max.y - area.min.y);
    let mut out = vec![None; parts.len()];
    if !(aw > 0.0 && ah > 0.0) {
        return out;
    }
    let cell = (aw * ah / MAX_CELLS).sqrt().max(0.25);
    let mut sheet = Sheet::new((aw / cell).floor() as usize, (ah / cell).floor() as usize);
    // Grown by half the gap on each part, and by most of a cell so that what the grid misses is never closer than the gap.
    let grow = gap.max(0.0) / 2.0 + cell * 0.75;
    let turns = turns.clamp(1, 36);
    // The biggest first (by the area of its box).
    let mut order: Vec<usize> = (0..parts.len()).collect();
    let size = |i: usize| crate::design::box_of(&parts[i]).map_or(0.0, |b| (b.max.x - b.min.x) * (b.max.y - b.min.y));
    order.sort_by(|&a, &b| size(b).total_cmp(&size(a)));
    for i in order {
        let polys: Vec<Polyline> = parts[i].iter().filter(|p| p.closed && p.pts.len() >= 3).cloned().collect();
        let Some(centre) = crate::design::box_of(&polys).map(|b| b.center()) else { continue };
        let mut best: Option<(usize, usize, Xf, Mask)> = None;
        for k in 0..turns {
            let turn = Xf::rotate_about(std::f64::consts::TAU * k as f64 / turns as f64, centre);
            let turned: Vec<Polyline> = polys.iter().map(|p| Polyline::new(p.pts.iter().map(|q| turn.apply(*q)).collect(), true)).collect();
            let Some(m) = mask_of(&turned, grow, cell) else { continue };
            let limit = best.as_ref().map_or(usize::MAX, |b| b.1);
            if let Some((cx, cy)) = sheet.first_fit(&m, limit) {
                if best.as_ref().is_none_or(|b| (cy, cx) < (b.1, b.0)) {
                    best = Some((cx, cy, turn, m));
                }
            }
        }
        if let Some((cx, cy, turn, m)) = best {
            sheet.take(&m, cx, cy);
            let to = Pt::new(area.min.x + cx as f64 * cell, area.min.y + cy as f64 * cell);
            out[i] = Some(turn.then(Xf::translate(to.x - m.corner.x, to.y - m.corner.y)));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(x: f64, y: f64, w: f64, h: f64) -> Polyline {
        Polyline::new(vec![Pt::new(x, y), Pt::new(x + w, y), Pt::new(x + w, y + h), Pt::new(x, y + h)], true)
    }

    fn moved(part: &[Polyline], xf: &Xf) -> Vec<Polyline> {
        part.iter().map(|p| Polyline::new(p.pts.iter().map(|q| xf.apply(*q)).collect(), true)).collect()
    }

    fn sheet(w: f64, h: f64) -> Rect {
        Rect { min: Pt::new(0.0, 0.0), max: Pt::new(w, h) }
    }

    /// No two placed parts overlap or come closer than `gap` (with a little for the flattening), and all are on the sheet.
    fn check(parts: &[Vec<Polyline>], places: &[Option<Xf>], area: Rect, gap: f64) {
        let placed: Vec<Vec<Polyline>> = parts.iter().zip(places).filter_map(|(p, x)| x.as_ref().map(|x| moved(p, x))).collect();
        for p in &placed {
            let b = crate::design::box_of(p).unwrap();
            assert!(b.min.x >= area.min.x - 1e-6 && b.min.y >= area.min.y - 1e-6 && b.max.x <= area.max.x + 1e-6 && b.max.y <= area.max.y + 1e-6, "{b:?}");
        }
        for i in 0..placed.len() {
            for j in i + 1..placed.len() {
                let a = ops::offset(&placed[i], gap / 2.0 - 0.05);
                let b = ops::offset(&placed[j], gap / 2.0 - 0.05);
                let both = ops::boolean(&a, &b, ops::BoolOp::Intersect);
                let area: f64 = both.iter().map(|p| p.area().abs()).sum();
                assert!(area < 1e-3, "parts {i} and {j} overlap by {area}");
            }
        }
    }

    #[test]
    fn rectangles_fill_the_sheet_without_touching() {
        let parts: Vec<Vec<Polyline>> = (0..6).map(|i| vec![rect(i as f64 * 50.0, 300.0, 40.0, 30.0)]).collect();
        let area = sheet(100.0, 100.0);
        let places = nest(&parts, area, 2.0, 1);
        assert!(places.iter().all(Option::is_some), "six 40 x 30 fit on 100 x 100 two by three");
        check(&parts, &places, area, 2.0);
        // Too many: the rest is left where it is.
        let more: Vec<Vec<Polyline>> = (0..12).map(|i| vec![rect(i as f64 * 50.0, 300.0, 40.0, 30.0)]).collect();
        let places = nest(&more, area, 2.0, 1);
        let n = places.iter().filter(|p| p.is_some()).count();
        assert!((6..12).contains(&n), "{n}");
        check(&more, &places, area, 2.0);
    }

    #[test]
    fn a_small_part_goes_into_the_hole_of_a_big_one() {
        // A frame 60 x 60 with a 40 x 40 hole, and a 20 x 20 square: the sheet is only 64 high, so the square must go in the hole.
        let mut hole = rect(10.0, 10.0, 40.0, 40.0);
        hole.pts.reverse();
        let parts = vec![vec![rect(0.0, 0.0, 60.0, 60.0), hole], vec![rect(200.0, 200.0, 20.0, 20.0)]];
        let area = sheet(64.0, 64.0);
        let places = nest(&parts, area, 2.0, 1);
        assert!(places.iter().all(Option::is_some), "{places:?}");
        check(&parts, &places, area, 2.0);
    }

    #[test]
    fn turning_lets_long_parts_fit_and_ls_lock_into_each_other() {
        // 80 x 10 bars on a 30 x 90 sheet: only turned by a quarter.
        let bars: Vec<Vec<Polyline>> = (0..2).map(|i| vec![rect(0.0, i as f64 * 20.0, 80.0, 10.0)]).collect();
        let area = sheet(30.0, 90.0);
        assert!(nest(&bars, area, 1.0, 1).iter().all(Option::is_none));
        let places = nest(&bars, area, 1.0, 4);
        assert!(places.iter().all(Option::is_some));
        check(&bars, &places, area, 1.0);
        // Two L shapes of 40 x 40 (arms 15 wide) fit on 59 x 59 only one in the corner of the other (turned half a turn).
        let l = |x: f64| vec![Polyline::new(vec![Pt::new(x, 0.0), Pt::new(x + 40.0, 0.0), Pt::new(x + 40.0, 15.0), Pt::new(x + 15.0, 15.0), Pt::new(x + 15.0, 40.0), Pt::new(x, 40.0)], true)];
        let ls = vec![l(0.0), l(100.0)];
        let area = sheet(59.0, 59.0);
        let places = nest(&ls, area, 1.0, 2);
        assert!(places.iter().all(Option::is_some), "{places:?}");
        check(&ls, &places, area, 1.0);
        let boxes = super::super::design::nest_boxes(&[(40.0, 40.0), (40.0, 40.0)], (59.0, 59.0), 1.0, true);
        assert!(boxes.is_none(), "by their boxes they do not fit");
    }
}
