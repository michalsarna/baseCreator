//! The navigation cube of the 3D views, in the manner of the one of FreeCAD: a cube with a label on each face that shows how the
//! model is turned. A click on a face, an edge or a corner looks at the model from there; the arrows round it turn the view by
//! 15 degrees; a drag on the cube turns it freely; the small cube under it brings the view home. The axes are drawn beside it.
//!
//! The cube knows nothing about how a view keeps its camera: it is given the three directions of the picture in the world (to the
//! right, up, and to the eye) and answers with [`Nav::LookFrom`], the direction in the world to look from. The world is the one of
//! the programs: X to the right, Y away from the person, Z up (from above is the plan, from the front is looking along +Y).
use crate::i18n::tr;
use crate::theme;
use egui::{Align2, Color32, FontId, Pos2, Rect, Sense, Shape, Stroke, Vec2};

/// What the person asked for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Nav {
    /// Look at the model from this direction (a unit vector from the model to the eye, in the world).
    LookFrom([f32; 3]),
    /// Back to the first view.
    Home,
}

/// The directions of the picture in the world: to the right, up and towards the eye.
pub type Basis = [[f32; 3]; 3];

/// The step of an arrow, in degrees.
pub const STEP_DEG: f32 = 15.0;

/// Half the size of the cube on the screen, in points.
const HALF: f32 = 32.0;
/// The distance of the arrows from the middle of the cube.
const RING: f32 = HALF * 1.85;
/// How much of the face the chamfer takes (the edges and the corners are the strips and the triangles it makes).
const CHAMFER: f32 = 0.24;

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit(a: [f32; 3]) -> [f32; 3] {
    let l = dot(a, a).sqrt().max(1e-9);
    [a[0] / l, a[1] / l, a[2] / l]
}

/// One of the 26 places of the cube that can be clicked: 6 faces, 12 edges and 8 corners.
#[derive(Clone, Debug)]
pub struct Place {
    /// The direction to look from when it is clicked.
    pub dir: [f32; 3],
    pub kind: Kind,
    corners: Vec<[f32; 3]>,
    /// The label of a face.
    pub label: Option<&'static str>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Face,
    Edge,
    Corner,
}

/// The places of the cube, in the world.
pub fn places() -> Vec<Place> {
    let inner = 1.0 - CHAMFER;
    let mut out = vec![];
    let axis = |a: usize, s: f32| {
        let mut v = [0.0; 3];
        v[a] = s;
        v
    };
    // faces
    for a in 0..3 {
        for s in [-1.0f32, 1.0] {
            let (b, c) = ((a + 1) % 3, (a + 2) % 3);
            let mut corners = vec![];
            for (sb, sc) in [(-1.0, -1.0), (1.0, -1.0), (1.0, 1.0), (-1.0, 1.0)] {
                let mut v = [0.0; 3];
                v[a] = s;
                v[b] = sb * inner;
                v[c] = sc * inner;
                corners.push(v);
            }
            let label = match (a, s > 0.0) {
                (0, true) => "Right",
                (0, false) => "Left",
                (1, true) => "Back",
                (1, false) => "Front",
                (_, true) => "Top",
                (_, false) => "Bottom",
            };
            out.push(Place { dir: axis(a, s), kind: Kind::Face, corners, label: Some(label) });
        }
    }
    // edges
    for a in 0..3 {
        let b = (a + 1) % 3;
        let t = (a + 2) % 3;
        for sa in [-1.0f32, 1.0] {
            for sb in [-1.0f32, 1.0] {
                let mk = |va: f32, vb: f32, vt: f32| {
                    let mut v = [0.0; 3];
                    v[a] = va;
                    v[b] = vb;
                    v[t] = vt;
                    v
                };
                let corners = vec![mk(sa, sb * inner, -inner), mk(sa, sb * inner, inner), mk(sa * inner, sb, inner), mk(sa * inner, sb, -inner)];
                let mut d = [0.0; 3];
                d[a] = sa;
                d[b] = sb;
                out.push(Place { dir: unit(d), kind: Kind::Edge, corners, label: None });
            }
        }
    }
    // corners
    for s0 in [-1.0f32, 1.0] {
        for s1 in [-1.0f32, 1.0] {
            for s2 in [-1.0f32, 1.0] {
                let corners = vec![[s0, s1 * inner, s2 * inner], [s0 * inner, s1, s2 * inner], [s0 * inner, s1 * inner, s2]];
                out.push(Place { dir: unit([s0, s1, s2]), kind: Kind::Corner, corners, label: None });
            }
        }
    }
    out
}

/// A point of the world on the picture: (to the right, up, towards the eye).
fn to_view(b: &Basis, p: [f32; 3]) -> [f32; 3] {
    [dot(b[0], p), dot(b[1], p), dot(b[2], p)]
}

/// The places that face the eye, with their corners on the picture (relative to the middle of the cube, in cube units, Y down) and
/// their depth, the far ones first.
pub fn visible(places: &[Place], b: &Basis) -> Vec<(usize, Vec<Vec2>, f32)> {
    let mut out: Vec<(usize, Vec<Vec2>, f32)> = vec![];
    for (i, p) in places.iter().enumerate() {
        let d = to_view(b, p.dir);
        if d[2] <= 0.08 {
            continue;
        }
        let pts = p.corners.iter().map(|c| to_view(b, *c)).map(|v| Vec2::new(v[0], -v[1])).collect();
        out.push((i, pts, d[2]));
    }
    out.sort_by(|a, b| a.2.total_cmp(&b.2));
    out
}

fn inside(poly: &[Vec2], p: Vec2) -> bool {
    let mut sign = 0.0f32;
    for i in 0..poly.len() {
        let (a, c) = (poly[i], poly[(i + 1) % poly.len()]);
        let cross = (c.x - a.x) * (p.y - a.y) - (c.y - a.y) * (p.x - a.x);
        if cross.abs() < 1e-9 {
            continue;
        }
        if sign == 0.0 {
            sign = cross.signum();
        } else if cross.signum() != sign {
            return false;
        }
    }
    true
}

/// The place under a point (relative to the middle of the cube, in cube units), the nearest one to the eye first.
pub fn hit(places: &[Place], b: &Basis, p: Vec2) -> Option<usize> {
    visible(places, b).into_iter().rev().find(|(_, poly, _)| inside(poly, p)).map(|(i, _, _)| i)
}

/// The view turned as the model would be turned about the axis of the picture: `right` > 0 turns the model to the right (the
/// face towards the eye goes to the right), `up` > 0 lifts the face towards the eye. Angles in degrees. Returns the new direction
/// to the eye.
pub fn turned(b: &Basis, right_deg: f32, up_deg: f32) -> [f32; 3] {
    let (sr, cr) = right_deg.to_radians().sin_cos();
    let (su, cu) = up_deg.to_radians().sin_cos();
    // about the vertical axis of the picture first: z' = z cos - right sin
    let z1: [f32; 3] = std::array::from_fn(|i| b[2][i] * cr - b[0][i] * sr);
    // then about the horizontal one: z'' = z' cos - up sin (the up direction is not changed by the first turn)
    unit(std::array::from_fn(|i| z1[i] * cu - b[1][i] * su))
}

/// The yaw and the pitch (radians) of a camera that looks from the direction `d` the way the G-code viewer and the model view of
/// the import window do: the pitch is 0 from above and half a turn from below, the yaw turns about Z. From straight above or below
/// the yaw is not defined, `keep_yaw` is returned then.
pub fn yaw_pitch(d: [f32; 3], keep_yaw: f32) -> (f32, f32) {
    let pitch = d[2].clamp(-1.0, 1.0).acos();
    let horizontal = (d[0] * d[0] + d[1] * d[1]).sqrt();
    let yaw = if horizontal < 1e-4 { keep_yaw } else { (-d[0]).atan2(-d[1]) };
    (yaw, pitch)
}

/// The basis of a camera of the convention of [`yaw_pitch`].
pub fn basis_of(yaw: f32, pitch: f32) -> Basis {
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    [[cy, -sy, 0.0], [sy * cp, cy * cp, sp], [-sy * sp, -cy * sp, cp]]
}

fn shade(c: u8, f: f32) -> u8 {
    (c as f32 * f).round().clamp(0.0, 255.0) as u8
}

/// Draws the cube in the top right corner of `area` and answers what the person did with it. Call it after the view has taken its
/// own input over `area`, so that the cube is on top.
pub fn show(ui: &mut egui::Ui, area: Rect, basis: &Basis) -> Option<Nav> {
    let centre = Pos2::new(area.right() - RING - 16.0, area.top() + RING + 16.0);
    if area.width() < 2.0 * RING + 40.0 || area.height() < 2.0 * RING + 40.0 {
        return None;
    }
    let places = places();
    let painter = ui.painter_at(area);
    let mut nav = None;

    // The cube.
    let cube_rect = Rect::from_center_size(centre, Vec2::splat(2.0 * HALF * 1.45));
    let resp = ui.interact(cube_rect, ui.scope_id().with(("navicube", area.min.x as i32, area.min.y as i32)), Sense::click_and_drag());
    let rel = |p: Pos2| (p - centre) / HALF;
    let over = resp.hover_pos().and_then(|p| hit(&places, basis, rel(p)));
    if resp.dragged() {
        let d = resp.drag_delta();
        nav = Some(Nav::LookFrom(turned(basis, d.x * 0.6, -d.y * 0.6)));
    } else if resp.clicked() {
        if let Some(i) = over {
            nav = Some(Nav::LookFrom(places[i].dir));
        }
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(if over.is_some() { egui::CursorIcon::PointingHand } else { egui::CursorIcon::Grab });
    }
    let accent = theme::accent();
    let light = unit([-0.35, 0.45, 0.82]);
    for (i, poly, depth) in visible(&places, basis) {
        let p = &places[i];
        let hovered = over == Some(i);
        let lit = (dot(to_view(basis, p.dir), light)).clamp(0.0, 1.0);
        let base = if p.kind == Kind::Face { 0.55 + 0.45 * lit } else { 0.42 + 0.35 * lit };
        let fill = if hovered { accent } else { Color32::from_rgb(shade(215, base), shade(216, base), shade(222, base)) };
        let pts: Vec<Pos2> = poly.iter().map(|v| centre + *v * HALF).collect();
        painter.add(Shape::convex_polygon(pts, fill, Stroke::new(1.0, Color32::from_rgb(70, 72, 80))));
        if let (Some(label), true) = (p.label, depth > 0.4) {
            let mid = pts_mid(&poly.iter().map(|v| centre + *v * HALF).collect::<Vec<_>>());
            let alpha = ((depth - 0.4) / 0.25).clamp(0.0, 1.0);
            let ink = if hovered { Color32::WHITE } else { Color32::from_rgba_unmultiplied(50, 52, 60, (255.0 * alpha) as u8) };
            painter.text(mid, Align2::CENTER_CENTER, tr(label), FontId::proportional(8.0 + 3.0 * depth), ink);
        }
    }

    // The arrows.
    for (dx, dy, right, up) in [(0.0, -1.0, 0.0, STEP_DEG), (0.0, 1.0, 0.0, -STEP_DEG), (-1.0, 0.0, -STEP_DEG, 0.0), (1.0, 0.0, STEP_DEG, 0.0)] {
        let at = centre + Vec2::new(dx, dy) * RING;
        let r = ui.interact(Rect::from_center_size(at, Vec2::splat(22.0)), ui.scope_id().with(("navicube_arrow", area.min.x as i32, area.min.y as i32, dx as i32, dy as i32)), Sense::click());
        let col = if r.hovered() { accent } else { Color32::from_rgb(150, 152, 160) };
        let (u, v) = (Vec2::new(dx, dy), Vec2::new(-dy, dx));
        painter.add(Shape::convex_polygon(vec![at + u * 7.0, at - u * 4.0 + v * 7.0, at - u * 4.0 - v * 7.0], col, Stroke::NONE));
        if r.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
        if r.clicked() {
            nav = Some(Nav::LookFrom(turned(basis, right, up)));
        }
    }

    // The axes, beside the cube at its lower left.
    let origin = centre + Vec2::new(-HALF * 1.25, HALF * 1.25);
    let mut axes: Vec<(f32, Color32, Vec2)> = vec![];
    for (v, col) in [([1.0, 0.0, 0.0], Color32::from_rgb(220, 60, 60)), ([0.0, 1.0, 0.0], Color32::from_rgb(70, 190, 70)), ([0.0, 0.0, 1.0], Color32::from_rgb(70, 120, 235))] {
        let w = to_view(basis, v);
        axes.push((w[2], col, Vec2::new(w[0], -w[1]) * 22.0));
    }
    axes.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (_, col, d) in axes {
        painter.line_segment([origin, origin + d], Stroke::new(1.8, col));
    }

    // The small cube that brings the view home.
    let home_at = centre + Vec2::new(RING * 0.95, RING * 0.95);
    let r = ui.interact(Rect::from_center_size(home_at, Vec2::splat(22.0)), ui.scope_id().with(("navicube_home", area.min.x as i32, area.min.y as i32)), Sense::click());
    let col = if r.hovered() { accent } else { Color32::from_rgb(150, 152, 160) };
    let s = 7.0;
    let hex: Vec<Pos2> = (0..6).map(|k| home_at + Vec2::angled(std::f32::consts::FRAC_PI_6 + k as f32 * std::f32::consts::FRAC_PI_3) * s).collect();
    painter.add(Shape::closed_line(hex, Stroke::new(1.4, col)));
    painter.line_segment([home_at, home_at + Vec2::angled(std::f32::consts::FRAC_PI_6) * s], Stroke::new(1.2, col));
    painter.line_segment([home_at, home_at + Vec2::angled(std::f32::consts::FRAC_PI_6 + 2.0 * std::f32::consts::FRAC_PI_3) * s], Stroke::new(1.2, col));
    painter.line_segment([home_at, home_at + Vec2::angled(std::f32::consts::FRAC_PI_6 + 4.0 * std::f32::consts::FRAC_PI_3) * s], Stroke::new(1.2, col));
    let r = r.on_hover_text(tr("Reset view"));
    if r.clicked() {
        nav = Some(Nav::Home);
    }
    nav
}

fn pts_mid(p: &[Pos2]) -> Pos2 {
    let s = p.iter().fold(Vec2::ZERO, |a, q| a + q.to_vec2());
    (s / p.len() as f32).to_pos2()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: [f32; 3], b: [f32; 3]) -> bool {
        (0..3).all(|i| (a[i] - b[i]).abs() < 1e-4)
    }

    #[test]
    fn there_are_26_places_6_faces_12_edges_8_corners() {
        let p = places();
        assert_eq!(p.len(), 26);
        assert_eq!(p.iter().filter(|x| x.kind == Kind::Face).count(), 6);
        assert_eq!(p.iter().filter(|x| x.kind == Kind::Edge).count(), 12);
        assert_eq!(p.iter().filter(|x| x.kind == Kind::Corner).count(), 8);
        assert!(p.iter().all(|x| (dot(x.dir, x.dir) - 1.0).abs() < 1e-4));
    }

    #[test]
    fn from_above_the_top_face_is_in_the_middle_and_a_click_there_looks_from_above() {
        let b = basis_of(0.0, 0.0);
        let p = places();
        let i = hit(&p, &b, Vec2::ZERO).unwrap();
        assert_eq!(p[i].label, Some("Top"));
        assert!(close(p[i].dir, [0.0, 0.0, 1.0]));
    }

    #[test]
    fn from_the_front_the_front_face_is_in_the_middle_with_the_top_above_it_and_the_right_one_at_the_right() {
        let b = basis_of(0.0, std::f32::consts::FRAC_PI_2);
        let p = places();
        assert_eq!(p[hit(&p, &b, Vec2::ZERO).unwrap()].label, Some("Front"));
        // Slightly tilted, the top and the right faces show up at their sides.
        let b = basis_of(-0.5, 1.2);
        let top = hit(&p, &b, Vec2::new(0.0, -0.7));
        assert_eq!(top.and_then(|i| p[i].label), Some("Top"));
    }

    #[test]
    fn a_camera_and_its_basis_agree_both_ways() {
        for (yaw, pitch) in [(0.3f32, 0.9f32), (-1.2, 2.0), (2.5, 0.4)] {
            let b = basis_of(yaw, pitch);
            let (y, p) = yaw_pitch(b[2], 0.0);
            assert!((y - yaw).abs() < 1e-4 && (p - pitch).abs() < 1e-4, "{yaw} {pitch} -> {y} {p}");
            // the three directions are at right angles
            assert!(dot(b[0], b[1]).abs() < 1e-5 && dot(b[0], b[2]).abs() < 1e-5 && dot(b[1], b[2]).abs() < 1e-5);
        }
    }

    #[test]
    fn straight_above_or_below_keeps_the_yaw() {
        assert_eq!(yaw_pitch([0.0, 0.0, 1.0], 1.25), (1.25, 0.0));
        assert!((yaw_pitch([0.0, 0.0, -1.0], 0.5).1 - std::f32::consts::PI).abs() < 1e-5);
    }

    #[test]
    fn the_arrows_turn_the_view_by_a_step_and_back() {
        let b = basis_of(0.4, 1.0);
        let r = turned(&b, STEP_DEG, 0.0);
        let angle = dot(r, b[2]).clamp(-1.0, 1.0).acos().to_degrees();
        assert!((angle - STEP_DEG).abs() < 1e-3);
        // The face towards the eye goes to the right: the eye moves to the left, so it now has a negative part along "right".
        assert!(dot(r, b[0]) < 0.0);
        let u = turned(&b, 0.0, STEP_DEG);
        assert!(dot(u, b[1]) < 0.0, "the face towards the eye goes up, the eye goes down");
        // From the front the arrow turns the yaw by a step and leaves the pitch.
        let front = basis_of(0.0, std::f32::consts::FRAC_PI_2);
        let (y, pt) = yaw_pitch(turned(&front, STEP_DEG, 0.0), 0.0);
        assert!((y.to_degrees() - STEP_DEG).abs() < 1e-3 && (pt - std::f32::consts::FRAC_PI_2).abs() < 1e-4);
    }

    #[test]
    fn the_front_view_looks_along_plus_y_and_the_right_one_from_plus_x() {
        let p = places();
        let front = p.iter().find(|x| x.label == Some("Front")).unwrap();
        assert!(close(front.dir, [0.0, -1.0, 0.0]));
        let right = p.iter().find(|x| x.label == Some("Right")).unwrap();
        assert!(close(right.dir, [1.0, 0.0, 0.0]));
        // and the camera of those views is what the import window calls Front (yaw 0, pitch 90) and Right (yaw -90, pitch 90)
        let (y, pt) = yaw_pitch(front.dir, 0.0);
        assert!(y.abs() < 1e-5 && (pt - std::f32::consts::FRAC_PI_2).abs() < 1e-5);
        let (y, _) = yaw_pitch(right.dir, 0.0);
        assert!((y + std::f32::consts::FRAC_PI_2).abs() < 1e-5);
    }
}
