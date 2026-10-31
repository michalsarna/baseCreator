//! A background picture under the drawing: a photo of the material, a plan, a scan; from a file or, where the program has a camera,
//! the picture of the camera. Its four corners are put on the work area (or dragged onto the corners of the piece), with a perspective
//! transform, and it is drawn faintly under the shapes. It is not saved with the project.
use super::{CanvasView, EditorHost};
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::popup;
use base_core::geom::Pt;
use egui::{Color32, Pos2, RichText, Stroke};

pub struct Background {
    pub tex: egui::TextureHandle,
    pub size: (usize, usize),
    /// The work-area positions (mm) of the top-left, top-right, bottom-right and bottom-left corners of the picture.
    pub corners: [Pt; 4],
    pub opacity: f32,
    pub name: String,
    pub visible: bool,
    /// The corners show handles that can be dragged.
    pub edit: bool,
    drag: Option<usize>,
}

/// The point of the quad `c` that the unit square point (`u`, `v`) goes to (a perspective transform).
pub fn quad_map(c: &[Pt; 4], u: f64, v: f64) -> Pt {
    let (x0, y0, x1, y1, x2, y2, x3, y3) = (c[0].x, c[0].y, c[1].x, c[1].y, c[2].x, c[2].y, c[3].x, c[3].y);
    let (dx1, dx2, dx3) = (x1 - x2, x3 - x2, x0 - x1 + x2 - x3);
    let (dy1, dy2, dy3) = (y1 - y2, y3 - y2, y0 - y1 + y2 - y3);
    let det = dx1 * dy2 - dy1 * dx2;
    let (g, h) = if (dx3.abs() < 1e-12 && dy3.abs() < 1e-12) || det.abs() < 1e-12 { (0.0, 0.0) } else { ((dx3 * dy2 - dy3 * dx2) / det, (dx1 * dy3 - dy1 * dx3) / det) };
    let (a, b, cc) = (x1 - x0 + g * x1, x3 - x0 + h * x3, x0);
    let (d, e, f) = (y1 - y0 + g * y1, y3 - y0 + h * y3, y0);
    let w = g * u + h * v + 1.0;
    Pt::new((a * u + b * v + cc) / w, (d * u + e * v + f) / w)
}

const GRID: usize = 12;
const HANDLE: f32 = 9.0;

/// The most pixels a picture may have (a photo of 100 megapixels would only fill the memory).
const MAX_SIDE: u32 = 4096;

/// A picture file as an image for the texture, made smaller when it is huge.
pub fn read_picture(path: &std::path::Path) -> Result<egui::ColorImage, String> {
    let img = image::open(path).map_err(|e| e.to_string())?;
    let img = if img.width() > MAX_SIDE || img.height() > MAX_SIDE { img.resize(MAX_SIDE, MAX_SIDE, image::imageops::FilterType::Triangle) } else { img };
    let rgba = img.to_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied([rgba.width() as usize, rgba.height() as usize], rgba.as_raw()))
}

pub trait EditorBackground: EditorHost {
    /// Put a picture under the drawing, its corners on the work area (the corners the last picture had are kept when the picture
    /// has the same size).
    fn set_background(&mut self, ctx: &egui::Context, img: egui::ColorImage, name: &str) {
        let (w, h) = self.work_size();
        let size = (img.width(), img.height());
        let keep = self.ed().background.as_ref().filter(|b| b.size == size).map(|b| (b.corners, b.opacity));
        let (corners, opacity) = keep.unwrap_or(([Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)], 0.6));
        let tex = ctx.load_texture("editor-background", img, egui::TextureOptions::LINEAR);
        self.ed_mut().background = Some(Background { tex, size, corners, opacity, name: name.to_string(), visible: true, edit: false, drag: None });
    }

    /// Ask for a picture file and put it under the drawing.
    fn load_background_file(&mut self, ctx: &egui::Context) {
        let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"]).pick_file() else { return };
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match read_picture(&p) {
            Ok(img) => {
                self.set_background(ctx, img, &name);
                self.say(trf("Background picture: {}", &[&name]));
            }
            Err(e) => self.say(trf("Could not read the picture: {}", &[&e])),
        }
    }

    /// The corners are the corners of the work area again.
    fn fit_background(&mut self) {
        let (w, h) = self.work_size();
        if let Some(b) = self.ed_mut().background.as_mut() {
            b.corners = [Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)];
        }
    }

    /// Drawn under the shapes: the picture in a mesh whose corners follow the transform, and the handles while they can be dragged.
    fn paint_background(&self, v: &CanvasView) {
        let Some(b) = self.ed().background.as_ref().filter(|b| b.visible) else { return };
        let o = v.origin;
        let tint = Color32::from_white_alpha((b.opacity.clamp(0.05, 1.0) * 255.0) as u8);
        let mut mesh = egui::Mesh::with_texture(b.tex.id());
        for j in 0..=GRID {
            for i in 0..=GRID {
                let (u, w) = (i as f64 / GRID as f64, j as f64 / GRID as f64);
                mesh.vertices.push(egui::epaint::Vertex { pos: self.w2s(o, quad_map(&b.corners, u, w)), uv: egui::pos2(u as f32, w as f32), color: tint });
            }
        }
        for j in 0..GRID {
            for i in 0..GRID {
                let a = (j * (GRID + 1) + i) as u32;
                let (b1, c, d) = (a + 1, a + (GRID + 1) as u32, a + (GRID + 1) as u32 + 1);
                mesh.indices.extend([a, b1, d, a, d, c]);
            }
        }
        v.painter.add(egui::Shape::mesh(mesh));
        if b.edit {
            let pts: Vec<Pos2> = b.corners.iter().map(|c| self.w2s(o, *c)).collect();
            v.painter.add(egui::Shape::closed_line(pts.clone(), Stroke::new(1.5, theme::accent())));
            for p in pts {
                v.painter.rect_filled(egui::Rect::from_center_size(p, egui::vec2(HANDLE * 1.6, HANDLE * 1.6)), 2.0, Color32::WHITE);
                v.painter.rect_stroke(egui::Rect::from_center_size(p, egui::vec2(HANDLE * 1.6, HANDLE * 1.6)), 2.0, Stroke::new(1.5, theme::accent()), egui::StrokeKind::Inside);
            }
        }
    }

    /// Dragging a corner of the picture. True while the pointer belongs to it (the tools leave it alone then).
    fn background_interact(&mut self, resp: &egui::Response, o: Pos2, press: Option<Pos2>) -> bool {
        let Some(b) = self.ed().background.as_ref().filter(|b| b.visible && b.edit) else {
            if let Some(b) = self.ed_mut().background.as_mut() {
                b.drag = None;
            }
            return false;
        };
        if let Some(m) = press {
            let hit = (0..4).find(|i| (self.w2s(o, b.corners[*i]) - m).length() <= HANDLE * 1.6);
            if hit.is_some() {
                self.ed_mut().background.as_mut().unwrap().drag = hit;
            }
        }
        let Some(i) = self.ed().background.as_ref().and_then(|b| b.drag) else { return false };
        if resp.dragged() {
            if let Some(m) = resp.interact_pointer_pos() {
                let w = self.s2w(o, m);
                if let Some(b) = self.ed_mut().background.as_mut() {
                    b.corners[i] = w;
                }
            }
        } else if !resp.is_pointer_button_down_on() {
            self.ed_mut().background.as_mut().unwrap().drag = None;
        }
        true
    }

    /// The window of the background picture.
    fn background_window(&mut self, ctx: &egui::Context) {
        if !self.ed().show_background {
            return;
        }
        let mut open = true;
        let (mut load, mut cam, mut fit, mut remove) = (false, false, false, false);
        let has_cam = self.background_camera_image().is_some();
        let (mut opacity, mut visible, mut edit, name) = match self.ed().background.as_ref() {
            Some(b) => (b.opacity, b.visible, b.edit, Some(b.name.clone())),
            None => (0.6, true, false, None),
        };
        popup(ctx, "background_window", &tr("Background picture"), [380.0, 260.0], &mut open, |ui| {
            ui.horizontal_wrapped(|ui| {
                load = ui.button(tr("Load a picture from a file…")).clicked();
                if has_cam {
                    cam = ui.button(tr("Use the camera picture")).on_hover_text(tr("The picture of the camera as it is now")).clicked();
                }
            });
            if let Some(n) = &name {
                ui.label(RichText::new(n).strong());
                ui.add(egui::Slider::new(&mut opacity, 0.05..=1.0).text(tr("Opacity")));
                ui.checkbox(&mut visible, tr("Show"));
                ui.checkbox(&mut edit, tr("Move the corners"));
                ui.horizontal(|ui| {
                    fit = ui.button(tr("Fit to the work area")).clicked();
                    remove = ui.button(tr("Remove")).clicked();
                });
            }
            ui.label(RichText::new(tr("The picture lies under the drawing: drag its corners onto the corners of the material or of the work area. It is not saved with the project.")).color(theme::text_dim()));
        });
        if let Some(b) = self.ed_mut().background.as_mut() {
            b.opacity = opacity;
            b.visible = visible;
            b.edit = edit;
        }
        if load {
            self.load_background_file(ctx);
        }
        if cam {
            if let Some((img, name)) = self.background_camera_image() {
                self.set_background(ctx, img, &name);
            }
        }
        if fit {
            self.fit_background();
        }
        if remove {
            self.ed_mut().background = None;
        }
        self.ed_mut().show_background = open;
    }
}

impl<T: EditorHost> EditorBackground for T {}
