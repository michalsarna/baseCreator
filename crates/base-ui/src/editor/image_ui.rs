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
/// The controls of the trace dialog (sliders with their value boxes) are wider.
const TRACE_CONTROLS_W: f32 = 300.0;

/// How a picture is seen in the frame of a dialog: the zoom (1 = the whole picture in the frame) and where its middle lies.
/// Scrolling zooms about the pointer, dragging moves the picture, a double click shows it whole again; the buttons do the same
/// for a mouse without a wheel.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PictureView {
    pub zoom: f32,
    pub pan: egui::Vec2,
}

impl Default for PictureView {
    fn default() -> Self {
        PictureView { zoom: 1.0, pan: egui::Vec2::ZERO }
    }
}

impl PictureView {
    const MAX_ZOOM: f32 = 40.0;

    /// The wheel, the drag and the double click on the frame `rect` (`resp` is its response, sensing clicks and drags).
    pub fn input(&mut self, ui: &egui::Ui, resp: &egui::Response, rect: egui::Rect) {
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if let (Some(m), true) = (resp.hover_pos(), scroll != 0.0) {
                let z = (self.zoom * (scroll * 0.0025).exp()).clamp(1.0, Self::MAX_ZOOM);
                let k = z / self.zoom;
                // The point under the pointer stays where it is.
                self.pan = (m - rect.center()) * (1.0 - k) + self.pan * k;
                self.zoom = z;
            }
        }
        if resp.dragged() {
            self.pan += resp.drag_delta();
        }
        if resp.double_clicked() {
            *self = PictureView::default();
        }
    }

    /// Where a picture of `size` pixels lies in the frame `rect`. It may not leave the frame altogether.
    pub fn place(&mut self, rect: egui::Rect, size: egui::Vec2) -> egui::Rect {
        let k = (rect.width() / size.x).min(rect.height() / size.y) * self.zoom;
        let half = size * k / 2.0;
        self.pan.x = self.pan.x.clamp(-(half.x + rect.width() / 2.0 - 20.0), half.x + rect.width() / 2.0 - 20.0);
        self.pan.y = self.pan.y.clamp(-(half.y + rect.height() / 2.0 - 20.0), half.y + rect.height() / 2.0 - 20.0);
        egui::Rect::from_center_size(rect.center() + self.pan, size * k)
    }

    /// Zoom by `k` about the middle of the frame.
    pub fn zoom_by(&mut self, k: f32) {
        let z = (self.zoom * k).clamp(1.0, Self::MAX_ZOOM);
        self.pan *= z / self.zoom;
        self.zoom = z;
    }

    /// The zoom bar in the lower left corner of the frame `rect`.
    pub fn bar(&mut self, ui: &mut egui::Ui, rect: egui::Rect) {
        match crate::zoom_bar::overlay(ui, rect, self.zoom, &tr("Scroll to zoom, drag to move, double click to see the whole picture")) {
            Some(crate::zoom_bar::ZoomCmd::In) => self.zoom_by(crate::zoom_bar::STEP),
            Some(crate::zoom_bar::ZoomCmd::Out) => self.zoom_by(1.0 / crate::zoom_bar::STEP),
            Some(crate::zoom_bar::ZoomCmd::Whole) => *self = PictureView::default(),
            None => {}
        }
    }
}

/// A column of a dialog in a fixed place: what is in it is cut at its edges and never makes the window larger.
pub(super) fn column(ui: &mut egui::Ui, r: egui::Rect, add: impl FnOnce(&mut egui::Ui)) {
    let mut c = ui.new_child(egui::UiBuilder::new().max_rect(r).layout(egui::Layout::top_down(egui::Align::Min)));
    c.set_clip_rect(r.intersect(ui.clip_rect()));
    c.set_width(r.width());
    add(&mut c);
}

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
    /// How the picture is seen in the frame.
    pub view: PictureView,
}

impl ImgDlg {
    pub fn new(target: Option<u64>, base: ImageData) -> ImgDlg {
        let thumb = base.downscaled(PREVIEW_PX);
        ImgDlg { target, adj: Adjust::default(), negative: base.invert, width: base.w, hist_in: thumb.histogram(), hist_out: [0; 256], base, thumb, tex: None, shown: None, aspect: 1.0, view: PictureView::default() }
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
    ui.horizontal_wrapped(|ui| {
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
    ui.checkbox(&mut d.negative, tr("Negative image")).on_hover_text(tr("Swap the light and the dark parts of the picture"));
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
    /// How the picture is seen in the frame.
    view: PictureView,
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

    /// Read an SVG, PDF or Illustrator file into the drawing. A file with Illustrator layers first asks how to put the paths on the
    /// layers of the program.
    fn import_drawing_path(&mut self, p: std::path::PathBuf) {
        match std::fs::read(&p).map_err(|e| e.to_string()).and_then(|d| base_core::svg::read_drawing(&d)) {
            Ok(outlines) if !base_core::svg::layer_names(&outlines).is_empty() => self.ed_mut().drawing_choice = Some((p, outlines)),
            Ok(outlines) => self.place_drawing(&p, outlines, false),
            Err(e) => self.say(trf("Import failed: {}", &[&e])),
        }
    }

    /// Add read outlines to the drawing, selected and undoable.
    fn place_drawing(&mut self, p: &std::path::Path, outlines: Vec<base_core::svg::Outline>, by_layers: bool) {
        self.checkpoint();
        let ids = self.scene_mut().append(base_core::svg::into_scene(outlines, by_layers), base_core::layers::LAYER_COUNT - 1);
        self.say(trf("Imported {} paths from {}", &[&ids.len(), &p.display()]));
        let ed = self.ed_mut();
        ed.sel = ids;
        ed.node_sel.clear();
        ed.view.need_fit = true;
        self.touch();
    }

    /// The question after reading a file with Illustrator layers: by colour or by layers.
    fn drawing_choice_window(&mut self, ctx: &egui::Context) {
        let Some((p, outlines)) = self.ed_mut().drawing_choice.take() else { return };
        let n = base_core::svg::layer_names(&outlines).len();
        let mut open = true;
        let mut pick: Option<bool> = None;
        let mut cancel = false;
        popup(ctx, "drawing_layers", &tr("Illustrator layers"), [480.0, 170.0], &mut open, |ui| {
            ui.label(trf("The file has {} Illustrator layers. Put the paths on the layers of the program by:", &[&n]));
            ui.label(egui::RichText::new(tr("The first Illustrator layer goes on layer 1, the second on layer 2, and so on; the rest go on the last layer.")).small().weak());
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                if ui.button(tr("Colour")).clicked() {
                    pick = Some(false);
                }
                if ui.button(tr("Illustrator layers")).clicked() {
                    pick = Some(true);
                }
                if ui.button(tr("Cancel")).clicked() {
                    cancel = true;
                }
            });
        });
        match pick {
            Some(by_layers) => self.place_drawing(&p, outlines, by_layers),
            None if open && !cancel => self.ed_mut().drawing_choice = Some((p, outlines)),
            None => {}
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
            self.ed_mut().trace_dlg = Some(TraceDlg { shape: id, params: TraceParams::default(), delete_original: false, thumb: im.downscaled(1200), tex: None, preview: vec![], shown: None, view: PictureView::default() });
        }
    }

    fn image_dialogs(&mut self, ctx: &egui::Context) {
        self.import_dialog(ctx);
        self.drawing_choice_window(ctx);
        self.trace_dialog(ctx);
        super::EditorShapeOps::shape_op_windows(self, ctx);
        super::EditorDesign::design_windows(self, ctx);
        super::EditorBackground::background_window(self, ctx);
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
            // The page is cut into three fixed parts. Nothing in a column can make the window wider: a text longer in another
            // language than the column once widened the window a little every frame, up to the width of the screen.
            let gap = ui.spacing().item_spacing.x;
            let (page, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), page_h.max(160.0)), egui::Sense::hover());
            // The picture takes what the two columns leave, as large as the page.
            let pw = (page.width() - CONTROLS_W - HIST_W - 2.0 * gap).max(160.0);
            let rect = egui::Rect::from_min_size(page.min, egui::vec2(pw, page.height()));
            let resp = ui.interact(rect, ui.scope_id().with("image_dlg_picture"), egui::Sense::click_and_drag());
            d.view.input(ui, &resp, rect);
            let painter = ui.painter_at(rect);
            painter.rect_filled(rect, 4.0, Color32::from_gray(0x80));
            if let Some(t) = &d.tex {
                let r = d.view.place(rect, t.size_vec2());
                painter.image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
            }
            d.view.bar(ui, rect);
            let controls = egui::Rect::from_min_size(egui::pos2(rect.right() + gap, page.top()), egui::vec2(CONTROLS_W, page.height()));
            column(ui, controls, |ui| {
                egui::ScrollArea::vertical().id_salt("image_dlg_controls").auto_shrink([false, false]).max_height(page.height()).show(ui, |ui| image_controls(ui, &mut d, units));
            });
            let hist = egui::Rect::from_min_size(egui::pos2(controls.right() + gap, page.top()), egui::vec2(HIST_W, page.height()));
            column(ui, hist, |ui| levels_panel(ui, &mut d));
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
                    let pw = (ui.available_width() - TRACE_CONTROLS_W - gap).max(160.0);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(pw, page_h.max(160.0)), egui::Sense::click_and_drag());
                    let painter = ui.painter_at(rect);
                    painter.rect_filled(rect, 4.0, Color32::WHITE);
                    d.view.input(ui, &resp, rect);
                    if let Some(t) = &d.tex {
                        let r = d.view.place(rect, t.size_vec2());
                        painter.image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::from_white_alpha(70));
                        let (sx, sy) = (r.width() / d.thumb.w as f32, r.height() / d.thumb.h as f32);
                        for poly in &d.preview {
                            let pts: Vec<egui::Pos2> = poly.iter().map(|p| r.min + egui::vec2(p.x as f32 * sx, p.y as f32 * sy)).collect();
                            if pts.len() > 1 {
                                painter.add(egui::Shape::closed_line(pts, Stroke::new(1.3, theme::accent())));
                            }
                        }
                    }
                    d.view.bar(ui, rect);
                    // A fixed column, so that a long text cannot widen the window frame after frame.
                    let (cols, _) = ui.allocate_exact_size(egui::vec2(TRACE_CONTROLS_W, rect.height()), egui::Sense::hover());
                    column(ui, cols, |ui| {
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
            });
            crate::window::footer(ui, "trace_dlg", |ui| {
                ui.horizontal(|ui| {
                    // The button that does the work is green.
                    accept = ui.add(egui::Button::new(RichText::new(tr("Trace")).color(Color32::WHITE).strong()).fill(Color32::from_rgb(0x2e, 0xa0, 0x4f))).clicked();
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
