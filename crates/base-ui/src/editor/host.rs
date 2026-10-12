//! What an editor needs from the application that hosts it, and the editing operations built on that.
//!
//! The application implements the required methods of `EditorHost` (where its scene and its `EditorState` are, how
//! its undo works, which layers are shown); everything else is provided here.
use super::{handle_pts, EditorState, NodeRef, Tool};
use base_core::bezier::{Contour, Node};
use base_core::geom::{Pt, Rect, Xf};
use base_core::scene::{Kind, Scene};
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
    /// Show a message in the status line.
    fn say(&mut self, msg: String);
    /// The message now shown in the status line.
    fn said(&self) -> &str;

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
}
