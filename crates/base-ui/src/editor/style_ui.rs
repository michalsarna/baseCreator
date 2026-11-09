//! The style trace window: a picture drawn again as lines in a style (base_core::style_trace), with a live preview. The lines
//! are made in a thread of their own, so a slow style never stops the window; what the preview shows is what is put on the
//! drawing.
use super::image_ui::{column, PictureView};
use super::EditorHost;
use crate::i18n::{tr, trf};
use crate::theme;
use base_core::image::ImageData;
use base_core::scene::Kind;
use base_core::style_trace::{style_trace, Style, StyleOut, StyleParams};
use egui::{Color32, RichText, Stroke};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{mpsc, Arc};

/// The width of the column with the settings.
const CONTROLS_W: f32 = 300.0;

/// A result above this many points gets a warning: the drawing gets slow to edit.
const MANY_POINTS: usize = 300_000;

pub struct StyleDlg {
    pub shape: u64,
    pub params: StyleParams,
    pub delete_original: bool,
    /// The layer the lines go on.
    pub layer: usize,
    picture: Arc<ImageData>,
    thumb: ImageData,
    tex: Option<egui::TextureHandle>,
    view: PictureView,
    /// The newest request; a thread whose number is older stops.
    generation: Arc<AtomicU64>,
    /// The request being worked on.
    job: Option<(StyleParams, u64, mpsc::Receiver<StyleOut>)>,
    /// The last finished result and what it was made with.
    result: Option<(StyleParams, Arc<StyleOut>)>,
}

impl StyleDlg {
    pub fn new(shape: u64, im: &ImageData, layer: usize) -> StyleDlg {
        StyleDlg {
            shape,
            params: StyleParams::default().with_style(Style::default()),
            delete_original: false,
            layer,
            picture: Arc::new(im.clone()),
            thumb: im.downscaled(1200),
            tex: None,
            view: PictureView::default(),
            generation: Arc::new(AtomicU64::new(0)),
            job: None,
            result: None,
        }
    }

    /// The result for the settings now chosen, once it is ready.
    pub fn ready(&self) -> Option<&Arc<StyleOut>> {
        self.result.as_ref().filter(|(p, _)| *p == self.params).map(|(_, o)| o)
    }

    /// Start the work for the current settings when nothing works on them yet, and take a finished result.
    fn update(&mut self, ctx: &egui::Context) {
        if let Some((p, id, rx)) = &self.job {
            match rx.try_recv() {
                Ok(out) => {
                    if *id == self.generation.load(Ordering::SeqCst) {
                        self.result = Some((*p, Arc::new(out)));
                    }
                    self.job = None;
                }
                Err(mpsc::TryRecvError::Disconnected) => self.job = None,
                Err(mpsc::TryRecvError::Empty) => {}
            }
        }
        let wanted = self.params;
        let done = self.result.as_ref().is_some_and(|(p, _)| *p == wanted);
        let working = self.job.as_ref().is_some_and(|(p, _, _)| *p == wanted);
        if !done && !working {
            let id = self.generation.fetch_add(1, Ordering::SeqCst) + 1;
            let (tx, rx) = mpsc::channel();
            let (picture, generation, ctx2) = (self.picture.clone(), self.generation.clone(), ctx.clone());
            std::thread::spawn(move || {
                let stop = || generation.load(Ordering::SeqCst) != id;
                let out = style_trace(&picture, &wanted, &stop);
                if !stop() {
                    let _ = tx.send(out);
                    ctx2.request_repaint();
                }
            });
            self.job = Some((wanted, id, rx));
        }
        if self.job.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(60));
        }
    }

    /// The work of a closed window stops.
    fn stop(&self) {
        self.generation.fetch_add(1, Ordering::SeqCst);
    }
}

/// The style trace window. Implemented for every `EditorHost`.
pub trait EditorStyleTrace: EditorHost {
    fn open_style_trace(&mut self) {
        let Some(id) = self.ed().sel.first().copied() else { return };
        let Some(shape) = self.scene().shape(id).cloned() else { return };
        if let Kind::Image(im) = &shape.kind {
            let layer = self.line_layer(shape.layer);
            self.ed_mut().style_dlg = Some(StyleDlg::new(id, im, layer));
        }
    }

    fn style_trace_window(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.ed_mut().style_dlg.take() else { return };
        if self.scene().shape(d.shape).is_none() {
            d.stop();
            return;
        }
        d.update(ctx);
        if d.tex.is_none() {
            let px: Vec<u8> = if d.thumb.invert { d.thumb.gray.iter().map(|g| 255 - g).collect() } else { d.thumb.gray.clone() };
            let img = egui::ColorImage::from_gray([d.thumb.px_w as usize, d.thumb.px_h as usize], &px);
            d.tex = Some(ctx.load_texture("style-trace", img, egui::TextureOptions::LINEAR));
        }
        let mut open = true;
        let (mut accept, mut cancel) = (false, false);
        let layers: Vec<(usize, String, Color32)> = (0..self.layer_count()).map(|l| (l, self.layer_name(l), self.layer_color(l))).collect();
        crate::window::popup(ctx, "w_style_trace", tr("Style trace"), [1000.0, 660.0], &mut open, |ui| {
            let page_h = crate::window::height_above(ui, "style_dlg", crate::window::fill_size(ui).y);
            ui.allocate_ui(egui::vec2(ui.available_width(), page_h), |ui| {
                ui.horizontal_top(|ui| {
                    ui.set_max_height(page_h);
                    let gap = ui.spacing().item_spacing.x;
                    let pw = (ui.available_width() - CONTROLS_W - gap).max(160.0);
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(pw, page_h.max(160.0)), egui::Sense::click_and_drag());
                    preview(ui, &mut d, rect, &resp);
                    let (cols, _) = ui.allocate_exact_size(egui::vec2(CONTROLS_W, rect.height()), egui::Sense::hover());
                    column(ui, cols, |ui| {
                        egui::ScrollArea::vertical().id_salt("style_controls").auto_shrink([false, false]).show(ui, |ui| controls(ui, &mut d, &layers));
                    });
                });
            });
            crate::window::footer(ui, "style_dlg", |ui| {
                ui.horizontal(|ui| {
                    let ready = d.ready().is_some_and(|o| !o.lines.is_empty());
                    accept = ui.add_enabled(ready, egui::Button::new(RichText::new(tr("Trace")).color(Color32::WHITE).strong()).fill(Color32::from_rgb(0x2e, 0xa0, 0x4f))).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                    ui.label(RichText::new(summary(&d)).color(theme::text_dim()));
                });
            });
        });
        if accept {
            if let Some(out) = d.ready().cloned() {
                self.apply_style_trace(&d, &out);
            }
            d.stop();
            return;
        }
        if cancel || !open {
            d.stop();
            return;
        }
        self.ed_mut().style_dlg = Some(d);
    }

    fn apply_style_trace(&mut self, d: &StyleDlg, out: &StyleOut) {
        let Some(shape) = self.scene().shape(d.shape).cloned() else { return };
        if out.lines.is_empty() {
            self.say(tr("The result is empty.").to_string());
            return;
        }
        // Only a layer with nothing on it is made to draw lines: a layer the drawing already uses keeps how it cuts.
        let empty = !self.scene().shapes.iter().any(|s| s.layer == d.layer);
        self.checkpoint();
        if d.delete_original {
            self.scene_mut().shapes.retain(|s| s.id != d.shape || s.locked);
        }
        if empty {
            self.make_line_layer(d.layer);
        }
        // The lines are in the millimetres of the picture: they take its place and turn with it.
        let id = self.scene_mut().add(d.layer, Kind::Path(out.lines.clone()), shape.xf);
        self.ed_mut().sel = vec![id];
        self.say(trf("{} lines in the style {}", &[&out.lines.len(), &tr(d.params.style.label())]));
        self.touch();
    }
}

impl<T: EditorHost> EditorStyleTrace for T {}

fn summary(d: &StyleDlg) -> String {
    match d.ready() {
        None => tr("Working…").to_string(),
        Some(o) => {
            let pts: usize = o.lines.iter().map(|l| l.pts.len()).sum();
            let mut s = trf("{} lines, {} m of line", &[&o.lines.len(), &format!("{:.1}", o.length() / 1000.0)]);
            if o.truncated {
                s.push_str(" · ");
                s.push_str(tr("only a part: too many points"));
            } else if pts > MANY_POINTS {
                s.push_str(" · ");
                s.push_str(tr("many points: the drawing gets slow"));
            }
            s
        }
    }
}

fn preview(ui: &mut egui::Ui, d: &mut StyleDlg, rect: egui::Rect, resp: &egui::Response) {
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, Color32::WHITE);
    d.view.input(ui, resp, rect);
    if let Some(t) = &d.tex {
        let r = d.view.place(rect, t.size_vec2());
        painter.image(t.id(), r, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::from_white_alpha(40));
        // The last result, also while a newer one is worked on (greyed then).
        if let Some((p, out)) = &d.result {
            let ink = if *p == d.params { Color32::from_rgb(20, 20, 30) } else { Color32::from_gray(150) };
            let (sx, sy) = (r.width() / d.picture.w as f32, r.height() / d.picture.h as f32);
            let stroke = Stroke::new(1.0, ink);
            for l in &out.lines {
                let pts: Vec<egui::Pos2> = l.pts.iter().map(|q| r.min + egui::vec2(q.x as f32 * sx, q.y as f32 * sy)).collect();
                painter.add(egui::Shape::line(pts, stroke));
            }
        }
    }
    d.view.bar(ui, rect);
}

fn controls(ui: &mut egui::Ui, d: &mut StyleDlg, layers: &[(usize, String, Color32)]) {
    let p = &mut d.params;
    egui::Grid::new("style_params").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Style"));
        let mut style = p.style;
        egui::ComboBox::from_id_salt("style_kind").width(170.0).selected_text(tr(style.label())).height(520.0).show_ui(ui, |ui| {
            for s in Style::ALL {
                ui.selectable_value(&mut style, s, tr(s.label())).on_hover_text(tr(s.hint()));
            }
        });
        if style != p.style {
            *p = p.with_style(style);
        }
        ui.end_row();
    });
    ui.label(RichText::new(tr(p.style.hint())).color(theme::text_dim()));
    ui.add_space(4.0);
    egui::Grid::new("style_knobs").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
        let variants = p.style.variants();
        if !variants.is_empty() {
            ui.label(tr("Kind"));
            let mut v = p.variant.min(variants.len() as u8 - 1);
            egui::ComboBox::from_id_salt("style_variant").width(170.0).selected_text(tr(variants[v as usize])).show_ui(ui, |ui| {
                for (i, name) in variants.iter().enumerate() {
                    ui.selectable_value(&mut v, i as u8, tr(name));
                }
            });
            p.variant = v;
            ui.end_row();
        }
        for k in p.style.knobs() {
            ui.label(tr(k.label));
            let mut v = p.get(k.knob);
            let mut s = egui::Slider::new(&mut v, k.min..=k.max).fixed_decimals(k.decimals).suffix(k.unit);
            if k.max / k.min.abs().max(1e-3) > 200.0 && k.min > 0.0 {
                s = s.logarithmic(true);
            }
            if k.decimals == 0 {
                s = s.step_by(1.0);
            }
            ui.add(s);
            p.set(k.knob, v);
            ui.end_row();
        }
        if p.style.random() {
            ui.label(tr("Random pattern"));
            ui.horizontal(|ui| {
                let mut seed = p.seed;
                ui.add(egui::DragValue::new(&mut seed).range(1..=99_999));
                if ui.button(tr("Another")).clicked() {
                    seed = seed % 99_999 + 1;
                }
                p.seed = seed;
            });
            ui.end_row();
        }
    });
    ui.separator();
    ui.label(RichText::new(tr("Tone")).strong());
    egui::Grid::new("style_tone").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Black point"));
        ui.add(egui::Slider::new(&mut p.black, 0.0..=0.95).fixed_decimals(2));
        ui.end_row();
        ui.label(tr("White point"));
        ui.add(egui::Slider::new(&mut p.white, 0.05..=1.0).fixed_decimals(2));
        ui.end_row();
        ui.label(tr("Gamma"));
        ui.add(egui::Slider::new(&mut p.gamma, 0.2..=5.0).logarithmic(true).fixed_decimals(2));
        ui.end_row();
    });
    if p.white <= p.black + 0.01 {
        p.white = (p.black + 0.05).min(1.0);
        p.black = p.black.min(p.white - 0.05);
    }
    ui.checkbox(&mut p.invert, tr("Trace the light areas"));
    ui.separator();
    egui::Grid::new("style_out").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Put on the layer"));
        let cur = layers.iter().find(|l| l.0 == d.layer).map(|l| l.1.clone()).unwrap_or_default();
        egui::ComboBox::from_id_salt("style_layer").width(170.0).selected_text(cur).height(320.0).show_ui(ui, |ui| {
            for (l, name, color) in layers {
                ui.horizontal(|ui| {
                    let (r, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter().rect_filled(r, 2.0, *color);
                    ui.selectable_value(&mut d.layer, *l, name);
                });
            }
        });
        ui.end_row();
    });
    ui.checkbox(&mut d.delete_original, tr("Delete the image afterwards"));
}
