//! The window of the project setup, the same in every program that works on a piece of material (File → Project setup…, and after
//! File → New): a block (its size, thickness, the zero of the program, how many of its sides are worked on: one, two, the four long
//! faces of a post or the six faces of a block) or a cylinder on the rotary attachment (its diameter and length), with pictures of
//! it. The program puts the answer into its project (`Setup` holds what was chosen).
use crate::i18n::tr;
use crate::theme;
use crate::units_ui::drag_len;
use crate::window::popup;
use base_core::faces::{Body, Solid};
use base_core::setup::{FlipAxis, ZeroPoint};
use base_core::units::Units;
use egui::{Color32, Pos2, RichText, Stroke};

/// The thickest material.
pub const MAX_THICKNESS: f64 = 500.0;

/// What the window shows and what was chosen in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Setup {
    pub open: bool,
    pub w: f64,
    pub h: f64,
    pub t: f64,
    pub zero: ZeroPoint,
    pub z_bottom: bool,
    pub work_area: bool,
    /// A cylinder on the rotary attachment instead of a block.
    pub round: bool,
    /// The diameter and the length of the cylinder (mm).
    pub diameter: f64,
    pub length: f64,
    /// How many sides are worked on (a block only). Two: the material turned over round `flip`, with alignment marks when `marks`
    /// is set.
    pub sides: Sides,
    pub flip: FlipAxis,
    pub marks: bool,
    /// A post: the sides `a` and `b` of its base and its height (mm).
    pub post: [f64; 3],
    /// A block: `a` (from left to right), `b` (from the back to the front) and `c` (up), mm.
    pub block: [f64; 3],
    /// The largest piece the machine takes (its work area), for the list of paper sizes.
    pub max: (f64, f64),
    /// Opened for a new project (the start window): Skip, or closing the window, leaves the project on one side and as big as the
    /// work area of the machine (`Answer::Skip` tells the program to do so).
    pub start: bool,
}

impl Default for Setup {
    fn default() -> Setup {
        Setup { open: false, w: 200.0, h: 150.0, t: 10.0, zero: ZeroPoint::FrontLeft, z_bottom: false, work_area: false, round: false, diameter: 50.0, length: 150.0, sides: Sides::One, flip: FlipAxis::SideToSide, marks: true, post: [40.0, 40.0, 300.0], block: [100.0, 80.0, 60.0], max: (f64::INFINITY, f64::INFINITY), start: false }
    }
}

/// How many sides of the material are worked on.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Sides {
    #[default]
    One,
    /// A sheet turned over: side A and side B.
    Two,
    /// The four long faces of a post.
    Post,
    /// The six faces of a block.
    Block,
}

impl Sides {
    pub fn label(self) -> &'static str {
        match self {
            Sides::One => "1 side",
            Sides::Two => "2 sides",
            Sides::Post => "4 sides (post)",
            Sides::Block => "6 sides (block)",
        }
    }
    /// More than one side: every side has a work area of its own, and the work area is the material.
    pub fn many(self) -> bool {
        self != Sides::One
    }
    /// What a project with `solid` (or none) was set up as.
    pub fn of(solid: Option<&Solid>) -> Sides {
        match solid.map(|s| s.body) {
            None => Sides::One,
            Some(Body::Sheet(_)) => Sides::Two,
            Some(Body::Post) => Sides::Post,
            Some(Body::Block) => Sides::Block,
        }
    }
}

impl Setup {
    /// The object of a job on more than one side, from what was chosen (none for one side).
    pub fn solid(&self) -> Option<Solid> {
        match self.sides {
            Sides::One => None,
            Sides::Two => Some(Solid::sheet(self.w, self.h, self.t, self.flip)),
            Sides::Post => Some(Solid::post(self.post[0], self.post[1], self.post[2])),
            Sides::Block => Some(Solid::block(self.block[0], self.block[1], self.block[2])),
        }
    }
    /// Take the sizes of an object the project has already.
    pub fn take_solid(&mut self, solid: Option<&Solid>) {
        self.sides = Sides::of(solid);
        if let Some(s) = solid {
            match s.body {
                Body::Sheet(flip) => self.flip = flip,
                Body::Post => self.post = s.size,
                Body::Block => self.block = s.size,
            }
        }
    }
}

/// What a program asks of the window.
#[derive(Clone, Copy)]
pub struct Kind {
    /// The zero of the program and Z counted from the bottom (a mill); without them the material only lies on the work area.
    pub zero: bool,
    /// A cylinder on the rotary attachment can be chosen.
    pub cylinder: bool,
    /// What the alignment marks are in this program (the tip of their switch).
    pub marks_tip: &'static str,
    /// The words under the block.
    pub block_note: &'static str,
    /// The words under the cylinder.
    pub cylinder_note: &'static str,
    /// The colour of the material in the pictures.
    pub colour: [u8; 3],
    /// The piece is a sheet of paper: a list of paper sizes (those that fit the machine), and no thickness.
    pub paper: bool,
    /// The four faces of a post and the six of a block can be chosen.
    pub solids: bool,
}

/// The usual sizes of paper (portrait, mm).
pub const PAPER: [(&str, f64, f64); 11] = [
    ("A6", 105.0, 148.0),
    ("A5", 148.0, 210.0),
    ("A4", 210.0, 297.0),
    ("A3", 297.0, 420.0),
    ("A2", 420.0, 594.0),
    ("A1", 594.0, 841.0),
    ("B5", 176.0, 250.0),
    ("B4", 250.0, 353.0),
    ("Letter", 215.9, 279.4),
    ("Legal", 215.9, 355.6),
    ("Tabloid", 279.4, 431.8),
];

/// The paper sizes that fit a machine of `max`, each as (name, width, height): upright when it fits so, otherwise lying.
pub fn papers_that_fit(max: (f64, f64)) -> Vec<(String, f64, f64)> {
    let fits = |w: f64, h: f64| w <= max.0 + 1e-6 && h <= max.1 + 1e-6;
    let mut out = vec![];
    for (name, w, h) in PAPER {
        if fits(w, h) {
            out.push((format!("{name} ↕"), w, h));
        }
        if fits(h, w) {
            out.push((format!("{name} ↔"), h, w));
        }
    }
    out
}

/// What was pressed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Answer {
    Apply,
    /// Skip, or the window closed.
    Skip,
}

/// The window, while `st.open`. Returns what was pressed (the window is then closed).
pub fn window(ctx: &egui::Context, st: &mut Setup, units: Units, kind: Kind) -> Option<Answer> {
    if !st.open {
        return None;
    }
    let mut open = true;
    let (mut apply, mut skip) = (false, false);
    let title = tr("Project setup…").trim_end_matches('…').to_string();
    // As tall as the pictures ask, but not taller than the screen (the body scrolls then).
    let room = ctx.content_rect().height() - 40.0;
    let size = if kind.solids { [700.0, 900.0f32.min(room)] } else { [620.0, 800.0f32.min(room)] };
    // A new id: the size kept by egui for the old, smaller window does not come back.
    popup(ctx, "setup_window_2", &title, size, &mut open, |ui| {
        // Apply and Skip at the bottom of the window, under a line, in the middle; what is above them scrolls when it is too tall.
        egui::Panel::bottom("setup_buttons").frame(egui::Frame::NONE).resizable(false).show_separator_line(true).show(ui, |ui| {
            ui.add_space(10.0);
            (apply, skip) = answer_buttons(ui);
            ui.add_space(2.0);
        });
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| setup_body(ui, st, units, kind));
    });
    if apply {
        st.open = false;
        Some(Answer::Apply)
    } else if skip || !open {
        st.open = false;
        Some(Answer::Skip)
    } else {
        None
    }
}

/// The green Apply and the blue Skip, half as big again as a button, side by side in the middle. Returns which was clicked.
pub fn answer_buttons(ui: &mut egui::Ui) -> (bool, bool) {
    let size = ui.style().text_styles.get(&egui::TextStyle::Button).map_or(14.0, |f| f.size) * 1.5;
    let height = ui.spacing().interact_size.y * 1.5;
    let big = move |text: &'static str, fill: Color32| egui::Button::new(RichText::new(tr(text)).size(size).color(Color32::WHITE)).fill(fill).min_size(egui::vec2(132.0, height)).corner_radius(5.0);
    let gap = ui.spacing().item_spacing.x * 3.0;
    let width = 2.0 * 132.0 + gap;
    let mut out = (false, false);
    ui.horizontal(|ui| {
        ui.add_space(((ui.available_width() - width) / 2.0).max(0.0));
        out.0 = ui.add(big("Apply", APPLY_GREEN)).clicked();
        ui.add_space(gap - ui.spacing().item_spacing.x);
        out.1 = ui.add(big("Skip", SKIP_BLUE)).clicked();
    });
    out
}

/// The colours of the two answers of the start window.
pub const APPLY_GREEN: Color32 = Color32::from_rgb(0x2e, 0x9b, 0x4c);
pub const SKIP_BLUE: Color32 = Color32::from_rgb(0x2e, 0x74, 0xc8);

/// What is above the answers: the shape, the sides, the picture and the sizes.
fn setup_body(ui: &mut egui::Ui, st: &mut Setup, units: Units, kind: Kind) {
    {
        if kind.cylinder {
            ui.horizontal(|ui| {
                ui.label(tr("Shape of the material"));
                ui.selectable_value(&mut st.round, false, tr(if kind.paper { "Sheet" } else { "Block" }));
                ui.selectable_value(&mut st.round, true, tr("Cylinder (rotary)")).on_hover_text(tr("A round workpiece turned by the rotary attachment of the device. Applying switches the rotary attachment on with this diameter."));
            });
            ui.add_space(4.0);
        } else {
            st.round = false;
        }
        if st.round {
            cylinder_picture(ui, st, kind.colour, units, kind.zero);
            ui.add_space(4.0);
            egui::Grid::new("setup_grid_round").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                ui.label(tr("Workpiece diameter"));
                drag_len(ui, units, &mut st.diameter, 0.5, Some((1.0, 1000.0)));
                ui.end_row();
                ui.label(tr("Workpiece length"));
                drag_len(ui, units, &mut st.length, 1.0, Some((1.0, 20_000.0)));
                ui.end_row();
                ui.label(tr("Circumference"));
                ui.label(crate::units_ui::fmt_len(units, std::f64::consts::PI * st.diameter, 1)).on_hover_text(tr("The way round the workpiece: the design may not be higher."));
                ui.end_row();
                if kind.zero {
                    zero_row(ui, st, "setup_zero_round");
                }
                ui.label("");
                ui.checkbox(&mut st.work_area, tr("Work area is the material"));
                ui.end_row();
            });
            ui.label(RichText::new(tr(kind.cylinder_note)).color(theme::text_dim()));
        } else {
            sides_row(ui, st, kind);
            ui.add_space(4.0);
            match st.sides {
                Sides::Post | Sides::Block => solid_part(ui, st, units, kind),
                Sides::One | Sides::Two => sheet_part(ui, st, units, kind),
            }
        }
    }
}

/// The choice of how many sides are worked on: one, two (turned over), the four faces of a post or the six of a block.
fn sides_row(ui: &mut egui::Ui, st: &mut Setup, kind: Kind) {
    let all: &[Sides] = if kind.solids { &[Sides::One, Sides::Two, Sides::Post, Sides::Block] } else { &[Sides::One, Sides::Two] };
    if !all.contains(&st.sides) {
        st.sides = Sides::One;
    }
    let before = st.sides;
    ui.horizontal_wrapped(|ui| {
        ui.label(tr("Sides to work on"));
        for s in all {
            let tip = match s {
                Sides::One => "The material is worked on from the top only.",
                Sides::Two => "Side A and side B are two work areas: the material is cut on one side, turned over and cut on the other. Each side shows the shapes of the other one turned over, as ghosts.",
                Sides::Post => "A post lying along X, worked on its four long faces A, B, C and D: it is rolled a quarter turn back between them. Each face is a work area of its own; the faces beside it are unfolded round it and the one under it shows through, as ghosts.",
                Sides::Block => "A block worked on all its six faces: A to D rolled a quarter turn back each time, E and F the left and the right end turned up. Each face is a work area of its own; the faces beside it are unfolded round it and the one under it shows through, as ghosts.",
            };
            ui.selectable_value(&mut st.sides, *s, tr(s.label())).on_hover_text(tr(tip));
        }
    });
    // Back to one side: the material is as big as the work area of the machine to begin with.
    if before.many() && st.sides == Sides::One && st.max.0.is_finite() && st.max.1.is_finite() && !kind.paper {
        st.w = st.max.0;
        st.h = st.max.1;
    }
}

/// "Work area is the material": a choice for one side; always on (and greyed) when more sides are worked on, so that every side
/// lies on the machine the same way, in the corner of the zero.
fn work_area_row(ui: &mut egui::Ui, st: &mut Setup, kind: Kind) {
    ui.label("");
    if st.sides.many() && !kind.paper {
        st.work_area = true;
        ui.add_enabled(false, egui::Checkbox::new(&mut true, tr("Work area is the material"))).on_disabled_hover_text(tr("Always on when more than one side is worked on: every side has a work area of its own, as big as the material is there, and the material lies the same way on the machine for each of them."));
    } else {
        ui.checkbox(&mut st.work_area, tr("Work area is the material"));
    }
    ui.end_row();
}

/// A sheet or a block worked on one side or on two.
fn sheet_part(ui: &mut egui::Ui, st: &mut Setup, units: Units, kind: Kind) {
    block_picture(ui, &Setup { t: if kind.paper { 0.2 } else { st.t }, ..st.clone() }, kind.colour, units, kind.zero);
    ui.add_space(4.0);
    egui::Grid::new("setup_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Material width"));
        drag_len(ui, units, &mut st.w, 1.0, Some((1.0, 20_000.0)));
        ui.end_row();
        ui.label(tr("Material height"));
        drag_len(ui, units, &mut st.h, 1.0, Some((1.0, 20_000.0)));
        ui.end_row();
        if kind.paper {
            ui.label(tr("Paper size"));
            let list = papers_that_fit(st.max);
            let now = list.iter().find(|(_, w, h)| (w - st.w).abs() < 0.05 && (h - st.h).abs() < 0.05).map_or_else(|| tr("Custom").to_string(), |(n, _, _)| n.clone());
            egui::ComboBox::from_id_salt("setup_paper").selected_text(now).show_ui(ui, |ui| {
                for (name, w, h) in &list {
                    if ui.selectable_label((w - st.w).abs() < 0.05 && (h - st.h).abs() < 0.05, format!("{name}  {} × {}", crate::units_ui::fmt_len(units, *w, 1), crate::units_ui::fmt_len(units, *h, 1))).clicked() {
                        st.w = *w;
                        st.h = *h;
                    }
                }
            }).response.on_hover_text(tr("The sizes that fit the work area of the machine: ↕ upright, ↔ lying."));
            ui.end_row();
        } else {
            ui.label(tr("Thickness"));
            drag_len(ui, units, &mut st.t, 0.5, Some((0.1, MAX_THICKNESS)));
            ui.end_row();
        }
        if kind.zero {
            zero_row(ui, st, "setup_zero");
            ui.label("");
            ui.checkbox(&mut st.z_bottom, tr("Z counts from the bottom of the material"));
            ui.end_row();
        }
        if !kind.paper {
            work_area_row(ui, st, kind);
        }
        if st.sides == Sides::Two {
            ui.label(tr("Turned over round"));
            egui::ComboBox::from_id_salt("setup_flip").selected_text(flip_name(st.flip)).width(220.0).show_ui(ui, |ui| {
                for f in FlipAxis::ALL {
                    ui.selectable_value(&mut st.flip, f, flip_name(f));
                }
            });
            ui.end_row();
            ui.label("");
            ui.checkbox(&mut st.marks, tr("Alignment marks")).on_hover_text(tr(kind.marks_tip));
            ui.end_row();
        }
    });
    ui.label(RichText::new(tr(kind.block_note)).color(theme::text_dim()));
}

/// A post (four faces) or a block (six): its picture, the faces unfolded, its sizes.
fn solid_part(ui: &mut egui::Ui, st: &mut Setup, units: Units, kind: Kind) {
    let post = st.sides == Sides::Post;
    if let Some(solid) = st.solid() {
        // The object and its net side by side, half as big again as they were, in the middle.
        let full = ui.available_width();
        let w = full.min(860.0);
        ui.horizontal(|ui| {
            ui.add_space(((full - w) / 2.0).max(0.0));
            ui.allocate_ui(egui::vec2(w * 0.6, 285.0), |ui| solid_picture(ui, &solid, kind.colour, units, kind.zero.then_some((st.zero, st.z_bottom))));
            ui.allocate_ui(egui::vec2(w * 0.4 - ui.spacing().item_spacing.x, 285.0), |ui| {
                net_picture(ui, &solid, kind.colour, None);
            });
        });
    }
    ui.add_space(4.0);
    egui::Grid::new("setup_grid_solid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        let (names, v) = if post { (["Base side a", "Base side b", "Height of the post H"], &mut st.post) } else { (["Side a (left to right)", "Side b (back to front)", "Side c (height)"], &mut st.block) };
        for (k, name) in names.iter().enumerate() {
            ui.label(tr(name));
            drag_len(ui, units, &mut v[k], 0.5, Some((1.0, 20_000.0)));
            ui.end_row();
        }
        if kind.zero {
            zero_row(ui, st, "setup_zero_solid");
            ui.label("");
            ui.checkbox(&mut st.z_bottom, tr("Z counts from the bottom of the material"));
            ui.end_row();
        }
        work_area_row(ui, st, kind);
    });
    let note = if post {
        "The post lies along X with face A up, in the corner of the zero. Between the faces it is rolled a quarter turn back, so that the front comes up: faces A, B, C and D. Each face is drawn on a work area as big as the face, and the material under it is as thick as the post is deep there."
    } else {
        "The block lies with face A up, in the corner of the zero. Faces A to D: it is rolled a quarter turn back each time, so that the front comes up; E and F: its left and its right end turned up. Each face is drawn on a work area as big as the face, and the material under it is as thick as the block is deep there."
    };
    ui.label(RichText::new(tr(note)).color(theme::text_dim()));
}

/// The name of an axis in the list: H or V and what changes places.
pub fn flip_name(f: FlipAxis) -> String {
    match f {
        FlipAxis::EndToEnd => format!("H — {}", tr("the horizontal axis (back and front change places)")),
        FlipAxis::SideToSide => format!("V — {}", tr("the vertical axis (left and right change places)")),
    }
}

fn zero_row(ui: &mut egui::Ui, st: &mut Setup, id: &str) {
    ui.label(tr("Zero of the program"));
    egui::ComboBox::from_id_salt(id).selected_text(tr(st.zero.label())).show_ui(ui, |ui| {
        for z in ZeroPoint::ALL {
            ui.selectable_value(&mut st.zero, z, tr(z.label()));
        }
    });
    ui.end_row();
}

fn shade(colour: [u8; 3], f: f32) -> Color32 {
    Color32::from_rgb((colour[0] as f32 * f) as u8, (colour[1] as f32 * f) as u8, (colour[2] as f32 * f) as u8)
}

/// The picture of the block: the material seen at a slant with its width, height and thickness, the zero of the program with its
/// axes (when the program has one), and for a two-sided job the line it is turned over round. The block is drawn thicker than it is
/// when it is thin, so that it can be seen; the numbers are the real ones.
pub fn block_picture(ui: &mut egui::Ui, st: &Setup, colour: [u8; 3], units: Units, with_zero: bool) {
    let rect = picture_rect(ui, 570.0, 315.0);
    let p = ui.painter_at(rect);
    let big = st.w.max(st.h).max(1e-6);
    let (w, h) = (st.w / big, st.h / big);
    let t = (st.t / big).clamp(0.1, 0.6);
    // A slanted view: X to the right, Y back and up at a slant, Z up; the block in the middle with room for the dimensions.
    let (sx, sy) = (0.55, 0.32);
    let (span_x, span_y) = (w + h * sx, h * sy + t);
    let k = ((rect.width() as f64 - 110.0) / span_x).min((rect.height() as f64 - 70.0) / span_y).max(1.0);
    let o = Pos2::new(rect.center().x - (span_x * k / 2.0) as f32 - 10.0, rect.center().y + (span_y * k / 2.0) as f32 - 6.0);
    let at = |q: [f64; 3]| Pos2::new(o.x + ((q[0] + q[1] * sx) * k) as f32, o.y - ((q[1] * sy + q[2]) * k) as f32);
    let edge = Stroke::new(1.0, shade(colour, 0.45));
    let face = |pts: [[f64; 3]; 4], f: f32| p.add(egui::Shape::convex_polygon(pts.iter().map(|q| at(*q)).collect(), shade(colour, f), edge));
    face([[0.0, 0.0, 0.0], [w, 0.0, 0.0], [w, 0.0, t], [0.0, 0.0, t]], 0.82);
    face([[w, 0.0, 0.0], [w, h, 0.0], [w, h, t], [w, 0.0, t]], 0.68);
    face([[0.0, 0.0, t], [w, 0.0, t], [w, h, t], [0.0, h, t]], 1.0);
    // The width under the front, the height along the right side, the thickness at the back right (away from the zero).
    let ink = crate::dims::DimStyle::on(theme::panel());
    let n = |v: f64| crate::dims::number(units, v);
    crate::dims::dim(&p, at([0.0, 0.0, 0.0]), at([w, 0.0, 0.0]), egui::vec2(0.0, 1.0), 16.0, &n(st.w), &ink);
    let along = (at([w, h, 0.0]) - at([w, 0.0, 0.0])).normalized();
    crate::dims::dim(&p, at([w, 0.0, 0.0]), at([w, h, 0.0]), egui::vec2(-along.y, along.x), 16.0, &n(st.h), &ink);
    crate::dims::dim(&p, at([w, h, 0.0]), at([w, h, t]), egui::vec2(1.0, 0.0), 14.0, &n(st.t), &ink);
    // A two-sided job: the middle line the material is turned over round, dashed over the top, with A and B on its two sides.
    if st.sides == Sides::Two {
        let line = match st.flip {
            FlipAxis::SideToSide => [[w / 2.0, -0.08, t], [w / 2.0, h + 0.08, t]],
            FlipAxis::EndToEnd => [[-0.08, h / 2.0, t], [w + 0.08, h / 2.0, t]],
        };
        let flip_ink = Color32::from_rgb(0x2e, 0x7d, 0xd0);
        let (a, b) = (at(line[0]), at(line[1]));
        let steps = 14;
        for i in (0..steps).step_by(2) {
            let f0 = i as f32 / steps as f32;
            let f1 = (i + 1) as f32 / steps as f32;
            p.line_segment([a + (b - a) * f0, a + (b - a) * f1], Stroke::new(2.0, flip_ink));
        }
        p.text(b + (b - a).normalized() * 10.0, egui::Align2::CENTER_CENTER, st.flip.short(), egui::FontId::proportional(13.0), flip_ink);
    }
    if !with_zero {
        return;
    }
    // The zero of the program and its axes.
    let (zx, zy) = match st.zero {
        ZeroPoint::FrontLeft => (0.0, 0.0),
        ZeroPoint::FrontRight => (w, 0.0),
        ZeroPoint::BackLeft => (0.0, h),
        ZeroPoint::BackRight => (w, h),
        ZeroPoint::Center => (w / 2.0, h / 2.0),
    };
    let zz = if st.z_bottom { 0.0 } else { t };
    let zero = at([zx, zy, zz]);
    let len = 0.18;
    for (d, name, c) in [([len, 0.0, 0.0], "X", Color32::from_rgb(0xd0, 0x30, 0x30)), ([0.0, len, 0.0], "Y", Color32::from_rgb(0x30, 0xa0, 0x30)), ([0.0, 0.0, len], "Z", Color32::from_rgb(0x30, 0x60, 0xd0))] {
        let end = at([zx + d[0], zy + d[1], zz + d[2]]);
        p.line_segment([zero, end], Stroke::new(2.0, c));
        p.text(end + (end - zero).normalized() * 8.0, egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(12.0), c);
    }
    p.circle_filled(zero, 4.5, Color32::from_rgb(0xe0, 0x30, 0x30));
    p.circle_stroke(zero, 4.5, Stroke::new(1.2, Color32::WHITE));
}

/// The place of a picture of the window: `w` × `h` (narrower when the room is), in the middle of the width.
fn picture_rect(ui: &mut egui::Ui, w: f32, h: f32) -> egui::Rect {
    let (row, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::hover());
    egui::Rect::from_center_size(row.center(), egui::vec2(w.min(row.width()), h))
}

/// The letters of the faces drawn on the pictures.
fn letter_ink() -> Color32 {
    Color32::from_rgb(0x2a, 0x1c, 0x0c)
}

/// The picture of a post or a block: the object seen at a slant as it lies for face A, with the letters of the faces that are seen
/// (the others are on the net beside it), the arrow of the quarter turn back between the faces A to D, its sizes and the zero of
/// the program (when the program has one: `zero`, with Z from the bottom).
pub fn solid_picture(ui: &mut egui::Ui, solid: &Solid, colour: [u8; 3], units: Units, zero: Option<(ZeroPoint, bool)>) {
    let rect = picture_rect(ui, 570.0, 285.0);
    let p = ui.painter_at(rect);
    let [bx, by, bz] = solid.bbox();
    let big = bx.max(by).max(bz).max(1e-6);
    // A long thin post is drawn thicker than it is, so that its faces can be seen; the numbers are the real ones.
    let (w, d, t) = (bx / big, (by / big).max(0.3), (bz / big).max(0.22));
    // As in the picture of the block: X to the right, the depth back and up at a slant, Z up; room for the sizes and the arrow.
    let (sx, sy) = (0.55, 0.32);
    let (span_x, span_y) = (w + d * sx, d * sy + t);
    // Room above and beside for the arrow of the quarter turn, which goes round higher than the object.
    let k = ((rect.width() as f64 - 130.0) / span_x).min((rect.height() as f64 - 56.0) * 0.72 / span_y).max(1.0);
    let o = Pos2::new(rect.center().x - (span_x * k / 2.0) as f32 - 30.0, rect.center().y + (span_y * k / 2.0) as f32 - 2.0);
    // q: [x, the depth from the front back, z].
    let at = |q: [f64; 3]| Pos2::new(o.x + ((q[0] + q[1] * sx) * k) as f32, o.y - ((q[1] * sy + q[2]) * k) as f32);
    let edge = Stroke::new(1.0, shade(colour, 0.45));
    let face = |pts: [[f64; 3]; 4], f: f32, letter: Option<char>| {
        let poly: Vec<Pos2> = pts.iter().map(|q| at(*q)).collect();
        let mid = poly.iter().fold(egui::Vec2::ZERO, |a, q| a + q.to_vec2()) / 4.0;
        p.add(egui::Shape::convex_polygon(poly, shade(colour, f), edge));
        if let Some(l) = letter {
            p.text(mid.to_pos2(), egui::Align2::CENTER_CENTER, l, egui::FontId::proportional(16.0), letter_ink());
        }
    };
    let block = solid.body == Body::Block;
    face([[0.0, 0.0, 0.0], [w, 0.0, 0.0], [w, 0.0, t], [0.0, 0.0, t]], 0.82, Some('B'));
    face([[w, 0.0, 0.0], [w, d, 0.0], [w, d, t], [w, 0.0, t]], 0.62, block.then_some('F'));
    face([[0.0, 0.0, t], [w, 0.0, t], [w, d, t], [0.0, d, t]], 1.0, Some('A'));
    // The end of a post is not worked on: its grain.
    if !block {
        let c = at([w, d / 2.0, t / 2.0]);
        for f in [0.18f32, 0.34] {
            let r = (at([w, d, t]) - at([w, 0.0, 0.0])).length() * f;
            p.add(egui::Shape::ellipse_stroke(c, egui::vec2(r * 0.55, r * 0.7), Stroke::new(1.0, shade(colour, 0.45))));
        }
    }
    // The quarter turn back round the length (X), drawn round the right end: the front comes up.
    let blue = Color32::from_rgb(0x2e, 0x7d, 0xd0);
    let r = d.max(t) * 0.78;
    let arc: Vec<Pos2> = (0..=24)
        .map(|i| {
            let th = (150.0 - 120.0 * i as f64 / 24.0f64).to_radians();
            // th = 180° is the front, 90° the top, 0° the back.
            at([w + 0.06 + r * 0.9, d / 2.0 + r * th.cos(), t / 2.0 + r * th.sin()])
        })
        .collect();
    let (tip, before) = (arc[arc.len() - 1], arc[arc.len() - 3]);
    p.add(egui::Shape::line(arc.clone(), Stroke::new(2.0, blue)));
    let dir = (tip - before).normalized();
    let side = egui::vec2(-dir.y, dir.x);
    p.add(egui::Shape::convex_polygon(vec![tip + dir * 5.0, tip - dir * 4.0 + side * 4.5, tip - dir * 4.0 - side * 4.5], blue, Stroke::NONE));
    p.text(arc[0] + egui::vec2(-4.0, 2.0), egui::Align2::RIGHT_TOP, "¼", egui::FontId::proportional(13.0), blue);
    // The sizes: along the front, along the right side and up at the back right.
    let ink = crate::dims::DimStyle::on(theme::panel());
    let n = |v: f64| crate::dims::number(units, v);
    let names = match solid.body {
        Body::Post => ["H", "a", "b"],
        _ => ["a", "b", "c"],
    };
    crate::dims::dim(&p, at([0.0, 0.0, 0.0]), at([w, 0.0, 0.0]), egui::vec2(0.0, 1.0), 16.0, &format!("{} = {}", names[0], n(bx)), &ink);
    let along = (at([w, d, 0.0]) - at([w, 0.0, 0.0])).normalized();
    crate::dims::dim(&p, at([w, 0.0, 0.0]), at([w, d, 0.0]), egui::vec2(-along.y, along.x), 16.0, &format!("{} = {}", names[1], n(by)), &ink);
    crate::dims::dim(&p, at([0.0, 0.0, 0.0]), at([0.0, 0.0, t]), egui::vec2(-1.0, 0.0), 12.0, &format!("{} = {}", names[2], n(bz)), &ink);
    let Some((zp, z_bottom)) = zero else { return };
    let (zx, zy) = match zp {
        ZeroPoint::FrontLeft => (0.0, 0.0),
        ZeroPoint::FrontRight => (w, 0.0),
        ZeroPoint::BackLeft => (0.0, d),
        ZeroPoint::BackRight => (w, d),
        ZeroPoint::Center => (w / 2.0, d / 2.0),
    };
    let zz = if z_bottom { 0.0 } else { t };
    let z0 = at([zx, zy, zz]);
    p.circle_filled(z0, 4.5, Color32::from_rgb(0xe0, 0x30, 0x30));
    p.circle_stroke(z0, 4.5, Stroke::new(1.2, Color32::WHITE));
}

/// Where the faces of an object lie on its net (unfolded round face A, as the ghosts are round the shown face): their outlines in
/// the drawing of face A. The face on the other side of A comes after the front one; a sheet has its side B beside A.
pub fn net_rects(solid: &Solid) -> Vec<(usize, egui::Rect)> {
    let fa = solid.face(0);
    let rect = |pts: [base_core::geom::Pt; 4]| {
        let (mut lo, mut hi) = (Pos2::new(f32::MAX, f32::MAX), Pos2::new(f32::MIN, f32::MIN));
        for q in pts {
            lo = lo.min(Pos2::new(q.x as f32, q.y as f32));
            hi = hi.max(Pos2::new(q.x as f32, q.y as f32));
        }
        egui::Rect::from_min_max(lo, hi)
    };
    let mut out = vec![(0, rect(fa.corners()))];
    for j in 1..solid.count() {
        let r = match solid.neighbour(j, 0) {
            Some(pts) => rect(pts),
            None => {
                let f = solid.face(j);
                match solid.body {
                    // Side B of a sheet beside A, with a gap.
                    Body::Sheet(_) => egui::Rect::from_min_size(Pos2::new((fa.w * 1.15) as f32, 0.0), egui::vec2(f.w as f32, f.h as f32)),
                    // Under the front face (B), as the faces follow when it is rolled.
                    _ => {
                        let b = solid.face(1);
                        egui::Rect::from_min_size(Pos2::new(0.0, (fa.h + b.h) as f32), egui::vec2(f.w as f32, f.h as f32))
                    }
                }
            }
        };
        out.push((j, r));
    }
    out
}

/// The net of the object: every face with its letter, `shown` marked. Returns the face that was clicked.
pub fn net_picture(ui: &mut egui::Ui, solid: &Solid, colour: [u8; 3], shown: Option<usize>) -> Option<usize> {
    let size = egui::vec2(ui.available_width().max(60.0), ui.available_height().clamp(60.0, 285.0));
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let p = ui.painter_at(rect);
    let rects = net_rects(solid);
    let all = rects.iter().fold(egui::Rect::NOTHING, |a, (_, r)| a.union(*r));
    let k = ((rect.width() - 16.0) / all.width().max(1e-3)).min((rect.height() - 24.0) / all.height().max(1e-3));
    let place = |r: egui::Rect| {
        let o = rect.center() - all.center().to_vec2() * k;
        egui::Rect::from_min_max(o + r.min.to_vec2() * k, o + r.max.to_vec2() * k)
    };
    let mut clicked = None;
    for (j, r) in &rects {
        let sr = place(*r);
        let on = shown == Some(*j);
        p.rect(sr, 1.0, shade(colour, if *j == 0 { 1.0 } else { 0.86 }), Stroke::new(if on { 2.5 } else { 1.0 }, if on { theme::accent() } else { shade(colour, 0.45) }), egui::StrokeKind::Inside);
        p.text(sr.center(), egui::Align2::CENTER_CENTER, base_core::faces::Solid::letter(*j), egui::FontId::proportional((sr.height().min(sr.width()) * 0.5).clamp(9.0, 16.0)), letter_ink());
        if resp.clicked() && resp.interact_pointer_pos().is_some_and(|q| sr.contains(q)) {
            clicked = Some(*j);
        }
    }
    p.text(rect.center_bottom() + egui::vec2(0.0, -2.0), egui::Align2::CENTER_BOTTOM, tr("The faces unfolded round A"), egui::FontId::proportional(11.0), theme::text_dim());
    clicked
}

/// The picture of a cylinder on the rotary: the workpiece along X seen at a slant, its length and diameter, the axis of the rotary with
/// the arrow of the turn (Y goes round), the jaws of a chuck at its left end, and the zero of the program on the surface.
pub fn cylinder_picture(ui: &mut egui::Ui, st: &Setup, colour: [u8; 3], units: Units, with_zero: bool) {
    let rect = picture_rect(ui, 570.0, 300.0);
    let p = ui.painter_at(rect);
    // The proportions of the drawing follow the numbers, within limits so that a long thin bar and a short thick one both show.
    let ratio = (st.length / st.diameter.max(1e-6)).clamp(1.2, 6.0) as f32;
    // Room on the left for the chuck, on the right for the diameter and the arrow of the turn.
    let room_w = rect.width() - 60.0 - 150.0;
    let dia = (room_w / ratio).min(rect.height() - 80.0).max(20.0);
    let len = (dia * ratio).min(room_w);
    let rx = dia * 0.18; // the ends are ellipses: the cylinder is seen a little from the side
    let left = rect.min.x + 60.0 + (room_w - len) / 2.0;
    let cy = rect.center().y - 4.0;
    let (top, bottom) = (cy - dia / 2.0, cy + dia / 2.0);
    let right = left + len;
    // The chuck at the left end: a block with three jaws.
    let chuck = egui::Rect::from_min_max(Pos2::new(left - rx - 26.0, cy - dia * 0.62), Pos2::new(left - rx + 2.0, cy + dia * 0.62));
    p.rect_filled(chuck, 3.0, Color32::from_gray(0x70));
    for k in [-0.45f32, 0.0, 0.45] {
        p.rect_filled(egui::Rect::from_center_size(Pos2::new(left - rx + 4.0, cy + k * dia), egui::vec2(8.0, dia * 0.16)), 1.0, Color32::from_gray(0x55));
    }
    // The axis of the rotary, behind the workpiece.
    let axis = Stroke::new(1.0, Color32::from_gray(0x90));
    let mut x = chuck.min.x - 8.0;
    while x < right + rx + 84.0 {
        p.line_segment([Pos2::new(x, cy), Pos2::new((x + 7.0).min(right + rx + 84.0), cy)], axis);
        x += 11.0;
    }
    // The body: the side, light at the top and darker below, then the end on the right.
    let n = 24;
    let mut body: Vec<Pos2> = vec![Pos2::new(left, top), Pos2::new(right, top)];
    for i in 0..=n {
        let a = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * i as f32 / n as f32;
        body.push(Pos2::new(right + rx * a.cos(), cy + dia / 2.0 * a.sin()));
    }
    body.push(Pos2::new(left, bottom));
    p.add(egui::Shape::convex_polygon(body, shade(colour, 0.85), Stroke::NONE));
    p.rect_filled(egui::Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, cy - dia * 0.15)), 0.0, shade(colour, 1.0));
    let edge = Stroke::new(1.0, shade(colour, 0.45));
    p.line_segment([Pos2::new(left, top), Pos2::new(right, top)], edge);
    p.line_segment([Pos2::new(left, bottom), Pos2::new(right, bottom)], edge);
    let end: Vec<Pos2> = (0..=48)
        .map(|i| {
            let a = std::f32::consts::TAU * i as f32 / 48.0;
            Pos2::new(right + rx * a.cos(), cy + dia / 2.0 * a.sin())
        })
        .collect();
    p.add(egui::Shape::convex_polygon(end, shade(colour, 0.7), edge));
    let green = Color32::from_rgb(0x30, 0xa0, 0x40);
    let turn: Vec<Pos2> = (0..=20)
        .map(|i| {
            let a = -2.3 + 2.0 * i as f32 / 20.0;
            Pos2::new(right + rx + 66.0 + 8.0 * a.cos(), cy + dia * 0.42 * a.sin())
        })
        .collect();
    let tip = turn[turn.len() - 1];
    let before = turn[turn.len() - 3];
    p.add(egui::Shape::line(turn, Stroke::new(2.0, green)));
    let dir = (tip - before).normalized();
    let side = egui::vec2(-dir.y, dir.x);
    p.add(egui::Shape::convex_polygon(vec![tip + dir * 4.0, tip - dir * 5.0 + side * 4.0, tip - dir * 5.0 - side * 4.0], green, Stroke::NONE));
    p.text(Pos2::new(right + rx + 80.0, cy - dia * 0.42), egui::Align2::LEFT_CENTER, "Y", egui::FontId::proportional(12.0), green);
    // The length under the cylinder and the diameter at its right end.
    let ink = crate::dims::DimStyle::on(theme::panel());
    let num = |v: f64| crate::dims::number(units, v);
    crate::dims::dim(&p, Pos2::new(left, bottom), Pos2::new(right, bottom), egui::vec2(0.0, 1.0), 16.0, &num(st.length), &ink);
    crate::dims::dim(&p, Pos2::new(right + rx, top), Pos2::new(right + rx, bottom), egui::vec2(1.0, 0.0), 16.0, &format!("Ø {}", num(st.diameter)), &ink);
    if !with_zero {
        return;
    }
    // The zero of the program on the top of the surface: at the left or the right end, or in the middle.
    let zx = match st.zero {
        ZeroPoint::FrontLeft | ZeroPoint::BackLeft => left,
        ZeroPoint::FrontRight | ZeroPoint::BackRight => right,
        ZeroPoint::Center => (left + right) / 2.0,
    };
    let zero = Pos2::new(zx, top);
    let red = Color32::from_rgb(0xd0, 0x30, 0x30);
    let blue = Color32::from_rgb(0x30, 0x60, 0xd0);
    for (end, name, c) in [(zero + egui::vec2(28.0, 0.0), "X", red), (zero + egui::vec2(0.0, -26.0), "Z", blue)] {
        p.line_segment([zero, end], Stroke::new(2.0, c));
        p.text(end + (end - zero).normalized() * 8.0, egui::Align2::CENTER_CENTER, name, egui::FontId::proportional(12.0), c);
    }
    p.circle_filled(zero, 4.5, Color32::from_rgb(0xe0, 0x30, 0x30));
    p.circle_stroke(zero, 4.5, Stroke::new(1.2, Color32::WHITE));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_paper_that_fits_the_machine_is_offered() {
        // A plotter of 300 × 220: A4 lying fits, A4 upright does not, A3 does not.
        let list = papers_that_fit((300.0, 220.0));
        let names: Vec<&str> = list.iter().map(|(n, _, _)| n.as_str()).collect();
        assert!(names.contains(&"A4 ↔") && !names.contains(&"A4 ↕") && !names.iter().any(|n| n.starts_with("A3")), "{names:?}");
        assert!(list.iter().all(|(_, w, h)| *w <= 300.0 && *h <= 220.0));
        assert!(papers_that_fit((1000.0, 1000.0)).len() == 2 * PAPER.len());
    }
}
