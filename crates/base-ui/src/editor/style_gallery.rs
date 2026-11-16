//! The panel of the picture styles: a tab of the side panel that is there while a picture is selected. At its top a button as wide
//! as the panel opens Trace image; under it, as in the gallery, a picture of every style of Style trace (a triangle drawn in that
//! style), and a click on one opens Style trace on that style. The pictures are made once, in the background, the first time the
//! panel is seen.
use super::{EditorHost, EditorImages, EditorStyleTrace};
use crate::i18n::tr;
use crate::theme;
use base_core::image::ImageData;
use base_core::scene::Kind;
use base_core::style_trace::{style_trace, Style, StyleParams};
use egui::{Color32, RichText, Sense};
use std::sync::{Mutex, OnceLock};

/// The id of the tab.
pub const TAB_ID: &str = "picture_styles";

/// The translated title of the tab.
pub fn tab_label() -> &'static str {
    tr("Picture styles")
}

/// Is the first selected shape a picture (the tab is shown then)?
pub fn picture_selected<H: EditorHost + ?Sized>(h: &H) -> bool {
    h.ed().sel.first().and_then(|id| h.scene().shape(*id)).is_some_and(|s| matches!(s.kind, Kind::Image(_)))
}

/// The side of a picture of a style, in pixels.
const THUMB_PX: usize = 128;

/// The pictures of the styles: `None` while they are made; each is the coverage of the lines (0..255) of `THUMB_PX`².
static THUMBS: OnceLock<Mutex<Option<Vec<(Style, Vec<u8>)>>>> = OnceLock::new();

/// The pictures of the styles, started in the background the first time they are asked for.
fn thumbs() -> Option<Vec<(Style, Vec<u8>)>> {
    let cell = THUMBS.get_or_init(|| {
        std::thread::spawn(|| {
            let made: Vec<(Style, Vec<u8>)> = Style::ALL.iter().map(|s| (*s, thumbnail(*s))).collect();
            if let Some(c) = THUMBS.get() {
                if let Ok(mut g) = c.lock() {
                    *g = Some(made);
                }
            }
        });
        Mutex::new(None)
    });
    cell.lock().ok().and_then(|g| g.clone())
}

/// A triangle, dark on the left and lighter to the right, on white: what every style draws in its picture.
pub fn triangle() -> ImageData {
    let n = 240usize;
    let gray = (0..n * n)
        .map(|i| {
            let (x, y) = ((i % n) as f64, (i / n) as f64);
            // The apex at the top in the middle, the base along the bottom.
            let t = (y - 16.0) / (n as f64 - 32.0);
            let half = t * (n as f64 - 32.0) / 2.0;
            let inside = (0.0..=1.0).contains(&t) && (x - n as f64 / 2.0).abs() <= half;
            if inside {
                (20.0 + 100.0 * x / n as f64) as u8
            } else {
                255
            }
        })
        .collect();
    ImageData { name: "triangle".into(), px_w: n as u32, px_h: n as u32, gray, w: 40.0, h: 40.0, invert: false }
}

/// The settings of a style for its small picture: its own, a few slow ones made lighter.
pub fn thumb_params(s: Style) -> StyleParams {
    let mut p = s.defaults();
    match s {
        Style::Stipple => p.detail = 900.0,
        Style::Tsp => p.detail = 500.0,
        Style::Delaunay => p.detail = 250.0,
        Style::Sketch => p.detail = 700.0,
        Style::Scribble => p.detail = 6.0,
        Style::Hilbert => p.spacing = 0.8,
        Style::Geometric => {
            p.detail = 24.0;
            p.spacing = 0.8;
        }
        Style::VCarve => p.spacing = 2.5,
        _ => {}
    }
    p
}

/// The picture of style `s`: the triangle drawn in it, as the coverage of its lines.
pub fn thumbnail(s: Style) -> Vec<u8> {
    let im = triangle();
    let out = style_trace(&im, &thumb_params(s), &|| false);
    // Drawn three times as big and made smaller, so the lines are smooth.
    let k = 3usize;
    let big = THUMB_PX * k;
    let scale = big as f64 / im.w.max(im.h);
    let mut cover = vec![false; big * big];
    let mut dot = |x: f64, y: f64| {
        let r = 1.6;
        let (x0, x1) = ((x - r).floor().max(0.0) as usize, ((x + r).ceil() as usize).min(big - 1));
        let (y0, y1) = ((y - r).floor().max(0.0) as usize, ((y + r).ceil() as usize).min(big - 1));
        for yy in y0..=y1 {
            for xx in x0..=x1 {
                if (xx as f64 + 0.5 - x).powi(2) + (yy as f64 + 0.5 - y).powi(2) <= r * r {
                    cover[yy * big + xx] = true;
                }
            }
        }
    };
    for l in &out.lines {
        let n = l.pts.len();
        let segs = if l.closed { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            let (a, b) = (l.pts[i], l.pts[(i + 1) % n]);
            let (ax, ay, bx, by) = (a.x * scale, a.y * scale, b.x * scale, b.y * scale);
            let steps = ((bx - ax).hypot(by - ay) / 0.8).ceil().max(1.0) as usize;
            for j in 0..=steps {
                let t = j as f64 / steps as f64;
                dot(ax + (bx - ax) * t, ay + (by - ay) * t);
            }
        }
    }
    (0..THUMB_PX * THUMB_PX)
        .map(|i| {
            let (x, y) = (i % THUMB_PX, i / THUMB_PX);
            let mut c = 0;
            for dy in 0..k {
                for dx in 0..k {
                    c += cover[(y * k + dy) * big + x * k + dx] as u32;
                }
            }
            (c * 255 / (k * k) as u32) as u8
        })
        .collect()
}

/// The textures of the pictures, made once for the window.
fn textures(ctx: &egui::Context) -> Option<Vec<(Style, egui::TextureHandle)>> {
    let id = egui::Id::unique("picture_style_thumbs");
    if let Some(t) = ctx.data(|d| d.get_temp::<Vec<(Style, egui::TextureHandle)>>(id)) {
        return Some(t);
    }
    let made = thumbs()?;
    let t: Vec<(Style, egui::TextureHandle)> = made
        .into_iter()
        .map(|(s, cover)| {
            let px: Vec<Color32> = cover.iter().map(|a| Color32::from_white_alpha(*a)).collect();
            let img = egui::ColorImage::new([THUMB_PX, THUMB_PX], px);
            (s, ctx.load_texture(format!("style-thumb-{}", s.label()), img, egui::TextureOptions::LINEAR))
        })
        .collect();
    ctx.data_mut(|d| d.insert_temp(id, t.clone()));
    Some(t)
}

/// The panel: the button of Trace image, then the pictures of the styles in a grid as wide as the panel.
pub fn panel<H: EditorHost + EditorImages + EditorStyleTrace + ?Sized>(h: &mut H, ui: &mut egui::Ui) {
    egui::ScrollArea::vertical().id_salt("picture_styles").auto_shrink([false, false]).show(ui, |ui| {
        let b = egui::Button::image_and_text(crate::icons::slot(Some("portrait"), theme::text()), tr("Trace image…"));
        if ui.add_sized([ui.available_width(), 30.0], b).clicked() {
            h.open_trace();
        }
        ui.add_space(6.0);
        let Some(tex) = textures(ui.ctx()) else {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new(tr("Drawing the styles…")).color(theme::text_dim()));
            });
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(150));
            return;
        };
        let gap = 6.0;
        let width = ui.available_width();
        // Two thirds of the cell the gallery of clipart would have (at least 96 points): as many columns as fit cells that size.
        let gallery_cols = ((width + gap) / (96.0 + gap)).floor().max(2.0);
        let gallery_side = (width - gap * (gallery_cols - 1.0)) / gallery_cols;
        let cols = ((width + gap) / (gallery_side * 2.0 / 3.0 + gap)).round().max(3.0) as usize;
        let side = ((width - gap * (cols as f32 - 1.0)) / cols as f32).floor();
        let label_h = 28.0;
        for row in tex.chunks(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for (style, t) in row {
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(side, side + label_h), Sense::click());
                    let pic = egui::Rect::from_min_size(rect.min, egui::vec2(side, side));
                    // Black lines on white, as they come out on the material; a frame of the accent colour under the pointer.
                    ui.painter().rect_filled(pic, 6.0, Color32::WHITE);
                    if resp.hovered() {
                        ui.painter().rect_stroke(pic, 6.0, egui::Stroke::new(2.0, theme::accent()), egui::StrokeKind::Inside);
                    }
                    ui.painter().image(t.id(), pic.shrink(5.0), egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), Color32::BLACK);
                    let name = tr(style.label());
                    let galley = ui.painter().layout(name.to_string(), egui::FontId::proportional(10.5), theme::text(), side);
                    let at = egui::pos2(rect.center().x - galley.size().x / 2.0, pic.bottom() + 3.0);
                    ui.painter().galley(at, galley, theme::text());
                    if resp.on_hover_text(tr(style.hint())).clicked() {
                        h.open_style_trace_as(*style);
                    }
                }
            });
            ui.add_space(gap);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every style has a picture of the triangle with some lines in it, and it is quick enough to make them all at once.
    #[test]
    fn every_style_draws_the_triangle() {
        for s in Style::ALL {
            let t = thumbnail(s);
            assert_eq!(t.len(), THUMB_PX * THUMB_PX);
            let ink = t.iter().filter(|a| **a > 0).count();
            assert!(ink > 100, "{s:?} drew {ink} pixels");
        }
    }
}
