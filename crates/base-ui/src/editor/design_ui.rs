//! The design commands of the editor: grid and circular arrays, copies along a path, text on a path, trimming lines by a shape,
//! removing duplicates, breaking shapes apart, simplifying, serial texts, barcodes and QR codes, nesting and measuring. The geometry is
//! in `base_core::design`; this file picks the shapes, asks for the numbers and puts the results on the page.
use super::EditorHost;
use crate::i18n::{tr, trf};
use crate::theme;
use crate::units_ui::drag_len;
use crate::window::popup;
use base_core::design;
use base_core::geom::{Polyline, Pt, Xf};
use base_core::scene::{Kind, Shape};
use egui::RichText;

/// Which design window is open.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignCmd {
    Grid,
    Circle,
    AlongPath,
    TextPath,
    Trim,
    Simplify,
    Serial,
    Code,
    Nest,
    Measure,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CodeKind {
    Qr,
    Code39,
}

/// The numbers the design windows hold between uses.
#[derive(Clone, Debug)]
pub struct DesignState {
    pub open: Option<DesignCmd>,
    pub cols: usize,
    pub rows: usize,
    pub dx: f64,
    pub dy: f64,
    pub count: usize,
    pub total: f64,
    pub turn: bool,
    pub spacing: f64,
    pub start: f64,
    pub keep_inside: bool,
    pub tol: f64,
    pub tpl: String,
    pub first: i64,
    pub step: i64,
    pub digits: usize,
    pub csv: Vec<Vec<String>>,
    pub code: CodeKind,
    pub data: String,
    pub size: f64,
    pub height: f64,
    pub gap: f64,
    pub allow_turn: bool,
    pub area: (f64, f64),
}

impl Default for DesignState {
    fn default() -> DesignState {
        DesignState { open: None, cols: 3, rows: 3, dx: 20.0, dy: 20.0, count: 6, total: 360.0, turn: true, spacing: 10.0, start: 0.0, keep_inside: true, tol: 0.05, tpl: String::new(), first: 1, step: 1, digits: 3, csv: vec![], code: CodeKind::Qr, data: "https://".into(), size: 30.0, height: 12.0, gap: 2.0, allow_turn: true, area: (300.0, 200.0) }
    }
}

const MAX_COPIES: usize = 2000;

fn centre(r: base_core::geom::Rect) -> Pt {
    Pt::new((r.min.x + r.max.x) / 2.0, (r.min.y + r.max.y) / 2.0)
}

pub trait EditorDesign: EditorHost {
    /// The selected shapes that can be worked on (not locked).
    fn design_ids(&self) -> Vec<u64> {
        self.ed().sel.iter().copied().filter(|id| self.scene().shape(*id).is_some_and(|s| !s.locked)).collect()
    }

    /// A copy of a shape with the transform changed.
    fn copy_of(&mut self, id: u64, xf: Xf) -> Option<u64> {
        let s = self.scene().shape(id)?;
        let (layer, kind) = (s.layer, s.kind.clone());
        Some(self.scene_mut().add(layer, kind, xf))
    }

    fn grid_array(&mut self, cols: usize, rows: usize, dx: f64, dy: f64) {
        let ids = self.design_ids();
        let offsets = design::grid_offsets(cols, rows, dx, dy);
        if ids.is_empty() || offsets.is_empty() || offsets.len() * ids.len() > MAX_COPIES {
            self.say(tr("Select the shapes to copy (at most 2000 copies are made).").to_string());
            return;
        }
        self.checkpoint();
        let mut all = ids.clone();
        for (ox, oy) in offsets {
            for id in &ids {
                let Some(xf) = self.scene().shape(*id).map(|s| s.xf) else { continue };
                if let Some(n) = self.copy_of(*id, xf.then(Xf::translate(ox, oy))) {
                    all.push(n);
                }
            }
        }
        let n = all.len() - ids.len();
        self.ed_mut().sel = all;
        self.say(trf("{} copies made", &[&n]));
    }

    fn circle_array(&mut self, count: usize, total: f64, turn: bool) {
        let ids = self.design_ids();
        let Some(b) = self.scene().bounds_of(&ids) else {
            self.say(tr("Select the shapes to copy (at most 2000 copies are made).").to_string());
            return;
        };
        let xs = design::circle_transforms(centre(b), count, total);
        if xs.is_empty() || xs.len() * ids.len() > MAX_COPIES {
            self.say(tr("Select the shapes to copy (at most 2000 copies are made).").to_string());
            return;
        }
        self.checkpoint();
        let mut all = ids.clone();
        for t in &xs {
            for id in &ids {
                let Some(s) = self.scene().shape(*id) else { continue };
                let (xf, c0) = (s.xf, s.bounds().map(centre).unwrap_or(Pt::new(0.0, 0.0)));
                let new_xf = if turn {
                    xf.then(*t)
                } else {
                    let c1 = t.apply(c0);
                    xf.then(Xf::translate(c1.x - c0.x, c1.y - c0.y))
                };
                if let Some(n) = self.copy_of(*id, new_xf) {
                    all.push(n);
                }
            }
        }
        let n = all.len() - ids.len();
        self.ed_mut().sel = all;
        self.say(trf("{} copies made", &[&n]));
    }

    /// The path of the command: the last selected shape; the others are what is copied or bent.
    fn design_path(&self, ids: &[u64]) -> Option<(u64, Polyline)> {
        let last = *ids.last()?;
        let p = self.scene().shape(last)?.polys().into_iter().filter(|p| p.pts.len() >= 2).max_by(|a, b| a.length().partial_cmp(&b.length()).unwrap())?;
        Some((last, p))
    }

    fn copy_along_path(&mut self, spacing: f64, turn: bool) {
        let ids = self.design_ids();
        let Some((path_id, path)) = self.design_path(&ids) else {
            self.say(tr("Select the shapes first and the path (or the trimming shape) last.").to_string());
            return;
        };
        let src: Vec<u64> = ids.iter().copied().filter(|i| *i != path_id).collect();
        let Some(b) = self.scene().bounds_of(&src) else {
            self.say(tr("Select the shapes first and the path (or the trimming shape) last.").to_string());
            return;
        };
        let places = design::along_path(&path, spacing, MAX_COPIES / src.len().max(1));
        if places.is_empty() {
            self.say(tr("Select the shapes first and the path (or the trimming shape) last.").to_string());
            return;
        }
        let c0 = centre(b);
        self.checkpoint();
        let mut made = vec![];
        for (p, ang) in places {
            for id in &src {
                let Some(xf) = self.scene().shape(*id).map(|s| s.xf) else { continue };
                let mut x = xf.then(Xf::translate(p.x - c0.x, p.y - c0.y));
                if turn {
                    x = x.then(Xf::rotate_about(ang, p));
                }
                if let Some(n) = self.copy_of(*id, x) {
                    made.push(n);
                }
            }
        }
        // The originals are the first copy's places: they go.
        self.scene_mut().shapes.retain(|s| !src.contains(&s.id));
        let n = made.len();
        self.ed_mut().sel = made;
        self.say(trf("{} copies made", &[&n]));
    }

    fn text_on_path(&mut self, start: f64) {
        let ids = self.design_ids();
        let Some((path_id, path)) = self.design_path(&ids) else {
            self.say(tr("Select the shapes first and the path (or the trimming shape) last.").to_string());
            return;
        };
        let texts: Vec<u64> = ids.iter().copied().filter(|i| *i != path_id && self.scene().shape(*i).is_some_and(|s| matches!(s.kind, Kind::Text(_))) ).collect();
        if texts.is_empty() {
            self.say(tr("Select the shapes first and the path (or the trimming shape) last.").to_string());
            return;
        }
        self.checkpoint();
        let mut made = vec![];
        for id in texts {
            let Some(s) = self.scene().shape(id) else { continue };
            let (layer, polys) = (s.layer, s.polys());
            let Some(b) = design::box_of(&polys) else { continue };
            // The page runs downwards: the height above the baseline (the bottom of the box) is `b.max.y - y`; the bend wants it as `y - baseline`.
            let flat: Vec<Polyline> = polys.iter().map(|p| Polyline::new(p.pts.iter().map(|q| Pt::new(q.x, q.y - b.max.y)).collect(), p.closed)).collect();
            let bent = design::bend_along(&path, &flat, b.min.x, start);
            self.scene_mut().shapes.retain(|s| s.id != id);
            made.push(self.scene_mut().add(layer, Kind::Path(bent), Xf::IDENTITY));
        }
        self.ed_mut().sel = made;
        self.say(tr("The text follows the path now (it is outlines, no longer text).").to_string());
    }

    fn trim_by_shape(&mut self, keep_inside: bool) {
        let ids = self.design_ids();
        let Some((region_id, _)) = self.design_path(&ids) else {
            self.say(tr("Select the shapes first and the path (or the trimming shape) last.").to_string());
            return;
        };
        let region = self.scene().shape(region_id).map(|s| s.polys()).unwrap_or_default();
        if !region.iter().any(|p| p.closed) || ids.len() < 2 {
            self.say(tr("Select the shapes first and the path (or the trimming shape) last.").to_string());
            return;
        }
        self.checkpoint();
        let mut made = vec![];
        for id in ids.iter().filter(|i| **i != region_id) {
            let Some(s) = self.scene().shape(*id) else { continue };
            if s.is_image() {
                continue;
            }
            let (layer, polys) = (s.layer, s.polys());
            let parts = design::clip_lines(&polys, &region, keep_inside);
            self.scene_mut().shapes.retain(|s| s.id != *id);
            if !parts.is_empty() {
                made.push(self.scene_mut().add(layer, Kind::Path(parts), Xf::IDENTITY));
            }
        }
        self.ed_mut().sel = made;
    }

    fn remove_duplicates(&mut self) {
        // The selection, or everything when nothing is selected.
        let ids: Vec<u64> = if self.ed().sel.is_empty() { self.scene().shapes.iter().filter(|s| !s.locked).map(|s| s.id).collect() } else { self.design_ids() };
        let mut keep: Vec<(u64, usize, Vec<Polyline>)> = vec![];
        let mut gone = vec![];
        for id in ids {
            let Some(s) = self.scene().shape(id) else { continue };
            let polys = s.polys();
            if keep.iter().any(|(_, l, p)| *l == s.layer && design::same_geometry(p, &polys, 1e-3)) {
                gone.push(id);
            } else {
                keep.push((id, s.layer, polys));
            }
        }
        if gone.is_empty() {
            self.say(tr("There are no duplicates.").to_string());
            return;
        }
        self.checkpoint();
        self.scene_mut().shapes.retain(|s| !gone.contains(&s.id));
        self.ed_mut().sel.retain(|i| !gone.contains(i));
        self.say(trf("{} duplicates removed", &[&gone.len()]));
    }

    fn break_apart(&mut self) {
        let ids = self.design_ids();
        let mut work: Vec<(u64, usize, Xf, Kind)> = vec![];
        for id in &ids {
            let Some(s) = self.scene().shape(*id) else { continue };
            match &s.kind {
                Kind::Path(p) if p.len() > 1 => work.push((*id, s.layer, s.xf, Kind::Path(p.clone()))),
                Kind::Bezier(c) if c.len() > 1 => work.push((*id, s.layer, s.xf, Kind::Bezier(c.clone()))),
                _ => {}
            }
        }
        if work.is_empty() {
            self.say(tr("Nothing to break apart: the shapes have one outline each.").to_string());
            return;
        }
        self.checkpoint();
        let mut made = vec![];
        let gone: Vec<u64> = work.iter().map(|w| w.0).collect();
        self.scene_mut().shapes.retain(|s| !gone.contains(&s.id));
        for (_, layer, xf, kind) in work {
            match kind {
                Kind::Path(ps) => made.extend(ps.into_iter().map(|p| self.scene_mut().add(layer, Kind::Path(vec![p]), xf))),
                Kind::Bezier(cs) => made.extend(cs.into_iter().map(|c| self.scene_mut().add(layer, Kind::Bezier(vec![c]), xf))),
                _ => {}
            }
        }
        let n = made.len();
        self.ed_mut().sel = made;
        self.say(trf("Broken apart into {} shapes", &[&n]));
    }

    fn simplify_selected(&mut self, tol: f64) {
        let ids: Vec<u64> = self.design_ids().into_iter().filter(|id| self.scene().shape(*id).is_some_and(|s| !s.is_image())).collect();
        if ids.is_empty() {
            self.say(tr("Select something first.").to_string());
            return;
        }
        self.checkpoint();
        let (mut before, mut after) = (0usize, 0usize);
        let mut made = vec![];
        for id in ids {
            let Some(s) = self.scene().shape(id) else { continue };
            let (layer, polys) = (s.layer, s.polys());
            let simple: Vec<Polyline> = polys.iter().map(|p| design::simplify(p, tol)).collect();
            before += polys.iter().map(|p| p.pts.len()).sum::<usize>();
            after += simple.iter().map(|p| p.pts.len()).sum::<usize>();
            self.scene_mut().shapes.retain(|s| s.id != id);
            made.push(self.scene_mut().add(layer, Kind::Path(simple), Xf::IDENTITY));
        }
        self.ed_mut().sel = made;
        self.say(trf("Simplified: {} points became {}", &[&before, &after]));
    }

    /// The selected text, copied in a grid with its text counted up (`{n}` in `template`) or filled from the rows of a CSV file
    /// (`{1}`, `{2}`… in `template`).
    fn serial_text(&mut self, template: &str, texts: &[String], cols: usize, dx: f64, dy: f64) {
        let ids: Vec<u64> = self.design_ids().into_iter().filter(|id| self.scene().shape(*id).is_some_and(|s| matches!(s.kind, Kind::Text(_)))).collect();
        let Some(&first) = ids.first() else {
            self.say(tr("Select something first.").to_string());
            return;
        };
        let _ = template;
        if texts.is_empty() || texts.len() > MAX_COPIES {
            self.say(tr("There is nothing to make: no numbers and no rows.").to_string());
            return;
        }
        self.checkpoint();
        let mut made = vec![];
        let cols = cols.max(1);
        for (i, t) in texts.iter().enumerate() {
            let Some(s) = self.scene().shape(first) else { return };
            let (layer, xf, mut kind) = (s.layer, s.xf, s.kind.clone());
            if let Kind::Text(d) = &mut kind {
                d.text = t.clone();
            }
            let at = xf.then(Xf::translate((i % cols) as f64 * dx, (i / cols) as f64 * dy));
            made.push(self.scene_mut().add(layer, kind, at));
        }
        self.scene_mut().shapes.retain(|s| s.id != first);
        let n = made.len();
        self.ed_mut().sel = made;
        self.say(trf("{} texts made", &[&n]));
    }

    /// A QR code or a Code 39 barcode as shapes on the active layer: dark squares or bars, joined into outlines.
    fn make_code(&mut self, kind: CodeKind, data: &str, size: f64, height: f64) {
        let mut polys: Vec<Polyline> = vec![];
        let rect = |x: f64, y: f64, w: f64, h: f64| Polyline::new(vec![Pt::new(x, y), Pt::new(x + w, y), Pt::new(x + w, y + h), Pt::new(x, y + h)], true);
        match kind {
            CodeKind::Qr => {
                let Some((w, dark)) = design::qr_modules(data) else {
                    self.say(tr("The text does not fit in a QR code.").to_string());
                    return;
                };
                let m = size / w as f64;
                for r in 0..w {
                    let mut c = 0;
                    while c < w {
                        if !dark[r * w + c] {
                            c += 1;
                            continue;
                        }
                        let s = c;
                        while c < w && dark[r * w + c] {
                            c += 1;
                        }
                        polys.push(rect(s as f64 * m, r as f64 * m, (c - s) as f64 * m, m));
                    }
                }
            }
            CodeKind::Code39 => {
                // `size` is the width of the narrow bar here.
                let Some(bars) = design::code39_bars(data, size) else {
                    self.say(tr("A Code 39 barcode has capitals, digits and - . $ / + % and spaces only.").to_string());
                    return;
                };
                polys.extend(bars.into_iter().map(|(x, w)| rect(x, 0.0, w, height)));
            }
        }
        let polys = design::union_all(&polys);
        let layer = self.ed().active_layer;
        // Where the view is: the corner of the page nearest to its top left, a little in.
        self.checkpoint();
        let id = self.scene_mut().add(layer, Kind::Path(polys), Xf::translate(10.0, 10.0));
        self.ed_mut().sel = vec![id];
    }

    fn nest_selected(&mut self, area: (f64, f64), gap: f64, allow_turn: bool) {
        let ids: Vec<u64> = self.design_ids();
        let boxes: Vec<(u64, base_core::geom::Rect)> = ids.iter().filter_map(|id| self.scene().shape(*id).and_then(|s| s.bounds()).map(|b| (*id, b))).collect();
        if boxes.is_empty() {
            self.say(tr("Select something first.").to_string());
            return;
        }
        let sizes: Vec<(f64, f64)> = boxes.iter().map(|(_, b)| (b.max.x - b.min.x, b.max.y - b.min.y)).collect();
        let Some(places) = design::nest_boxes(&sizes, area, gap, allow_turn) else {
            self.say(tr("The shapes do not fit in the area.").to_string());
            return;
        };
        self.checkpoint();
        for ((id, b), (x, y, turned)) in boxes.iter().zip(places) {
            let Some(s) = self.scene_mut().shape_mut(*id) else { continue };
            if turned {
                s.xf = s.xf.then(Xf::rotate_about(std::f64::consts::FRAC_PI_2, centre(*b)));
            }
            let Some(nb) = s.bounds() else { continue };
            s.xf = s.xf.then(Xf::translate(x - nb.min.x, y - nb.min.y));
        }
        self.say(trf("{} shapes nested", &[&boxes.len()]));
    }

    fn design_windows(&mut self, ctx: &egui::Context) {
        let Some(cmd) = self.ed().design.open else { return };
        let units = self.units();
        let mut st = self.ed().design.clone();
        let mut open = true;
        let mut go = false;
        let title: String = match cmd {
            DesignCmd::Grid => tr("Grid array…").trim_end_matches('…').to_string(),
            DesignCmd::Circle => tr("Circular array…").trim_end_matches('…').to_string(),
            DesignCmd::AlongPath => tr("Copy along path…").trim_end_matches('…').to_string(),
            DesignCmd::TextPath => tr("Text on path…").trim_end_matches('…').to_string(),
            DesignCmd::Trim => tr("Trim by shape…").trim_end_matches('…').to_string(),
            DesignCmd::Simplify => tr("Simplify…").trim_end_matches('…').to_string(),
            DesignCmd::Serial => tr("Serial text…").trim_end_matches('…').to_string(),
            DesignCmd::Code => tr("Barcode / QR code…").trim_end_matches('…').to_string(),
            DesignCmd::Nest => tr("Nest shapes…").trim_end_matches('…').to_string(),
            DesignCmd::Measure => tr("Measure…").trim_end_matches('…').to_string(),
        };
        let sel_text = self.design_ids().iter().find_map(|id| match self.scene().shape(*id).map(|s| &s.kind) { Some(Kind::Text(d)) => Some(d.text.clone()), _ => None });
        let measure = if cmd == DesignCmd::Measure { Some(self.measure_lines()) } else { None };
        popup(ctx, "design_window", &title, [340.0, 280.0], &mut open, |ui| {
            egui::Grid::new("design_grid").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                match cmd {
                    DesignCmd::Grid => {
                        ui.label(tr("Columns"));
                        ui.add(egui::DragValue::new(&mut st.cols).range(1..=200));
                        ui.end_row();
                        ui.label(tr("Rows"));
                        ui.add(egui::DragValue::new(&mut st.rows).range(1..=200));
                        ui.end_row();
                        ui.label(tr("Column distance"));
                        drag_len(ui, units, &mut st.dx, 0.5, Some((-10_000.0, 10_000.0)));
                        ui.end_row();
                        ui.label(tr("Row distance"));
                        drag_len(ui, units, &mut st.dy, 0.5, Some((-10_000.0, 10_000.0)));
                        ui.end_row();
                    }
                    DesignCmd::Circle => {
                        ui.label(tr("Places"));
                        ui.add(egui::DragValue::new(&mut st.count).range(2..=500));
                        ui.end_row();
                        ui.label(tr("Total angle"));
                        ui.add(egui::DragValue::new(&mut st.total).range(-360.0..=360.0).suffix(" °"));
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut st.turn, tr("Turn the copies"));
                        ui.end_row();
                    }
                    DesignCmd::AlongPath => {
                        ui.label(tr("Distance"));
                        drag_len(ui, units, &mut st.spacing, 0.5, Some((0.1, 10_000.0)));
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut st.turn, tr("Turn the copies"));
                        ui.end_row();
                    }
                    DesignCmd::TextPath => {
                        ui.label(tr("Start"));
                        drag_len(ui, units, &mut st.start, 0.5, Some((0.0, 100_000.0)));
                        ui.end_row();
                    }
                    DesignCmd::Trim => {
                        ui.label("");
                        ui.checkbox(&mut st.keep_inside, tr("Keep the inside of the shape"));
                        ui.end_row();
                    }
                    DesignCmd::Simplify => {
                        ui.label(tr("Tolerance"));
                        drag_len(ui, units, &mut st.tol, 0.01, Some((0.001, 50.0)));
                        ui.end_row();
                    }
                    DesignCmd::Serial => {
                        if st.tpl.is_empty() {
                            st.tpl = sel_text.clone().unwrap_or_else(|| "{n}".into());
                        }
                        ui.label(tr("Text"));
                        ui.text_edit_singleline(&mut st.tpl).on_hover_text(tr("{n} is the number; with a CSV file {1}, {2}… are its columns"));
                        ui.end_row();
                        ui.label(tr("First number"));
                        ui.add(egui::DragValue::new(&mut st.first));
                        ui.end_row();
                        ui.label(tr("Step"));
                        ui.add(egui::DragValue::new(&mut st.step));
                        ui.end_row();
                        ui.label(tr("Digits"));
                        ui.add(egui::DragValue::new(&mut st.digits).range(1..=12));
                        ui.end_row();
                        ui.label(tr("Copies"));
                        ui.add(egui::DragValue::new(&mut st.count).range(1..=500));
                        ui.end_row();
                        ui.label(tr("Columns"));
                        ui.add(egui::DragValue::new(&mut st.cols).range(1..=200));
                        ui.end_row();
                        ui.label(tr("Column distance"));
                        drag_len(ui, units, &mut st.dx, 0.5, Some((-10_000.0, 10_000.0)));
                        ui.end_row();
                        ui.label(tr("Row distance"));
                        drag_len(ui, units, &mut st.dy, 0.5, Some((-10_000.0, 10_000.0)));
                        ui.end_row();
                        ui.label("");
                        ui.horizontal(|ui| {
                            if ui.button(tr("Read a CSV file…")).clicked() {
                                if let Some(p) = rfd::FileDialog::new().add_filter("CSV", &["csv", "txt"]).pick_file() {
                                    if let Ok(t) = std::fs::read_to_string(p) {
                                        st.csv = design::csv_rows(&t);
                                    }
                                }
                            }
                            if !st.csv.is_empty() {
                                ui.label(trf("{} rows from the file", &[&st.csv.len()]));
                                if ui.small_button("✖").clicked() {
                                    st.csv.clear();
                                }
                            }
                        });
                        ui.end_row();
                    }
                    DesignCmd::Code => {
                        ui.label(tr("Type"));
                        egui::ComboBox::from_id_salt("code_kind").selected_text(if st.code == CodeKind::Qr { "QR" } else { "Code 39" }).show_ui(ui, |ui| {
                            ui.selectable_value(&mut st.code, CodeKind::Qr, "QR");
                            ui.selectable_value(&mut st.code, CodeKind::Code39, "Code 39");
                        });
                        ui.end_row();
                        ui.label(tr("Text"));
                        ui.text_edit_singleline(&mut st.data);
                        ui.end_row();
                        ui.label(if st.code == CodeKind::Qr { tr("Size") } else { tr("Bar width") });
                        drag_len(ui, units, &mut st.size, 0.1, Some((0.05, 1000.0)));
                        ui.end_row();
                        if st.code == CodeKind::Code39 {
                            ui.label(tr("Height"));
                            drag_len(ui, units, &mut st.height, 0.5, Some((1.0, 1000.0)));
                            ui.end_row();
                        }
                    }
                    DesignCmd::Nest => {
                        ui.label(tr("Area width"));
                        drag_len(ui, units, &mut st.area.0, 1.0, Some((1.0, 100_000.0)));
                        ui.end_row();
                        ui.label(tr("Area height"));
                        drag_len(ui, units, &mut st.area.1, 1.0, Some((1.0, 100_000.0)));
                        ui.end_row();
                        ui.label(tr("Distance"));
                        drag_len(ui, units, &mut st.gap, 0.1, Some((0.0, 1000.0)));
                        ui.end_row();
                        ui.label("");
                        ui.checkbox(&mut st.allow_turn, tr("Allow turning"));
                        ui.end_row();
                    }
                    DesignCmd::Measure => {}
                }
            });
            match cmd {
                DesignCmd::AlongPath | DesignCmd::TextPath | DesignCmd::Trim => {
                    ui.label(RichText::new(tr("Select the shapes first and the path (or the trimming shape) last.")).color(theme::text_dim()));
                }
                DesignCmd::Nest => {
                    ui.label(RichText::new(tr("The shapes are packed by their boxes in rows from the corner of the page.")).color(theme::text_dim()));
                }
                _ => {}
            }
            if let Some(lines) = &measure {
                for l in lines {
                    ui.label(l);
                }
            } else {
                go = ui.button(tr("Apply")).clicked();
            }
        });
        if go {
            match cmd {
                DesignCmd::Grid => self.grid_array(st.cols, st.rows, st.dx, st.dy),
                DesignCmd::Circle => self.circle_array(st.count, st.total, st.turn),
                DesignCmd::AlongPath => self.copy_along_path(st.spacing, st.turn),
                DesignCmd::TextPath => self.text_on_path(st.start),
                DesignCmd::Trim => self.trim_by_shape(st.keep_inside),
                DesignCmd::Simplify => self.simplify_selected(st.tol),
                DesignCmd::Serial => {
                    let texts: Vec<String> = if st.csv.is_empty() { design::serial_texts(&st.tpl, st.first, st.step, st.count, st.digits) } else { st.csv.iter().map(|r| design::fill_row(&st.tpl, r)).collect() };
                    let tpl = st.tpl.clone();
                    self.serial_text(&tpl, &texts, st.cols, st.dx, st.dy);
                }
                DesignCmd::Code => self.make_code(st.code, &st.data, st.size, st.height),
                DesignCmd::Nest => self.nest_selected(st.area, st.gap, st.allow_turn),
                DesignCmd::Measure => {}
            }
            open = false;
        }
        st.open = if open { Some(cmd) } else { None };
        self.ed_mut().design = st;
    }

    /// The lines of the Measure window: the size, area and length of the selection, and the distance between two shapes.
    fn measure_lines(&self) -> Vec<String> {
        let ids = self.design_ids();
        if ids.is_empty() {
            return vec![tr("Select something first.").to_string()];
        }
        let shapes: Vec<&Shape> = ids.iter().filter_map(|i| self.scene().shape(*i)).collect();
        let polys: Vec<Polyline> = shapes.iter().flat_map(|s| s.polys()).collect();
        let mut v = vec![];
        if let Some(b) = design::box_of(&polys) {
            v.push(trf("Width {}  Height {}", &[&format!("{:.3}", b.max.x - b.min.x), &format!("{:.3}", b.max.y - b.min.y)]));
            v.push(trf("From X {} Y {}", &[&format!("{:.3}", b.min.x), &format!("{:.3}", b.min.y)]));
        }
        let area: f64 = polys.iter().filter(|p| p.closed).map(|p| p.area().abs()).sum();
        let len: f64 = polys.iter().map(|p| p.length()).sum();
        v.push(trf("Area {}  Length {}", &[&format!("{area:.3}"), &format!("{len:.3}")]));
        if shapes.len() == 2 {
            if let (Some(a), Some(b)) = (shapes[0].bounds(), shapes[1].bounds()) {
                let (ca, cb) = (centre(a), centre(b));
                let gx = (b.min.x - a.max.x).max(a.min.x - b.max.x).max(0.0);
                let gy = (b.min.y - a.max.y).max(a.min.y - b.max.y).max(0.0);
                v.push(trf("Between the centres: {} (dX {}, dY {})", &[&format!("{:.3}", (cb.x - ca.x).hypot(cb.y - ca.y)), &format!("{:.3}", cb.x - ca.x), &format!("{:.3}", cb.y - ca.y)]));
                v.push(trf("Gap between the boxes: {}", &[&format!("{:.3}", gx.hypot(gy))]));
            }
        }
        v
    }
}

impl<T: EditorHost> EditorDesign for T {}
