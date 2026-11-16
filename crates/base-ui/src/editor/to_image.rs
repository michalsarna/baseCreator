//! The selected shapes as one picture: a window with the preview, the resolution and the size, the fill of the closed shapes and
//! the width of the lines. The picture takes the place of the shapes (they may be deleted then). "Hide the objects" leaves their
//! outlines out of the preview only.
use super::image_ui::{column, PictureView};
use super::EditorHost;
use crate::i18n::{tr, trf};
use crate::theme;
use crate::units_ui::drag_len;
use base_core::geom::{Polyline, Xf};
use base_core::image::ImageData;
use base_core::scene::Kind;
use egui::{Color32, RichText, Sense, Stroke};

/// The width of the column with the settings.
const CONTROLS_W: f32 = 280.0;
/// The most pixels a picture may have.
const MAX_PIXELS: f64 = 40_000_000.0;
/// The longest side a picture may have, in pixels.
const MAX_SIDE: f64 = 8192.0;
/// The longest side of the preview, in pixels.
const PREVIEW_PX: f64 = 1000.0;

/// The settings that change the picture.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Raster {
    /// Dots per inch.
    pub dpi: f64,
    /// The width of the picture in millimetres (the height follows).
    pub width: f64,
    /// Are the closed shapes filled?
    pub fill: bool,
    /// The width of the lines, in millimetres (0: no lines).
    pub line: f64,
}

/// The window.
pub struct ToImageDlg {
    pub ids: Vec<u64>,
    /// The outlines of each shape, in millimetres, as they are on the page (each shape is filled on its own, so shapes that
    /// overlap do not cut holes in one another).
    polys: Vec<Vec<Polyline>>,
    /// The corner and the size of the shapes (with half a line round them).
    origin: (f64, f64),
    size: (f64, f64),
    pub raster: Raster,
    pub delete_objects: bool,
    /// Their outlines are not drawn over the preview (the preview only).
    pub hide_objects: bool,
    tex: Option<egui::TextureHandle>,
    shown: Option<Raster>,
    view: PictureView,
}

/// Are there shapes selected, and none of them a picture?
pub fn can_convert<H: EditorHost + ?Sized>(h: &H) -> bool {
    let shapes: Vec<_> = h.ed().sel.iter().filter_map(|id| h.scene().shape(*id)).collect();
    !shapes.is_empty() && !shapes.iter().any(|s| matches!(s.kind, Kind::Image(_)))
}

impl ToImageDlg {
    pub fn new(ids: Vec<u64>, polys: Vec<Vec<Polyline>>) -> Option<ToImageDlg> {
        let mut d = ToImageDlg { ids, polys, origin: (0.0, 0.0), size: (1.0, 1.0), raster: Raster { dpi: 254.0, width: 1.0, fill: true, line: 0.3 }, delete_objects: false, hide_objects: false, tex: None, shown: None, view: PictureView::default() };
        d.measure()?;
        d.raster.width = d.size.0;
        Some(d)
    }

    /// The box of the shapes, half a line larger on each side so the lines along the edge are whole.
    fn measure(&mut self) -> Option<()> {
        let b = self.polys.iter().flatten().filter_map(|p| p.bounds()).reduce(base_core::geom::Rect::union)?;
        let m = self.raster.line / 2.0;
        let (w, h) = (b.max.x - b.min.x + 2.0 * m, b.max.y - b.min.y + 2.0 * m);
        if w <= 1e-6 || h <= 1e-6 {
            return None;
        }
        self.origin = (b.min.x - m, b.min.y - m);
        self.size = (w, h);
        Some(())
    }

    /// The height of the picture that goes with its width.
    pub fn height(&self) -> f64 {
        self.raster.width * self.size.1 / self.size.0
    }

    /// The pixels of the picture at the chosen resolution (fewer, when there would be too many).
    pub fn pixels(&self) -> (usize, usize) {
        let ppmm = self.raster.dpi / 25.4;
        let (mut w, mut h) = (self.raster.width * ppmm, self.height() * ppmm);
        if w * h > MAX_PIXELS {
            let k = (MAX_PIXELS / (w * h)).sqrt();
            w *= k;
            h *= k;
        }
        // No side longer than a picture of the program may have (the graphics card shows no larger one).
        if w.max(h) > MAX_SIDE {
            let k = MAX_SIDE / w.max(h);
            w *= k;
            h *= k;
        }
        ((w.round() as usize).max(1), (h.round() as usize).max(1))
    }

    /// The shapes drawn into a picture of `px_w` × `px_h` pixels.
    fn draw(&self, px_w: usize, px_h: usize) -> Vec<u8> {
        let k = px_w as f64 / self.size.0;
        let line_px = self.raster.line * k;
        let shapes: Vec<Vec<(Vec<(f64, f64)>, bool)>> = self.polys.iter().map(|shape| shape.iter().map(|p| (p.pts.iter().map(|q| ((q.x - self.origin.0) * k, (q.y - self.origin.1) * k)).collect(), p.closed)).collect()).collect();
        rasterize(&shapes, px_w, px_h, self.raster.fill, line_px)
    }

    /// The finished picture.
    pub fn result(&self) -> ImageData {
        let (w, h) = self.pixels();
        ImageData { name: tr("Objects").to_string(), px_w: w as u32, px_h: h as u32, gray: self.draw(w, h), w: self.raster.width, h: self.height(), invert: false }
    }
}

/// Coverage of shapes on a grid of pixels, as greys (white is 255): each shape is a list of paths (with whether each is closed). The
/// closed paths of a shape are filled by the even-odd rule (when `fill`), so the hole of an O stays; shapes that overlap simply cover
/// one another. Every line is `line_px` wide. The edges are smooth: each row is looked at on several lines, each line by its distance.
pub fn rasterize(shapes: &[Vec<(Vec<(f64, f64)>, bool)>], w: usize, h: usize, fill: bool, line_px: f64) -> Vec<u8> {
    let mut cover = vec![0f32; w * h];
    if fill {
        let mut row = vec![0f32; w];
        let mut xs: Vec<f64> = vec![];
        let sub = 4;
        for shape in shapes {
            let mut edges: Vec<((f64, f64), (f64, f64))> = vec![];
            for (p, closed) in shape {
                if !*closed || p.len() < 3 {
                    continue;
                }
                for i in 0..p.len() {
                    let (a, b) = (p[i], p[(i + 1) % p.len()]);
                    if a.1 != b.1 {
                        edges.push((a, b));
                    }
                }
            }
            if edges.is_empty() {
                continue;
            }
            let (mut ymin, mut ymax, mut xmin, mut xmax) = (f64::MAX, f64::MIN, f64::MAX, f64::MIN);
            for (a, b) in &edges {
                ymin = ymin.min(a.1).min(b.1);
                ymax = ymax.max(a.1).max(b.1);
                xmin = xmin.min(a.0).min(b.0);
                xmax = xmax.max(a.0).max(b.0);
            }
            let (y0, y1) = (ymin.floor().max(0.0) as usize, (ymax.ceil().max(0.0) as usize).min(h));
            let (c0, c1) = (xmin.floor().max(0.0) as usize, (xmax.ceil().max(0.0) as usize).min(w));
            // The edges by the rows they cross, so each row looks only at its own.
            let mut by_row: Vec<Vec<usize>> = vec![vec![]; y1.saturating_sub(y0)];
            for (i, (a, b)) in edges.iter().enumerate() {
                let (ey0, ey1) = (a.1.min(b.1).floor().max(y0 as f64) as usize, (a.1.max(b.1).ceil().max(0.0) as usize).min(y1));
                for r in ey0..ey1 {
                    by_row[r - y0].push(i);
                }
            }
            for y in y0..y1 {
                row[c0..c1].iter_mut().for_each(|v| *v = 0.0);
                for s in 0..sub {
                    let sy = y as f64 + (s as f64 + 0.5) / sub as f64;
                    xs.clear();
                    for &i in &by_row[y - y0] {
                        let (a, b) = edges[i];
                        if (a.1 <= sy) != (b.1 <= sy) {
                            xs.push(a.0 + (sy - a.1) / (b.1 - a.1) * (b.0 - a.0));
                        }
                    }
                    xs.sort_by(|p, q| p.total_cmp(q));
                    for pair in xs.chunks_exact(2) {
                        let (x0, x1) = (pair[0].max(0.0), pair[1].min(w as f64));
                        if x1 <= x0 {
                            continue;
                        }
                        for x in (x0.floor() as usize)..((x1.ceil() as usize).min(w)) {
                            row[x] += ((x1.min(x as f64 + 1.0) - x0.max(x as f64)).max(0.0) / sub as f64) as f32;
                        }
                    }
                }
                for x in c0..c1 {
                    let i = y * w + x;
                    cover[i] = cover[i].max(row[x]);
                }
            }
        }
    }
    if line_px > 0.0 {
        let hw = line_px / 2.0;
        for (p, closed) in shapes.iter().flatten() {
            let n = p.len();
            let segs = if *closed { n } else { n.saturating_sub(1) };
            for i in 0..segs {
                let (a, b) = (p[i], p[(i + 1) % n]);
                let x0 = (a.0.min(b.0) - hw - 1.0).floor().max(0.0) as usize;
                let x1 = ((a.0.max(b.0) + hw + 1.0).ceil().max(0.0) as usize).min(w);
                let y0 = (a.1.min(b.1) - hw - 1.0).floor().max(0.0) as usize;
                let y1 = ((a.1.max(b.1) + hw + 1.0).ceil().max(0.0) as usize).min(h);
                let (dx, dy) = (b.0 - a.0, b.1 - a.1);
                let len2 = dx * dx + dy * dy;
                for y in y0..y1 {
                    for x in x0..x1 {
                        let (px, py) = (x as f64 + 0.5, y as f64 + 0.5);
                        let t = if len2 > 0.0 { (((px - a.0) * dx + (py - a.1) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
                        let d = (px - a.0 - t * dx).hypot(py - a.1 - t * dy);
                        // A line thinner than a pixel is drawn as a pixel wide, lighter.
                        let c = if hw < 0.5 { ((1.0 - d).clamp(0.0, 1.0) * hw * 2.0) as f32 } else { (hw + 0.5 - d).clamp(0.0, 1.0) as f32 };
                        let i = y * w + x;
                        cover[i] = cover[i].max(c);
                    }
                }
            }
        }
    }
    cover.iter().map(|c| (255.0 * (1.0 - c.clamp(0.0, 1.0))).round() as u8).collect()
}

/// The layer of the new picture. A layer that holds other shapes is left alone (a program may switch a layer to pictures when the
/// first one comes on it, and the shapes on it would then be done as a picture): the layer of the shapes when they are deleted and
/// nothing else is on it, else a layer that already has pictures, else an empty one, else the active layer.
fn picture_layer<H: EditorHost + ?Sized>(h: &H, ids: &[u64], deleting: bool) -> usize {
    let shapes = &h.scene().shapes;
    if let Some(first) = ids.first().and_then(|id| h.scene().shape(*id)) {
        if deleting && shapes.iter().filter(|s| s.layer == first.layer).all(|s| ids.contains(&s.id) && !s.locked) {
            return first.layer;
        }
    }
    if let Some(s) = shapes.iter().find(|s| matches!(s.kind, Kind::Image(_))) {
        return s.layer;
    }
    (0..base_core::layers::LAYER_COUNT).find(|l| !shapes.iter().any(|s| s.layer == *l)).unwrap_or(h.ed().active_layer)
}

/// Open the window on the selected shapes.
pub fn open<H: EditorHost + ?Sized>(h: &mut H) {
    if !can_convert(h) {
        return;
    }
    let ids = h.ed().sel.clone();
    let polys: Vec<Vec<Polyline>> = ids.iter().filter_map(|id| h.scene().shape(*id)).map(|s| s.polys()).collect();
    match ToImageDlg::new(ids, polys) {
        Some(d) => h.ed_mut().to_image_dlg = Some(d),
        None => h.say(tr("The shapes have no size").to_string()),
    }
}

fn controls(ui: &mut egui::Ui, d: &mut ToImageDlg, units: base_core::units::Units) {
    ui.label(RichText::new(tr("Resolution")).strong());
    ui.horizontal(|ui| {
        ui.add(egui::DragValue::new(&mut d.raster.dpi).range(25.0..=1200.0).speed(1.0).suffix(" DPI").max_decimals(0));
        for v in [127.0, 254.0, 318.0, 508.0] {
            if ui.selectable_label(d.raster.dpi == v, format!("{v:.0}")).clicked() {
                d.raster.dpi = v;
            }
        }
    });
    ui.label(RichText::new(trf("{} lines per millimetre", &[&format!("{:.1}", d.raster.dpi / 25.4)])).small().color(theme::text_dim()));
    ui.add_space(8.0);
    ui.label(RichText::new(tr("Size")).strong());
    egui::Grid::new("to_image_size").num_columns(2).show(ui, |ui| {
        ui.label(tr("Width"));
        drag_len(ui, units, &mut d.raster.width, 0.5, Some((1.0, 5000.0)));
        ui.end_row();
        ui.label(tr("Height"));
        let mut hgt = d.height();
        if drag_len(ui, units, &mut hgt, 0.5, Some((1.0, 5000.0))).changed() {
            d.raster.width = hgt * d.size.0 / d.size.1;
        }
        ui.end_row();
    });
    if ui.small_button(tr("The size of the objects")).clicked() {
        d.raster.width = d.size.0;
    }
    let (pw, ph) = d.pixels();
    ui.label(RichText::new(trf("{} × {} pixels", &[&pw, &ph])).small().color(theme::text_dim()));
    ui.add_space(8.0);
    ui.label(RichText::new(tr("Drawing")).strong());
    ui.checkbox(&mut d.raster.fill, tr("Fill the closed shapes"));
    ui.horizontal(|ui| {
        ui.label(tr("Line width"));
        let before = d.raster.line;
        drag_len(ui, units, &mut d.raster.line, 0.01, Some((0.0, 20.0)));
        if d.raster.line != before {
            // The box round the shapes grows with the line; the picture keeps its scale.
            let scale = d.raster.width / d.size.0;
            d.measure();
            d.raster.width = d.size.0 * scale;
        }
    });
    ui.add_space(8.0);
    ui.checkbox(&mut d.delete_objects, tr("Delete the objects afterwards"));
    ui.checkbox(&mut d.hide_objects, tr("Hide the objects")).on_hover_text(tr("Only in the preview: the picture alone, without the outlines of the objects"));
}

/// The window, while it is open.
pub fn window<H: EditorHost + ?Sized>(h: &mut H, ctx: &egui::Context) {
    let Some(mut d) = h.ed_mut().to_image_dlg.take() else { return };
    let units = h.units();
    if d.shown != Some(d.raster) || d.tex.is_none() {
        // The preview at the resolution chosen, made smaller when it is large (the pixels of a low resolution are seen as they are).
        let (w, hh) = d.pixels();
        let k = (PREVIEW_PX / w.max(hh) as f64).min(1.0);
        let (pw, ph) = (((w as f64 * k).round() as usize).max(1), ((hh as f64 * k).round() as usize).max(1));
        let px = d.draw(pw, ph);
        let img = egui::ColorImage::from_gray([pw, ph], &px);
        d.tex = Some(ctx.load_texture("to-image", img, egui::TextureOptions::NEAREST));
        d.shown = Some(d.raster);
    }
    let mut open = true;
    let (mut accept, mut cancel) = (false, false);
    crate::window::popup(ctx, "w_to_image", &tr("Convert to image"), [960.0, 600.0], &mut open, |ui| {
        let page_h = crate::window::height_above(ui, "to_image_dlg", crate::window::fill_size(ui).y);
        let gap = ui.spacing().item_spacing.x;
        let (page, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), page_h.max(160.0)), Sense::hover());
        let pw = (page.width() - CONTROLS_W - gap).max(160.0);
        let rect = egui::Rect::from_min_size(page.min, egui::vec2(pw, page.height()));
        let resp = ui.interact(rect, ui.scope_id().with("to_image_picture"), Sense::click_and_drag());
        d.view.input(ui, &resp, rect);
        let painter = ui.painter_at(rect);
        painter.rect_filled(rect, 4.0, Color32::from_gray(0x80));
        let pic = d.view.place(rect, egui::vec2(d.size.0 as f32, d.size.1 as f32));
        if let Some(t) = &d.tex {
            painter.image(t.id(), pic, egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::WHITE);
        }
        if !d.hide_objects {
            let k = pic.width() as f64 / d.size.0;
            let stroke = Stroke::new(1.2, theme::accent());
            for p in d.polys.iter().flatten() {
                let pts: Vec<egui::Pos2> = p.pts.iter().map(|q| pic.min + egui::vec2(((q.x - d.origin.0) * k) as f32, ((q.y - d.origin.1) * k) as f32)).collect();
                if p.closed {
                    painter.add(egui::Shape::closed_line(pts, stroke));
                } else {
                    painter.add(egui::Shape::line(pts, stroke));
                }
            }
        }
        d.view.bar(ui, rect);
        let side = egui::Rect::from_min_size(egui::pos2(rect.right() + gap, page.top()), egui::vec2(CONTROLS_W, page.height()));
        column(ui, side, |ui| {
            egui::ScrollArea::vertical().id_salt("to_image_controls").auto_shrink([false, false]).max_height(page.height()).show(ui, |ui| controls(ui, &mut d, units));
        });
        crate::window::footer(ui, "to_image_dlg", |ui| {
            ui.horizontal(|ui| {
                accept = crate::widgets::apply_button(ui, &tr("Convert")).clicked();
                cancel = ui.button(tr("Cancel")).clicked();
            });
        });
    });
    if accept {
        let im = d.result();
        h.checkpoint();
        let layer = picture_layer(h, &d.ids, d.delete_objects);
        let id = h.scene_mut().add(layer, Kind::Image(im), Xf::translate(d.origin.0, d.origin.1));
        if d.delete_objects {
            let gone = d.ids.clone();
            h.scene_mut().shapes.retain(|s| !gone.contains(&s.id) || s.locked);
        }
        h.ed_mut().sel = vec![id];
        h.touch();
        return;
    }
    if cancel || !open {
        return;
    }
    h.ed_mut().to_image_dlg = Some(d);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A filled square is black inside and white outside, with grey on its edge where it cuts a pixel.
    #[test]
    fn a_square_is_filled_with_smooth_edges() {
        let sq = vec![(2.0, 2.0), (8.5, 2.0), (8.5, 8.0), (2.0, 8.0)];
        let g = rasterize(&[vec![(sq, true)]], 10, 10, true, 0.0);
        assert_eq!(g[5 * 10 + 5], 0);
        assert_eq!(g[0], 255);
        assert!((120..=135).contains(&g[5 * 10 + 8]), "half a pixel covered: {}", g[5 * 10 + 8]);
    }

    /// A ring (two closed paths, one in the other) leaves its hole white; an open line has its width.
    #[test]
    fn holes_stay_white_and_lines_have_width() {
        let outer = vec![(1.0, 1.0), (19.0, 1.0), (19.0, 19.0), (1.0, 19.0)];
        let inner = vec![(6.0, 6.0), (14.0, 6.0), (14.0, 14.0), (6.0, 14.0)];
        let g = rasterize(&[vec![(outer.clone(), true), (inner.clone(), true)]], 20, 20, true, 0.0);
        assert_eq!(g[10 * 20 + 10], 255, "the hole");
        assert_eq!(g[3 * 20 + 3], 0, "the ring");
        // The same square as a shape of its own, on the ring: shapes cover one another, they do not cut holes.
        let g = rasterize(&[vec![(outer, true)], vec![(inner, true)]], 20, 20, true, 0.0);
        assert_eq!(g[10 * 20 + 10], 0, "two shapes that overlap are both filled");
        let line = vec![(0.0, 5.0), (20.0, 5.0)];
        let g = rasterize(&[vec![(line, false)]], 20, 10, true, 3.0);
        assert_eq!(g[5 * 20 + 10], 0);
        assert_eq!(g[9 * 20 + 10], 255);
    }

    /// The picture keeps the proportions of the shapes and has as many pixels as the resolution says.
    #[test]
    fn the_picture_has_the_size_and_resolution() {
        let p = Polyline::new(vec![base_core::geom::Pt::new(0.0, 0.0), base_core::geom::Pt::new(40.0, 0.0), base_core::geom::Pt::new(40.0, 20.0), base_core::geom::Pt::new(0.0, 20.0)], true);
        let mut d = ToImageDlg::new(vec![1], vec![vec![p]]).unwrap();
        d.raster.line = 0.0;
        d.measure();
        d.raster.width = 40.0;
        d.raster.dpi = 254.0;
        assert_eq!(d.pixels(), (400, 200));
        let im = d.result();
        assert_eq!((im.w, im.h), (40.0, 20.0));
        assert_eq!(im.gray[100 * 400 + 200], 0);
        // A long strip at a high resolution keeps its sides within what can be shown.
        d.raster.width = 400.0;
        d.raster.dpi = 1200.0;
        let (w, hh) = d.pixels();
        assert!(w <= 8192 && hh <= 8192, "{w} × {hh}");
    }
}
