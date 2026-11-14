//! The 3D preview the programs share: a camera that turns round the object (and goes back home), the projection of the object onto
//! the screen, the moves between the faces of a post or a block, the plain block lit from the upper left, and the picture of an
//! object carved on its faces, made by casting a ray through it for every pixel. How fine that picture is follows the quality of
//! the rendering of the program options (1 to 5).
//!
//! The world is the one of the programs (and of the navigation cube): X to the right, Y away from the person, Z up. The object of a
//! post or a block is drawn in the space of the shown face: X along it, Y up its drawing, Z up from its top (0) down to its depth.
use crate::i18n::tr;
use base_core::faces::Solid;
use base_core::geom::Pt;
use egui::{Color32, Pos2, Rect, Shape, Stroke, Vec2};
use std::cell::RefCell;

/// A point of the world.
pub type P3 = [f64; 3];

pub const START_YAW: f32 = 0.55;
pub const START_PITCH: f32 = 0.62;
pub const MIN_PITCH: f32 = 0.03;
pub const MAX_PITCH: f32 = 1.5;
const MIN_ZOOM: f32 = 0.2;
const MAX_ZOOM: f32 = 40.0;

/// How the object is looked at: turned round the vertical (`yaw`), from how high (`pitch`), how close (`zoom`, 1 = it fills the
/// picture) and moved across the picture (`pan`, points).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Camera {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub pan: Vec2,
    /// Looked at from straight above: the plan. It cannot be turned.
    pub top: bool,
    /// The camera may go under the object (its bottom is worked on too).
    pub under: bool,
}

impl Default for Camera {
    fn default() -> Camera {
        Camera { yaw: START_YAW, pitch: START_PITCH, zoom: 1.0, pan: Vec2::ZERO, top: false, under: false }
    }
}

impl Camera {
    /// Back to the first position (from above and from under stay as they are).
    pub fn reset(&mut self) {
        *self = Camera { top: self.top, under: self.under, ..Camera::default() };
    }

    /// The lowest the camera goes: just above the table, or under the object when it may.
    fn min_pitch(&self) -> f32 {
        if self.under { -MAX_PITCH } else { MIN_PITCH }
    }

    /// Turn by a drag of the mouse of `dx`, `dy` points.
    pub fn rotate(&mut self, dx: f32, dy: f32) {
        if self.top {
            return;
        }
        self.yaw = (self.yaw - dx * 0.01).rem_euclid(std::f32::consts::TAU);
        self.pitch = (self.pitch + dy * 0.01).clamp(self.min_pitch(), MAX_PITCH);
    }

    /// Look at the object from a direction of the world (from the object to the eye), as the navigation cube asks. The camera does
    /// not go below the horizon (unless it may go under) or straight above, so those are as near as it goes.
    pub fn look_from(&mut self, d: [f32; 3]) {
        if self.top {
            return;
        }
        self.pitch = d[2].clamp(-1.0, 1.0).asin().clamp(self.min_pitch(), MAX_PITCH);
        if d[0].hypot(d[1]) > 1e-4 {
            self.yaw = d[0].atan2(-d[1]).rem_euclid(std::f32::consts::TAU);
        }
    }

    pub fn zoom_by(&mut self, factor: f32) {
        self.zoom = (self.zoom * factor).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Zoom about the middle of the picture: what is there stays there.
    pub fn zoom_about_middle(&mut self, factor: f32) {
        let before = self.zoom;
        self.zoom_by(factor);
        self.pan *= self.zoom / before;
    }

    /// The direction from the object to the eye, and the right and the up of the picture, in the world.
    pub fn axes(&self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        let (yaw, pitch) = if self.top { (0.0, std::f32::consts::FRAC_PI_2) } else { (self.yaw, self.pitch) };
        let (sy, cy, sp, cp) = (yaw.sin(), yaw.cos(), pitch.sin(), pitch.cos());
        ([sy * cp, -cy * cp, sp], [cy, sy, 0.0], [-sy * sp, cy * sp, cp])
    }
}

/// Turns points of the world into points of the picture (a parallel view).
#[derive(Clone, Copy, Debug)]
pub struct Projector {
    pub right: [f32; 3],
    pub up: [f32; 3],
    pub eye: [f32; 3],
    pub target: [f32; 3],
    /// Points of the picture for a millimetre.
    pub scale: f32,
    pub centre: Pos2,
}

impl Projector {
    /// `target` in the middle of `area`; a sphere of `radius` round it fills the picture at zoom 1.
    pub fn new(cam: &Camera, target: P3, radius: f64, area: Rect) -> Projector {
        let (eye, right, up) = cam.axes();
        let fit = area.width().min(area.height()) / (2.0 * radius.max(1.0) as f32) * 0.92;
        Projector { right, up, eye, target: [target[0] as f32, target[1] as f32, target[2] as f32], scale: fit * cam.zoom, centre: area.center() + cam.pan }
    }

    pub fn project(&self, p: P3) -> Pos2 {
        let d = [p[0] as f32 - self.target[0], p[1] as f32 - self.target[1], p[2] as f32 - self.target[2]];
        let dot = |a: &[f32; 3]| a[0] * d[0] + a[1] * d[1] + a[2] * d[2];
        Pos2::new(self.centre.x + dot(&self.right) * self.scale, self.centre.y - dot(&self.up) * self.scale)
    }

    /// How much a surface with the outward direction `n` (of length 1) is turned to the eye: above 0 it is seen.
    pub fn facing(&self, n: P3) -> f64 {
        n[0] * self.eye[0] as f64 + n[1] * self.eye[1] as f64 + n[2] * self.eye[2] as f64
    }
}

/// The colour `c` made lighter or darker by `f`.
pub fn shade(c: [u8; 3], f: f32) -> Color32 {
    let m = |v: u8| ((v as f32) * f).round().clamp(0.0, 255.0) as u8;
    Color32::from_rgb(m(c[0]), m(c[1]), m(c[2]))
}

/// Where the light comes from: the upper left in front (of length 1).
pub const LIGHT: P3 = [-0.3462, -0.4451, 0.8259];

/// How light a surface with the outward direction `n` is.
pub fn light_of(n: P3) -> f32 {
    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
    (0.62 + 0.38 * ((n[0] * LIGHT[0] + n[1] * LIGHT[1] + n[2] * LIGHT[2]) / len).max(0.0)) as f32
}

/// A move of space: a turn and a shift, `p' = m·p + t`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Affine3 {
    pub m: [[f64; 3]; 3],
    pub t: [f64; 3],
}

impl Affine3 {
    pub const IDENTITY: Affine3 = Affine3 { m: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]], t: [0.0; 3] };

    /// The move that `f` makes (it must be one: a turn and a shift).
    pub fn of(f: impl Fn(P3) -> P3) -> Affine3 {
        let t = f([0.0; 3]);
        let col = |k: usize| {
            let mut e = [0.0; 3];
            e[k] = 1.0;
            let p = f(e);
            [p[0] - t[0], p[1] - t[1], p[2] - t[2]]
        };
        let (c0, c1, c2) = (col(0), col(1), col(2));
        Affine3 { m: [[c0[0], c1[0], c2[0]], [c0[1], c1[1], c2[1]], [c0[2], c1[2], c2[2]]], t }
    }

    pub fn apply(&self, p: P3) -> P3 {
        let d = self.dir(p);
        [d[0] + self.t[0], d[1] + self.t[1], d[2] + self.t[2]]
    }

    /// A point moved back (the move undone).
    pub fn back(&self, p: P3) -> P3 {
        self.dir_back([p[0] - self.t[0], p[1] - self.t[1], p[2] - self.t[2]])
    }

    /// A direction moved (the turn alone).
    pub fn dir(&self, p: P3) -> P3 {
        let m = &self.m;
        [m[0][0] * p[0] + m[0][1] * p[1] + m[0][2] * p[2], m[1][0] * p[0] + m[1][1] * p[1] + m[1][2] * p[2], m[2][0] * p[0] + m[2][1] * p[1] + m[2][2] * p[2]]
    }

    /// A direction moved back (the turn undone: its transpose).
    pub fn dir_back(&self, p: P3) -> P3 {
        let m = &self.m;
        [m[0][0] * p[0] + m[1][0] * p[1] + m[2][0] * p[2], m[0][1] * p[0] + m[1][1] * p[1] + m[2][1] * p[2], m[0][2] * p[0] + m[1][2] * p[1] + m[2][2] * p[2]]
    }
}

/// From the space of face `from` of `solid` (X along it, Y up its drawing, Z up from its top) into the space of face `to`.
pub fn face_move(solid: &Solid, from: usize, to: usize) -> Affine3 {
    let (f, t) = (solid.face(from), solid.face(to));
    Affine3::of(|p| {
        let (d, depth) = t.local(f.at(Pt::new(p[0], f.h - p[1]), -p[2]));
        [d.x, t.h - d.y, -depth]
    })
}

/// A point of the drawing of face `face` (millimetres from its top left corner, y down) in its own space.
pub fn on_face(solid: &Solid, face: usize, p: Pt) -> P3 {
    [p.x, solid.face(face).h - p.y, 0.0]
}

/// The space of the shown face of an object, where it is drawn: the moves of all its faces into it, and the box of the object there.
#[derive(Clone, Debug, PartialEq)]
pub struct FaceFrame {
    pub moves: Vec<Affine3>,
    pub lo: P3,
    pub hi: P3,
}

impl FaceFrame {
    /// `solid` turned so that face `shown` is up, as it lies on the machine for that face.
    pub fn of(solid: &Solid, shown: usize) -> FaceFrame {
        let f = solid.face(shown);
        FaceFrame { moves: (0..solid.count()).map(|j| face_move(solid, j, shown)).collect(), lo: [0.0, 0.0, -f.depth], hi: [f.w, f.h, 0.0] }
    }

    /// The middle of the object and the radius of the sphere round it.
    pub fn framing(&self) -> (P3, f64) {
        let mid = [(self.lo[0] + self.hi[0]) / 2.0, (self.lo[1] + self.hi[1]) / 2.0, (self.lo[2] + self.hi[2]) / 2.0];
        let d = [self.hi[0] - self.lo[0], self.hi[1] - self.lo[1], self.hi[2] - self.lo[2]];
        (mid, (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt() / 2.0)
    }

    /// The outward direction of face `j` here.
    pub fn out(&self, j: usize) -> P3 {
        self.moves[j].dir([0.0, 0.0, 1.0])
    }

    /// A point of the drawing of face `j` here.
    pub fn at(&self, solid: &Solid, j: usize, p: Pt) -> P3 {
        self.moves[j].apply(on_face(solid, j, p))
    }
}

/// The six sides of a box: the corners and the outward direction.
pub fn box_sides(lo: P3, hi: P3) -> [([P3; 4], P3); 6] {
    let (x0, y0, z0, x1, y1, z1) = (lo[0], lo[1], lo[2], hi[0], hi[1], hi[2]);
    [
        ([[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]], [0.0, 0.0, 1.0]),
        ([[x0, y0, z0], [x1, y0, z0], [x1, y1, z0], [x0, y1, z0]], [0.0, 0.0, -1.0]),
        ([[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]], [0.0, -1.0, 0.0]),
        ([[x0, y1, z0], [x1, y1, z0], [x1, y1, z1], [x0, y1, z1]], [0.0, 1.0, 0.0]),
        ([[x0, y0, z0], [x0, y1, z0], [x0, y1, z1], [x0, y0, z1]], [-1.0, 0.0, 0.0]),
        ([[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]], [1.0, 0.0, 0.0]),
    ]
}

/// The box `lo`..`hi` in the colour `c`, its sides that are turned to the eye, each lit by how it turns to the light.
pub fn paint_box(painter: &egui::Painter, pj: &Projector, lo: P3, hi: P3, c: [u8; 3]) {
    for (corners, n) in box_sides(lo, hi) {
        if pj.facing(n) <= 1e-6 {
            continue;
        }
        let lit = light_of(n);
        let pts: Vec<Pos2> = corners.iter().map(|q| pj.project(*q)).collect();
        painter.add(Shape::convex_polygon(pts, shade(c, lit), Stroke::new(1.0, shade(c, lit * 0.62))));
    }
}

/// The letters of the faces turned to the eye, faint, in their middle: (the move of the face, its width and height, its letter).
pub fn paint_letters(painter: &egui::Painter, pj: &Projector, faces: &[(Affine3, [f64; 2], char)], c: [u8; 3]) {
    for (mv, size, letter) in faces {
        if pj.facing(mv.dir([0.0, 0.0, 1.0])) <= 0.2 {
            continue;
        }
        let mid = pj.project(mv.apply([size[0] / 2.0, size[1] / 2.0, 0.0]));
        let px = ((size[0].min(size[1]) * pj.scale as f64) as f32 * 0.3).clamp(12.0, 40.0);
        painter.text(mid, egui::Align2::CENTER_CENTER, *letter, egui::FontId::proportional(px), shade(c, 0.45).gamma_multiply(0.6));
    }
}

/// The quality of the rendering of the previews (1 to 5, 3 as it always was).
pub const QUALITY_DEFAULT: u8 = 3;

/// How many pixels a picture of the preview may have at most, for quality `q`.
pub fn quality_pixels(q: u8) -> f32 {
    [400_000.0, 800_000.0, 1_500_000.0, 2_500_000.0, 4_000_000.0][(q.clamp(1, 5) - 1) as usize]
}

/// The pixels of a picture of the preview for a pixel of the screen, for quality `q` (above 1 they are averaged to one).
pub fn quality_sharpness(q: u8) -> f32 {
    [0.5, 0.75, 1.0, 1.5, 2.0][(q.clamp(1, 5) - 1) as usize]
}

/// How fine a grid of the preview (a carved surface, the round of a cylinder) is, for quality `q`: 1 as it was at quality 3.
pub fn quality_detail(q: u8) -> f64 {
    [0.5, 0.75, 1.0, 1.5, 2.2][(q.clamp(1, 5) - 1) as usize]
}

/// The row of the program options: the quality of the rendering of the previews, a slider 1 to 5.
pub fn quality_row(ui: &mut egui::Ui, q: &mut u8) -> bool {
    ui.label(tr("Rendering quality"));
    let r = ui
        .add(egui::Slider::new(q, 1..=5).step_by(1.0))
        .on_hover_text(tr("How fine the pictures of the preview are: 1 is quick and coarse, 5 sharp and fine but slower. While the view is turned it is drawn coarser for a moment."));
    ui.end_row();
    r.changed()
}

/// A surface of the object carved by the job of one face: how deep it is cut at a point of the face's space (X, Y; 0 off it).
/// `Sync`: the picture is made by several threads at once.
pub trait Carved: Sync {
    fn depth_at(&self, x: f64, y: f64) -> f64;
    /// The distance between the points of its grid.
    fn step(&self) -> f64;
    /// The deepest cut.
    fn deepest(&self) -> f64;
}

/// A face of an object in the picture.
pub struct SolidFace<'a> {
    /// The face carved by its job; `None` while nothing is cut on it.
    pub carved: Option<&'a dyn Carved>,
    /// From its space to the one of the picture (the space of the shown face).
    pub to_view: Affine3,
    /// Its width and height, and how deep the object is under it.
    pub size: [f64; 2],
    pub depth: f64,
    pub letter: char,
}

/// An object carved on its faces, for [`draw_solid`].
pub struct SolidView<'a> {
    pub faces: Vec<SolidFace<'a>>,
    /// The face that is up (the space of the picture).
    pub shown: usize,
    pub colour: [u8; 3],
    /// What the faces were made of: the picture is made again when it changes (or when the view, the size or the quality does).
    pub key: u64,
    /// The picture that was made last, and its key.
    pub cache: &'a RefCell<Option<(u64, egui::TextureHandle)>>,
}

/// The object carved on its faces, drawn into a picture that is kept while nothing changes: a point of it is material only when no
/// face has cut it away, so the cuts of a face take the material of the faces beside it (see [`solid_picture`]). `quality` is the
/// one of the program options; while `moving` (the view is being turned) the picture is made coarser, so that it follows the mouse.
pub fn draw_solid(painter: &egui::Painter, pj: &Projector, solid: &SolidView, quality: u8, moving: bool) {
    use std::hash::{Hash, Hasher};
    let rect = painter.clip_rect();
    if rect.width() < 2.0 || rect.height() < 2.0 {
        return;
    }
    let q = if moving { quality.min(2) } else { quality };
    let ppp = painter.ctx().pixels_per_point();
    let s = (ppp * quality_sharpness(q)).min((quality_pixels(q) / (rect.width() * rect.height())).sqrt()).max(0.2);
    let (iw, ih) = ((rect.width() * s).round() as usize, (rect.height() * s).round() as usize);
    let mut h = std::collections::hash_map::DefaultHasher::new();
    solid.key.hash(&mut h);
    for v in pj.right.iter().chain(pj.up.iter()).chain(pj.target.iter()).chain([pj.scale, pj.centre.x, pj.centre.y, rect.min.x, rect.min.y].iter()) {
        v.to_bits().hash(&mut h);
    }
    (iw, ih, solid.colour, q, solid.shown).hash(&mut h);
    let key = h.finish();
    let mut cache = solid.cache.borrow_mut();
    if cache.as_ref().map(|(k, _)| *k) != Some(key) {
        let img = solid_picture(pj, solid, rect, s, iw, ih, quality_detail(q));
        match cache.as_mut() {
            Some((k, tex)) => {
                tex.set(img, egui::TextureOptions::LINEAR);
                *k = key;
            }
            None => *cache = Some((key, painter.ctx().load_texture("solid_view", img, egui::TextureOptions::LINEAR))),
        }
    }
    if let Some((_, tex)) = cache.as_ref() {
        painter.image(tex.id(), rect, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
    }
    let letters: Vec<(Affine3, [f64; 2], char)> = solid.faces.iter().map(|f| (f.to_view, f.size, f.letter)).collect();
    paint_letters(painter, pj, &letters, solid.colour);
}

/// The picture of the object (see [`draw_solid`]): `iw` × `ih` pixels over `rect`, `s` pixels a point. Every pixel is a ray into the
/// object (the view is parallel): the first point of it that no face has cut away is what is seen there. A ray through a cut that
/// goes through the object sees what is behind it, or nothing (the pixel stays clear). `detail` makes the steps along the ray finer.
pub fn solid_picture(pj: &Projector, solid: &SolidView, rect: Rect, s: f32, iw: usize, ih: usize, detail: f64) -> egui::ColorImage {
    let deepest: Vec<f64> = solid.faces.iter().map(|f| f.carved.map_or(0.0, |c| c.deepest())).collect();
    let step = solid.faces.iter().filter_map(|f| f.carved).map(|c| c.step()).fold(f64::INFINITY, f64::min);
    // Half a point of the grid between the steps along a ray (the surface between two of them is found by halving); a finer grid
    // does not need finer steps, only a coarse quality makes them longer.
    let step = if step.is_finite() { step * 0.5 / detail.clamp(0.1, 1.0) } else { 1.0 };
    // The object in the space of the picture: the block of the shown face.
    let shown = &solid.faces[solid.shown];
    let (lo, hi) = ([0.0, 0.0, -shown.depth], [shown.size[0], shown.size[1], 0.0]);
    let f3 = |v: [f32; 3]| [v[0] as f64, v[1] as f64, v[2] as f64];
    let (right, up, eye) = (f3(pj.right), f3(pj.up), f3(pj.eye));
    let target = f3(pj.target);
    let dir = [-eye[0], -eye[1], -eye[2]];
    // Which face (if any) has cut the point away.
    let faces = &solid.faces;
    let cutter = |p: P3| -> Option<usize> {
        for (g, f) in faces.iter().enumerate() {
            let Some(c) = f.carved else { continue };
            let q = f.to_view.back(p);
            let under = -q[2];
            if under > deepest[g] + 0.02 || q[0] < 0.0 || q[1] < 0.0 || q[0] > f.size[0] || q[1] > f.size[1] {
                continue;
            }
            // A cut the whole depth of the object takes it all, to the far side.
            let cut = c.depth_at(q[0], q[1]);
            if under < cut || cut >= f.depth - 0.01 {
                return Some(g);
            }
        }
        None
    };
    let colour = solid.colour;
    let shade_of = |n: P3, deep: f64| {
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt().max(1e-12);
        let lit = 0.4 + 0.6 * ((n[0] * LIGHT[0] + n[1] * LIGHT[1] + n[2] * LIGHT[2]) / len).max(0.0);
        shade(colour, (lit * (1.0 - 0.25 * deep.clamp(0.0, 1.0))) as f32)
    };
    // What is seen at pixel (i, j).
    let pixel = |i: usize, j: usize| -> Color32 {
        {
            // The point of the pixel on the plane through the middle of the picture, and the ray from it away from the eye.
            let sx = (rect.min.x + (i as f32 + 0.5) / s - pj.centre.x) as f64 / pj.scale as f64;
            let sy = -((rect.min.y + (j as f32 + 0.5) / s - pj.centre.y) as f64) / pj.scale as f64;
            let o = [target[0] + right[0] * sx + up[0] * sy, target[1] + right[1] * sx + up[1] * sy, target[2] + right[2] * sx + up[2] * sy];
            // Where the ray is in the block.
            let (mut t0, mut t1, mut axis) = (f64::NEG_INFINITY, f64::INFINITY, 0usize);
            let mut hit_box = true;
            for k in 0..3 {
                if dir[k].abs() < 1e-12 {
                    if o[k] < lo[k] || o[k] > hi[k] {
                        hit_box = false;
                    }
                    continue;
                }
                let (a, b) = ((lo[k] - o[k]) / dir[k], (hi[k] - o[k]) / dir[k]);
                let (a, b) = if a < b { (a, b) } else { (b, a) };
                if a > t0 {
                    t0 = a;
                    axis = k;
                }
                t1 = t1.min(b);
            }
            if !hit_box || t0 > t1 {
                return Color32::TRANSPARENT;
            }
            let at = |t: f64| [o[0] + dir[0] * t, o[1] + dir[1] * t, o[2] + dir[2] * t];
            let first = at(t0 + 1e-6);
            let Some(mut by) = cutter(first) else {
                // The side of the block itself.
                let mut n = [0.0; 3];
                n[axis] = -dir[axis].signum();
                return shade_of(n, 0.0);
            };
            // Through the air of the cuts until the material.
            let mut t = t0;
            let mut found = None;
            while t < t1 {
                let next = (t + step).min(t1);
                match cutter(at(next)) {
                    Some(g) => by = g,
                    None => {
                        // Closer to the surface between the last point in the air and the first in the material.
                        let (mut a, mut b) = (t, next);
                        for _ in 0..5 {
                            let m = (a + b) / 2.0;
                            match cutter(at(m)) {
                                Some(g) => {
                                    by = g;
                                    a = m;
                                }
                                None => b = m,
                            }
                        }
                        found = Some(b);
                        break;
                    }
                }
                t = next;
            }
            let Some(th) = found else { return Color32::TRANSPARENT };
            // The surface of the face that cut the air there: its slope, turned into the picture.
            let f = &faces[by];
            let c = f.carved.expect("a face that cuts has a surface");
            let q = f.to_view.back(at(th));
            let e = c.step();
            let d = |x: f64, y: f64| c.depth_at(x, y);
            let dzdx = -(d(q[0] + e, q[1]) - d(q[0] - e, q[1])) / (2.0 * e);
            let dzdy = -(d(q[0], q[1] + e) - d(q[0], q[1] - e)) / (2.0 * e);
            let n = f.to_view.dir([-dzdx, -dzdy, 1.0]);
            shade_of(n, -q[2] / f.depth.max(0.1))
        }
    };
    // The rows shared out among the cores, so that a fine picture does not hold the program up for long.
    let mut px = vec![Color32::TRANSPARENT; iw * ih];
    let threads = std::thread::available_parallelism().map_or(1, |n| n.get()).clamp(1, 16);
    let rows = ih.div_ceil(threads).max(1);
    std::thread::scope(|sc| {
        for (k, chunk) in px.chunks_mut(rows * iw.max(1)).enumerate() {
            let pixel = &pixel;
            sc.spawn(move || {
                for (n, p) in chunk.iter_mut().enumerate() {
                    *p = pixel(n % iw, k * rows + n / iw);
                }
            });
        }
    });
    egui::ColorImage::new([iw, ih], px)
}

/// What a button of the zoom bar asks the program to fit the view to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fit {
    /// The whole work area (the view is also back home).
    Area,
    /// The objects (the cuts or the burns).
    Elements,
}

/// The mouse on the picture, before it is drawn: a drag turns the view, a drag with the right or the middle button (or with Shift)
/// moves it, the wheel zooms and a double click goes back home. True while the view is being turned or moved.
pub fn handle_input(ui: &egui::Ui, resp: &egui::Response, cam: &mut Camera) -> bool {
    let shift = ui.input(|i| i.modifiers.shift);
    let mut moving = false;
    if resp.dragged_by(egui::PointerButton::Primary) && !shift {
        let d = resp.drag_delta();
        cam.rotate(d.x, d.y);
        moving = true;
    }
    if resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) || (shift && resp.dragged_by(egui::PointerButton::Primary)) {
        cam.pan += resp.drag_delta();
        moving = true;
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            cam.zoom_by((scroll * 0.004).exp());
            moving = true;
        }
    }
    if resp.double_clicked() {
        cam.reset();
    }
    moving
}

/// The navigation cube (not on the plan from above) and the zoom bar over the drawn picture `rect`. Returns what the bar asks to fit.
pub fn overlays(ui: &mut egui::Ui, rect: Rect, cam: &mut Camera) -> Option<Fit> {
    if !cam.top {
        let (eye, right, up) = cam.axes();
        match crate::navicube::show(ui, rect, &[right, up, eye]) {
            Some(crate::navicube::Nav::LookFrom(d)) => cam.look_from(d),
            Some(crate::navicube::Nav::Home) => cam.reset(),
            None => {}
        }
    }
    match crate::zoom_bar::overlay_with(ui, rect, cam.zoom, &tr("Drag: rotate · right drag or Shift+drag: move · wheel: zoom"), true) {
        Some(crate::zoom_bar::ZoomCmd::FitObjects) => Some(Fit::Elements),
        Some(crate::zoom_bar::ZoomCmd::In) => {
            cam.zoom_about_middle(crate::zoom_bar::STEP);
            None
        }
        Some(crate::zoom_bar::ZoomCmd::Out) => {
            cam.zoom_about_middle(1.0 / crate::zoom_bar::STEP);
            None
        }
        Some(crate::zoom_bar::ZoomCmd::Whole) => {
            cam.reset();
            Some(Fit::Area)
        }
        None => None,
    }
}

/// Zoom and pan `cam` so that the points `pts` fill `rect` (the middle and the radius of the picture as the drawing has them).
pub fn fit_points(cam: &mut Camera, rect: Rect, target: P3, radius: f64, pts: &[P3]) {
    let base = Camera { zoom: 1.0, pan: Vec2::ZERO, ..*cam };
    let pj = Projector::new(&base, target, radius, rect);
    let (mut min, mut max) = (Pos2::new(f32::MAX, f32::MAX), Pos2::new(f32::MIN, f32::MIN));
    for p in pts {
        let q = pj.project(*p);
        min = min.min(q);
        max = max.max(q);
    }
    if min.x > max.x {
        return;
    }
    let (bw, bh) = ((max.x - min.x).max(1.0), (max.y - min.y).max(1.0));
    let zoom = (rect.width() / bw).min(rect.height() / bh) * 0.92;
    let offset = (min + (max - min) / 2.0) - rect.center();
    cam.zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
    cam.pan = -offset * cam.zoom;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect::from_min_size(Pos2::ZERO, egui::vec2(800.0, 600.0))
    }

    #[test]
    fn the_camera_turns_zooms_and_goes_back_home_and_the_cube_sets_it_from_a_direction() {
        let mut c = Camera::default();
        c.rotate(100.0, 20.0);
        c.zoom_by(2.0);
        c.pan = egui::vec2(5.0, 5.0);
        assert!(c.yaw != START_YAW && c.zoom == 2.0);
        c.reset();
        assert_eq!(c, Camera::default());
        c.look_from([0.0, -1.0, 0.0]);
        let (eye, _, _) = c.axes();
        assert!(eye[1] < -0.99, "from the front: {eye:?}");
        // Not under the table unless it may.
        c.look_from([0.0, 0.0, -1.0]);
        assert!(c.pitch > 0.0);
        c.under = true;
        c.look_from([0.0, 0.0, -1.0]);
        assert!(c.pitch < -1.0);
        // The plan cannot be turned.
        let mut top = Camera { top: true, ..Camera::default() };
        top.rotate(50.0, 50.0);
        assert_eq!(top.axes().0, [0.0, -0.0f32.cos() * std::f32::consts::FRAC_PI_2.cos(), 1.0]);
    }

    #[test]
    fn the_target_is_in_the_middle_and_from_above_only_the_top_is_seen() {
        let pj = Projector::new(&Camera::default(), [10.0, 20.0, -5.0], 50.0, rect());
        assert!((pj.project([10.0, 20.0, -5.0]) - rect().center()).length() < 1e-3);
        assert!(pj.facing([0.0, 0.0, 1.0]) > 0.0 && pj.facing([0.0, 0.0, -1.0]) < 0.0);
        // The front (−Y in the world) is turned to the eye at the first turn, the back is not.
        assert!(pj.facing([0.0, -1.0, 0.0]) > 0.0 && pj.facing([0.0, 1.0, 0.0]) < 0.0);
    }

    #[test]
    fn the_shown_face_of_a_post_or_a_block_is_up_and_the_faces_meet_at_their_edges() {
        for solid in [Solid::post(40.0, 30.0, 250.0), Solid::block(100.0, 80.0, 60.0)] {
            for shown in 0..solid.count() {
                let fr = FaceFrame::of(&solid, shown);
                let up = fr.out(shown);
                assert!((up[2] - 1.0).abs() < 1e-9, "face {shown} up: {up:?}");
                let f = solid.face(shown);
                assert_eq!((fr.lo, fr.hi), ([0.0, 0.0, -f.depth], [f.w, f.h, 0.0]));
                // Every face's corners lie on the box of the object.
                for j in 0..solid.count() {
                    let g = solid.face(j);
                    for c in [Pt::new(0.0, 0.0), Pt::new(g.w, 0.0), Pt::new(g.w, g.h), Pt::new(0.0, g.h)] {
                        let p = fr.at(&solid, j, c);
                        for k in 0..3 {
                            assert!(p[k] > fr.lo[k] - 1e-6 && p[k] < fr.hi[k] + 1e-6, "face {j} of {shown}: {p:?}");
                        }
                    }
                }
            }
            // Turned from face B to face A and back is no move at all.
            let ab = face_move(&solid, 1, 0);
            let ba = face_move(&solid, 0, 1);
            let p = [3.0, 4.0, -2.0];
            let q = ba.apply(ab.apply(p));
            assert!((0..3).all(|k| (p[k] - q[k]).abs() < 1e-9));
            assert!((0..3).all(|k| (ab.back(ab.apply(p))[k] - p[k]).abs() < 1e-9));
        }
    }

    /// A face carved by a function.
    struct Hole(f64, f64, f64, f64);
    impl Carved for Hole {
        fn depth_at(&self, x: f64, y: f64) -> f64 {
            if (x - self.0).abs() < self.2 && (y - self.1).abs() < self.2 { self.3 } else { 0.0 }
        }
        fn step(&self) -> f64 {
            0.5
        }
        fn deepest(&self) -> f64 {
            self.3
        }
    }

    #[test]
    fn a_window_through_the_object_is_clear_and_a_pocket_is_not() {
        let solid = Solid::block(40.0, 40.0, 20.0);
        let fr = FaceFrame::of(&solid, 0);
        let cache = RefCell::new(None);
        let picture = |carve: &Hole| {
            let faces = (0..solid.count()).map(|j| {
                let f = solid.face(j);
                SolidFace { carved: (j == 0).then_some(carve as &dyn Carved), to_view: fr.moves[j], size: [f.w, f.h], depth: f.depth, letter: Solid::letter(j) }
            }).collect();
            let view = SolidView { faces, shown: 0, colour: [200, 160, 120], key: 1, cache: &cache };
            let (target, radius) = fr.framing();
            let cam = Camera { top: true, ..Camera::default() };
            let pj = Projector::new(&cam, target, radius, rect());
            let img = solid_picture(&pj, &view, rect(), 0.25, 200, 150, 1.0);
            // The pixel over the middle of face A.
            let mid = pj.project([20.0, 20.0, 0.0]);
            img.pixels[(mid.y * 0.25) as usize * 200 + (mid.x * 0.25) as usize]
        };
        assert_eq!(picture(&Hole(20.0, 20.0, 5.0, 20.0)), Color32::TRANSPARENT, "through: the background");
        let pocket = picture(&Hole(20.0, 20.0, 5.0, 5.0));
        assert!(pocket != Color32::TRANSPARENT && pocket.a() == 255, "a pocket has a bottom");
        assert!(picture(&Hole(0.0, 0.0, 1.0, 5.0)).a() == 255, "no cut there: the top");
    }

    #[test]
    fn the_quality_goes_from_coarse_to_fine_and_three_is_as_it_was() {
        assert!(quality_pixels(1) < quality_pixels(5) && quality_sharpness(1) < quality_sharpness(5) && quality_detail(1) < quality_detail(5));
        assert_eq!((quality_pixels(QUALITY_DEFAULT), quality_sharpness(QUALITY_DEFAULT), quality_detail(QUALITY_DEFAULT)), (1_500_000.0, 1.0, 1.0));
        assert_eq!(quality_pixels(0), quality_pixels(1));
        assert_eq!(quality_pixels(9), quality_pixels(5));
    }
}
