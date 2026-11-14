//! Lines of many short pieces (the moves of a program) drawn as few shapes, so that a program of hundreds of thousands of moves does
//! not hold the picture up: see [`Lines`].
use egui::{Pos2, Rect, Shape, Stroke};

/// Lines of many short moves drawn as few shapes: moves that follow each other in the same stroke are one polyline, points closer
/// than half a pixel to the last one kept are left out, and what lies wholly outside `clip` is not drawn. A program of hundreds of
/// thousands of moves is then as many points as the picture can show, not a shape for every move.
pub struct Lines {
    shapes: Vec<Shape>,
    pts: Vec<Pos2>,
    /// The end of the last move, when it was not kept (too close to the last point kept).
    pending: Option<Pos2>,
    stroke: Stroke,
    dashed: bool,
    clip: Rect,
}

impl Lines {
    /// The smallest distance (in points) between two points kept.
    const MIN_STEP: f32 = 0.5;

    /// More rapid moves than this are drawn plain: hundreds of dashes for each of thousands of moves would hold the picture up.
    pub const MAX_DASHED: usize = 2_000;

    pub fn new(clip: Rect) -> Lines {
        Lines { shapes: vec![], pts: vec![], pending: None, stroke: Stroke::NONE, dashed: false, clip: clip.expand(4.0) }
    }

    /// The end of the polyline that is being made.
    fn end(&self) -> Option<Pos2> {
        self.pending.or_else(|| self.pts.last().copied())
    }

    /// A piece from `a` to `b`, plain or dashed.
    pub fn add(&mut self, a: Pos2, b: Pos2, stroke: Stroke, dashed: bool) {
        // Wholly on one side of the picture: not seen.
        let c = self.clip;
        if (a.x < c.min.x && b.x < c.min.x) || (a.x > c.max.x && b.x > c.max.x) || (a.y < c.min.y && b.y < c.min.y) || (a.y > c.max.y && b.y > c.max.y) {
            self.flush();
            return;
        }
        let follows = stroke == self.stroke && dashed == self.dashed && self.end().is_some_and(|e| (e - a).length_sq() < 0.01);
        if !follows {
            self.flush();
            self.stroke = stroke;
            self.dashed = dashed;
            self.pts.push(a);
        }
        if self.pts.last().is_some_and(|l| (*l - b).length() < Self::MIN_STEP) {
            self.pending = Some(b);
            return;
        }
        self.pending = None;
        // A point on the straight line between the one before it and the new one (within a fifth of a pixel, going on the same
        // way) is not needed: a straight run of many moves is one piece.
        let n = self.pts.len();
        if n >= 2 {
            let (p0, p1) = (self.pts[n - 2], self.pts[n - 1]);
            let (d, e) = (b - p0, p1 - p0);
            let len = d.length();
            if len > 0.0 && e.dot(d) > 0.0 && e.length() <= len && (d.x * e.y - d.y * e.x).abs() / len < 0.2 {
                self.pts[n - 1] = b;
                return;
            }
        }
        self.pts.push(b);
    }

    /// Every piece of a path given as points.
    pub fn add_path(&mut self, pts: &[Pos2], stroke: Stroke, dashed: bool) {
        for w in pts.windows(2) {
            self.add(w[0], w[1], stroke, dashed);
        }
    }

    fn flush(&mut self) {
        if let Some(p) = self.pending.take() {
            self.pts.push(p);
        }
        if self.pts.len() >= 2 {
            let pts = std::mem::take(&mut self.pts);
            if self.dashed {
                self.shapes.extend(Shape::dashed_line(&pts, self.stroke, 5.0, 4.0));
            } else {
                self.shapes.push(Shape::line(pts, self.stroke));
            }
        }
        self.pts.clear();
    }

    /// The shapes, onto the painter.
    pub fn paint(self, painter: &egui::Painter) {
        painter.extend(self.shapes());
    }

    /// The shapes made.
    pub fn shapes(mut self) -> Vec<Shape> {
        self.flush();
        self.shapes
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::Color32;

    /// Moves that follow each other are one polyline; a straight run is one piece, the corners stay, points that are too close are
    /// left out and what is out of sight is not drawn.
    #[test]
    fn many_short_moves_are_few_lines() {
        let clip = egui::Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0));
        let st = Stroke::new(1.0, Color32::RED);
        let mut l = Lines::new(clip);
        // A straight row of a thousand moves a tenth of a pixel to 0.6 pixels long, then a corner and a short way down.
        let mut x = 10.0;
        for k in 0..1000 {
            let step = if k % 2 == 0 { 0.1 } else { 0.6 };
            l.add(Pos2::new(x, 100.0), Pos2::new(x + step, 100.0), st, false);
            x += step;
        }
        for k in 0..10 {
            l.add(Pos2::new(x, 100.0 + k as f32), Pos2::new(x, 101.0 + k as f32), st, false);
        }
        // Another colour, and a move wholly out of sight.
        l.add(Pos2::new(x, 110.0), Pos2::new(x + 5.0, 110.0), Stroke::new(1.0, Color32::BLUE), false);
        l.add(Pos2::new(-50.0, -50.0), Pos2::new(-10.0, -60.0), st, false);
        l.flush();
        assert_eq!(l.shapes.len(), 2, "one line for each stroke: {:?}", l.shapes);
        let Shape::Path(p) = &l.shapes[0] else { panic!("a polyline") };
        assert_eq!(p.points.len(), 3, "the row, the corner and the end: {:?}", p.points);
        assert_eq!(p.points[0], Pos2::new(10.0, 100.0));
        assert!((p.points[1].x - x).abs() < 1e-3 && (p.points[2].y - 110.0).abs() < 1e-3);
    }
}
