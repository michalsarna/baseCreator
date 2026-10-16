//! Image dialogs: import / adjust a bitmap (rotate, flip, brightness, contrast ...) and trace it into curves.
use super::EditorHost;
use crate::i18n::{tr, trf};
use crate::theme;
use crate::units_ui::{drag_len, fmt_len};
use crate::window::{import_button, popup};
use crate::icons;
use base_core::geom::{Pt, Xf};
use base_core::image::{Adjust, ImageData};
use base_core::scene::Kind;
use base_core::trace::{trace, TraceParams};
use egui::{Color32, RichText, Stroke};

const PREVIEW_PX: u32 = 520;

/// The picture files that can be imported (what a dropped file is checked against too).
pub const IMAGE_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "bmp", "gif", "webp"];

/// Is the file a picture that can be imported?
pub fn is_image_file(p: &std::path::Path) -> bool {
    p.extension().is_some_and(|e| IMAGE_EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Import of a new picture, or adjusting an existing image shape.
pub struct ImgDlg {
    /// `None` imports a new picture; `Some(id)` replaces the pixels of that shape.
    pub target: Option<u64>,
    pub base: ImageData,
    pub adj: Adjust,
    pub negative: bool,
    pub width: f64,
    thumb: ImageData,
    tex: Option<egui::TextureHandle>,
    shown: Option<(Adjust, bool)>,
    /// Height / width of the adjusted picture.
    aspect: f64,
}

impl ImgDlg {
    pub fn new(target: Option<u64>, base: ImageData) -> ImgDlg {
        let thumb = base.downscaled(PREVIEW_PX);
        ImgDlg { target, adj: Adjust::default(), negative: base.invert, width: base.w, base, thumb, tex: None, shown: None, aspect: 1.0 }
    }

    /// The final picture with all adjustments, sized to `width` millimetres.
    pub fn result(&self) -> ImageData {
        let mut im = self.base.adjusted(&self.adj);
        let k = self.width / im.w.max(1e-9);
        im.w *= k;
        im.h *= k;
        im.invert = self.negative;
        im
    }
}

pub struct TraceDlg {
    pub shape: u64,
    pub params: TraceParams,
    pub delete_original: bool,
    thumb: ImageData,
    tex: Option<egui::TextureHandle>,
    preview: Vec<Vec<Pt>>,
    shown: Option<TraceParams>,
}

/// The picture dialogs: import, adjust and trace. Implemented for every `EditorHost`.
pub trait EditorImages: EditorHost {
    /// Read a picture file and open the import dialog.
    fn open_image_dialog(&mut self, p: std::path::PathBuf) {
        let name = p.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let (bw, bh) = self.work_size();
        let longest = (bw.min(bh) * 0.5).clamp(10.0, 200.0);
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|b| ImageData::from_bytes(&b, &name, longest)) {
            Ok(im) => self.ed_mut().img_dlg = Some(ImgDlg::new(None, im)),
            Err(e) => self.say(trf("Import failed: {}", &[&e])),
        }
    }

    /// Ask for a picture file and open the import dialog.
    fn import_image_dialog(&mut self) {
        if let Some(p) = rfd::FileDialog::new().add_filter("Images", IMAGE_EXTENSIONS).pick_file() {
            self.open_image_dialog(p);
        }
    }

    /// Open the import dialog for a picture that is already decoded.
    fn open_import_dialog_with(&mut self, im: ImageData) {
        self.ed_mut().img_dlg = Some(ImgDlg::new(None, im));
    }

    fn open_adjust(&mut self) {
        if let Some(Kind::Image(im)) = self.ed().sel.first().and_then(|id| self.scene().shape(*id)).map(|s| s.kind.clone()) {
            self.ed_mut().img_dlg = Some(ImgDlg::new(self.ed().sel.first().copied(), im));
        }
    }

    fn open_trace(&mut self) {
        let Some(id) = self.ed().sel.first().copied() else { return };
        if let Some(Kind::Image(im)) = self.scene().shape(id).map(|s| s.kind.clone()) {
            self.ed_mut().trace_dlg = Some(TraceDlg { shape: id, params: TraceParams::default(), delete_original: false, thumb: im.downscaled(600), tex: None, preview: vec![], shown: None });
        }
    }

    fn image_dialogs(&mut self, ctx: &egui::Context) {
        self.import_dialog(ctx);
        self.trace_dialog(ctx);
        super::EditorShapeOps::shape_op_windows(self, ctx);
    }

    fn import_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.ed_mut().img_dlg.take() else { return };
        let units = self.units();
        // Refresh the preview texture when something changed.
        let key = (d.adj, d.negative);
        if d.shown != Some(key) {
            let mut pv = d.thumb.adjusted(&d.adj);
            pv.invert = d.negative;
            let px: Vec<u8> = if d.negative { pv.gray.iter().map(|g| 255 - g).collect() } else { pv.gray.clone() };
            let img = egui::ColorImage::from_gray([pv.px_w as usize, pv.px_h as usize], &px);
            d.aspect = pv.px_h as f64 / pv.px_w.max(1) as f64;
            d.tex = Some(ctx.load_texture("img-dialog", img, egui::TextureOptions::LINEAR));
            d.shown = Some(key);
        }
        let mut open = true;
        let (mut accept, mut cancel) = (false, false);
        let title = if d.target.is_some() { tr("Adjust image") } else { tr("Import image") };
        popup(ctx, "w_image_ui_6", &title, [340.0, 220.0], &mut open, |ui| {
            ui.horizontal_top(|ui| {
                // Preview on a checkerboard-free white sheet, as it will look on the material.
                let (rect, _) = ui.allocate_exact_size(egui::vec2(PREVIEW_PX as f32 * 0.78, PREVIEW_PX as f32 * 0.78), egui::Sense::hover());
                ui.painter().rect_filled(rect, 4.0, Color32::from_gray(0x80));
                if let Some(t) = &d.tex {
                    let sz = t.size_vec2();
                    let k = (rect.width() / sz.x).min(rect.height() / sz.y);
                    let r = egui::Rect::from_center_size(rect.center(), sz * k);
                    ui.painter().image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                }
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    ui.label(RichText::new(&d.base.name).color(theme::text_dim()));
                    ui.add_space(4.0);
                    ui.label(RichText::new(tr("Rotate and flip")).strong());
                    ui.horizontal(|ui| {
                        if ui.add(egui::Button::image_and_text(icons::slot(Some("rotate-ccw"), theme::text()), "90°")).clicked() {
                            d.adj.quarter_turns = (d.adj.quarter_turns + 3) % 4;
                        }
                        if ui.add(egui::Button::image_and_text(icons::slot(Some("rotate-cw"), theme::text()), "90°")).clicked() {
                            d.adj.quarter_turns = (d.adj.quarter_turns + 1) % 4;
                        }
                        ui.checkbox(&mut d.adj.flip_h, tr("Flip H"));
                        ui.checkbox(&mut d.adj.flip_v, tr("Flip V"));
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("Angle"));
                        ui.add(egui::Slider::new(&mut d.adj.angle_deg, -45.0..=45.0).suffix("°"));
                    });
                    ui.add_space(4.0);
                    ui.label(RichText::new(tr("Tone")).strong());
                    egui::Grid::new("tone").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        ui.label(tr("Brightness"));
                        ui.add(egui::Slider::new(&mut d.adj.brightness, -1.0..=1.0));
                        ui.end_row();
                        ui.label(tr("Contrast"));
                        ui.add(egui::Slider::new(&mut d.adj.contrast, -0.95..=0.95));
                        ui.end_row();
                        ui.label(tr("Gamma"));
                        ui.add(egui::Slider::new(&mut d.adj.gamma, 0.2..=3.0).logarithmic(true));
                        ui.end_row();
                    });
                    ui.checkbox(&mut d.adj.auto_levels, tr("Auto levels"));
                    ui.checkbox(&mut d.negative, tr("Negative image")).on_hover_text(tr("Engrave the light parts instead of the dark parts"));
                    ui.add_space(4.0);
                    ui.label(RichText::new(tr("Size")).strong());
                    ui.horizontal(|ui| {
                        ui.label(tr("Width"));
                        drag_len(ui, units, &mut d.width, 0.5, Some((1.0, 5000.0)));
                        ui.label(format!("× {}", fmt_len(units, d.width * d.aspect, 1)));
                    });
                    ui.label(RichText::new(trf("{} × {} px", &[&d.base.px_w, &d.base.px_h])).color(theme::text_dim()));
                    if ui.button(tr("Reset")).clicked() {
                        d.adj = Adjust::default();
                        d.negative = false;
                    }
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                accept = if d.target.is_some() { ui.button(tr("Apply")) } else { import_button(ui, &tr("Import")) }.clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
        if accept {
            let im = d.result();
            match d.target {
                Some(id) => {
                    self.checkpoint();
                    if let Some(s) = self.scene_mut().unlocked_mut(id) {
                        s.kind = Kind::Image(im);
                    }
                }
                None => {
                    let (bw, bh) = self.work_size();
                    self.checkpoint();
                    let (w, h) = (im.w, im.h);
                    let name = im.name.clone();
                    let layer = self.ed().active_layer;
                    let id = self.scene_mut().add(layer, Kind::Image(im), Xf::translate(((bw - w) / 2.0).max(0.0), ((bh - h) / 2.0).max(0.0)));
                    self.ed_mut().sel = vec![id];
                    self.say(trf("Imported image {}", &[&name]));
                }
            }
            return;
        }
        if cancel || !open {
            return;
        }
        self.ed_mut().img_dlg = Some(d);
    }

    fn trace_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.ed_mut().trace_dlg.take() else { return };
        if self.scene().shape(d.shape).is_none() {
            return;
        }
        if d.shown != Some(d.params) {
            let flat: Vec<Vec<Pt>> = trace(&d.thumb, &d.params).iter().map(|c| c.flatten(0.2).pts).collect();
            d.preview = flat;
            let px: Vec<u8> = if d.thumb.invert { d.thumb.gray.iter().map(|g| 255 - g).collect() } else { d.thumb.gray.clone() };
            let img = egui::ColorImage::from_gray([d.thumb.px_w as usize, d.thumb.px_h as usize], &px);
            d.tex = Some(ctx.load_texture("trace-dialog", img, egui::TextureOptions::LINEAR));
            d.shown = Some(d.params);
        }
        let mut open = true;
        let (mut accept, mut cancel) = (false, false);
        popup(ctx, "w_image_ui_7", &tr("Trace image"), [340.0, 220.0], &mut open, |ui| {
            ui.horizontal_top(|ui| {
                let sz = egui::vec2(430.0, 430.0);
                let (rect, _) = ui.allocate_exact_size(sz, egui::Sense::hover());
                ui.painter().rect_filled(rect, 4.0, Color32::WHITE);
                if let Some(t) = &d.tex {
                    let ts = t.size_vec2();
                    let k = (rect.width() / ts.x).min(rect.height() / ts.y);
                    let r = egui::Rect::from_center_size(rect.center(), ts * k);
                    ui.painter().image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::from_white_alpha(70));
                    let (sx, sy) = (r.width() / d.thumb.w as f32, r.height() / d.thumb.h as f32);
                    for poly in &d.preview {
                        let pts: Vec<egui::Pos2> = poly.iter().map(|p| r.min + egui::vec2(p.x as f32 * sx, p.y as f32 * sy)).collect();
                        if pts.len() > 1 {
                            ui.painter().add(egui::Shape::closed_line(pts, Stroke::new(1.3, theme::accent())));
                        }
                    }
                }
                ui.vertical(|ui| {
                    ui.set_width(250.0);
                    egui::Grid::new("trace_params").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
                        let mut th = d.params.threshold as i32;
                        ui.label(tr("Threshold"));
                        ui.add(egui::Slider::new(&mut th, 1..=254));
                        d.params.threshold = th as u8;
                        ui.end_row();
                        ui.label(tr("Ignore small areas"));
                        ui.add(egui::DragValue::new(&mut d.params.min_area).range(0.0..=5000.0).suffix(" px²"));
                        ui.end_row();
                        ui.label(tr("Simplify"));
                        ui.add(egui::Slider::new(&mut d.params.tolerance, 0.0..=5.0));
                        ui.end_row();
                    });
                    ui.checkbox(&mut d.params.smooth, tr("Smooth curves"));
                    ui.checkbox(&mut d.params.invert, tr("Trace the light areas"));
                    ui.checkbox(&mut d.delete_original, tr("Delete the image afterwards"));
                    ui.label(RichText::new(trf("{} outlines", &[&d.preview.len()])).color(theme::text_dim()));
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                accept = ui.button(tr("Trace")).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
        if accept {
            self.apply_trace(&d);
            return;
        }
        if cancel || !open {
            return;
        }
        self.ed_mut().trace_dlg = Some(d);
    }

    fn apply_trace(&mut self, d: &TraceDlg) {
        let Some(shape) = self.scene().shape(d.shape).cloned() else { return };
        let Kind::Image(im) = &shape.kind else { return };
        let contours: Vec<_> = trace(im, &d.params).iter().map(|c| c.transformed(&shape.xf)).collect();
        if contours.is_empty() {
            self.say(tr("The result is empty.").to_string().to_string());
            return;
        }
        self.checkpoint();
        if d.delete_original {
            self.scene_mut().shapes.retain(|s| s.id != d.shape || s.locked);
        }
        let n = contours.len();
        let id = self.scene_mut().add(shape.layer, Kind::Bezier(contours), Xf::IDENTITY);
        self.ed_mut().sel = vec![id];
        self.say(trf("Traced {} outlines", &[&n]));
    }
}

impl<T: EditorHost> EditorImages for T {}
