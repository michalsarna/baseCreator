//! A two-sided job in the editor, the same in every program: the switch between side A and side B (two work areas of one project),
//! whether the ghosts of the other side are shown, and the ghosts themselves: the shapes of the other side, turned over as the
//! material is (`base_core::setup::turn`), drawn faint under the shapes of the shown side and never picked or moved.
use crate::i18n::tr;
use crate::theme;
use base_core::geom::Pt;
use base_core::scene::Shape;
use base_core::setup::FlipAxis;
use egui::{Color32, Pos2, RichText, Stroke};

/// What the switch changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Changed {
    /// The other side is to be shown.
    pub side: bool,
    pub ghosts: bool,
}

/// The switch: **Side A** and **Side B**, and **Ghosts of the other side**. `shown_b` is the side the program shows now; the program
/// swaps the sides itself when `side` comes back true.
pub fn bar(ui: &mut egui::Ui, shown_b: bool, ghosts: &mut bool, flip: FlipAxis) -> Changed {
    let mut out = Changed::default();
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        ui.label(RichText::new(tr("Two-sided job")).strong()).on_hover_text(crate::project_setup::flip_name(flip));
        for (b, name) in [(false, tr("Side A")), (true, tr("Side B"))] {
            if ui.selectable_label(shown_b == b, name).clicked() && shown_b != b {
                out.side = true;
            }
        }
        out.ghosts = ui.checkbox(ghosts, tr("Ghosts of the other side")).on_hover_text(tr("The shapes of the other side, turned over as the material is, drawn faint under this side: they cannot be picked or moved.")).changed();
    });
    out
}

/// The colour of the ghosts on a work area of colour `bg`.
pub fn ghost_colour(bg: Color32) -> Color32 {
    let lum = 0.299 * bg.r() as f32 + 0.587 * bg.g() as f32 + 0.114 * bg.b() as f32;
    if lum > 128.0 {
        Color32::from_rgba_unmultiplied(0x1e, 0x5a, 0xb4, 190)
    } else {
        Color32::from_rgba_unmultiplied(0x9c, 0xc4, 0xf0, 200)
    }
}

/// Draw the ghosts (already turned over) with `to` from design millimetres to the screen.
pub fn paint_ghosts(painter: &egui::Painter, to: &dyn Fn(Pt) -> Pos2, shapes: &[Shape], colour: Color32) {
    let stroke = Stroke::new(1.6, colour);
    for s in shapes {
        for poly in s.polys() {
            let mut pts: Vec<Pos2> = poly.pts.iter().map(|p| to(*p)).collect();
            if poly.closed {
                if let Some(first) = pts.first().copied() {
                    pts.push(first);
                }
            }
            if pts.len() >= 2 {
                painter.add(egui::Shape::dashed_line(&pts, stroke, 5.0, 3.0));
            }
        }
    }
}

/// The name of the side on the work area (in a corner).
pub fn paint_side_name(painter: &egui::Painter, at: Pos2, shown_b: bool) {
    let text = if shown_b { tr("Side B") } else { tr("Side A") };
    painter.text(at, egui::Align2::LEFT_TOP, text, egui::FontId::proportional(14.0), theme::accent());
}
