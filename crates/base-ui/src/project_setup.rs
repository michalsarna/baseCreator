//! The window of the project setup, the same in every program that works on a piece of material (File → Project setup…, and after
//! File → New): a block (its size, thickness, the zero of the program, a two-sided job) or a cylinder on the rotary attachment (its
//! diameter and length), with a picture of it. The program puts the answer into its project (`Setup` holds what was chosen).
use crate::i18n::tr;
use crate::theme;
use crate::units_ui::drag_len;
use crate::window::popup;
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
    /// A two-sided job (a block only): the material turned over round `flip`, with alignment marks when `marks` is set.
    pub two_sided: bool,
    pub flip: FlipAxis,
    pub marks: bool,
    /// The largest piece the machine takes (its work area), for the list of paper sizes.
    pub max: (f64, f64),
}

impl Default for Setup {
    fn default() -> Setup {
        Setup { open: false, w: 200.0, h: 150.0, t: 10.0, zero: ZeroPoint::FrontLeft, z_bottom: false, work_area: false, round: false, diameter: 50.0, length: 150.0, two_sided: false, flip: FlipAxis::SideToSide, marks: true, max: (f64::INFINITY, f64::INFINITY) }
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
    popup(ctx, "setup_window", &title, [400.0, 640.0], &mut open, |ui| {
        if kind.cylinder {
            ui.horizontal(|ui| {
                ui.label(tr("Shape of the material"));
                ui.selectable_value(&mut st.round, false, tr("Block"));
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
                ui.label("");
                ui.checkbox(&mut st.work_area, tr("Work area is the material"));
                ui.end_row();
                ui.label("");
                ui.checkbox(&mut st.two_sided, tr("Two-sided job")).on_hover_text(tr("Side A and side B are two work areas: the material is cut on one side, turned over and cut on the other. Each side shows the shapes of the other one turned over, as ghosts."));
                ui.end_row();
                if st.two_sided {
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
        ui.horizontal(|ui| {
            apply = ui.button(tr("Apply")).clicked();
            skip = ui.button(tr("Skip")).clicked();
        });
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
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(380.0), 210.0), egui::Sense::hover());
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
    if st.two_sided {
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

/// The picture of a cylinder on the rotary: the workpiece along X seen at a slant, its length and diameter, the axis of the rotary with
/// the arrow of the turn (Y goes round), the jaws of a chuck at its left end, and the zero of the program on the surface.
pub fn cylinder_picture(ui: &mut egui::Ui, st: &Setup, colour: [u8; 3], units: Units, with_zero: bool) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width().min(380.0), 200.0), egui::Sense::hover());
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
