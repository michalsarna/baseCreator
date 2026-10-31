//! Another open drawing shown for looking only: the work area, the shapes in the colours of their layers, a zoom and a shift of
//! its own. No handles, no selection, no tools; a click tells the caller that the person wants to work on this drawing.
use super::{fit_view, View};
use base_core::scene::{Kind, Scene};
use egui::{Color32, PointerButton, Sense, Stroke};

/// How the drawing looks: the colours of the area around the work area and of the work area itself, and the colour of a layer
/// (`None` for a hidden layer).
pub struct Look<'a> {
    pub workspace: Color32,
    pub bed: Color32,
    pub layer_colour: &'a dyn Fn(usize) -> Option<Color32>,
}

/// Draw `scene` on a work area of `bed` millimetres in the rest of `ui`. `view` is the zoom and shift of this pane; `None` fits
/// the work area to the pane, and it keeps fitting until the person zooms or shifts it. Returns `true` when the pane was clicked.
pub fn show_other_doc(ui: &mut egui::Ui, scene: &Scene, bed: (f64, f64), look: &Look, view: &mut Option<View>) -> bool {
    let (resp, painter) = ui.allocate_painter(ui.available_size(), Sense::click_and_drag());
    let rect = resp.rect;
    painter.rect_filled(rect, 0.0, look.workspace);
    let v = view.get_or_insert(View { zoom: 1.0, pan: egui::Vec2::ZERO, need_fit: true, auto_fit: true });
    if (v.need_fit || v.auto_fit) && rect.width() > 50.0 && bed.0 > 0.0 && bed.1 > 0.0 {
        let (z, pan) = fit_view((rect.width(), rect.height()), bed, 20.0);
        v.zoom = z;
        v.pan = pan;
        v.need_fit = false;
    }
    let o = rect.min;
    if resp.hovered() {
        let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
        let f = (scroll * 0.0015).exp() * pinch;
        if (f - 1.0).abs() > 1e-4 {
            if let Some(m) = resp.hover_pos() {
                let at = (m - o - v.pan) / v.zoom;
                v.zoom = (v.zoom * f).clamp(0.05, 200.0);
                v.pan = m - o - at * v.zoom;
                v.auto_fit = false;
            }
        }
    }
    if resp.dragged_by(PointerButton::Middle) {
        v.pan += resp.drag_delta();
        v.auto_fit = false;
    }
    let (zoom, pan) = (v.zoom, v.pan);
    let w2s = |x: f64, y: f64| o + pan + egui::vec2(x as f32 * zoom, y as f32 * zoom);
    let painter = painter.with_clip_rect(rect);
    let bed_rect = egui::Rect::from_min_max(w2s(0.0, 0.0), w2s(bed.0, bed.1));
    painter.rect_filled(bed_rect.translate(egui::vec2(3.0, 3.0)), 0.0, Color32::from_black_alpha(25));
    painter.rect_filled(bed_rect, 0.0, look.bed);
    painter.rect_stroke(bed_rect, 0.0, Stroke::new(1.0, Color32::from_gray(0x99)), egui::StrokeKind::Outside);
    for s in &scene.shapes {
        let Some(c) = (look.layer_colour)(s.layer) else { continue };
        if let Kind::Image(im) = &s.kind {
            // A picture is shown by its frame only.
            let (w, h) = (im.w, im.h);
            let pts: Vec<egui::Pos2> = [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h), (0.0, 0.0)].iter().map(|&(x, y)| {
                let p = s.xf.apply(base_core::geom::Pt::new(x, y));
                w2s(p.x, p.y)
            }).collect();
            painter.add(egui::Shape::line(pts, Stroke::new(1.0, c.gamma_multiply(0.6))));
            continue;
        }
        for p in s.polys() {
            if p.pts.len() < 2 {
                continue;
            }
            let mut pts: Vec<egui::Pos2> = p.pts.iter().map(|q| w2s(q.x, q.y)).collect();
            if p.closed {
                pts.push(pts[0]);
            }
            painter.add(egui::Shape::line(pts, Stroke::new(1.0, c)));
        }
    }
    resp.clicked_by(PointerButton::Primary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::geom::Xf;

    #[test]
    fn another_drawing_is_fitted_drawn_and_a_click_is_reported() {
        let ctx = egui::Context::default();
        let mut scene = Scene::default();
        scene.add(0, Kind::Rect { w: 20.0, h: 10.0 }, Xf::translate(10.0, 10.0));
        scene.add(5, Kind::Rect { w: 5.0, h: 5.0 }, Xf::translate(40.0, 10.0));
        let colour = |l: usize| if l == 5 { None } else { Some(Color32::RED) };
        let look = Look { workspace: Color32::DARK_GRAY, bed: Color32::WHITE, layer_colour: &colour };
        let mut view = None;
        let mut clicked = true;
        let mut input = egui::RawInput::default();
        input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(400.0, 300.0)));
        let mut out = ctx.run_ui(input, |ui| {
            clicked = show_other_doc(ui, &scene, (100.0, 50.0), &look, &mut view);
        });
        out.textures_delta.clear();
        assert!(!clicked);
        let v = view.expect("the view is made on the first frame");
        assert!(v.zoom > 1.0 && v.auto_fit, "the work area is fitted to the pane: {v:?}");
        // The hidden layer draws nothing: only the rectangle of layer 0 (a line of 5 points) and the work area.
        let lines = out.shapes.iter().filter(|s| matches!(s.shape, egui::epaint::Shape::Path(_))).count();
        assert_eq!(lines, 1, "{:?}", out.shapes.iter().map(|s| format!("{:?}", s.shape).chars().take(40).collect::<String>()).collect::<Vec<_>>());
    }
}
