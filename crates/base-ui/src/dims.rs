//! Dimensions in the style of a technical drawing, painted over a preview or a drawing: extension lines, a dimension line with an
//! arrow at each end, and the number along the line. `plan` dimensions shapes seen from above (the size of the whole and of each part,
//! and where they are from the origin), `box3d` the edges of a block seen in 3D.
use base_core::geom::{Pt, Rect};
use base_core::units::Units;
use egui::{Color32, Pos2, Stroke, Vec2};

/// How the dimensions look.
#[derive(Clone, Copy, Debug)]
pub struct DimStyle {
    pub ink: Color32,
    /// The height of the numbers, in points.
    pub size: f32,
}

impl DimStyle {
    /// Dark lines on a light background, light ones on a dark background.
    pub fn on(background: Color32) -> DimStyle {
        let light = background.r() as u32 + background.g() as u32 + background.b() as u32 > 3 * 128;
        DimStyle { ink: if light { Color32::from_rgb(0x20, 0x20, 0x28) } else { Color32::from_rgb(0xe8, 0xe8, 0xf0) }, size: 12.0 }
    }
}

/// The number of a dimension: millimetres with at most one decimal (as a drawing writes them), inches with three.
pub fn number(units: Units, mm: f64) -> String {
    match units {
        Units::Mm => {
            let s = format!("{:.1}", mm);
            s.trim_end_matches('0').trim_end_matches('.').to_string()
        }
        Units::Inch => format!("{:.3}\"", units.from_mm(mm)),
    }
}

/// A filled arrowhead at `tip`, pointing along `dir` (a unit vector).
fn arrow(painter: &egui::Painter, tip: Pos2, dir: Vec2, ink: Color32) {
    let back = tip - dir * 8.0;
    let side = Vec2::new(-dir.y, dir.x) * 2.6;
    painter.add(egui::Shape::convex_polygon(vec![tip, back + side, back - side], ink, Stroke::NONE));
}

/// The text `label` centred on `at`, turned along `dir` and kept upright.
fn along(painter: &egui::Painter, at: Pos2, dir: Vec2, label: &str, st: &DimStyle) {
    let mut angle = dir.y.atan2(dir.x);
    if angle > std::f32::consts::FRAC_PI_2 + 0.01 {
        angle -= std::f32::consts::PI;
    } else if angle < -std::f32::consts::FRAC_PI_2 + 0.01 {
        angle += std::f32::consts::PI;
    }
    let galley = painter.layout_no_wrap(label.to_string(), egui::FontId::proportional(st.size), st.ink);
    let half = galley.size() / 2.0;
    let (s, c) = angle.sin_cos();
    // The galley turns about its top left corner: start where that corner lands when the middle is at `at`.
    let corner = at - Vec2::new(half.x * c - half.y * s, half.x * s + half.y * c);
    painter.add(egui::epaint::TextShape::new(corner, galley, st.ink).with_angle(angle));
}

/// One dimension between the screen points `a` and `b`: its line lies `off` points away from them towards `side` (a unit vector
/// pointing away from the object), with extension lines from the points to it. Nothing is drawn for points almost on each other.
pub fn dim(painter: &egui::Painter, a: Pos2, b: Pos2, side: Vec2, off: f32, label: &str, st: &DimStyle) {
    let len = (b - a).length();
    if len < 4.0 {
        return;
    }
    let dir = (b - a) / len;
    let thin = Stroke::new(0.8, st.ink);
    let (da, db) = (a + side * off, b + side * off);
    for (p, q) in [(a, da), (b, db)] {
        painter.line_segment([p + side * 3.0, q + side * 4.0], thin);
    }
    painter.line_segment([da, db], thin);
    // Arrows inside when there is room, outside with a short tail when the dimension is short.
    if len > 22.0 {
        arrow(painter, da, -dir, st.ink);
        arrow(painter, db, dir, st.ink);
    } else {
        arrow(painter, da, dir, st.ink);
        arrow(painter, db, -dir, st.ink);
        painter.line_segment([da - dir * 12.0, db + dir * 12.0], thin);
    }
    along(painter, da + (db - da) / 2.0 + side * (st.size * 0.75), dir, label, st);
}

/// Dimensions of shapes seen from above, in the style of a drawing: the width of the whole over it and its height on its right, the
/// width and height of every part (when there are a few and they are big enough on the screen), and how far the whole is from the
/// origin `origin` (the zero of the work area) along its left and its bottom. `to_screen` turns millimetres into points of the
/// picture; Y may point either way.
pub fn plan(painter: &egui::Painter, to_screen: &dyn Fn(Pt) -> Pos2, parts: &[Rect], origin: Option<Pt>, units: Units, st: &DimStyle) {
    let Some(all) = parts.iter().copied().reduce(|a, b| Rect { min: Pt::new(a.min.x.min(b.min.x), a.min.y.min(b.min.y)), max: Pt::new(a.max.x.max(b.max.x), a.max.y.max(b.max.y)) }) else { return };
    let screen = |r: &Rect| {
        let (p, q) = (to_screen(r.min), to_screen(r.max));
        egui::Rect::from_two_pos(p, q)
    };
    let s_all = screen(&all);
    let up = Vec2::new(0.0, -1.0);
    let right = Vec2::new(1.0, 0.0);
    // The parts first (near the shapes), then the whole further out.
    let few = parts.len() > 1 && parts.len() <= 12;
    let mut out_top = 16.0;
    let mut out_right = 16.0;
    if few {
        let same = |a: f64, b: f64| (a - b).abs() < 0.05;
        for (r, s) in parts.iter().map(|r| (r, screen(r))) {
            // A part as wide (or as high) as the whole already has its dimension there.
            let whole_w = same(r.min.x, all.min.x) && same(r.max.x, all.max.x);
            let whole_h = same(r.min.y, all.min.y) && same(r.max.y, all.max.y);
            if s.width() > 28.0 && !whole_w {
                dim(painter, s.left_top(), s.right_top(), up, 10.0, &number(units, r.max.x - r.min.x), st);
            }
            if s.height() > 28.0 && !whole_h {
                dim(painter, s.right_top(), s.right_bottom(), right, 10.0, &number(units, r.max.y - r.min.y), st);
            }
        }
        out_top = 34.0;
        out_right = 34.0;
    }
    let top_gap = s_all.top() - parts.iter().map(|r| screen(r).top()).fold(f32::MAX, f32::min);
    dim(painter, s_all.left_top(), s_all.right_top(), up, out_top + top_gap, &number(units, all.max.x - all.min.x), st);
    dim(painter, s_all.right_top(), s_all.right_bottom(), right, out_right, &number(units, all.max.y - all.min.y), st);
    // Where the whole lies from the zero of the work area: along the bottom from the zero to the left edge, and along the left
    // from the zero to the nearer edge in Y.
    // Only when the zero is in the picture: from a zero far away the lines would run out of it.
    if let Some(o) = origin.filter(|o| painter.clip_rect().contains(to_screen(*o))) {
        let so = to_screen(o);
        if (all.min.x - o.x).abs() > 0.05 {
            let y = s_all.bottom().max(so.y);
            dim(painter, Pos2::new(so.x, y), Pos2::new(s_all.left(), y), Vec2::new(0.0, 1.0), 18.0, &number(units, (all.min.x - o.x).abs()), st);
        }
        let near_y = if (all.min.y - o.y).abs() < (all.max.y - o.y).abs() { all.min.y } else { all.max.y };
        if (near_y - o.y).abs() > 0.05 {
            let x = s_all.left().min(so.x);
            dim(painter, Pos2::new(x, so.y), Pos2::new(x, to_screen(Pt::new(o.x, near_y)).y), Vec2::new(-1.0, 0.0), 18.0, &number(units, (near_y - o.y).abs()), st);
        }
    }
}

/// Dimensions of a block seen in 3D: one edge along X, one along Y and one along Z, those that face the viewer most, each with its
/// dimension on the outer side. `project` turns a point (millimetres, Z up) into the picture.
pub fn box3d(painter: &egui::Painter, project: &dyn Fn([f64; 3]) -> Pos2, min: [f64; 3], max: [f64; 3], units: Units, st: &DimStyle) {
    let labels = [0, 1, 2].map(|a| number(units, max[a] - min[a]));
    box3d_labelled(painter, project, min, max, &labels, st);
}

/// The same with the numbers given (a picture that is not to scale, such as a block drawn thicker than it is to be seen).
pub fn box3d_labelled(painter: &egui::Painter, project: &dyn Fn([f64; 3]) -> Pos2, min: [f64; 3], max: [f64; 3], labels: &[String; 3], st: &DimStyle) {
    let corner = |i: usize| [if i & 1 == 0 { min[0] } else { max[0] }, if i & 2 == 0 { min[1] } else { max[1] }, if i & 4 == 0 { min[2] } else { max[2] }];
    let pts: Vec<Pos2> = (0..8).map(|i| project(corner(i))).collect();
    let centre = pts.iter().fold(Vec2::ZERO, |s, p| s + p.to_vec2()) / 8.0;
    for axis in 0..3 {
        let bit = 1 << axis;
        // The four edges along this axis; take the one whose middle is farthest from the middle of the picture of the block, below
        // it for X and Y (the front of the block) and to a side for Z.
        let edges = (0..8).filter(|i| i & bit == 0).map(|i| (i, i | bit));
        let score = |(a, b): (usize, usize)| {
            let m = (pts[a].to_vec2() + pts[b].to_vec2()) / 2.0 - centre;
            if axis == 2 { -m.x } else { m.y + m.x.abs() * 0.25 }
        };
        let Some((a, b)) = edges.max_by(|x, y| score(*x).total_cmp(&score(*y))) else { continue };
        let (pa, pb) = (pts[a], pts[b]);
        let mid = (pa.to_vec2() + pb.to_vec2()) / 2.0 - centre;
        let along = (pb - pa).normalized();
        let mut side = Vec2::new(-along.y, along.x);
        if side.dot(mid) < 0.0 {
            side = -side;
        }
        dim(painter, pa, pb, side, 22.0, &labels[axis], st);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numbers_and_the_dimensions_of_a_plan_and_a_block_are_drawn() {
        assert_eq!(number(Units::Mm, 120.0), "120");
        assert_eq!(number(Units::Mm, 12.25), "12.2");
        assert_eq!(number(Units::Inch, 25.4), "1.000\"");
        let ctx = egui::Context::default();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let painter = ui.painter().clone();
            let st = DimStyle::on(Color32::WHITE);
            let to = |p: Pt| Pos2::new(50.0 + p.x as f32 * 2.0, 400.0 - p.y as f32 * 2.0);
            let parts = [Rect { min: Pt::new(10.0, 10.0), max: Pt::new(60.0, 40.0) }, Rect { min: Pt::new(80.0, 10.0), max: Pt::new(120.0, 90.0) }];
            plan(&painter, &to, &parts, Some(Pt::new(0.0, 0.0)), Units::Mm, &st);
            let iso = |p: [f64; 3]| Pos2::new(300.0 + (p[0] - p[1]) as f32 * 1.5, 300.0 + ((p[0] + p[1]) * 0.5 - p[2] * 1.2) as f32 * 1.5);
            box3d(&painter, &iso, [0.0, 0.0, -10.0], [100.0, 60.0, 0.0], Units::Mm, &st);
        });
        out.textures_delta.clear();
        let shapes = out.shapes.len();
        // Every dimension is two extension lines, a line, two arrows and its number.
        assert!(shapes >= 6 * 9, "the dimensions were drawn: {shapes} shapes");
        assert_eq!(DimStyle::on(Color32::BLACK).ink, Color32::from_rgb(0xe8, 0xe8, 0xf0));
    }
}
