//! Operations on whole shapes: boolean operations, offsetting, rounding corners and conversion to curves, with the two small
//! windows that ask for a distance or a radius. The same in every program.
use super::EditorHost;
use crate::i18n::{tr, trf};
use crate::units_ui::drag_len;
use crate::window::popup;
use crate::theme;
use base_core::bezier::Contour;
use base_core::geom::{Pt, Xf};
use base_core::ops::{self, BoolOp};
use base_core::scene::Kind;
use egui::RichText;

pub trait EditorShapeOps: EditorHost {
    /// Combine the selected shapes (in selection order) with a boolean operation.
    fn bool_op(&mut self, op: BoolOp) {
        let ids: Vec<u64> = self.ed().sel.iter().copied().filter(|id| self.scene().shape(*id).is_some_and(|s| !s.is_image() && !s.locked)).collect();
        if ids.len() < 2 {
            self.say(tr("Select at least two shapes (not images).").to_string());
            return;
        }
        let mut acc = self.scene().shape(ids[0]).map(|s| s.polys()).unwrap_or_default();
        for id in &ids[1..] {
            let next = self.scene().shape(*id).map(|s| s.polys()).unwrap_or_default();
            acc = ops::boolean(&acc, &next, op);
        }
        if acc.is_empty() {
            self.say(tr("The result is empty.").to_string());
            return;
        }
        let layer = self.scene().shape(ids[0]).map(|s| s.layer).unwrap_or(self.ed().active_layer);
        self.checkpoint();
        self.scene_mut().shapes.retain(|s| !ids.contains(&s.id));
        let id = self.scene_mut().add(layer, Kind::Path(acc), Xf::IDENTITY);
        self.ed_mut().sel = vec![id];
        self.ed_mut().node_sel.clear();
    }

    /// Join the open paths of the selected shapes that meet within `tol` mm into longer ones and close the ones that come round
    /// (`join`), or only close the paths whose two ends are within `tol` (`!join`).
    fn join_selected(&mut self, tol: f64, join: bool) {
        use base_core::export::contours_of;
        let ids: Vec<u64> = self.ed().sel.iter().copied().filter(|id| self.scene().shape(*id).is_some_and(|s| !s.is_image() && !s.locked)).collect();
        // The contours of each layer.
        let mut layers: Vec<(usize, Vec<Contour>)> = vec![];
        for id in &ids {
            let Some(s) = self.scene().shape(*id) else { continue };
            let cs = contours_of(s);
            match layers.iter_mut().find(|(l, _)| *l == s.layer) {
                Some((_, v)) => v.extend(cs),
                None => layers.push((s.layer, cs)),
            }
        }
        let before: usize = layers.iter().map(|(_, v)| v.iter().filter(|c| !c.closed).count()).sum();
        let mut made: Vec<(usize, Contour)> = vec![];
        let (mut closed_now, mut total_after) = (0, 0);
        for (layer, cs) in layers {
            let open_before = cs.iter().filter(|c| !c.closed).count();
            let result: Vec<Contour> = if join {
                base_core::join::join(cs, tol)
            } else {
                cs.into_iter().map(|mut c| { base_core::join::close_if_near(&mut c, tol); c }).collect()
            };
            closed_now += open_before - result.iter().filter(|c| !c.closed).count().min(open_before);
            total_after += result.len();
            made.extend(result.into_iter().map(|c| (layer, c)));
        }
        let open_after = made.iter().filter(|(_, c)| !c.closed).count();
        if ids.is_empty() || (open_after == before && total_after == ids.len() && closed_now == 0) {
            self.say(tr("Nothing was joined or closed.").to_string());
            return;
        }
        self.checkpoint();
        self.scene_mut().shapes.retain(|s| !ids.contains(&s.id));
        let mut new_ids = vec![];
        for (layer, c) in made {
            new_ids.push(self.scene_mut().add(layer, Kind::Bezier(vec![c]), Xf::IDENTITY));
        }
        self.ed_mut().sel = new_ids;
        self.ed_mut().node_sel.clear();
        self.say(trf("Joined into {} paths ({} open left)", &[&total_after, &open_after]));
    }

    /// Write the selected shapes (all of them when nothing is selected) as an SVG or a DXF file.
    fn export_selection(&mut self, dxf: bool) {
        let ids: Vec<u64> = self.ed().sel.clone();
        let (w, h) = self.work_size();
        let name: &str = if dxf { "selection.dxf" } else { "selection.svg" };
        let filter = if dxf { ("DXF", "dxf") } else { ("SVG", "svg") };
        let Some(path) = rfd::FileDialog::new().add_filter(filter.0, &[filter.1]).set_file_name(name).save_file() else { return };
        let layer_name = |l: usize| self.layer_title(l);
        let text = if dxf {
            base_core::export::dxf(self.scene(), &ids, h, &layer_name)
        } else {
            let color = |l: usize| -> [u8; 3] {
                let c = self.layer_color(l);
                [c.r(), c.g(), c.b()]
            };
            base_core::export::svg(self.scene(), &ids, w, h, &layer_name, &color)
        };
        match std::fs::write(&path, text) {
            Ok(()) => self.say(trf("Saved {}", &[&path.display()])),
            Err(e) => self.say(trf("Could not save: {}", &[&e])),
        }
    }

    /// Offset the outlines of the selected shapes by `d` mm (negative shrinks).
    fn offset_selected(&mut self, d: f64, keep_original: bool) {
        let ids: Vec<u64> = self.ed().sel.clone();
        let mut created: Vec<(usize, Vec<base_core::geom::Polyline>)> = vec![];
        for id in &ids {
            let Some(s) = self.scene().shape(*id) else { continue };
            if s.is_image() {
                continue;
            }
            let res = ops::offset(&s.polys(), d);
            if !res.is_empty() {
                created.push((s.layer, res));
            }
        }
        if created.is_empty() {
            self.say(tr("The result is empty.").to_string());
            return;
        }
        self.checkpoint();
        if !keep_original {
            self.scene_mut().shapes.retain(|s| !ids.contains(&s.id) || s.locked);
        }
        let made: Vec<u64> = created.into_iter().map(|(layer, polys)| self.scene_mut().add(layer, Kind::Path(polys), Xf::IDENTITY)).collect();
        self.ed_mut().sel = made;
        self.ed_mut().node_sel.clear();
    }

    /// The two selected shapes, if both are single straight lines (end points in design coordinates).
    fn selected_line_pair(&self) -> Option<[(u64, (Pt, Pt)); 2]> {
        let ids = self.scene().unlocked_ids(&self.ed().sel);
        if ids.len() != 2 {
            return None;
        }
        let seg = |id: u64| -> Option<(u64, (Pt, Pt))> {
            let s = self.scene().shape(id)?;
            if s.is_image() {
                return None;
            }
            let cs: Vec<Contour> = s.local_contours().iter().map(|c| c.transformed(&s.xf)).collect();
            base_core::fillet::as_segment(&cs).map(|l| (id, l))
        };
        Some([seg(ids[0])?, seg(ids[1])?])
    }

    /// Round corners with `radius` mm: two selected straight lines are joined by a rounded corner, otherwise every straight
    /// corner of the selected shapes is rounded.
    fn round_corners(&mut self, radius: f64) {
        if radius <= 0.0 {
            return;
        }
        if let Some([(a, la), (b, lb)]) = self.selected_line_pair() {
            let Some(contour) = base_core::fillet::fillet_lines(la, lb, radius) else {
                self.say(tr("These lines cannot be joined (parallel, or the radius does not fit).").to_string());
                return;
            };
            let layer = self.scene().shape(a).map(|s| s.layer).unwrap_or(self.ed().active_layer);
            self.checkpoint();
            self.scene_mut().shapes.retain(|s| s.id != a && s.id != b);
            let id = self.scene_mut().add(layer, Kind::Bezier(vec![contour]), Xf::IDENTITY);
            self.ed_mut().sel = vec![id];
            self.ed_mut().node_sel.clear();
            self.say(tr("Joined the two lines with a rounded corner.").to_string());
            return;
        }
        let ids: Vec<u64> = self.scene().unlocked_ids(&self.ed().sel).into_iter().filter(|id| self.scene().shape(*id).is_some_and(|s| !s.is_image())).collect();
        if ids.is_empty() {
            self.say(tr("Select shapes to round first.").to_string());
            return;
        }
        self.checkpoint();
        let mut total = 0;
        for id in ids {
            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                s.to_bezier();
                if let Kind::Bezier(cs) = &mut s.kind {
                    for c in cs.iter_mut() {
                        let (rounded, n) = base_core::fillet::round_contour(c, radius, None);
                        *c = rounded;
                        total += n;
                    }
                }
            }
        }
        if total == 0 {
            self.drop_checkpoint();
            self.say(tr("There are no straight corners to round.").to_string());
        } else {
            self.ed_mut().node_sel.clear();
            self.say(trf("Rounded {} corner(s).", &[&total]));
        }
    }

    /// Turn the selected shapes (not pictures) into Bézier curves.
    fn to_curves(&mut self) {
        if self.ed().sel.is_empty() {
            return;
        }
        self.checkpoint();
        for id in self.ed().sel.clone() {
            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                if !s.is_image() {
                    s.to_bezier();
                }
            }
        }
        self.ed_mut().node_sel.clear();
    }

    /// The small windows for the offset distance and the radius of the corners.
    fn shape_op_windows(&mut self, ctx: &egui::Context) {
        let units = self.units();
        if self.ed().show_offset {
            let (mut dist, mut keep) = (self.ed().offset_dist, self.ed().offset_keep);
            let mut open = true;
            let mut apply = false;
            popup(ctx, "offset_window", &tr("Offset shape"), [300.0, 190.0], &mut open, |ui| {
                egui::Grid::new("offs").num_columns(2).show(ui, |ui| {
                    ui.label(tr("Distance"));
                    drag_len(ui, units, &mut dist, 0.1, Some((-500.0, 500.0)));
                    ui.end_row();
                });
                ui.label(RichText::new(tr("Positive grows the outline, negative shrinks it.")).color(theme::text_dim()));
                ui.checkbox(&mut keep, tr("Keep the original"));
                apply = ui.button(tr("Create offset")).clicked();
            });
            self.ed_mut().offset_dist = dist;
            self.ed_mut().offset_keep = keep;
            if apply {
                self.offset_selected(dist, keep);
                open = false;
            }
            self.ed_mut().show_offset = open;
        }
        if self.ed().show_join {
            let mut tol = self.ed().join_tol;
            let mut open = true;
            let (mut join, mut close) = (false, false);
            popup(ctx, "join_window", &tr("Join paths"), [320.0, 190.0], &mut open, |ui| {
                egui::Grid::new("join").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("Tolerance"));
                    drag_len(ui, units, &mut tol, 0.01, Some((0.001, 50.0)));
                    ui.end_row();
                });
                ui.label(RichText::new(tr("Paths whose ends are nearer than the tolerance are joined; a path whose two ends are that near is closed.")).color(theme::text_dim()));
                ui.horizontal(|ui| {
                    join = ui.button(tr("Auto-join")).clicked();
                    close = ui.button(tr("Close paths")).clicked();
                });
            });
            self.ed_mut().join_tol = tol;
            if join || close {
                self.join_selected(tol, join);
                open = false;
            }
            self.ed_mut().show_join = open;
        }
        if self.ed().show_round {
            let mut radius = self.ed().fillet_radius;
            let mut open = true;
            let mut apply = false;
            let pair = self.selected_line_pair().is_some();
            popup(ctx, "round_window", &tr("Round corners"), [300.0, 200.0], &mut open, |ui| {
                egui::Grid::new("round").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                    ui.label(tr("Radius"));
                    drag_len(ui, units, &mut radius, 0.1, Some((0.05, 500.0)));
                    ui.end_row();
                });
                let text = if pair { tr("Two straight lines are selected: they are joined with a rounded corner.") } else { tr("Every straight corner of the selected shapes is rounded.") };
                ui.label(RichText::new(text).color(theme::text_dim()));
                apply = ui.button(tr("Round")).clicked();
            });
            self.ed_mut().fillet_radius = radius;
            if apply {
                self.round_corners(radius);
                open = false;
            }
            self.ed_mut().show_round = open;
        }
    }
}

impl<T: EditorHost> EditorShapeOps for T {}
