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
    plan_parts(painter, to_screen, parts, parts.len(), origin, units, st);
}

/// `plan`, where only the first `own` parts get dimensions of their own (the rest, such as circles, only count in the whole).
fn plan_parts(painter: &egui::Painter, to_screen: &dyn Fn(Pt) -> Pos2, parts: &[Rect], own: usize, origin: Option<Pt>, units: Units, st: &DimStyle) {
    let Some(all) = parts.iter().copied().reduce(|a, b| Rect { min: Pt::new(a.min.x.min(b.min.x), a.min.y.min(b.min.y)), max: Pt::new(a.max.x.max(b.max.x), a.max.y.max(b.max.y)) }) else { return };
    let screen = |r: &Rect| {
        let (p, q) = (to_screen(r.min), to_screen(r.max));
        egui::Rect::from_two_pos(p, q)
    };
    let s_all = screen(&all);
    let up = Vec2::new(0.0, -1.0);
    let right = Vec2::new(1.0, 0.0);
    // The parts first (near the shapes), then the whole further out.
    let few = parts.len() > 1 && own <= 12;
    let mut out_top = 16.0;
    let mut out_right = 16.0;
    if few {
        let same = |a: f64, b: f64| (a - b).abs() < 0.05;
        for (r, s) in parts.iter().take(own).map(|r| (r, screen(r))) {
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

/// What a drawing is dimensioned by: the boxes of its shapes and its circles (centre and radius), in millimetres.
#[derive(Clone, Debug, Default)]
pub struct Features {
    pub boxes: Vec<Rect>,
    pub circles: Vec<(Pt, f64)>,
}

impl Features {
    /// The features of shapes: a circle (an ellipse as wide as it is high, not slanted) by its centre and radius, every other shape
    /// by its box. `map` turns a point of the scene into the coordinates of the picture (Y up for a machine, for example).
    pub fn of<'a>(shapes: impl Iterator<Item = &'a base_core::scene::Shape>, map: &dyn Fn(Pt) -> Pt) -> Features {
        let mut f = Features::default();
        for s in shapes {
            if let base_core::scene::Kind::Ellipse { w, h } = s.kind {
                let (sx, sy) = (s.xf.a.hypot(s.xf.b), s.xf.c.hypot(s.xf.d));
                let square = (s.xf.a * s.xf.c + s.xf.b * s.xf.d).abs() < 1e-9;
                if square && (w * sx - h * sy).abs() < 1e-6 {
                    f.circles.push((map(s.xf.apply(Pt::new(w / 2.0, h / 2.0))), w * sx / 2.0));
                    continue;
                }
            }
            if s.is_image() {
                if let Some(b) = s.bounds() {
                    f.boxes.push(mapped(&b, map));
                }
                continue;
            }
            // Every outline of the shape by itself: the outside and the holes in it, a round one as a circle.
            for c in base_core::export::contours_of(s) {
                let p = c.flatten(0.02);
                if p.pts.len() < 2 {
                    continue;
                }
                let pts: Vec<Pt> = p.pts.iter().map(|q| map(*q)).collect();
                if let Some(circle) = p.closed.then(|| round(&pts)).flatten() {
                    f.circles.push(circle);
                } else if let Some(b) = base_core::geom::Polyline::new(pts, p.closed).bounds() {
                    f.boxes.push(b);
                }
            }
        }
        f
    }
}

fn mapped(b: &Rect, map: &dyn Fn(Pt) -> Pt) -> Rect {
    let (p, q) = (map(b.min), map(b.max));
    Rect { min: Pt::new(p.x.min(q.x), p.y.min(q.y)), max: Pt::new(p.x.max(q.x), p.y.max(q.y)) }
}

/// The circle of a closed outline that is round: centre and radius, when every point is within 0.5 % (and 0.02 mm) of the circle
/// through three of its points spread round it.
fn round(pts: &[Pt]) -> Option<(Pt, f64)> {
    let n = pts.len();
    if n < 8 {
        return None;
    }
    let (a, b, c) = (pts[0], pts[n / 3], pts[2 * n / 3]);
    let d = 2.0 * (a.x * (b.y - c.y) + b.x * (c.y - a.y) + c.x * (a.y - b.y));
    if d.abs() < 1e-9 {
        return None;
    }
    let (a2, b2, c2) = (a.x * a.x + a.y * a.y, b.x * b.x + b.y * b.y, c.x * c.x + c.y * c.y);
    let centre = Pt::new((a2 * (b.y - c.y) + b2 * (c.y - a.y) + c2 * (a.y - b.y)) / d, (a2 * (c.x - b.x) + b2 * (a.x - c.x) + c2 * (b.x - a.x)) / d);
    let r = centre.dist(a);
    let tol = (r * 0.005).max(0.02);
    pts.iter().all(|p| (p.dist(centre) - r).abs() <= tol).then_some((centre, r))
}

/// The dimensions of the features lying on a plane seen in 3D (the top of a block at the height `z`): every box by its edge in X
/// (in front) and its edge in Y (on the right), on the outer side, and the diameters of the circles. `project` turns a point
/// (millimetres, Z up) into the picture.
pub fn features3d(painter: &egui::Painter, project: &dyn Fn([f64; 3]) -> Pos2, f: &Features, z: f64, units: Units, st: &DimStyle) {
    for b in f.boxes.iter().take(16) {
        let c = project([(b.min.x + b.max.x) / 2.0, (b.min.y + b.max.y) / 2.0, z]);
        for (p, q, size) in [([b.min.x, b.min.y, z], [b.max.x, b.min.y, z], b.max.x - b.min.x), ([b.max.x, b.min.y, z], [b.max.x, b.max.y, z], b.max.y - b.min.y)] {
            let (pa, pb) = (project(p), project(q));
            if (pb - pa).length() < 24.0 {
                continue;
            }
            let along = (pb - pa).normalized();
            let mut side = Vec2::new(-along.y, along.x);
            if side.dot((pa + (pb - pa) / 2.0) - c) < 0.0 {
                side = -side;
            }
            dim(painter, pa, pb, side, 12.0, &number(units, size), st);
        }
    }
    let to = |p: Pt| project([p.x, p.y, z]);
    diameters(painter, &to, &f.circles, units, st);
}


/// The diameters of circles with a leader: from the circle at 45° out to the number "Ø 12"; circles of the same size are named once,
/// "4 × Ø 6", at the first of them.
pub fn diameters(painter: &egui::Painter, to_screen: &dyn Fn(Pt) -> Pos2, circles: &[(Pt, f64)], units: Units, st: &DimStyle) {
    let mut done = vec![false; circles.len()];
    let thin = Stroke::new(0.8, st.ink);
    for i in 0..circles.len() {
        if done[i] {
            continue;
        }
        let (c, r) = circles[i];
        let same: Vec<usize> = (i..circles.len()).filter(|j| !done[*j] && (circles[*j].1 - r).abs() < 0.025).collect();
        for j in &same {
            done[*j] = true;
        }
        let sc = to_screen(c);
        let edge = to_screen(Pt::new(c.x + r, c.y));
        let rs = (edge - sc).length();
        if rs < 2.0 {
            continue;
        }
        let dir = Vec2::new(1.0, -1.0).normalized();
        let (near, far) = (sc - dir * rs, sc + dir * rs);
        painter.line_segment([near, far], thin);
        arrow(painter, near, -dir, st.ink);
        arrow(painter, far, dir, st.ink);
        let elbow = far + dir * 18.0;
        let label = if same.len() > 1 { format!("{} × Ø {}", same.len(), number(units, 2.0 * r)) } else { format!("Ø {}", number(units, 2.0 * r)) };
        let galley = painter.layout_no_wrap(label, egui::FontId::proportional(st.size), st.ink);
        let shelf = elbow + Vec2::new(galley.size().x + 4.0, 0.0);
        painter.line_segment([far, elbow], thin);
        painter.line_segment([elbow, shelf], thin);
        painter.galley(elbow + Vec2::new(2.0, -galley.size().y - 1.0), galley, st.ink);
    }
}

/// The dimensions of a drawing: `plan` of the boxes and of the whole (the circles count in the whole), and the diameters of the circles.
pub fn drawing(painter: &egui::Painter, to_screen: &dyn Fn(Pt) -> Pos2, f: &Features, origin: Option<Pt>, units: Units, st: &DimStyle) {
    let circle_boxes = f.circles.iter().map(|(c, r)| Rect { min: Pt::new(c.x - r, c.y - r), max: Pt::new(c.x + r, c.y + r) });
    let all: Vec<Rect> = f.boxes.iter().copied().chain(circle_boxes).collect();
    plan_parts(painter, to_screen, &all, f.boxes.len(), origin, units, st);
    diameters(painter, to_screen, &f.circles, units, st);
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

    #[test]
    fn circles_are_named_by_their_diameter_and_the_same_ones_once() {
        use base_core::geom::Xf;
        use base_core::scene::{Kind, Scene};
        let mut sc = Scene::default();
        sc.add(0, Kind::Rect { w: 100.0, h: 50.0 }, Xf::IDENTITY);
        for x in [10.0, 80.0] {
            sc.add(0, Kind::Ellipse { w: 6.0, h: 6.0 }, Xf::translate(x, 10.0));
        }
        sc.add(0, Kind::Ellipse { w: 20.0, h: 10.0 }, Xf::translate(40.0, 20.0));
        let f = Features::of(sc.shapes.iter(), &|p| p);
        assert_eq!(f.circles.len(), 2, "the two round holes");
        assert_eq!(f.boxes.len(), 2, "the plate and the ellipse by their boxes");
        assert_eq!(f.circles[0], (Pt::new(13.0, 13.0), 3.0));
        let ctx = egui::Context::default();
        let mut texts = vec![];
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let to = |p: Pt| Pos2::new(p.x as f32 * 4.0, p.y as f32 * 4.0);
            drawing(ui.painter(), &to, &f, None, Units::Mm, &DimStyle::on(Color32::WHITE));
        });
        out.textures_delta.clear();
        for s in &out.shapes {
            if let egui::Shape::Text(t) = &s.shape {
                texts.push(t.galley.text().to_string());
            }
        }
        assert!(texts.iter().any(|t| t == "2 × Ø 6"), "{texts:?}");
        assert!(texts.iter().any(|t| t == "100") && texts.iter().any(|t| t == "50"), "{texts:?}");
    }
}
