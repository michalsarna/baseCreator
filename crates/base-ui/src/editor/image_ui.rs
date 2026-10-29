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

const PREVIEW_PX: u32 = 800;
/// Width of the column with the controls and of the one with the histogram.
const CONTROLS_W: f32 = 250.0;
const HIST_W: f32 = 240.0;

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
    /// The greys of the picture as it was read, and as it is with the adjustments (for the histogram).
    hist_in: [u32; 256],
    hist_out: [u32; 256],
    /// Height / width of the adjusted picture.
    aspect: f64,
}

impl ImgDlg {
    pub fn new(target: Option<u64>, base: ImageData) -> ImgDlg {
        let thumb = base.downscaled(PREVIEW_PX);
        ImgDlg { target, adj: Adjust::default(), negative: base.invert, width: base.w, hist_in: thumb.histogram(), hist_out: [0; 256], base, thumb, tex: None, shown: None, aspect: 1.0 }
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

/// The sliders and options of the dialog (everything but the histogram).
fn image_controls(ui: &mut egui::Ui, d: &mut ImgDlg, units: base_core::units::Units) {
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
}

/// The histogram of the greys, with the two sliders of the range under it.
fn levels_panel(ui: &mut egui::Ui, d: &mut ImgDlg) {
    ui.label(RichText::new(tr("Levels")).strong());
    let (rect, _) = ui.allocate_exact_size(egui::vec2(HIST_W, 150.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0, theme::panel_dark());
    // The square root keeps a tall peak (white background) from flattening the rest.
    let top = d.hist_in.iter().chain(d.hist_out.iter()).map(|c| (*c as f32).sqrt()).fold(1.0, f32::max);
    let x_of = |v: f32| rect.left() + v.clamp(0.0, 1.0) * rect.width();
    let col = rect.width() / 256.0;
    for (i, c) in d.hist_in.iter().enumerate() {
        let h = (*c as f32).sqrt() / top * (rect.height() - 4.0);
        let x = rect.left() + i as f32 * col;
        p.rect_filled(egui::Rect::from_min_max(egui::pos2(x, rect.bottom() - h), egui::pos2(x + col.max(2.0), rect.bottom())), 0.0, Color32::from_gray(0x70));
    }
    // What is made of it (with the adjustments) as a line over the bars.
    let pts: Vec<egui::Pos2> = d.hist_out.iter().enumerate().map(|(i, c)| egui::pos2(rect.left() + (i as f32 + 0.5) * col, rect.bottom() - (*c as f32).sqrt() / top * (rect.height() - 4.0))).collect();
    p.add(egui::Shape::line(pts, Stroke::new(1.2, theme::accent())));
    // The grey outside the range is dimmed, the two edges are lines.
    let (xb, xw) = (x_of(d.adj.black), x_of(d.adj.white));
    let dim = Color32::from_black_alpha(140);
    p.rect_filled(egui::Rect::from_min_max(rect.min, egui::pos2(xb, rect.bottom())), 0.0, dim);
    p.rect_filled(egui::Rect::from_min_max(egui::pos2(xw, rect.top()), rect.max), 0.0, dim);
    for x in [xb, xw] {
        p.line_segment([egui::pos2(x, rect.top()), egui::pos2(x, rect.bottom())], Stroke::new(1.5, Color32::WHITE));
    }
    ui.spacing_mut().slider_width = HIST_W - 70.0;
    let (mut black, mut white) = (d.adj.black * 255.0, d.adj.white * 255.0);
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut black, 0.0..=254.0).integer()).on_hover_text(tr("Everything darker than this turns black"));
    });
    ui.horizontal(|ui| {
        ui.add(egui::Slider::new(&mut white, 1.0..=255.0).integer()).on_hover_text(tr("Everything lighter than this turns white"));
    });
    // The two never cross: the one that was not touched gives way.
    if (black / 255.0 - d.adj.black).abs() > 1e-6 {
        black = black.min(white - 1.0);
    } else if (white / 255.0 - d.adj.white).abs() > 1e-6 {
        white = white.max(black + 1.0);
    }
    d.adj.black = black / 255.0;
    d.adj.white = white / 255.0;
}

pub struct TraceDlg {
    pub shape: u64,
    pub params: TraceParams,
    pub delete_original: bool,
    thumb: ImageData,
    tex: Option<egui::TextureHandle>,
    preview: Vec<Vec<Pt>>,
    shown: Option<TraceParams>,
    /// The view of the picture: the zoom (1 = the whole picture in the frame) and where the middle of the picture lies.
    zoom: f32,
    pan: egui::Vec2,
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
            self.ed_mut().trace_dlg = Some(TraceDlg { shape: id, params: TraceParams::default(), delete_original: false, thumb: im.downscaled(1200), tex: None, preview: vec![], shown: None, zoom: 1.0, pan: egui::Vec2::ZERO });
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
            d.hist_out = pv.histogram();
            d.tex = Some(ctx.load_texture("img-dialog", img, egui::TextureOptions::LINEAR));
            d.shown = Some(key);
        }
        let mut open = true;
        let (mut accept, mut cancel) = (false, false);
        let title = if d.target.is_some() { tr("Adjust image") } else { tr("Import image") };
        popup(ctx, "w_image_ui_6", &title, [1060.0, 640.0], &mut open, |ui| {
            // The page takes the height that is left above the buttons, so they stay at the bottom whatever the size.
            let page_h = crate::window::height_above(ui, "image_dlg", crate::window::fill_size(ui).y);
            ui.allocate_ui(egui::vec2(ui.available_width(), page_h), |ui| {
                ui.horizontal_top(|ui| {
                    let gap = ui.spacing().item_spacing.x;
                    // The picture takes what the two columns leave, as large as the page.
                    let pw = (ui.available_width() - CONTROLS_W - HIST_W - 2.0 * gap).max(160.0);
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(pw, page_h.max(160.0)), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 4.0, Color32::from_gray(0x80));
                    if let Some(t) = &d.tex {
                        let sz = t.size_vec2();
                        let k = (rect.width() / sz.x).min(rect.height() / sz.y);
                        let r = egui::Rect::from_center_size(rect.center(), sz * k);
                        ui.painter().image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
                    }
                    ui.vertical(|ui| {
                        ui.set_width(CONTROLS_W);
                        egui::ScrollArea::vertical().id_salt("image_dlg_controls").auto_shrink([false, false]).max_height(page_h).show(ui, |ui| image_controls(ui, &mut d, units));
                    });
                    ui.vertical(|ui| {
                        ui.set_width(HIST_W);
                        levels_panel(ui, &mut d);
                    });
                });
            });
            crate::window::footer(ui, "image_dlg", |ui| {
                ui.horizontal(|ui| {
                    accept = if d.target.is_some() { ui.button(tr("Apply")) } else { import_button(ui, &tr("Import")) }.clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
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
        popup(ctx, "w_image_ui_7", &tr("Trace image"), [940.0, 640.0], &mut open, |ui| {
            let page_h = crate::window::height_above(ui, "trace_dlg", crate::window::fill_size(ui).y);
            ui.allocate_ui(egui::vec2(ui.available_width(), page_h), |ui| {
                ui.horizontal_top(|ui| {
                    let gap = ui.spacing().item_spacing.x;
                    let pw = (ui.available_width() - CONTROLS_W - gap).max(160.0);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(pw, page_h.max(160.0)), egui::Sense::click_and_drag());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 4.0, Color32::WHITE);
                    // Scrolling zooms about the pointer, dragging moves the picture, a double click shows it whole again.
                    if resp.hovered() {
                        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                        if let (Some(m), true) = (resp.hover_pos(), scroll != 0.0) {
                            let z = (d.zoom * (scroll * 0.0025).exp()).clamp(1.0, 40.0);
                            let k = z / d.zoom;
                            // The point under the pointer stays where it is.
                            d.pan = (m - rect.center()) * (1.0 - k) + d.pan * k;
                            d.zoom = z;
                        }
                    }
                    if resp.dragged() {
                        d.pan += resp.drag_delta();
                    }
                    if resp.double_clicked() {
                        d.zoom = 1.0;
                        d.pan = egui::Vec2::ZERO;
                    }
                    if let Some(t) = &d.tex {
                        let ts = t.size_vec2();
                        let k = (rect.width() / ts.x).min(rect.height() / ts.y) * d.zoom;
                        // The picture may not leave the frame altogether.
                        let half = ts * k / 2.0;
                        d.pan.x = d.pan.x.clamp(-(half.x + rect.width() / 2.0 - 20.0), half.x + rect.width() / 2.0 - 20.0);
                        d.pan.y = d.pan.y.clamp(-(half.y + rect.height() / 2.0 - 20.0), half.y + rect.height() / 2.0 - 20.0);
                        let r = egui::Rect::from_center_size(rect.center() + d.pan, ts * k);
                        painter.image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::from_white_alpha(70));
                        let (sx, sy) = (r.width() / d.thumb.w as f32, r.height() / d.thumb.h as f32);
                        for poly in &d.preview {
                            let pts: Vec<egui::Pos2> = poly.iter().map(|p| r.min + egui::vec2(p.x as f32 * sx, p.y as f32 * sy)).collect();
                            if pts.len() > 1 {
                                painter.add(egui::Shape::closed_line(pts, Stroke::new(1.3, theme::accent())));
                            }
                        }
                        painter.text(rect.left_bottom() + egui::vec2(8.0, -6.0), egui::Align2::LEFT_BOTTOM, format!("×{:.1}", d.zoom), egui::FontId::proportional(12.0), Color32::from_gray(0x60));
                    }
                    ui.vertical(|ui| {
                        ui.set_width(CONTROLS_W);
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
                        ui.add_space(6.0);
                        ui.label(RichText::new(tr("Scroll to zoom, drag to move, double click to see the whole picture")).color(theme::text_dim()));
                    });
                });
            });
            crate::window::footer(ui, "trace_dlg", |ui| {
                ui.horizontal(|ui| {
                    accept = ui.button(tr("Trace")).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
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
