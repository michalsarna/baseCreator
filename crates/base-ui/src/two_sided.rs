//! A job on more than one side in the editor, the same in every program: the switch between the sides (side A and side B of a sheet,
//! the faces A to D of a post or A to F of a block: work areas of one project), whether the ghosts of the other sides are shown, and
//! the ghosts themselves: the shapes of the other sides where they lie to the shown one (`base_core::faces`), drawn faint and never
//! moved; the question of how the G-code of all the sides is saved.
use crate::i18n::tr;
use crate::theme;
use base_core::faces::{Body, Solid};
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
    let c = faces_bar(ui, &Solid::sheet(1.0, 1.0, 1.0, flip), usize::from(shown_b), ghosts);
    Changed { side: c.face.is_some(), ghosts: c.ghosts }
}

/// What the switch of the faces changed.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FaceChanged {
    /// The face to show now.
    pub face: Option<usize>,
    pub ghosts: bool,
}

/// The name of side or face `i` on the switch: Side A, Side B of a sheet; Face A... of a post or a block.
pub fn face_label(solid: &Solid, i: usize) -> String {
    match solid.body {
        Body::Sheet(_) => tr(if i == 0 { "Side A" } else { "Side B" }).to_string(),
        _ => crate::i18n::trf("Face {}", &[&Solid::letter(i)]),
    }
}

/// What the job is, for the head of the switch: a two-sided job, a post or a block.
pub fn job_title(solid: &Solid) -> String {
    tr(match solid.body {
        Body::Sheet(_) => "Two-sided job",
        Body::Post => "Post: 4 faces",
        Body::Block => "Block: 6 faces",
    })
    .to_string()
}

/// Where face `i` is on the object and how big it is, for a tip.
pub fn face_tip(solid: &Solid, i: usize) -> String {
    let f = solid.face(i);
    let fmt = |v: f64| {
        let s = format!("{v:.1}");
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    };
    format!("{} — {} × {} mm, {} {} mm", tr(f.place.name()), fmt(f.w), fmt(f.h), tr("material"), fmt(f.depth))
}

/// The switch of the sides or faces, and **Ghosts of the other sides**. `shown` is the face the program shows now; the program
/// changes it itself when `face` comes back.
pub fn faces_bar(ui: &mut egui::Ui, solid: &Solid, shown: usize, ghosts: &mut bool) -> FaceChanged {
    let mut out = FaceChanged::default();
    ui.horizontal(|ui| {
        ui.add_space(4.0);
        let tip = match solid.body {
            Body::Sheet(flip) => crate::project_setup::flip_name(flip),
            _ => tr("Between the faces A to D the object is rolled a quarter turn back (the front comes up); E and F are its left and its right end turned up.").to_string(),
        };
        ui.label(RichText::new(job_title(solid)).strong()).on_hover_text(tip);
        for i in 0..solid.count() {
            let r = ui.selectable_label(shown == i, face_label(solid, i));
            let r = if solid.count() > 2 { r.on_hover_text(face_tip(solid, i)) } else { r };
            if r.clicked() && shown != i {
                out.face = Some(i);
            }
        }
        let (name, tip) = if solid.count() > 2 {
            ("Ghosts of the other faces", "The shapes of the faces beside this one, unfolded round its edges, and of the face under it, seen through the object; drawn faint. Click one to align or snap to it; it is never moved.")
        } else {
            ("Ghosts of the other side", "The shapes of the other side, turned over as the material is, drawn faint under this side. Click one to align or snap to it; it is never moved.")
        };
        out.ghosts = ui.checkbox(ghosts, tr(name)).on_hover_text(tr(tip)).changed();
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

/// The name of the shown face on the work area (in a corner), with where it is on the object.
pub fn paint_face_name(painter: &egui::Painter, at: Pos2, solid: &Solid, shown: usize) {
    let text = match solid.body {
        Body::Sheet(_) => face_label(solid, shown),
        _ => format!("{} · {}", face_label(solid, shown), tr(solid.face(shown).place.name())),
    };
    painter.text(at, egui::Align2::LEFT_TOP, text, egui::FontId::proportional(14.0), theme::accent());
}

/// The net round the shown face: the outlines of the faces beside it, unfolded over its edges (where their ghosts lie), each with
/// its letter. `to` goes from the millimetres of the drawing (of the shown face, its top left corner at `o`) to the screen.
pub fn paint_net(painter: &egui::Painter, to: &dyn Fn(Pt) -> Pos2, o: Pt, solid: &Solid, shown: usize, colour: Color32) {
    let faint = colour.gamma_multiply(0.55);
    for j in 0..solid.count() {
        let Some(pts) = solid.neighbour(j, shown) else { continue };
        let mut ring: Vec<Pos2> = pts.iter().map(|p| to(Pt::new(p.x + o.x, p.y + o.y))).collect();
        let mid = ring.iter().fold(egui::Vec2::ZERO, |a, q| a + q.to_vec2()) / 4.0;
        ring.push(ring[0]);
        painter.add(egui::Shape::dashed_line(&ring, Stroke::new(1.0, faint), 8.0, 5.0));
        let span = (ring[0] - ring[2]).abs().min_elem();
        let size = (span * 0.35).clamp(10.0, 40.0);
        painter.text(mid.to_pos2(), egui::Align2::CENTER_CENTER, Solid::letter(j), egui::FontId::proportional(size), faint);
    }
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
    /// The words of the choice for a job of `n` sides (translated).
    pub fn label_for(self, n: usize) -> String {
        if n <= 2 {
            return tr(self.label()).to_string();
        }
        match self {
            SaveSides::OneFileWithHold => tr("One file, with a stop before each face to turn the object").to_string(),
            SaveSides::OneFile => tr("One file, without stops").to_string(),
            SaveSides::Separate => crate::i18n::trf("A file for each face: name_A … name_{}", &[&Solid::letter(n - 1)]),
        }
    }
}

/// The question asked by Save G-code (and by Preview G-code) in a two-sided job.
#[derive(Clone, Debug, Default)]
pub struct SaveDialog {
    pub open: bool,
    pub choice: SaveSides,
    /// The G-code is to be shown in a tab (one, or one for each side), not saved.
    pub show: bool,
    /// The alignment marks go into a file (or a tab) of their own as well, when the program offers it.
    pub marks: bool,
}

impl SaveDialog {
    /// Ask before saving.
    pub fn ask_save(&mut self) {
        self.open = true;
        self.show = false;
    }
    /// Ask before showing.
    pub fn ask_show(&mut self) {
        self.open = true;
        self.show = true;
    }
}

/// Where the number of the face is in the key of a tab with G-code text.
const FACE_SHIFT: u32 = usize::BITS - 6;

/// The key of the tab with the G-code text of face `i` of the project `slot`, when the faces are shown apart (face A has the key of
/// the project). [`MARKS_FACE`] is the tab of the alignment marks.
pub fn face_text(slot: usize, i: usize) -> usize {
    slot | ((i & 7) << FACE_SHIFT)
}

/// The face of a tab of G-code text (0 for the tab of the project).
pub fn text_face(key: usize) -> usize {
    (key >> FACE_SHIFT) & 7
}

/// The "face" of the tab with the alignment marks.
pub const MARKS_FACE: usize = 7;

/// The key of the tab with the G-code text of side B, beside the one of the project (`slot`), when the sides are shown apart.
pub const SIDE_B_TEXT: usize = 1 << FACE_SHIFT;

/// The project a G-code text tab belongs to (its key without the face).
pub fn text_slot(key: usize) -> usize {
    key & !(7 << FACE_SHIFT)
}

/// The window of the question; `Some` once the person chose (the window closes), `None` while it is open or after Cancel.
pub fn save_window(ctx: &egui::Context, d: &mut SaveDialog, flip: FlipAxis) -> Option<SaveSides> {
    save_window_for(ctx, d, &Solid::sheet(1.0, 1.0, 1.0, flip), false)
}

/// The question for a job of any number of sides; `marks`: the project has alignment marks, which can go into a file of their own.
pub fn save_window_for(ctx: &egui::Context, d: &mut SaveDialog, solid: &Solid, marks: bool) -> Option<SaveSides> {
    if !d.open {
        return None;
    }
    let n = solid.count();
    let mut out = None;
    let mut open = true;
    let (title, question, button) = match (d.show, n > 2) {
        (true, false) => (tr("G-code of a two-sided job"), tr("The job has two sides. How should the G-code be shown?"), tr("Preview G-code")),
        (false, false) => (tr("Save G-code of a two-sided job"), tr("The job has two sides. How should the G-code be saved?"), tr("Save G-code")),
        (true, true) => (tr("G-code of a job on several faces"), tr("The job has several faces. How should the G-code be shown?"), tr("Preview G-code")),
        (false, true) => (tr("Save G-code of a job on several faces"), tr("The job has several faces. How should the G-code be saved?"), tr("Save G-code")),
    };
    egui::Window::new(title).collapsible(false).resizable(false).open(&mut open).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).show(ctx, |ui| {
        ui.label(question);
        ui.add_space(4.0);
        for c in SaveSides::ALL {
            ui.radio_value(&mut d.choice, c, c.label_for(n));
        }
        if marks {
            ui.add_space(4.0);
            ui.checkbox(&mut d.marks, tr("The alignment marks in a file of their own (name_marks)")).on_hover_text(tr("Burned first, on the sheet under the material, before it is laid down: they lie just outside the material, so they are not in the files of the sides."));
        }
        ui.add_space(4.0);
        let note = match solid.body {
            Body::Sheet(flip) => crate::project_setup::flip_name(flip),
            _ => format!("{} — {}", job_title(solid), (0..n).map(|i| format!("{} {}", Solid::letter(i), tr(solid.face(i).place.name()))).collect::<Vec<_>>().join(", ")),
        };
        ui.label(RichText::new(note).color(theme::text_dim()));
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            if ui.button(button).clicked() {
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
    let mut v = face_paths(chosen, 2).into_iter();
    let a = v.next().unwrap_or_default();
    (a, v.next().unwrap_or_default())
}

/// The name chosen without an ending of a face (`_A`…`_F`) or of the marks (`_marks`), and its extension.
fn stem_ext(chosen: &std::path::Path) -> (String, String) {
    let stem = chosen.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    let ends: Vec<String> = (0..6).map(|i| format!("_{}", Solid::letter(i))).chain(std::iter::once("_marks".to_string())).collect();
    let stem = ends.iter().find_map(|e| stem.strip_suffix(e.as_str())).unwrap_or(&stem).to_string();
    (stem, chosen.extension().map(|e| format!(".{}", e.to_string_lossy())).unwrap_or_default())
}

/// The files of a job of `n` faces saved apart: the name chosen with `_A`, `_B`... at the end of it (not doubled).
pub fn face_paths(chosen: &std::path::Path, n: usize) -> Vec<std::path::PathBuf> {
    let (stem, ext) = stem_ext(chosen);
    (0..n).map(|i| chosen.with_file_name(format!("{stem}_{}{ext}", Solid::letter(i)))).collect()
}

/// The file of the alignment marks beside the files of the job: the name chosen with `_marks` at the end of it.
pub fn marks_path(chosen: &std::path::Path) -> std::path::PathBuf {
    let (stem, ext) = stem_ext(chosen);
    chosen.with_file_name(format!("{stem}_marks{ext}"))
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

    #[test]
    fn the_files_of_the_faces_end_in_their_letters_and_the_marks_have_their_own() {
        let v = face_paths(std::path::Path::new("/x/post_C.nc"), 4);
        let names: Vec<&str> = v.iter().map(|p| p.to_str().unwrap()).collect();
        assert_eq!(names, ["/x/post_A.nc", "/x/post_B.nc", "/x/post_C.nc", "/x/post_D.nc"]);
        assert_eq!(marks_path(std::path::Path::new("/x/job_A.gcode")).to_str(), Some("/x/job_marks.gcode"));
        assert_eq!(marks_path(std::path::Path::new("/x/job_marks.gcode")).to_str(), Some("/x/job_marks.gcode"));
    }

    #[test]
    fn the_tab_of_every_face_belongs_to_its_project() {
        for slot in [0usize, 7, 12345] {
            for i in 0..=MARKS_FACE {
                assert_eq!(text_slot(face_text(slot, i)), slot);
                assert_eq!(text_face(face_text(slot, i)), i);
            }
            assert_eq!(face_text(slot, 0), slot);
            assert_eq!(face_text(slot, 1), slot | SIDE_B_TEXT);
        }
    }

    #[test]
    fn the_tab_of_side_b_belongs_to_its_project() {
        for slot in [0usize, 7, 12345] {
            assert_eq!(text_slot(slot | SIDE_B_TEXT), slot);
            assert_eq!(text_slot(slot), slot);
            assert_ne!(slot | SIDE_B_TEXT, slot);
        }
        let mut d = SaveDialog::default();
        d.ask_show();
        assert!(d.open && d.show);
        d.ask_save();
        assert!(d.open && !d.show);
    }
}
