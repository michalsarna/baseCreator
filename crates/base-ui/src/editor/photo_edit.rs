//! The photo editor: simple tools on the pixels of a picture that is on the page. A brush and an eraser, a clone stamp and a fill
//! of an area, all in greys only (the pictures of the programs are grey). The scroll wheel zooms, the right or the middle button
//! moves the picture, and nothing changes in the drawing until Apply.
use super::image_ui::{column, PictureView};
use super::EditorHost;
use crate::i18n::tr;
use crate::theme;
use base_core::scene::Kind;
use egui::{Color32, RichText, Sense, Stroke};

/// The width of the column with the tools.
const TOOLS_W: f32 = 250.0;
/// How much the steps of undo may take together, in bytes (a large picture keeps fewer of them).
const UNDO_BYTES: usize = 240_000_000;

/// The tools of the editor.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PhotoTool {
    Brush,
    Eraser,
    Clone,
    Fill,
}

impl PhotoTool {
    pub const ALL: [PhotoTool; 4] = [PhotoTool::Brush, PhotoTool::Eraser, PhotoTool::Clone, PhotoTool::Fill];

    pub fn label(self) -> &'static str {
        match self {
            PhotoTool::Brush => "Brush",
            PhotoTool::Eraser => "Eraser",
            PhotoTool::Clone => "Clone",
            PhotoTool::Fill => "Fill",
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            PhotoTool::Brush => "brush",
            PhotoTool::Eraser => "eraser",
            PhotoTool::Clone => "stamp",
            PhotoTool::Fill => "paint-bucket",
        }
    }

    pub fn hint(self) -> &'static str {
        match self {
            PhotoTool::Brush => "Paints with the grey that is chosen",
            PhotoTool::Eraser => "Paints white, as the material that is not touched",
            PhotoTool::Clone => "Copies one place of the picture to another: Ctrl+click (Cmd+click) where to copy from, then paint",
            PhotoTool::Fill => "Fills the area of similar greys around the point that is clicked",
        }
    }
}

/// The pixels being edited, the steps of undo and what one stroke needs. Separate from the window, so it can be tested.
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    /// The greys as they are seen (a negative picture is turned back when it is applied).
    pub gray: Vec<u8>,
    undo: Vec<Vec<u8>>,
    redo: Vec<Vec<u8>>,
    /// The pixels as they were when the stroke began, and how much of each the stroke has covered so far (0..255): a stroke
    /// that passes a place twice does not make it darker, as with a real brush of that opacity.
    base: Vec<u8>,
    mask: Vec<u8>,
    in_stroke: bool,
    last: Option<(f32, f32)>,
    /// The part of the picture that changed since the texture was last made (x0, y0, x1, y1; the ends not included).
    pub dirty: Option<[usize; 4]>,
}

/// What one dab puts down.
#[derive(Clone, Copy, Debug)]
pub enum Paint {
    /// A grey.
    Shade(u8),
    /// The pixels `(dx, dy)` away, as they were when the stroke began.
    Clone(f32, f32),
}

/// The brush: its diameter in pixels of the picture, how hard its edge is and how much it covers (both 0..1).
#[derive(Clone, Copy, Debug)]
pub struct Brush {
    pub size: f32,
    pub hardness: f32,
    pub opacity: f32,
}

impl Canvas {
    pub fn new(w: usize, h: usize, gray: Vec<u8>) -> Canvas {
        Canvas { w, h, gray, undo: vec![], redo: vec![], base: vec![], mask: vec![], in_stroke: false, last: None, dirty: None }
    }

    fn keep_undo(&mut self) {
        let max = (UNDO_BYTES / self.gray.len().max(1)).clamp(2, 50);
        self.undo.push(self.gray.clone());
        if self.undo.len() > max {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }

    pub fn undo(&mut self) {
        if let Some(g) = self.undo.pop() {
            self.redo.push(std::mem::replace(&mut self.gray, g));
            self.dirty = Some([0, 0, self.w, self.h]);
        }
    }

    pub fn redo(&mut self) {
        if let Some(g) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.gray, g));
            self.dirty = Some([0, 0, self.w, self.h]);
        }
    }

    /// Has any stroke or fill been made (that is not undone)?
    pub fn changed(&self) -> bool {
        !self.undo.is_empty()
    }

    fn touch(&mut self, x0: usize, y0: usize, x1: usize, y1: usize) {
        self.dirty = Some(match self.dirty {
            Some([a, b, c, d]) => [a.min(x0), b.min(y0), c.max(x1), d.max(y1)],
            None => [x0, y0, x1, y1],
        });
    }

    pub fn in_stroke(&self) -> bool {
        self.in_stroke
    }

    /// Begin a stroke at `(x, y)` (in pixels of the picture) with its first dab.
    pub fn begin(&mut self, x: f32, y: f32, paint: Paint, b: Brush) {
        self.keep_undo();
        self.base = self.gray.clone();
        self.mask = vec![0; self.gray.len()];
        self.in_stroke = true;
        self.last = Some((x, y));
        self.dab(x, y, paint, b);
    }

    /// Go on with the stroke to `(x, y)`: dabs close enough to make a line.
    pub fn to(&mut self, x: f32, y: f32, paint: Paint, b: Brush) {
        let Some((lx, ly)) = self.last else { return self.begin(x, y, paint, b) };
        let len = (x - lx).hypot(y - ly);
        let step = (b.size * 0.2).max(0.5);
        let n = (len / step).ceil() as usize;
        for i in 1..=n {
            let t = i as f32 / n as f32;
            self.dab(lx + (x - lx) * t, ly + (y - ly) * t, paint, b);
        }
        self.last = Some((x, y));
    }

    /// The end of the stroke.
    pub fn end(&mut self) {
        self.in_stroke = false;
        self.last = None;
        self.base = vec![];
        self.mask = vec![];
    }

    fn dab(&mut self, cx: f32, cy: f32, paint: Paint, b: Brush) {
        let r = (b.size / 2.0).max(0.5);
        // The full cover reaches `hard`, then it fades to nothing at `r`.
        let hard = r * b.hardness.clamp(0.0, 1.0);
        let x0 = (cx - r).floor().max(0.0) as usize;
        let y0 = (cy - r).floor().max(0.0) as usize;
        let x1 = ((cx + r).ceil().max(0.0) as usize).min(self.w);
        let y1 = ((cy + r).ceil().max(0.0) as usize).min(self.h);
        if x0 >= x1 || y0 >= y1 {
            return;
        }
        for y in y0..y1 {
            for x in x0..x1 {
                let d = (x as f32 + 0.5 - cx).hypot(y as f32 + 0.5 - cy);
                let cover = if d <= hard { 1.0 } else if d >= r { 0.0 } else { 1.0 - (d - hard) / (r - hard) };
                if cover <= 0.0 {
                    continue;
                }
                let i = y * self.w + x;
                let target = match paint {
                    Paint::Shade(g) => g,
                    Paint::Clone(dx, dy) => {
                        let (sx, sy) = ((x as f32 + dx).round(), (y as f32 + dy).round());
                        if sx < 0.0 || sy < 0.0 || sx as usize >= self.w || sy as usize >= self.h {
                            continue;
                        }
                        self.base[sy as usize * self.w + sx as usize]
                    }
                };
                let m = ((cover * 255.0) as u8).max(self.mask[i]);
                self.mask[i] = m;
                let k = m as f32 / 255.0 * b.opacity.clamp(0.0, 1.0);
                let from = self.base[i] as f32;
                self.gray[i] = (from + (target as f32 - from) * k).round() as u8;
            }
        }
        self.touch(x0, y0, x1, y1);
    }

    /// Fill the area of greys within `tolerance` of the one at `(x, y)` (joined side by side) with `shade`, covering `opacity` of it.
    pub fn fill(&mut self, x: usize, y: usize, shade: u8, tolerance: u8, opacity: f32) {
        if x >= self.w || y >= self.h {
            return;
        }
        let seed = self.gray[y * self.w + x];
        let near = |g: u8| (g as i16 - seed as i16).unsigned_abs() <= tolerance as u16;
        let mut seen = vec![false; self.gray.len()];
        let mut todo = vec![(x, y)];
        let mut area = vec![];
        seen[y * self.w + x] = true;
        while let Some((px, py)) = todo.pop() {
            area.push(py * self.w + px);
            let mut look = |nx: usize, ny: usize| {
                let i = ny * self.w + nx;
                if !seen[i] && near(self.gray[i]) {
                    seen[i] = true;
                    todo.push((nx, ny));
                }
            };
            if px > 0 {
                look(px - 1, py);
            }
            if px + 1 < self.w {
                look(px + 1, py);
            }
            if py > 0 {
                look(px, py - 1);
            }
            if py + 1 < self.h {
                look(px, py + 1);
            }
        }
        self.keep_undo();
        let k = opacity.clamp(0.0, 1.0);
        let (mut x0, mut y0, mut x1, mut y1) = (self.w, self.h, 0, 0);
        for i in area {
            let g = self.gray[i] as f32;
            self.gray[i] = (g + (shade as f32 - g) * k).round() as u8;
            let (px, py) = (i % self.w, i / self.w);
            x0 = x0.min(px);
            y0 = y0.min(py);
            x1 = x1.max(px + 1);
            y1 = y1.max(py + 1);
        }
        self.touch(x0, y0, x1, y1);
    }
}

/// The window of the editor.
pub struct PhotoDlg {
    pub shape: u64,
    pub canvas: Canvas,
    /// Is the picture shown as its negative (its greys are turned back when it is applied)?
    invert: bool,
    pub tool: PhotoTool,
    pub shade: u8,
    pub brush: Brush,
    pub tolerance: u8,
    /// The place the clone stamp copies from (pixels), and how far it is from where the painting is, once the first stroke fixed it.
    pub clone_src: Option<(f32, f32)>,
    clone_offset: Option<(f32, f32)>,
    tex: Option<egui::TextureHandle>,
    view: PictureView,
    note: Option<&'static str>,
}

impl PhotoDlg {
    pub fn new(shape: u64, im: &base_core::image::ImageData) -> PhotoDlg {
        let gray = if im.invert { im.gray.iter().map(|g| 255 - g).collect() } else { im.gray.clone() };
        let size = (im.px_w.max(im.px_h) as f32 / 40.0).clamp(4.0, 120.0).round();
        PhotoDlg {
            shape,
            canvas: Canvas::new(im.px_w as usize, im.px_h as usize, gray),
            invert: im.invert,
            tool: PhotoTool::Brush,
            shade: 0,
            brush: Brush { size, hardness: 0.6, opacity: 1.0 },
            tolerance: 24,
            clone_src: None,
            clone_offset: None,
            tex: None,
            view: PictureView::default(),
            note: None,
        }
    }

    /// The greys to store in the picture.
    pub fn result(&self) -> Vec<u8> {
        if self.invert {
            self.canvas.gray.iter().map(|g| 255 - g).collect()
        } else {
            self.canvas.gray.clone()
        }
    }

    /// Make the texture, or change the part of it that was painted.
    fn refresh(&mut self, ctx: &egui::Context) {
        let c = &mut self.canvas;
        match (&mut self.tex, c.dirty.take()) {
            (None, _) => {
                let img = egui::ColorImage::from_gray([c.w, c.h], &c.gray);
                self.tex = Some(ctx.load_texture("photo-editor", img, egui::TextureOptions::LINEAR));
            }
            (Some(t), Some([x0, y0, x1, y1])) if x1 > x0 && y1 > y0 => {
                let mut px = Vec::with_capacity((x1 - x0) * (y1 - y0));
                for y in y0..y1 {
                    px.extend_from_slice(&c.gray[y * c.w + x0..y * c.w + x1]);
                }
                t.set_partial([x0, y0], egui::ColorImage::from_gray([x1 - x0, y1 - y0], &px), egui::TextureOptions::LINEAR);
            }
            _ => {}
        }
    }

    fn paint(&self) -> Option<Paint> {
        match self.tool {
            PhotoTool::Brush => Some(Paint::Shade(self.shade)),
            PhotoTool::Eraser => Some(Paint::Shade(255)),
            PhotoTool::Clone => self.clone_offset.map(|(dx, dy)| Paint::Clone(dx, dy)),
            PhotoTool::Fill => None,
        }
    }

    /// The pointer on the picture: painting with the left button, moving it with the right or the middle one, zooming with the wheel.
    fn input(&mut self, ui: &egui::Ui, resp: &egui::Response, rect: egui::Rect, pic: egui::Rect) {
        let k = pic.width() / self.canvas.w.max(1) as f32;
        let to_px = |p: egui::Pos2| ((p.x - pic.left()) / k, (p.y - pic.top()) / k);
        if resp.hovered() {
            let scroll = ui.input(|i| i.smooth_scroll_delta.y);
            if let (Some(m), true) = (resp.hover_pos(), scroll != 0.0) {
                let z = (self.view.zoom * (scroll * 0.0025).exp()).clamp(1.0, 40.0);
                let f = z / self.view.zoom;
                self.view.pan = (m - rect.center()) * (1.0 - f) + self.view.pan * f;
                self.view.zoom = z;
            }
        }
        if resp.dragged_by(egui::PointerButton::Secondary) || resp.dragged_by(egui::PointerButton::Middle) {
            self.view.pan += resp.drag_delta();
        }
        let primary = ui.input(|i| i.pointer.primary_down());
        let pos = ui.input(|i| i.pointer.interact_pos());
        let command = ui.input(|i| i.modifiers.command || i.modifiers.alt);
        if self.tool == PhotoTool::Fill {
            if resp.clicked_by(egui::PointerButton::Primary) {
                if let Some(p) = pos {
                    let (x, y) = to_px(p);
                    if x >= 0.0 && y >= 0.0 {
                        self.canvas.fill(x as usize, y as usize, self.shade, self.tolerance, self.brush.opacity);
                    }
                }
            }
            return;
        }
        if self.canvas.in_stroke() {
            match (primary, pos) {
                (true, Some(p)) => {
                    let (x, y) = to_px(p);
                    if let Some(paint) = self.paint() {
                        self.canvas.to(x, y, paint, self.brush);
                    }
                }
                _ => self.canvas.end(),
            }
            return;
        }
        if primary && resp.is_pointer_button_down_on() && ui.input(|i| i.pointer.button_pressed(egui::PointerButton::Primary)) {
            let Some(p) = pos else { return };
            let (x, y) = to_px(p);
            if self.tool == PhotoTool::Clone {
                if command {
                    self.clone_src = Some((x, y));
                    self.clone_offset = None;
                    self.note = None;
                    return;
                }
                match (self.clone_src, self.clone_offset) {
                    (None, _) => {
                        self.note = Some("First Ctrl+click (Cmd+click) where to copy from");
                        return;
                    }
                    // The first stroke after the place was chosen fixes how far it is; the next strokes keep that distance.
                    (Some((sx, sy)), None) => self.clone_offset = Some((sx - x, sy - y)),
                    _ => {}
                }
            }
            if let Some(paint) = self.paint() {
                self.canvas.begin(x, y, paint, self.brush);
            }
        }
    }
}

/// The 9 greys to pick from at once.
const SWATCHES: [u8; 9] = [0, 32, 64, 96, 128, 160, 192, 224, 255];

fn tools_column(ui: &mut egui::Ui, d: &mut PhotoDlg) {
    ui.label(RichText::new(tr("Tool")).strong());
    for t in PhotoTool::ALL {
        let on = d.tool == t;
        let ink = if on { Color32::WHITE } else { theme::text() };
        let mut b = egui::Button::image_and_text(crate::icons::slot(Some(t.icon()), ink), RichText::new(tr(t.label())).color(ink));
        if on {
            b = b.fill(theme::accent());
        }
        if ui.add_sized([ui.available_width(), 26.0], b).on_hover_text(tr(t.hint())).clicked() {
            d.tool = t;
            d.note = None;
        }
    }
    ui.add_space(8.0);
    if matches!(d.tool, PhotoTool::Brush | PhotoTool::Fill) {
        ui.label(RichText::new(tr("Grey")).strong());
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(3.0, 3.0);
            for g in SWATCHES {
                let (r, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::click());
                ui.painter().rect_filled(r, 3.0, Color32::from_gray(g));
                let edge = if d.shade == g { Stroke::new(2.0, theme::accent()) } else { Stroke::new(1.0, theme::text_dim()) };
                ui.painter().rect_stroke(r, 3.0, edge, egui::StrokeKind::Inside);
                if resp.on_hover_text(format!("{}%", (100.0 - g as f32 / 2.55).round())).clicked() {
                    d.shade = g;
                }
            }
        });
        ui.horizontal(|ui| {
            let (r, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::hover());
            ui.painter().rect_filled(r, 3.0, Color32::from_gray(d.shade));
            ui.painter().rect_stroke(r, 3.0, Stroke::new(1.0, theme::text_dim()), egui::StrokeKind::Inside);
            // How dark: 0 % is white, 100 % black.
            let mut dark = 100.0 - d.shade as f32 / 2.55;
            if ui.add(egui::Slider::new(&mut dark, 0.0..=100.0).suffix(" %").fixed_decimals(0)).on_hover_text(tr("How dark: 0 % is white, 100 % black")).changed() {
                d.shade = (255.0 - dark * 2.55).round().clamp(0.0, 255.0) as u8;
            }
        });
        ui.add_space(6.0);
    }
    if d.tool != PhotoTool::Fill {
        ui.label(tr("Size"));
        ui.add(egui::Slider::new(&mut d.brush.size, 1.0..=400.0).logarithmic(true).suffix(" px").fixed_decimals(0));
        ui.label(tr("Hardness"));
        let mut hard = d.brush.hardness * 100.0;
        if ui.add(egui::Slider::new(&mut hard, 0.0..=100.0).suffix(" %").fixed_decimals(0)).changed() {
            d.brush.hardness = hard / 100.0;
        }
    } else {
        ui.label(tr("Tolerance"));
        ui.add(egui::Slider::new(&mut d.tolerance, 0..=128)).on_hover_text(tr("How different a grey may be and still be filled"));
    }
    ui.label(tr("Opacity"));
    let mut op = d.brush.opacity * 100.0;
    if ui.add(egui::Slider::new(&mut op, 1.0..=100.0).suffix(" %").fixed_decimals(0)).changed() {
        d.brush.opacity = op / 100.0;
    }
    if d.tool == PhotoTool::Clone {
        ui.add_space(6.0);
        let text = if d.clone_src.is_some() { tr("Ctrl+click (Cmd+click) chooses another place to copy from") } else { tr("First Ctrl+click (Cmd+click) where to copy from") };
        ui.label(RichText::new(text).small().color(theme::text_dim()));
    }
    if let Some(n) = d.note {
        ui.label(RichText::new(tr(n)).color(Color32::from_rgb(0xe0, 0x90, 0x30)));
    }
    ui.add_space(10.0);
    ui.horizontal(|ui| {
        if ui.add_enabled(d.canvas.can_undo(), egui::Button::image_and_text(crate::icons::slot(Some("undo-2"), theme::text()), tr("Undo"))).clicked() {
            d.canvas.undo();
        }
        if ui.add_enabled(d.canvas.can_redo(), egui::Button::new(tr("Redo"))).clicked() {
            d.canvas.redo();
        }
    });
    ui.add_space(6.0);
    ui.label(RichText::new(tr("Scroll to zoom; drag with the right or the middle button to move the picture.")).small().color(theme::text_dim()));
}

/// Open the editor on the selected picture.
pub fn open<H: EditorHost + ?Sized>(h: &mut H) {
    let Some(id) = h.ed().sel.first().copied() else { return };
    if let Some(Kind::Image(im)) = h.scene().shape(id).map(|s| &s.kind) {
        let d = PhotoDlg::new(id, im);
        h.ed_mut().photo_dlg = Some(d);
    }
}

/// The window, while it is open.
pub fn window<H: EditorHost + ?Sized>(h: &mut H, ctx: &egui::Context) {
    let Some(mut d) = h.ed_mut().photo_dlg.take() else { return };
    if h.scene().shape(d.shape).is_none() {
        return;
    }
    d.refresh(ctx);
    let mut open = true;
    let (mut accept, mut cancel) = (false, false);
    crate::window::popup(ctx, "w_photo_editor", &tr("Edit image"), [1060.0, 700.0], &mut open, |ui| {
        let page_h = crate::window::height_above(ui, "photo_dlg", crate::window::fill_size(ui).y);
        let gap = ui.spacing().item_spacing.x;
        let (page, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), page_h.max(160.0)), Sense::hover());
        let tools = egui::Rect::from_min_size(page.min, egui::vec2(TOOLS_W, page.height()));
        column(ui, tools, |ui| {
            egui::ScrollArea::vertical().id_salt("photo_tools").auto_shrink([false, false]).max_height(page.height()).show(ui, |ui| tools_column(ui, &mut d));
        });
        let rect = egui::Rect::from_min_max(egui::pos2(tools.right() + gap, page.top()), page.max);
        let resp = ui.interact(rect, ui.scope_id().with("photo_picture"), Sense::click_and_drag());
        let size = egui::vec2(d.canvas.w as f32, d.canvas.h as f32);
        let pic = d.view.place(rect, size);
        d.input(ui, &resp, rect, pic);
        let pic = d.view.place(rect, size);
        d.refresh(ui.ctx());
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::from_gray(0x80));
        if let Some(t) = &d.tex {
            painter.image(t.id(), pic, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        }
        let k = pic.width() / d.canvas.w.max(1) as f32;
        // Where the clone stamp copies from: a cross, following the brush while it paints.
        if d.tool == PhotoTool::Clone {
            let at = match (d.clone_offset, resp.hover_pos()) {
                (Some((dx, dy)), Some(m)) => Some(m + egui::vec2(dx, dy) * k),
                _ => d.clone_src.map(|(x, y)| pic.min + egui::vec2(x, y) * k),
            };
            if let Some(c) = at {
                for (w, col) in [(3.0, Color32::BLACK), (1.5, Color32::WHITE)] {
                    painter.line_segment([c - egui::vec2(7.0, 0.0), c + egui::vec2(7.0, 0.0)], Stroke::new(w, col));
                    painter.line_segment([c - egui::vec2(0.0, 7.0), c + egui::vec2(0.0, 7.0)], Stroke::new(w, col));
                }
            }
        }
        // The brush under the pointer: a circle of its size.
        if let (Some(m), true) = (resp.hover_pos(), d.tool != PhotoTool::Fill) {
            let r = (d.brush.size / 2.0 * k).max(1.5);
            painter.circle_stroke(m, r, Stroke::new(2.5, Color32::BLACK));
            painter.circle_stroke(m, r, Stroke::new(1.0, Color32::WHITE));
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        } else if resp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
        }
        d.view.bar(ui, rect);
        crate::window::footer(ui, "photo_dlg", |ui| {
            ui.horizontal(|ui| {
                accept = crate::widgets::apply_button(ui, &tr("Apply")).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
    });
    if accept {
        d.canvas.end();
        if d.canvas.changed() {
            let gray = d.result();
            h.checkpoint();
            match h.scene_mut().unlocked_mut(d.shape) {
                Some(s) => {
                    if let Kind::Image(im) = &mut s.kind {
                        im.gray = gray;
                    }
                }
                None => h.say(tr("The picture is locked").to_string()),
            }
            h.ed_mut().image_tex.retain(|k, _| k.0 != d.shape);
            h.touch();
            ctx.request_repaint();
        }
        return;
    }
    if cancel || !open {
        return;
    }
    h.ed_mut().photo_dlg = Some(d);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn white(w: usize, h: usize) -> Canvas {
        Canvas::new(w, h, vec![255; w * h])
    }

    /// A stroke paints its grey; going over the same place twice in one stroke does not make it darker than the opacity allows.
    #[test]
    fn a_stroke_paints_and_does_not_build_up() {
        let mut c = white(40, 40);
        let b = Brush { size: 8.0, hardness: 1.0, opacity: 0.5 };
        c.begin(10.0, 20.0, Paint::Shade(0), b);
        c.to(30.0, 20.0, Paint::Shade(0), b);
        c.to(10.0, 20.0, Paint::Shade(0), b);
        c.end();
        let g = c.gray[20 * 40 + 20];
        assert!((126..=129).contains(&g), "half of the way to black, {g}");
        assert_eq!(c.gray[2 * 40 + 20], 255, "far from the line nothing changed");
        c.undo();
        assert!(c.gray.iter().all(|g| *g == 255), "undo takes the stroke back");
        c.redo();
        assert_eq!(c.gray[20 * 40 + 20], g);
    }

    /// The fill takes the area joined to the point, within the tolerance, and stops at a darker line.
    #[test]
    fn the_fill_stops_at_a_line() {
        let mut c = white(20, 10);
        for y in 0..10 {
            c.gray[y * 20 + 10] = 0;
        }
        c.fill(2, 2, 100, 20, 1.0);
        assert_eq!(c.gray[5 * 20 + 3], 100);
        assert_eq!(c.gray[5 * 20 + 10], 0, "the line stays");
        assert_eq!(c.gray[5 * 20 + 15], 255, "beyond the line nothing is filled");
    }

    /// The clone stamp copies the pixels as they were when the stroke began.
    #[test]
    fn the_clone_stamp_copies_from_the_source() {
        let mut c = white(40, 20);
        for y in 0..20 {
            for x in 0..10 {
                c.gray[y * 40 + x] = 50;
            }
        }
        let b = Brush { size: 6.0, hardness: 1.0, opacity: 1.0 };
        // Copy from 25 pixels to the left.
        c.begin(30.0, 10.0, Paint::Clone(-25.0, 0.0), b);
        c.end();
        assert_eq!(c.gray[10 * 40 + 30], 50);
    }
}
