//! A picture under the drawing: a photo of the material, a plan, a scan; from a file or, where the program has a camera, the live picture
//! of the camera (the camera overlay). Its four corners are put on the work area (or dragged onto the corners of the piece, or found from
//! reference points of the camera), with a perspective transform, and it is drawn faintly under the shapes. It is a section of the project
//! (`base_core::overlay_data`), so the file brings it back, in any of the programs.
use super::{CanvasView, EditorHost};
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::popup;
use base_core::camimg;
use base_core::geom::Pt;
use base_core::overlay_data::{OverlayData, OVERLAY_SECTION};
use base_core::project::Project;
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
    /// The picture as it is shown, for the project file.
    pub image: std::sync::Arc<egui::ColorImage>,
}

impl Background {
    /// The picture as a project file keeps it (`None`: it cannot be written).
    pub fn data(&self) -> Option<OverlayData> {
        let rgb: Vec<u8> = self.image.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
        OverlayData::new(&self.name, self.corners, self.opacity as f64, self.visible, &rgb, self.size.0 as u32, self.size.1 as u32).ok()
    }
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

/// The section of the picture in a project that is being saved. One that was read and not shown yet, and one of a newer format, stay as
/// they are. Also for a tab that is not shown.
pub fn background_section(background: &Option<Background>, pending: &Option<OverlayData>, p: &mut Project) {
    if let Some(d) = background.as_ref().and_then(Background::data) {
        p.set_section(OVERLAY_SECTION, &d);
    } else if pending.is_none() && p.section::<OverlayData>(OVERLAY_SECTION).ok().flatten().is_some_and(|d| d.is_readable()) {
        p.remove_section(OVERLAY_SECTION);
    }
}

pub trait EditorBackground: EditorHost {
    /// Put a picture under the drawing, its corners on the work area or where they were kept for the device (the corners the last
    /// picture had are kept when the picture has the same size).
    fn set_background(&mut self, ctx: &egui::Context, img: egui::ColorImage, name: &str) {
        let (w, h) = self.work_size();
        let size = (img.width(), img.height());
        let image = std::sync::Arc::new(img);
        // The same size: only the picture changes, it may be the next one of the camera.
        if let Some(b) = self.ed_mut().background.as_mut().filter(|b| b.size == size) {
            b.tex.set(image.clone(), egui::TextureOptions::LINEAR);
            b.image = image;
            b.name = name.to_string();
            return;
        }
        let (corners, opacity) = self.overlay_calibration().unwrap_or(([Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, h), Pt::new(0.0, h)], 0.6));
        let tex = ctx.load_texture("editor-background", image.clone(), egui::TextureOptions::LINEAR);
        self.ed_mut().background = Some(Background { tex, size, corners, opacity, name: name.to_string(), visible: true, edit: false, drag: None, image });
    }

    /// Called every frame: the overlay of a project that was just opened, and the next picture of the camera.
    fn background_tick(&mut self, ctx: &egui::Context) {
        if let Some(d) = self.ed_mut().background_pending.take() {
            if let Ok((w, h, rgba)) = d.decode() {
                let image = std::sync::Arc::new(egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba));
                let tex = ctx.load_texture("editor-background", image.clone(), egui::TextureOptions::LINEAR);
                self.ed_mut().background = Some(Background { tex, size: (w as usize, h as usize), corners: d.corners, opacity: d.opacity as f32, name: d.name, visible: d.visible, edit: false, drag: None, image });
            }
        }
        if !self.ed().background_live {
            return;
        }
        let Some(seq) = self.live_camera(ctx) else {
            self.ed_mut().background_live = false;
            return;
        };
        if seq == self.ed().background_seq {
            return;
        }
        let Some((img, name)) = self.background_camera_image() else { return };
        self.ed_mut().background_seq = seq;
        let first = self.ed().background.as_ref().is_none_or(|b| b.size != (img.width(), img.height()));
        let (pw, ph) = (img.width() as f64, img.height() as f64);
        self.set_background(ctx, img, &name);
        // A camera that was aligned before needs no new alignment: its points are kept with the device.
        if first && self.overlay_calibration().is_none() {
            let saved = self.overlay_camera().and_then(|c| camimg::homography(&c.reference)).and_then(|h| camimg::corners(&h, pw, ph));
            if let (Some(c), Some(b)) = (saved, self.ed_mut().background.as_mut()) {
                b.corners = c;
            }
        }
    }

    /// Put the picture into a project that is being saved.
    fn background_to_project(&self, p: &mut Project) {
        background_section(&self.ed().background, &self.ed().background_pending, p);
    }

    /// The button that puts the live camera under the drawing, or takes it away (on the camera tab).
    fn overlay_toggle_button(&mut self, ui: &mut egui::Ui) {
        let live = self.ed().background_live;
        if ui.selectable_label(live, tr("Overlay")).on_hover_text(tr("Show a photo or the live camera under the design to position work on the material.")).clicked() {
            let ed = self.ed_mut();
            ed.background_live = !live;
            if live {
                ed.background = None;
            }
        }
    }

    /// The project shown now has changed: its own picture comes (in the next frame), the one of the former project goes.
    fn background_from_project(&mut self, p: &Project) {
        let ed = self.ed_mut();
        ed.background = None;
        ed.background_live = false;
        ed.background_pending = p.section::<OverlayData>(OVERLAY_SECTION).ok().flatten().filter(|d| d.is_readable());
    }

    /// Ask for a picture file and put it under the drawing.
    fn load_background_file(&mut self, ctx: &egui::Context) {
        let Some(p) = rfd::FileDialog::new().add_filter("Images", &["png", "jpg", "jpeg", "bmp", "webp", "gif"]).pick_file() else { return };
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match read_picture(&p) {
            Ok(img) => {
                self.set_background(ctx, img, &name);
                let said = if self.overlay_is_camera() { trf("Overlay: {}", &[&name]) } else { trf("Background picture: {}", &[&name]) };
                self.say(said);
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
            self.overlay_changed();
        }
        true
    }

    /// The window of the picture: *Camera overlay* where there is a camera, *Background picture* where there is none.
    fn background_window(&mut self, ctx: &egui::Context) {
        if !self.ed().show_background {
            return;
        }
        let mut open = true;
        let (mut load, mut fit, mut remove) = (false, false, false);
        let has_cam = self.overlay_is_camera();
        let can_start = self.overlay_can_start_camera();
        let cam_error = self.camera_error();
        let mut live = self.ed().background_live;
        let mut points = self.overlay_camera().map(|c| c.reference.clone());
        let before = points.clone();
        let title = if has_cam { tr("Camera overlay") } else { tr("Background picture") };
        let mut b = self.ed_mut().background.take();
        let state = |b: &Option<Background>| b.as_ref().map(|b| (b.corners, b.opacity, b.visible));
        let was = state(&b);
        popup(ctx, "background_window", &title, [400.0, 280.0], &mut open, |ui| {
            if has_cam {
                ui.label(RichText::new(tr("Show a photo or the live camera under the design to position work on the material.")).color(theme::text_dim()));
            } else {
                ui.label(RichText::new(tr("The picture lies under the drawing: drag its corners onto the corners of the material or of the work area. It is saved with the project.")).color(theme::text_dim()));
            }
            ui.horizontal_wrapped(|ui| {
                load = ui.button(tr("Load picture…")).clicked();
                if has_cam {
                    if live {
                        live = !ui.button(tr("Freeze frame")).clicked();
                    } else {
                        live = ui.add_enabled(can_start, egui::Button::new(tr("Start camera"))).clicked();
                    }
                }
                if b.is_some() {
                    remove = crate::window::delete_button(ui, &tr("Remove")).clicked();
                }
            });
            if has_cam {
                self.overlay_camera_ui(ui);
            }
            if !cam_error.is_empty() {
                ui.label(RichText::new(&cam_error).color(Color32::from_rgb(0xc0, 0x30, 0x30)));
            }
            let Some(o) = b.as_mut() else { return };
            ui.separator();
            ui.label(RichText::new(trf("Overlay: {}", &[&o.name])).strong());
            ui.checkbox(&mut o.visible, tr("Show overlay"));
            ui.add(egui::Slider::new(&mut o.opacity, 0.05..=1.0).text(tr("Opacity")));
            ui.checkbox(&mut o.edit, tr("Align: drag the four corners on the work area")).on_hover_text(tr("Drag each corner to the matching corner of the material or work area; perspective is corrected."));
            ui.horizontal_wrapped(|ui| {
                fit = ui.button(tr("Reset to work area")).clicked();
                if ui.button(tr("Flip H")).clicked() {
                    o.corners.swap(0, 1);
                    o.corners.swap(3, 2);
                }
                if ui.button(tr("Flip V")).clicked() {
                    o.corners.swap(0, 3);
                    o.corners.swap(1, 2);
                }
                if ui.button(tr("Rotate 90°")).clicked() {
                    o.corners.rotate_right(1);
                }
            });
            let (pw, ph) = (o.size.0 as f64, o.size.1 as f64);
            if let Some(points) = points.as_mut() {
                ui.collapsing(tr("Reference points"), |ui| {
                    ui.label(RichText::new(tr("Pairs of a point in the picture (pixels) and the place it has in the work area (mm). At least 3, 4 recommended; they fix the alignment, perspective included.")).color(theme::text_dim()));
                    let mut del = None;
                    for (i, r) in points.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", i + 1));
                            ui.label("px");
                            ui.add(egui::DragValue::new(&mut r.px.0).speed(1.0).range(0.0..=pw));
                            ui.add(egui::DragValue::new(&mut r.px.1).speed(1.0).range(0.0..=ph));
                            ui.label("mm");
                            ui.add(egui::DragValue::new(&mut r.mm.0).speed(0.5));
                            ui.add(egui::DragValue::new(&mut r.mm.1).speed(0.5));
                            if crate::window::delete_icon(ui, &tr("Delete this point")).clicked() {
                                del = Some(i);
                            }
                        });
                    }
                    if let Some(i) = del {
                        points.remove(i);
                    }
                    let found = camimg::homography(points);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr("Add a point")).clicked() && points.len() < 32 {
                            points.push(camimg::RefPoint { px: (pw / 2.0, ph / 2.0), mm: (0.0, 0.0) });
                        }
                        if ui.button(tr("Points from the current corners")).on_hover_text(tr("Replaces the points with the four corners of the picture and where they lie now.")).clicked() {
                            *points = camimg::points_from_corners(&o.corners, pw, ph);
                        }
                        if ui.add_enabled(found.is_some(), egui::Button::new(tr("Align by the points"))).on_hover_text(tr("Fits the picture to the points (least squares) and moves the corners accordingly.")).clicked() {
                            if let Some(c) = found.as_ref().and_then(|h| camimg::corners(h, pw, ph)) {
                                o.corners = c;
                            }
                        }
                    });
                    if found.is_none() {
                        ui.label(RichText::new(tr("Not enough points, or they lie on one line.")).color(Color32::from_rgb(0xc0, 0x80, 0x20)));
                    }
                });
            }
        });
        // The handles go with the window that switches them.
        if let Some(o) = b.as_mut().filter(|_| !open) {
            o.edit = false;
        }
        let moved = state(&b) != was;
        self.ed_mut().background = b;
        self.ed_mut().background_live = live;
        if points != before {
            if let (Some(c), Some(p)) = (self.overlay_camera(), points) {
                c.reference = p;
            }
            self.overlay_camera_changed();
        }
        if load {
            self.ed_mut().background_live = false;
            self.load_background_file(ctx);
        }
        if fit {
            self.fit_background();
        }
        if remove {
            let ed = self.ed_mut();
            ed.background = None;
            ed.background_live = false;
        }
        if moved || load || fit || remove {
            self.overlay_changed();
        }
        self.ed_mut().show_background = open;
    }
}

impl<T: EditorHost> EditorBackground for T {}

#[cfg(test)]
mod tests {
    use base_core::overlay_data::{OverlayData, OVERLAY_SECTION};
    use base_core::geom::Pt;
    use base_core::project::Project;
    use base_core::scene::Scene;

    /// What LightCreator writes today is what the other programs read.
    #[test]
    fn the_section_of_a_project_is_read_as_it_was_written() {
        let corners = [Pt::new(1.0, 2.0), Pt::new(300.0, 3.0), Pt::new(298.0, 200.0), Pt::new(0.5, 199.0)];
        let d = OverlayData::new("photo.jpg", corners, 0.4, true, &[120u8; 8 * 4 * 3], 8, 4).unwrap();
        let mut p = Project::new(Scene::default());
        p.set_section(OVERLAY_SECTION, &d);
        let back = Project::from_json(&p.to_json()).unwrap();
        assert_eq!(back.section::<OverlayData>("overlay").unwrap(), Some(d));
    }
}
