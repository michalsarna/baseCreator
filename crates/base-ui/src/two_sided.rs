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
        out.ghosts = ui.checkbox(ghosts, tr("Ghosts of the other side")).on_hover_text(tr("The shapes of the other side, turned over as the material is, drawn faint under this side. Click one to align or snap to it; it is never moved.")).changed();
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

/// How the G-code of a two-sided job is saved.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SaveSides {
    /// One file: side A, a stop (M0) to turn the material over, then side B.
    #[default]
    OneFileWithHold,
    /// One file: side A and then side B, with no stop between them.
    OneFile,
    /// Two files, the name of each ending in `_A` and `_B`.
    Separate,
}

impl SaveSides {
    pub const ALL: [SaveSides; 3] = [SaveSides::OneFileWithHold, SaveSides::OneFile, SaveSides::Separate];
    pub fn label(self) -> &'static str {
        match self {
            SaveSides::OneFileWithHold => "One file, with a stop to turn the material over",
            SaveSides::OneFile => "One file, without a stop",
            SaveSides::Separate => "Two files: name_A and name_B",
        }
    }
}

/// The question asked by Save G-code in a two-sided job.
#[derive(Clone, Debug, Default)]
pub struct SaveDialog {
    pub open: bool,
    pub choice: SaveSides,
}

/// The window of the question; `Some` once the person chose (the window closes), `None` while it is open or after Cancel.
pub fn save_window(ctx: &egui::Context, d: &mut SaveDialog, flip: FlipAxis) -> Option<SaveSides> {
    if !d.open {
        return None;
    }
    let mut out = None;
    let mut open = true;
    egui::Window::new(tr("Save G-code of a two-sided job")).collapsible(false).resizable(false).open(&mut open).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.label(tr("The job has two sides. How should the G-code be saved?"));
        ui.add_space(4.0);
        for c in SaveSides::ALL {
            ui.radio_value(&mut d.choice, c, tr(c.label()));
        }
        ui.add_space(4.0);
        ui.label(RichText::new(crate::project_setup::flip_name(flip)).color(theme::text_dim()));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(tr("Save G-code")).clicked() {
                out = Some(d.choice);
            }
            if ui.button(tr("Cancel")).clicked() {
                d.open = false;
            }
        });
    });
    if !open || out.is_some() {
        d.open = false;
    }
    out
}

/// The comment at the head of the G-code of a side (ASCII only: GRBL takes bytes above 127 for commands of its own).
pub fn side_comment(b: bool, flip: FlipAxis) -> String {
    if b {
        format!("Side B: the material is turned over {}", if flip.is_side_to_side() { "side to side" } else { "end to end" })
    } else {
        "Side A".to_string()
    }
}

/// The comment over the stop between the sides of one file (ASCII only).
pub fn hold_comment(flip: FlipAxis) -> String {
    format!("Turn the material over {} and resume to do side B", if flip.is_side_to_side() { "side to side" } else { "end to end" })
}

/// The two files of a job saved side by side: the name chosen with `_A` and `_B` at the end of it (an `_A` or `_B` that is there
/// already is not doubled).
pub fn side_paths(chosen: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf) {
    let stem = chosen.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let stem = stem.strip_suffix("_A").or_else(|| stem.strip_suffix("_B")).unwrap_or(&stem).to_string();
    let ext = chosen.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default();
    (chosen.with_file_name(format!("{stem}_A{ext}")), chosen.with_file_name(format!("{stem}_B{ext}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_files_of_the_sides_end_in_a_and_b() {
        let (a, b) = side_paths(std::path::Path::new("/x/job.nc"));
        assert_eq!((a.to_str().unwrap(), b.to_str().unwrap()), ("/x/job_A.nc", "/x/job_B.nc"));
        let (a, b) = side_paths(std::path::Path::new("/x/job_A.gcode"));
        assert_eq!((a.to_str().unwrap(), b.to_str().unwrap()), ("/x/job_A.gcode", "/x/job_B.gcode"));
        assert!(side_comment(true, FlipAxis::EndToEnd).is_ascii() && hold_comment(FlipAxis::SideToSide).is_ascii());
    }
}
