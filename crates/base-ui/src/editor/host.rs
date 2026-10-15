//! What an editor needs from the application that hosts it, and the editing operations built on that.
//!
//! The application implements the required methods of `EditorHost` (where its scene and its `EditorState` are, how
//! its undo works, which layers are shown); everything else is provided here.
use super::canvas::CanvasView;
use super::{handle_pts, EditorState, NodeRef, Tool};
use base_core::bezier::{Contour, Node};
use base_core::geom::{Pt, Rect, Xf};
use base_core::scene::{Kind, Scene};
use crate::i18n::{tr, trf};
use egui::{Pos2, Vec2};

pub trait EditorHost: Sized {
    fn ed(&self) -> &EditorState;
    fn ed_mut(&mut self) -> &mut EditorState;
    fn scene(&self) -> &Scene;
    fn scene_mut(&mut self) -> &mut Scene;
    /// Is the layer (group) of a shape shown? Hidden shapes cannot be picked.
    fn layer_visible(&self, layer: usize) -> bool;
    /// Remember the state for undo (called before a change) and note that something changed.
    fn checkpoint(&mut self);
    fn touch(&mut self);
    /// Undo and redo the last change, and whether there is something to undo or redo.
    fn undo(&mut self);
    fn redo(&mut self);
    fn can_undo(&self) -> bool;
    fn can_redo(&self) -> bool;
    /// Forget the state remembered by the last `checkpoint` (nothing was changed after all).
    fn drop_checkpoint(&mut self);
    /// A button that deletes something; applications may draw it with an icon.
    fn delete_button(&self, ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
        ui.add_enabled(enabled, egui::Button::new(text))
    }
    /// The two buttons at the bottom of the tool palette: open the window of the shape generators (plugins), and the search
    /// for clipart on the internet.
    fn open_shape_generators(&mut self);
    fn open_online_clipart_search(&mut self, ctx: &egui::Context);
    /// Show a message in the status line.
    fn say(&mut self, msg: String);
    /// The message now shown in the status line.
    fn said(&self) -> &str;

    // ---- what the application tells the canvas ----

    /// Size of the work area in millimetres.
    fn work_size(&self) -> (f64, f64);
    /// The units rulers and fields show.
    fn units(&self) -> base_core::units::Units;
    /// Colour of the space around the work area, and of the work area itself.
    fn workspace_fill(&self) -> egui::Color32;
    fn bed_fill(&self) -> egui::Color32;
    /// The colour a layer's shapes are drawn in, or `None` when the layer is hidden. Dimmed layers return a dimmed colour.
    fn shape_color(&self, layer: usize) -> Option<egui::Color32>;
    /// The plain colour of a layer (for the outline of a shape being drawn).
    fn layer_color(&self, layer: usize) -> egui::Color32;
    /// Draw the pictures faintly (while a preview is shown over them).
    fn dim_pictures(&self) -> bool {
        false
    }

    /// Drawn after the work area and its grid: markings of the machine, zones, home, the live position.
    fn paint_work_area(&mut self, _v: &CanvasView) {}
    /// Drawn under the shapes (a camera picture).
    fn paint_under_shapes(&mut self, _v: &CanvasView) {}
    /// Drawn over the shapes (a tool path preview).
    fn paint_over_shapes(&mut self, _v: &CanvasView) {}
    /// Draw the mark of a locked shape inside `rect`.
    fn paint_lock(&self, ui: &egui::Ui, rect: egui::Rect) {
        let c = crate::theme::accent();
        let body = egui::Rect::from_min_max(rect.left_center(), rect.right_bottom()).shrink2(egui::vec2(1.0, 0.0));
        ui.painter().rect_filled(body, 1.0, c);
        let r = body.width() * 0.32;
        let top = egui::pos2(body.center().x, body.top());
        ui.painter().circle_stroke(top, r, egui::Stroke::new(1.6, c));
    }
    /// Let the application take the pointer for itself (dragging the corners of a camera picture): return true
    /// while it does, and the canvas leaves the tools alone.
    fn pointer_claimed(&mut self, _resp: &egui::Response, _origin: Pos2, _press: Option<Pos2>, _hover: Option<Pos2>) -> bool {
        false
    }
    /// The context menu of the canvas (the application draws it and runs the command chosen).
    fn canvas_context_menu(&mut self, _ui: &mut egui::Ui, _resp: &egui::Response) {}
    /// The text tool was clicked at `at`.
    fn place_text(&mut self, _at: Pt) {}

    // ---- the view ----

    /// Screen position of a point of the work area; `o` is the top-left corner of the canvas.
    fn w2s(&self, o: Pos2, p: Pt) -> Pos2 {
        let v = &self.ed().view;
        o + v.pan + egui::vec2(p.x as f32 * v.zoom, p.y as f32 * v.zoom)
    }
    /// The point of the work area under a screen position.
    fn s2w(&self, o: Pos2, s: Pos2) -> Pt {
        let v = &self.ed().view;
        let d: Vec2 = s - o - v.pan;
        Pt::new((d.x / v.zoom) as f64, (d.y / v.zoom) as f64)
    }
    /// Zoom by `f` around the middle of the canvas.
    fn zoom_about_centre(&mut self, f: f32) {
        let (w, h) = self.ed().canvas_avail;
        let mid = egui::vec2(w / 2.0, h / 2.0);
        let v = &mut self.ed_mut().view;
        let before = (mid - v.pan) / v.zoom;
        v.zoom = (v.zoom * f).clamp(0.05, 200.0);
        v.pan = mid - before * v.zoom;
        v.auto_fit = false;
    }
    /// `p` moved to the grid when snapping is on.
    fn snapped(&self, p: Pt) -> Pt {
        let ed = self.ed();
        if ed.snap {
            let g = ed.grid;
            Pt::new((p.x / g).round() * g, (p.y / g).round() * g)
        } else {
            p
        }
    }

    // ---- selection and picking ----

    fn sel_bounds(&self) -> Option<Rect> {
        self.scene().bounds_of(&self.ed().sel)
    }

    /// The shape under `p`: on an outline first, else the smallest closed shape that contains the point.
    fn hit_shape(&self, p: Pt) -> Option<u64> {
        let tol = 5.0 / self.ed().view.zoom as f64;
        let visible = || self.scene().shapes.iter().rev().filter(|s| self.layer_visible(s.layer));
        if let Some(s) = visible().find(|s| s.polys().iter().any(|q| q.dist_to(p) <= tol)) {
            return Some(s.id);
        }
        // Inside a closed shape: pick the smallest one that contains the point.
        visible()
            .filter_map(|s| {
                let polys = s.polys();
                let inside = polys.iter().filter(|q| q.closed).fold(false, |a, q| a ^ q.contains(p));
                inside.then(|| (s.id, polys.iter().map(|q| q.area().abs()).sum::<f64>()))
            })
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .map(|(id, _)| id)
    }

    /// The resize handle of the selection under the mouse.
    fn hit_handle(&self, o: Pos2, mouse: Pos2) -> Option<usize> {
        let b = self.sel_bounds()?;
        handle_pts(&b).iter().position(|h| self.w2s(o, *h).distance(mouse) <= 8.0)
    }

    /// Shapes picked by a selection rectangle: dragged left to right only what lies wholly inside,
    /// right to left everything the rectangle touches. Groups are completed by the caller.
    fn marquee_hits(&self, r: &Rect, left_to_right: bool) -> Vec<u64> {
        self.scene()
            .shapes
            .iter()
            .filter(|s| self.layer_visible(s.layer))
            .filter(|s| {
                if left_to_right {
                    s.bounds().is_some_and(|b| r.contains(b.min) && r.contains(b.max))
                } else {
                    s.intersects_rect(r)
                }
            })
            .map(|s| s.id)
            .collect()
    }

    // ---- nodes of Bézier shapes ----

    /// Selected Bézier shapes with their id.
    fn bezier_shapes(&self) -> Vec<(u64, &Vec<Contour>)> {
        self.ed()
            .sel
            .iter()
            .filter_map(|id| {
                let s = self.scene().shape(*id).filter(|s| !s.locked)?;
                match &s.kind {
                    Kind::Bezier(cs) => Some((*id, cs)),
                    _ => None,
                }
            })
            .collect()
    }

    fn hit_node(&self, o: Pos2, mouse: Pos2) -> Option<NodeRef> {
        let mut best: Option<(f32, NodeRef)> = None;
        for (id, cs) in self.bezier_shapes() {
            for (ci, c) in cs.iter().enumerate() {
                for (ni, n) in c.nodes.iter().enumerate() {
                    let d = self.w2s(o, n.p).distance(mouse);
                    if d <= 8.0 && best.map_or(true, |b| d < b.0) {
                        best = Some((d, (id, ci, ni)));
                    }
                }
            }
        }
        best.map(|b| b.1)
    }

    /// A handle of a selected node under the mouse: (node, is_outgoing).
    fn hit_node_handle(&self, o: Pos2, mouse: Pos2) -> Option<(NodeRef, bool)> {
        for r in &self.ed().node_sel {
            let Some(n) = self.node_at(*r) else { continue };
            if n.has_out() && self.w2s(o, n.hout).distance(mouse) <= 7.0 {
                return Some((*r, true));
            }
            if n.has_in() && self.w2s(o, n.hin).distance(mouse) <= 7.0 {
                return Some((*r, false));
            }
        }
        None
    }

    /// The segment of a selected Bézier shape close to `w`: ((shape, contour, segment), t).
    fn hit_segment(&self, w: Pt) -> Option<(NodeRef, f64)> {
        let tol = 6.0 / self.ed().view.zoom as f64;
        let mut best: Option<(f64, NodeRef, f64)> = None;
        for (id, cs) in self.bezier_shapes() {
            for (ci, c) in cs.iter().enumerate() {
                if let Some((seg, t, d)) = c.nearest(w) {
                    if d <= tol && best.map_or(true, |b| d < b.0) {
                        best = Some((d, (id, ci, seg), t));
                    }
                }
            }
        }
        best.map(|b| (b.1, b.2))
    }

    fn node_at(&self, r: NodeRef) -> Option<Node> {
        match &self.scene().shape(r.0)?.kind {
            Kind::Bezier(cs) => cs.get(r.1)?.nodes.get(r.2).copied(),
            _ => None,
        }
    }

    fn contour_mut(&mut self, id: u64, ci: usize) -> Option<&mut Contour> {
        match &mut self.scene_mut().unlocked_mut(id)?.kind {
            Kind::Bezier(cs) => cs.get_mut(ci),
            _ => None,
        }
    }

    // ---- drawing ----

    /// Upload picture textures that are missing and drop those of deleted pictures.
    fn ensure_textures(&mut self, ctx: &egui::Context) {
        let mut keep = vec![];
        let mut fresh = vec![];
        for s in &self.scene().shapes {
            if let Kind::Image(im) = &s.kind {
                let key = (s.id, im.invert);
                keep.push(key);
                if !self.ed().image_tex.contains_key(&key) {
                    let px: Vec<u8> = if im.invert { im.gray.iter().map(|g| 255 - g).collect() } else { im.gray.clone() };
                    let img = egui::ColorImage::from_gray([im.px_w as usize, im.px_h as usize], &px);
                    let tex = ctx.load_texture(format!("img-{}-{}", s.id, im.invert), img, egui::TextureOptions::LINEAR);
                    fresh.push((key, tex));
                }
            }
        }
        let ed = self.ed_mut();
        ed.image_tex.extend(fresh);
        ed.image_tex.retain(|k, _| keep.contains(k));
    }

    /// Outline of the automatic shape for the active tool in a `w` x `h` mm box.
    fn auto_contour(&self, w: f64, h: f64) -> Option<Contour> {
        let ed = self.ed();
        match ed.tool {
            Tool::Triangle => Some(Contour::triangle(w, h)),
            Tool::Star => Some(Contour::star(5, 0.382, w, h)),
            Tool::Polygon => Some(Contour::polygon(ed.polygon_sides, w, h)),
            Tool::Heart => Some(Contour::heart(w, h)),
            Tool::Spiral => Some(Contour::spiral(ed.spiral_turns, w, h)),
            _ => None,
        }
    }

    /// Finish the line being drawn with the pen: make it a shape on the active layer.
    fn finish_pen(&mut self, closed: bool) {
        if self.ed().pen_pts.len() >= 2 {
            self.checkpoint();
            let pts = std::mem::take(&mut self.ed_mut().pen_pts);
            let layer = self.ed().active_layer;
            let id = self.scene_mut().add(layer, Kind::Path(vec![base_core::geom::Polyline::new(pts, closed)]), Xf::IDENTITY);
            self.ed_mut().sel = vec![id];
        }
        self.ed_mut().pen_pts.clear();
    }

    // ---- editing nodes ----

    /// Make every selected shape an editable Bézier shape with its transform baked in.
    fn nodes_prepare(&mut self) {
        let todo: Vec<u64> = self
            .ed()
            .sel
            .iter()
            .copied()
            .filter(|id| {
                self.scene().shape(*id).is_some_and(|s| {
                    // Text stays live text until you convert it to curves yourself.
                    let editable = !s.locked && !matches!(s.kind, Kind::Image { .. } | Kind::Text(_));
                    editable && (!matches!(s.kind, Kind::Bezier(_)) || s.xf != Xf::IDENTITY)
                })
            })
            .collect();
        let only_text_or_image = !self.ed().sel.is_empty()
            && self.ed().sel.iter().all(|id| self.scene().shape(*id).is_some_and(|s| matches!(s.kind, Kind::Text(_) | Kind::Image { .. })));
        if only_text_or_image && self.said() != tr("Convert text to curves, or trace the image, to edit its nodes.") {
            self.say(tr("Convert text to curves, or trace the image, to edit its nodes.").to_string());
        }
        if todo.is_empty() {
            return;
        }
        self.checkpoint();
        for id in todo {
            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                s.to_bezier();
            }
        }
        self.ed_mut().node_sel.clear();
    }

    /// Switch to the node tool for the selected shapes.
    fn start_node_edit(&mut self) {
        if self.ed().sel.is_empty() {
            self.say(tr("Select a shape first, then edit its nodes.").to_string());
            return;
        }
        let ed = self.ed_mut();
        ed.tool = Tool::Node;
        ed.pen_pts.clear();
    }

    /// Selected nodes grouped per contour, highest index first (safe for removals).
    fn sel_sorted(&self) -> Vec<NodeRef> {
        let mut v = self.ed().node_sel.clone();
        v.sort_by(|a, b| (b.0, b.1, b.2).cmp(&(a.0, a.1, a.2)));
        v.dedup();
        v
    }

    fn delete_nodes(&mut self) {
        if self.ed().node_sel.is_empty() {
            return;
        }
        self.checkpoint();
        for (id, ci, ni) in self.sel_sorted() {
            if let Some(c) = self.contour_mut(id, ci) {
                c.remove_node(ni);
            }
        }
        // Drop contours that no longer make sense, and shapes that became empty.
        for s in &mut self.scene_mut().shapes {
            if let Kind::Bezier(cs) = &mut s.kind {
                cs.retain(|c| c.nodes.len() >= 2);
            }
        }
        self.scene_mut().shapes.retain(|s| !matches!(&s.kind, Kind::Bezier(cs) if cs.is_empty()));
        let gone: Vec<u64> = self.ed().sel.iter().copied().filter(|id| self.scene().shape(*id).is_none()).collect();
        let ed = self.ed_mut();
        ed.sel.retain(|id| !gone.contains(id));
        ed.node_sel.clear();
        ed.active_seg = None;
        self.touch();
    }

    fn nodes_smooth(&mut self, smooth: bool) {
        if self.ed().node_sel.is_empty() {
            return;
        }
        self.checkpoint();
        for (id, ci, ni) in self.ed().node_sel.clone() {
            if let Some(c) = self.contour_mut(id, ci) {
                if ni < c.nodes.len() {
                    if smooth {
                        c.make_smooth(ni);
                    } else {
                        c.make_corner(ni);
                    }
                }
            }
        }
        self.touch();
    }

    /// Turn the segment after each selected node (or the clicked segment) into a line or a curve.
    fn segments_set(&mut self, curve: bool) {
        let mut segs: Vec<NodeRef> = self.ed().node_sel.clone();
        if let Some(s) = self.ed().active_seg {
            segs.push(s);
        }
        if segs.is_empty() {
            return;
        }
        self.checkpoint();
        for (id, ci, si) in segs {
            if let Some(c) = self.contour_mut(id, ci) {
                if si < c.seg_count() {
                    if curve {
                        c.segment_to_curve(si);
                    } else {
                        c.segment_to_line(si);
                    }
                }
            }
        }
        self.touch();
    }

    /// Insert a node on the clicked segment, or at the middle of the segment after each selected node.
    fn nodes_insert(&mut self, at: Option<(NodeRef, f64)>) {
        self.checkpoint();
        match at {
            Some(((id, ci, si), t)) => {
                if let Some(c) = self.contour_mut(id, ci) {
                    if si < c.seg_count() {
                        let ni = c.insert_node(si, t);
                        self.ed_mut().node_sel = vec![(id, ci, ni)];
                    }
                }
            }
            None => {
                let mut sel = self.sel_sorted();
                sel.retain(|(id, ci, ni)| self.contour_mut(*id, *ci).is_some_and(|c| *ni < c.seg_count()));
                let mut fresh = vec![];
                for (id, ci, si) in sel {
                    if let Some(c) = self.contour_mut(id, ci) {
                        let ni = c.insert_node(si, 0.5);
                        fresh.push((id, ci, ni));
                    }
                }
                self.ed_mut().node_sel = fresh;
            }
        }
        self.ed_mut().active_seg = None;
        self.touch();
    }

    /// Close an open contour, or open a closed one at the selected node.
    fn nodes_toggle_closed(&mut self) {
        let Some(&(id, ci, ni)) = self.ed().node_sel.first() else { return };
        self.checkpoint();
        if let Some(c) = self.contour_mut(id, ci) {
            if c.closed {
                // Break at the selected node: it becomes both end points.
                c.nodes.rotate_left(ni);
                let mut end = c.nodes[0];
                end.hout = end.p;
                c.nodes[0].hin = c.nodes[0].p;
                c.nodes.push(end);
                c.closed = false;
            } else if c.nodes.len() >= 3 {
                c.closed = true;
            }
        }
        self.ed_mut().node_sel.clear();
        self.touch();
    }

    /// Round the selected nodes (node tool) with `radius` mm.
    fn round_selected_nodes(&mut self, radius: f64) {
        if self.ed().node_sel.is_empty() || radius <= 0.0 {
            return;
        }
        self.checkpoint();
        let mut by_contour: std::collections::BTreeMap<(u64, usize), std::collections::HashSet<usize>> = Default::default();
        for (id, ci, ni) in &self.ed().node_sel {
            by_contour.entry((*id, *ci)).or_default().insert(*ni);
        }
        let mut total = 0;
        for ((id, ci), only) in by_contour {
            if let Some(c) = self.contour_mut(id, ci) {
                let (rounded, n) = base_core::fillet::round_contour(c, radius, Some(&only));
                *c = rounded;
                total += n;
            }
        }
        let ed = self.ed_mut();
        ed.node_sel.clear();
        ed.active_seg = None;
        if total == 0 {
            self.drop_checkpoint();
            self.say(tr("There are no straight corners to round.").to_string());
        } else {
            self.say(trf("Rounded {} corner(s).", &[&total]));
        }
    }

    /// The buttons of the node-editing bar.
    fn node_bar(&mut self, ui: &mut egui::Ui) {
        let has_nodes = !self.ed().node_sel.is_empty();
        let has_seg = has_nodes || self.ed().active_seg.is_some();
        if ui.add_enabled(has_nodes, egui::Button::new(tr("Corner"))).on_hover_text(tr("Make the selected nodes corners")).clicked() {
            self.nodes_smooth(false);
        }
        if ui.add_enabled(has_nodes, egui::Button::new(tr("Smooth"))).on_hover_text(tr("Make the selected nodes smooth")).clicked() {
            self.nodes_smooth(true);
        }
        if ui.add_enabled(has_seg, egui::Button::new(tr("To line"))).on_hover_text(tr("Make the segment after each selected node straight")).clicked() {
            self.segments_set(false);
        }
        if ui.add_enabled(has_seg, egui::Button::new(tr("To curve"))).on_hover_text(tr("Make the segment after each selected node a curve")).clicked() {
            self.segments_set(true);
        }
        if ui.add_enabled(has_seg, egui::Button::new(tr("Add node"))).on_hover_text(tr("Insert a node (or double-click a segment)")).clicked() {
            let at = self.ed().active_seg.map(|s| (s, 0.5));
            self.nodes_insert(at);
        }
        if self.delete_button(ui, has_nodes, &tr("Delete node")).clicked() {
            self.delete_nodes();
        }
        ui.add(egui::DragValue::new(&mut self.ed_mut().fillet_radius).range(0.05..=500.0).speed(0.1).suffix(" mm")).on_hover_text(tr("Radius for rounding"));
        if ui.add_enabled(has_nodes, egui::Button::new(tr("Round"))).on_hover_text(tr("Round the selected corners with this radius")).clicked() {
            let r = self.ed().fillet_radius;
            self.round_selected_nodes(r);
        }
        if ui.add_enabled(has_nodes, egui::Button::new(tr("Open / close"))).on_hover_text(tr("Close an open path, or open a closed one at the selected node")).clicked() {
            self.nodes_toggle_closed();
        }
    }
}
