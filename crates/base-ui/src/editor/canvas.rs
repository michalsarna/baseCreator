//! The work-area canvas: drawing the scene, the tools, selection, dragging, node overlays and the rulers.
//!
//! Everything that belongs to the application (the machine's bed, no-go zones, camera overlays, previews, menus)
//! is drawn or handled by the hooks of `EditorHost`; the defaults do nothing.
use super::{fit_region, fit_view, handle_pts, nice_step, Drag, EditorHost, NodeRef, Tool};
use crate::theme;
use base_core::bezier::Node;
use base_core::geom::{Polyline, Pt, Rect, Xf};
use base_core::scene::Kind;
use egui::{Color32, CursorIcon, PointerButton, Pos2, Sense, Stroke};

/// What the canvas hands to the hooks that draw: the painter (clipped to the canvas), the canvas rectangle on the
/// screen, its top-left corner `origin` (for `w2s` and `s2w`) and the work area (the "bed") on the screen.
pub struct CanvasView<'a> {
    pub painter: &'a egui::Painter,
    pub rect: egui::Rect,
    pub origin: Pos2,
    pub bed: egui::Rect,
}

/// The canvas. Implemented for every `EditorHost`.
pub trait EditorCanvas: EditorHost {
    fn canvas(&mut self, ui: &mut egui::Ui) {
        let avail = ui.available_size();
        let (resp, painter) = ui.allocate_painter(avail, Sense::click_and_drag());
        let rect = resp.rect;
        let o = rect.min;
        let (bw, bh) = self.work_size();

        self.ed_mut().canvas_avail = (rect.width(), rect.height());
        self.ed_mut().canvas_origin = rect.min;
        // Fit everything on the page once, on request.
        if self.ed().fit_all_req && rect.width() > 50.0 {
            self.ed_mut().fit_all_req = false;
            let all: Vec<u64> = self.scene().shapes.iter().filter(|s| self.layer_visible(s.layer)).map(|s| s.id).collect();
            let (min, size) = match self.scene().bounds_of(&all) {
                Some(b) => ((b.min.x, b.min.y), (b.width().max(1.0), b.height().max(1.0))),
                None => ((0.0, 0.0), (bw, bh)),
            };
            let (z, pan) = fit_region((rect.width(), rect.height()), min, size, 50.0);
            self.ed_mut().view.zoom = z;
            self.ed_mut().view.pan = pan;
            self.ed_mut().view.need_fit = false;
            self.ed_mut().view.auto_fit = false;
        }
        // Fit on request, and keep fitting while the window or the panels are resized.
        if (self.ed().view.need_fit || self.ed().view.auto_fit) && rect.width() > 50.0 {
            let (z, pan) = fit_view((rect.width(), rect.height()), (bw, bh), 50.0);
            self.ed_mut().view.zoom = z;
            self.ed_mut().view.pan = pan;
            self.ed_mut().view.need_fit = false;
        }

        // ---- zoom / pan ----
        if resp.hovered() {
            let (scroll, pinch) = ui.input(|i| (i.smooth_scroll_delta.y, i.zoom_delta()));
            let f = (scroll * 0.0015).exp() * pinch;
            if (f - 1.0).abs() > 1e-4 {
                if let Some(m) = resp.hover_pos() {
                    let before = self.s2w(o, m);
                    self.ed_mut().view.zoom = (self.ed().view.zoom * f).clamp(0.05, 200.0);
                    let after = self.w2s(o, before);
                    self.ed_mut().view.pan += m - after;
                    self.ed_mut().view.auto_fit = false;
                }
            }
        }
        if resp.dragged_by(PointerButton::Middle) || (self.ed().tool == Tool::Pan && resp.dragged_by(PointerButton::Primary)) {
            self.ed_mut().view.pan += resp.drag_delta();
            self.ed_mut().view.auto_fit = false;
        }
        self.ed_mut().cursor_mm = resp.hover_pos().map(|m| self.s2w(o, m));

        // ---- background, bed, grid ----
        painter.rect_filled(rect, 0.0, self.workspace_fill());
        let bed = egui::Rect::from_min_max(self.w2s(o, Pt::new(0.0, 0.0)), self.w2s(o, Pt::new(bw, bh)));
        painter.rect_filled(bed.translate(egui::vec2(3.0, 3.0)), 0.0, Color32::from_black_alpha(25));
        painter.rect_filled(bed, 0.0, self.bed_fill());
        let painter = painter.with_clip_rect(rect);
        {
            let gp = &self.ed().grid_prefs;
            let z = self.ed().view.zoom as f64;
            let lines = |step: f64, color: [u8; 4], width: f32| {
                // Skip grids so dense that the lines would merge into a tint.
                if step * z < 2.5 || step <= 0.0 {
                    return;
                }
                let st = Stroke::new(width, Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]));
                let mut x = 0.0;
                while x <= bw {
                    let sx = self.w2s(o, Pt::new(x, 0.0)).x;
                    painter.line_segment([Pos2::new(sx, bed.min.y), Pos2::new(sx, bed.max.y)], st);
                    x += step;
                }
                let mut y = 0.0;
                while y <= bh {
                    let sy = self.w2s(o, Pt::new(0.0, y)).y;
                    painter.line_segment([Pos2::new(bed.min.x, sy), Pos2::new(bed.max.x, sy)], st);
                    y += step;
                }
            };
            if gp.minor_on {
                lines(gp.minor_mm.min(self.ed().grid * 0.99), gp.minor_color, 0.6);
            }
            if self.ed().show_grid {
                lines(self.ed().grid, gp.main_color, 1.0);
            }
        }
        painter.rect_stroke(bed, 0.0, Stroke::new(1.0, Color32::from_gray(0x99)), egui::StrokeKind::Outside);
        let view = CanvasView { painter: &painter, rect, origin: o, bed };
        self.paint_work_area(&view);

        self.paint_under_shapes(&view);

        // ---- shapes ----
        let dim_pictures = self.dim_pictures();
        let ctx2 = ui.ctx().clone();
        self.ensure_textures(&ctx2);
        for s in &self.scene().shapes {
            let Kind::Image(im) = &s.kind else { continue };
            if !self.layer_visible(s.layer) {
                continue;
            }
            if let Some(tex) = self.ed().image_tex.get(&(s.id, im.invert)) {
                let corners = [Pt::new(0.0, 0.0), Pt::new(im.w, 0.0), Pt::new(im.w, im.h), Pt::new(0.0, im.h)];
                let uv = [egui::pos2(0.0, 0.0), egui::pos2(1.0, 0.0), egui::pos2(1.0, 1.0), egui::pos2(0.0, 1.0)];
                let tint = Color32::from_white_alpha(if dim_pictures { 70 } else { 255 });
                let mut mesh = egui::Mesh::with_texture(tex.id());
                for (c, t) in corners.iter().zip(uv) {
                    mesh.vertices.push(egui::epaint::Vertex { pos: self.w2s(o, s.xf.apply(*c)), uv: t, color: tint });
                }
                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                painter.add(egui::Shape::mesh(mesh));
            }
        }
        for s in &self.scene().shapes {
            let Some(c) = self.shape_color(s.layer) else { continue };
            let st = Stroke::new(self.ed().grid_prefs.line_width, c);
            for p in s.polys() {
                let pts: Vec<Pos2> = p.pts.iter().map(|q| self.w2s(o, *q)).collect();
                if pts.len() < 2 {
                    continue;
                }
                painter.add(if p.closed { egui::Shape::closed_line(pts, st) } else { egui::Shape::line(pts, st) });
            }
        }
        self.paint_over_shapes(&view);

        // Padlock on locked objects.
        let locked: Vec<Pt> = self.scene().shapes.iter().filter(|s| s.locked && self.layer_visible(s.layer)).filter_map(|s| s.bounds()).map(|b| Pt::new(b.max.x, b.min.y)).collect();
        for p in locked {
            let p = self.w2s(o, p);
            self.paint_lock(ui, egui::Rect::from_center_size(p + egui::vec2(-8.0, 8.0), egui::vec2(14.0, 14.0)));
        }
        // ---- selection ----
        if self.ed().sel.is_empty() || self.ed().tool != Tool::Select {
            self.ed_mut().rotate_mode = false;
        }
        if let Some(b) = self.sel_bounds() {
            let sr = egui::Rect::from_min_max(self.w2s(o, b.min), self.w2s(o, b.max));
            // Red while the handles resize, blue while they turn and slant.
            let ink = if self.ed().rotate_mode { Color32::from_rgb(0x2f, 0x7f, 0xe8) } else { Color32::from_rgb(0xe0, 0x30, 0x30) };
            painter.rect_stroke(sr, 0.0, Stroke::new(1.0, ink), egui::StrokeKind::Outside);
            let all_locked = self.ed().sel.iter().filter_map(|id| self.scene().shape(*id)).all(|s| s.locked);
            if self.ed().tool == Tool::Select && !all_locked {
                for (i, h) in handle_pts(&b).into_iter().enumerate() {
                    let p = self.w2s(o, h);
                    if !self.ed().rotate_mode {
                        let r = egui::Rect::from_center_size(p, egui::vec2(8.0, 8.0));
                        painter.rect_filled(r, 1.0, Color32::WHITE);
                        painter.rect_stroke(r, 1.0, Stroke::new(1.2, ink), egui::StrokeKind::Middle);
                    } else if i % 2 == 0 {
                        // Corners turn the selection: round handles.
                        painter.circle_filled(p, 5.0, Color32::WHITE);
                        painter.circle_stroke(p, 5.0, Stroke::new(1.4, ink));
                    } else {
                        // Edges slant it: diamond handles.
                        let d = 5.5;
                        let pts = vec![p + egui::vec2(0.0, -d), p + egui::vec2(d, 0.0), p + egui::vec2(0.0, d), p + egui::vec2(-d, 0.0)];
                        painter.add(egui::Shape::convex_polygon(pts, Color32::WHITE, Stroke::new(1.4, ink)));
                    }
                }
            }
        }

        // ---- node overlay ----
        if self.ed().tool == Tool::Node {
            let acc = theme::accent();
            let sel_nodes = self.ed().node_sel.clone();
            for (id, cs) in self.bezier_shapes() {
                for (ci, c) in cs.iter().enumerate() {
                    for (ni, n) in c.nodes.iter().enumerate() {
                        let selected = sel_nodes.contains(&(id, ci, ni));
                        let p = self.w2s(o, n.p);
                        if selected {
                            for (has, h) in [(n.has_in(), n.hin), (n.has_out(), n.hout)] {
                                if has {
                                    let hp = self.w2s(o, h);
                                    painter.line_segment([p, hp], Stroke::new(1.0, acc));
                                    painter.circle_filled(hp, 3.5, Color32::WHITE);
                                    painter.circle_stroke(hp, 3.5, Stroke::new(1.2, acc));
                                }
                            }
                        }
                        let fill = if selected { acc } else { Color32::WHITE };
                        if n.smooth {
                            painter.circle_filled(p, 4.5, fill);
                            painter.circle_stroke(p, 4.5, Stroke::new(1.3, acc));
                        } else {
                            let r = egui::Rect::from_center_size(p, egui::vec2(8.0, 8.0));
                            painter.rect_filled(r, 1.0, fill);
                            painter.rect_stroke(r, 1.0, Stroke::new(1.3, acc), egui::StrokeKind::Middle);
                        }
                    }
                    if let Some((aid, aci, asi)) = self.ed().active_seg {
                        if aid == id && aci == ci && asi < c.seg_count() {
                            let poly: Vec<Pos2> = (0..=16).map(|k| self.w2s(o, c.point_at(asi, k as f64 / 16.0))).collect();
                            painter.add(egui::Shape::line(poly, Stroke::new(3.0, acc.gamma_multiply(0.6))));
                        }
                    }
                }
            }
        }

        // ---- interaction ----
        let mouse = resp.hover_pos();
        let hpos = resp.interact_pointer_pos().or(mouse);
        let press = if resp.drag_started() { ui.input(|i| i.pointer.press_origin()).or(hpos) } else { hpos };
        let shift = ui.input(|i| i.modifiers.shift);
        // Shift or Ctrl (Cmd on macOS) adds to the selection (and takes out what is already in it) instead of replacing it.
        let multi = shift || ui.input(|i| i.modifiers.command);
        // Right click selects what is under the pointer, then the context menu offers the usual commands.
        if resp.secondary_clicked() && self.ed().tool == Tool::Select {
            if let Some(m) = hpos {
                match self.hit_shape(self.s2w(o, m)) {
                    Some(id) if !self.ed().sel.contains(&id) => self.ed_mut().sel = self.scene().group_of(id),
                    None => self.ed_mut().sel.clear(),
                    _ => {}
                }
            }
        }
        if !matches!(self.ed().tool, Tool::Pen | Tool::Pan | Tool::Zoom | Tool::ZoomOut) {
            self.canvas_context_menu(ui, &resp);
        }
        let overlay_busy = self.pointer_claimed(&resp, o, press, hpos);
        if !overlay_busy {
        match self.ed().tool {
            Tool::Select => {
                // Double-clicking a shape opens it in the node tool (text and pictures excluded).
                if resp.double_clicked() {
                    if let Some(m) = hpos {
                        if let Some(id) = self.hit_shape(self.s2w(o, m)) {
                            let editable = self.scene().shape(id).is_some_and(|s| !s.locked && !matches!(s.kind, Kind::Text(_) | Kind::Image(_)));
                            if editable {
                                self.ed_mut().sel = self.scene().group_of(id);
                                self.ed_mut().node_sel.clear();
                                self.ed_mut().tool = Tool::Node;
                            }
                        }
                    }
                }
                if let Some(m) = mouse {
                    if let Some(h) = self.hit_handle(o, m) {
                        ui.ctx().set_cursor_icon(match (self.ed().rotate_mode, h) {
                            (true, 0 | 2 | 4 | 6) => CursorIcon::Alias,
                            (true, 1 | 5) => CursorIcon::ResizeHorizontal,
                            (true, _) => CursorIcon::ResizeVertical,
                            (false, 0 | 4) => CursorIcon::ResizeNwSe,
                            (false, 2 | 6) => CursorIcon::ResizeNeSw,
                            (false, 1 | 5) => CursorIcon::ResizeVertical,
                            (false, _) => CursorIcon::ResizeHorizontal,
                        });
                    } else if self.hit_shape(self.s2w(o, m)).is_some() {
                        ui.ctx().set_cursor_icon(CursorIcon::Move);
                    }
                }
                if resp.drag_started_by(PointerButton::Primary) {
                    if let Some(m) = press {
                        let w = self.s2w(o, m);
                        if let Some(h) = self.hit_handle(o, m) {
                            let b = self.sel_bounds().unwrap();
                            let hp = handle_pts(&b);
                            self.checkpoint();
                            let orig = self.ed().sel.iter().filter_map(|i| self.scene().shape(*i).filter(|s| !s.locked).map(|s| (*i, s.xf))).collect();
                            self.ed_mut().drag = Some(if !self.ed().rotate_mode {
                                Drag::Scale { anchor: hp[(h + 4) % 8], grab: hp[h], handle: h, orig }
                            } else if h % 2 == 0 {
                                let centre = b.center();
                                Drag::Rotate { centre, from: (hp[h].y - centre.y).atan2(hp[h].x - centre.x), orig }
                            } else {
                                // Top and bottom edges slant sideways, left and right edges slant up and down.
                                Drag::Skew { horizontal: h % 4 == 1, anchor: hp[(h + 4) % 8], grab: hp[h], orig }
                            });
                        } else if let Some(id) = self.hit_shape(w) {
                            if !self.ed().sel.contains(&id) {
                                if !multi {
                                    self.ed_mut().sel.clear();
                                }
                                for m in self.scene().group_of(id) {
                                    if !self.ed().sel.contains(&m) {
                                        self.ed_mut().sel.push(m);
                                    }
                                }
                            }
                            self.checkpoint();
                            let orig = self.ed().sel.iter().filter_map(|i| self.scene().shape(*i).filter(|s| !s.locked).map(|s| (*i, s.xf))).collect();
                            self.ed_mut().drag = Some(Drag::Move { start: w, orig });
                        } else {
                            if !multi {
                                self.ed_mut().sel.clear();
                            }
                            self.ed_mut().drag = Some(Drag::Marquee { start: w });
                        }
                    }
                } else if resp.clicked() && hpos.is_some_and(|m| self.hit_handle(o, m).is_some()) {
                    // A click on a handle swaps between resizing and turning / slanting.
                    self.ed_mut().rotate_mode = !self.ed().rotate_mode;
                } else if resp.clicked() {
                    if let Some(m) = hpos {
                        let hit = self.hit_shape(self.s2w(o, m));
                        match hit {
                            Some(id) if multi => {
                                let members = self.scene().group_of(id);
                                if self.ed().sel.contains(&id) {
                                    self.ed_mut().sel.retain(|x| !members.contains(x));
                                } else {
                                    self.ed_mut().sel.extend(members);
                                }
                            }
                            Some(id) => self.ed_mut().sel = self.scene().group_of(id),
                            None => self.ed_mut().sel.clear(),
                        }
                    }
                }
            }
            Tool::Node => {
                self.nodes_prepare();
                if let Some(m) = mouse {
                    if self.hit_node_handle(o, m).is_some() || self.hit_node(o, m).is_some() {
                        ui.ctx().set_cursor_icon(CursorIcon::PointingHand);
                    }
                }
                if resp.drag_started_by(PointerButton::Primary) {
                    if let Some(m) = press {
                        let w = self.s2w(o, m);
                        if let Some((nr, out)) = self.hit_node_handle(o, m) {
                            self.checkpoint();
                            self.ed_mut().drag = Some(Drag::HandleMove { node: nr, out });
                        } else if let Some(nr) = self.hit_node(o, m) {
                            if !self.ed().node_sel.contains(&nr) {
                                if !multi {
                                    self.ed_mut().node_sel.clear();
                                }
                                self.ed_mut().node_sel.push(nr);
                            }
                            self.ed_mut().active_seg = None;
                            self.checkpoint();
                            let orig: Vec<(NodeRef, Node)> = self.ed().node_sel.iter().filter_map(|r| self.node_at(*r).map(|n| (*r, n))).collect();
                            self.ed_mut().drag = Some(Drag::NodeMove { start: w, orig });
                        } else {
                            if !multi {
                                self.ed_mut().node_sel.clear();
                            }
                            self.ed_mut().active_seg = None;
                            self.ed_mut().drag = Some(Drag::NodeMarquee { start: w });
                        }
                    }
                } else if resp.double_clicked() {
                    if let Some(m) = hpos {
                        if let Some((seg, t)) = self.hit_segment(self.s2w(o, m)) {
                            self.nodes_insert(Some((seg, t)));
                        }
                    }
                } else if resp.clicked() {
                    if let Some(m) = hpos {
                        let w = self.s2w(o, m);
                        if let Some(nr) = self.hit_node(o, m) {
                            if multi {
                                if let Some(i) = self.ed().node_sel.iter().position(|x| *x == nr) {
                                    self.ed_mut().node_sel.remove(i);
                                } else {
                                    self.ed_mut().node_sel.push(nr);
                                }
                            } else {
                                self.ed_mut().node_sel = vec![nr];
                            }
                            self.ed_mut().active_seg = None;
                        } else if let Some((seg, _)) = self.hit_segment(w) {
                            self.ed_mut().active_seg = Some(seg);
                            self.ed_mut().node_sel.clear();
                        } else if let Some(id) = self.hit_shape(w) {
                            self.ed_mut().sel = vec![id];
                            self.ed_mut().node_sel.clear();
                            self.ed_mut().active_seg = None;
                        } else {
                            self.ed_mut().sel.clear();
                            self.ed_mut().node_sel.clear();
                            self.ed_mut().active_seg = None;
                        }
                    }
                }
            }
            Tool::Text => {
                ui.ctx().set_cursor_icon(CursorIcon::Text);
                if resp.clicked_by(PointerButton::Primary) {
                    if let Some(m) = hpos {
                        let p = self.snapped(self.s2w(o, m));
                        self.place_text(p);
                    }
                }
            }
            Tool::Rect | Tool::Ellipse | Tool::Triangle | Tool::Star | Tool::Polygon | Tool::Heart | Tool::Spiral | Tool::Line => {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                if resp.drag_started_by(PointerButton::Primary) {
                    if let Some(m) = press {
                        // Starting another polygon or spiral brings the small window back.
                        self.ed_mut().show_shape_popup = matches!(self.ed().tool, Tool::Polygon | Tool::Spiral);
                        self.ed_mut().drag = Some(Drag::Create { start: self.snapped(self.s2w(o, m)) });
                    }
                }
            }
            Tool::Pen => {
                ui.ctx().set_cursor_icon(CursorIcon::Crosshair);
                if resp.clicked_by(PointerButton::Primary) {
                    if let Some(m) = hpos {
                        let p = self.snapped(self.s2w(o, m));
                        let close = self.ed().pen_pts.len() >= 3 && self.w2s(o, self.ed().pen_pts[0]).distance(m) < 9.0;
                        if close {
                            self.finish_pen(true);
                        } else {
                            self.ed_mut().pen_pts.push(p);
                        }
                    }
                }
                if resp.double_clicked() {
                    self.ed_mut().pen_pts.pop();
                    self.finish_pen(false);
                }
                if resp.secondary_clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.finish_pen(false);
                }
            }
            Tool::Zoom | Tool::ZoomOut => {
                // Zoom in clicks in, zoom out clicks out; Shift swaps them.
                let out = (self.ed().tool == Tool::ZoomOut) != shift;
                ui.ctx().set_cursor_icon(if out { CursorIcon::ZoomOut } else { CursorIcon::ZoomIn });
                if resp.clicked() {
                    if let Some(m) = hpos {
                        let f = if out { 0.5 } else { 2.0 };
                        let before = self.s2w(o, m);
                        self.ed_mut().view.zoom = (self.ed().view.zoom * f).clamp(0.05, 200.0);
                        let after = self.w2s(o, before);
                        self.ed_mut().view.pan += m - after;
                        self.ed_mut().view.auto_fit = false;
                    }
                }
            }
            Tool::Pan => ui.ctx().set_cursor_icon(if resp.dragged() { CursorIcon::Grabbing } else { CursorIcon::Grab }),
        }
        }

        // drag update
        if resp.dragged_by(PointerButton::Primary) {
            let drag = self.ed().drag.clone();
            if let (Some(m), Some(d)) = (hpos, drag.as_ref()) {
                let w = self.s2w(o, m);
                match d {
                    Drag::Move { start, orig } => {
                        let (mut dx, mut dy) = (w.x - start.x, w.y - start.y);
                        if self.ed().snap {
                            dx = (dx / self.ed().grid).round() * self.ed().grid;
                            dy = (dy / self.ed().grid).round() * self.ed().grid;
                        }
                        let orig = orig.clone();
                        if self.ed().snap_objects {
                            // Where the selection would be, against the other objects: lines within a few pixels join.
                            let ids: Vec<u64> = orig.iter().map(|(id, _)| *id).collect();
                            if self.ed().snap_targets.is_none() {
                                let others = self.scene().shapes.iter().filter(|s| !ids.contains(&s.id) && self.layer_visible(s.layer)).filter_map(|s| s.bounds()).collect();
                                self.ed_mut().snap_targets = Some(others);
                            }
                            for (id, xf) in &orig {
                                if let Some(s) = self.scene_mut().unlocked_mut(*id) {
                                    s.xf = *xf;
                                }
                            }
                            if let Some(b) = self.scene().bounds_of(&ids) {
                                let moved = Rect { min: Pt::new(b.min.x + dx, b.min.y + dy), max: Pt::new(b.max.x + dx, b.max.y + dy) };
                                let s = base_core::snap::snap_to_objects(moved, self.ed().snap_targets.as_deref().unwrap_or(&[]), 8.0 / self.ed().view.zoom as f64);
                                dx += s.dx;
                                dy += s.dy;
                                self.ed_mut().snap_guides = (s.guide_x, s.guide_y);
                            }
                        }
                        for (id, xf) in orig {
                            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                                s.xf = xf.then(Xf::translate(dx, dy));
                            }
                        }
                        self.touch();
                    }
                    Drag::Scale { anchor, grab, handle, orig } => {
                        let (anchor, grab, handle) = (*anchor, *grab, *handle);
                        let w = self.snapped(w);
                        let rx = if (grab.x - anchor.x).abs() > 1e-6 { (w.x - anchor.x) / (grab.x - anchor.x) } else { 1.0 };
                        let ry = if (grab.y - anchor.y).abs() > 1e-6 { (w.y - anchor.y) / (grab.y - anchor.y) } else { 1.0 };
                        let (mut sx, mut sy) = match handle {
                            1 | 5 => (1.0, ry),
                            3 | 7 => (rx, 1.0),
                            _ => (rx, ry),
                        };
                        if matches!(handle, 0 | 2 | 4 | 6) && !shift {
                            let k = if rx.abs() > ry.abs() { rx } else { ry };
                            sx = k;
                            sy = k;
                        }
                        let (sx, sy) = (if sx.abs() < 0.001 { 0.001 } else { sx }, if sy.abs() < 0.001 { 0.001 } else { sy });
                        let orig = orig.clone();
                        for (id, xf) in orig {
                            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                                s.xf = xf.then(Xf::scale_about(sx, sy, anchor));
                            }
                        }
                        self.touch();
                    }
                    Drag::Rotate { centre, from, orig } => {
                        let (centre, from) = (*centre, *from);
                        let mut a = (w.y - centre.y).atan2(w.x - centre.x) - from;
                        if shift {
                            let step = 15f64.to_radians();
                            a = (a / step).round() * step;
                        }
                        let orig = orig.clone();
                        for (id, xf) in orig {
                            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                                s.xf = xf.then(Xf::rotate_about(a, centre));
                            }
                        }
                        self.touch();
                    }
                    Drag::Skew { horizontal, anchor, grab, orig } => {
                        let (horizontal, anchor, grab) = (*horizontal, *anchor, *grab);
                        let t = if horizontal {
                            let span = grab.y - anchor.y;
                            let k = if span.abs() > 1e-6 { (w.x - grab.x) / span } else { 0.0 };
                            Xf::skew_about(k.clamp(-4.0, 4.0), 0.0, anchor)
                        } else {
                            let span = grab.x - anchor.x;
                            let k = if span.abs() > 1e-6 { (w.y - grab.y) / span } else { 0.0 };
                            Xf::skew_about(0.0, k.clamp(-4.0, 4.0), anchor)
                        };
                        let orig = orig.clone();
                        for (id, xf) in orig {
                            if let Some(s) = self.scene_mut().unlocked_mut(id) {
                                s.xf = xf.then(t);
                            }
                        }
                        self.touch();
                    }
                    Drag::NodeMove { start, orig } => {
                        let (mut dx, mut dy) = (w.x - start.x, w.y - start.y);
                        if self.ed().snap {
                            dx = (dx / self.ed().grid).round() * self.ed().grid;
                            dy = (dy / self.ed().grid).round() * self.ed().grid;
                        }
                        let orig = orig.clone();
                        for ((id, ci, ni), n0) in orig {
                            if let Some(c) = self.contour_mut(id, ci) {
                                if ni < c.nodes.len() {
                                    let mut n = n0;
                                    n.translate(dx, dy);
                                    c.nodes[ni] = n;
                                }
                            }
                        }
                        self.touch();
                    }
                    Drag::HandleMove { node, out } => {
                        let (node, out) = (*node, *out);
                        let w = self.snapped(w);
                        if let Some(c) = self.contour_mut(node.0, node.1) {
                            if node.2 < c.nodes.len() {
                                c.drag_handle(node.2, out, w);
                            }
                        }
                        self.touch();
                    }
                    Drag::NodeMarquee { start } => {
                        let r = egui::Rect::from_two_pos(self.w2s(o, *start), m);
                        painter.rect_filled(r, 0.0, theme::accent().gamma_multiply(0.1));
                        painter.rect_stroke(r, 0.0, Stroke::new(1.0, theme::accent()), egui::StrokeKind::Middle);
                    }
                    Drag::Marquee { start } => {
                        // Left to right selects what is wholly inside (blue); right to left also what is only touched (green).
                        let c = if w.x >= start.x { theme::accent() } else { Color32::from_rgb(0x2e, 0xa0, 0x4f) };
                        let r = egui::Rect::from_two_pos(self.w2s(o, *start), m);
                        painter.rect_filled(r, 0.0, c.gamma_multiply(0.1));
                        painter.rect_stroke(r, 0.0, Stroke::new(1.0, c), egui::StrokeKind::Middle);
                    }
                    Drag::Create { start } => {
                        let end = self.snapped(w);
                        let st = Stroke::new(self.ed().grid_prefs.line_width, self.layer_color(self.ed().active_layer));
                        let (a, b) = (self.w2s(o, *start), self.w2s(o, end));
                        match self.ed().tool {
                            Tool::Rect => {
                                painter.rect_stroke(egui::Rect::from_two_pos(a, b), 0.0, st, egui::StrokeKind::Middle);
                            }
                            Tool::Ellipse => {
                                let r = egui::Rect::from_two_pos(a, b);
                                let pts: Vec<Pos2> = (0..64)
                                    .map(|i| {
                                        let t = i as f32 / 64.0 * std::f32::consts::TAU;
                                        r.center() + egui::vec2(t.cos() * r.width() / 2.0, t.sin() * r.height() / 2.0)
                                    })
                                    .collect();
                                painter.add(egui::Shape::closed_line(pts, st));
                            }
                            Tool::Triangle | Tool::Star | Tool::Polygon | Tool::Heart | Tool::Spiral => {
                                let r = egui::Rect::from_two_pos(a, b);
                                let (wm, hm) = ((r.width() / self.ed().view.zoom) as f64, (r.height() / self.ed().view.zoom) as f64);
                                if let Some(ct) = self.auto_contour(wm, hm) {
                                    let pts: Vec<Pos2> = ct.flatten(0.1).pts.iter().map(|p| r.min + egui::vec2(p.x as f32 * self.ed().view.zoom, p.y as f32 * self.ed().view.zoom)).collect();
                                    painter.add(if ct.closed { egui::Shape::closed_line(pts, st) } else { egui::Shape::line(pts, st) });
                                }
                            }
                            _ => {
                                painter.line_segment([a, b], st);
                            }
                        }
                    }
                }
            }
        }
        // Guide lines of the object snapping, through the whole work area.
        if self.ed().drag.is_some() {
            let st = Stroke::new(1.0, Color32::from_rgb(0xe0, 0x30, 0xc0));
            if let Some(x) = self.ed().snap_guides.0 {
                let sx = self.w2s(o, Pt::new(x, 0.0)).x;
                painter.line_segment([Pos2::new(sx, bed.min.y), Pos2::new(sx, bed.max.y)], st);
            }
            if let Some(y) = self.ed().snap_guides.1 {
                let sy = self.w2s(o, Pt::new(0.0, y)).y;
                painter.line_segment([Pos2::new(bed.min.x, sy), Pos2::new(bed.max.x, sy)], st);
            }
        }
        // The tool cursors belong to the work area: outside it (and while nothing is being dragged) the pointer is the ordinary one.
        if self.ed().drag.is_none() && !mouse.is_some_and(|m| bed.contains(m)) {
            ui.ctx().set_cursor_icon(CursorIcon::Default);
        }
        if resp.drag_stopped_by(PointerButton::Primary) {
            self.ed_mut().snap_guides = (None, None);
            self.ed_mut().snap_targets = None;
            if let (Some(d), Some(m)) = (self.ed_mut().drag.take(), hpos) {
                let w = self.s2w(o, m);
                match d {
                    Drag::Marquee { start } => {
                        let r = Rect::from_pts([start, w]).unwrap();
                        let hits = self.marquee_hits(&r, w.x >= start.x);
                        for id in hits {
                            for m in self.scene().group_of(id) {
                                if !self.ed().sel.contains(&m) {
                                    self.ed_mut().sel.push(m);
                                }
                            }
                        }
                    }
                    Drag::NodeMarquee { start } => {
                        let r = Rect::from_pts([start, w]).unwrap();
                        let mut hits = vec![];
                        for (id, cs) in self.bezier_shapes() {
                            for (ci, c) in cs.iter().enumerate() {
                                for (ni, n) in c.nodes.iter().enumerate() {
                                    if r.contains(n.p) {
                                        hits.push((id, ci, ni));
                                    }
                                }
                            }
                        }
                        for h in hits {
                            if !self.ed().node_sel.contains(&h) {
                                self.ed_mut().node_sel.push(h);
                            }
                        }
                    }
                    Drag::Create { start } => {
                        let mut end = self.snapped(w);
                        if shift && matches!(self.ed().tool, Tool::Rect | Tool::Ellipse | Tool::Triangle | Tool::Star | Tool::Polygon | Tool::Heart | Tool::Spiral) {
                            let s = (end.x - start.x).abs().max((end.y - start.y).abs());
                            end = Pt::new(start.x + s * (end.x - start.x).signum(), start.y + s * (end.y - start.y).signum());
                        }
                        let r = Rect::from_pts([start, end]).unwrap();
                        if r.width().max(r.height()) > 0.2 {
                            self.checkpoint();
                            let layer = self.ed().active_layer;
                            let id = match self.ed().tool {
                                Tool::Rect => self.scene_mut().add(layer, Kind::Rect { w: r.width(), h: r.height() }, Xf::translate(r.min.x, r.min.y)),
                                Tool::Ellipse => self.scene_mut().add(layer, Kind::Ellipse { w: r.width(), h: r.height() }, Xf::translate(r.min.x, r.min.y)),
                                Tool::Triangle | Tool::Star | Tool::Polygon | Tool::Heart | Tool::Spiral => match self.auto_contour(r.width(), r.height()) {
                                    Some(ct) => self.scene_mut().add(layer, Kind::Bezier(vec![ct]), Xf::translate(r.min.x, r.min.y)),
                                    None => 0,
                                },
                                _ => self.scene_mut().add(layer, Kind::Path(vec![Polyline::new(vec![start, end], false)]), Xf::IDENTITY),
                            };
                            self.ed_mut().sel = vec![id];
                            if matches!(self.ed().tool, Tool::Polygon | Tool::Spiral) {
                                // The small window stays while the tool is active; its number now edits this shape.
                                self.ed_mut().last_param = Some((self.ed().tool, id, r.width(), r.height()));
                            }
                        }
                    }
                    _ => {}
                }
            }
        }

        // pen preview
        if self.ed().tool == Tool::Pen && !self.ed().pen_pts.is_empty() {
            let st = Stroke::new(self.ed().grid_prefs.line_width, self.layer_color(self.ed().active_layer));
            let mut pts: Vec<Pos2> = self.ed().pen_pts.iter().map(|p| self.w2s(o, *p)).collect();
            if let Some(m) = mouse {
                pts.push(m);
            }
            for p in &pts {
                painter.circle_filled(*p, 3.0, theme::accent());
            }
            painter.add(egui::Shape::line(pts, st));
        }

        self.rulers(&painter, rect);

        // Floating node-editing toolbar: it goes away when you click another window, such as the preview, and comes
        // back when you click the work area again.
        if self.ed().tool != Tool::Node {
            self.ed_mut().node_bar_active = true;
        } else if let Some(p) = ui.input(|i| if i.pointer.any_pressed() { i.pointer.interact_pos() } else { None }) {
            self.ed_mut().node_bar_active = ui.ctx().layer_id_at(p).map_or(true, |l| l.order == egui::Order::Background || l.id == egui::Id::unique("node_bar"));
        }
        if self.ed().tool == Tool::Node && self.ed().node_bar_active {
            let ctx = ui.ctx().clone();
            egui::Area::new(egui::Id::unique("node_bar")).order(egui::Order::Middle).fixed_pos(rect.min + egui::vec2(28.0, 26.0)).show(&ctx, |ui| {
                egui::Frame::popup(ui.style()).show(ui, |ui| {
                    ui.horizontal(|ui| self.node_bar(ui));
                });
            });
        }
    }


    fn rulers(&self, painter: &egui::Painter, rect: egui::Rect) {
        let t = 18.0;
        let bg = theme::panel();
        painter.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(rect.width(), t)), 0.0, bg);
        painter.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(t, rect.height())), 0.0, bg);
        // Work in display units: `k` is millimetres per unit, `z` pixels per millimetre.
        let units = self.units();
        let k = units.to_mm(1.0);
        let z = self.ed().view.zoom as f64;
        let major = nice_step(60.0 / (z * k));
        let minor = major / 5.0;
        let decimals = if major >= 1.0 { 0 } else { (-major.log10()).ceil() as usize };
        let label = |v: f64| format!("{:.*}", decimals, v);
        let font = egui::FontId::proportional(9.5);
        let tick = Stroke::new(1.0, theme::text_dim());
        // horizontal
        let (x0, x1) = ((-self.ed().view.pan.x) as f64 / (z * k), (rect.width() - self.ed().view.pan.x) as f64 / (z * k));
        let mut v = (x0 / minor).floor() * minor;
        while v <= x1 {
            let sx = rect.min.x + self.ed().view.pan.x + (v * k * z) as f32;
            let is_major = ((v / major).round() * major - v).abs() < minor * 0.01;
            if sx > rect.min.x + t {
                painter.line_segment([Pos2::new(sx, rect.min.y + if is_major { 2.0 } else { 12.0 }), Pos2::new(sx, rect.min.y + t)], tick);
                if is_major {
                    painter.text(Pos2::new(sx + 2.0, rect.min.y + 1.0), egui::Align2::LEFT_TOP, label(v), font.clone(), theme::text_dim());
                }
            }
            v += minor;
        }
        let (y0, y1) = ((-self.ed().view.pan.y) as f64 / (z * k), (rect.height() - self.ed().view.pan.y) as f64 / (z * k));
        let mut v = (y0 / minor).floor() * minor;
        while v <= y1 {
            let sy = rect.min.y + self.ed().view.pan.y + (v * k * z) as f32;
            let is_major = ((v / major).round() * major - v).abs() < minor * 0.01;
            if sy > rect.min.y + t {
                painter.line_segment([Pos2::new(rect.min.x + if is_major { 2.0 } else { 12.0 }, sy), Pos2::new(rect.min.x + t, sy)], tick);
                if is_major {
                    painter.text(Pos2::new(rect.min.x + 1.0, sy + 2.0), egui::Align2::LEFT_TOP, label(v), font.clone(), theme::text_dim());
                }
            }
            v += minor;
        }
        painter.rect_filled(egui::Rect::from_min_size(rect.min, egui::vec2(t, t)), 0.0, theme::panel_dark());
    }
}

impl<T: EditorHost> EditorCanvas for T {}
