//! Tests of the editor against a minimal host (a scene and an editor state, nothing else).
use super::*;
use crate::clipart_ui::{ClipGallery, ClipHost, ClipState};
use base_core::geom::Xf;
use base_core::scene::{Kind, Scene};

const CAT_SVG: &[u8] = br##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2,2L22,2L22,12L2,12Z M5,15L19,15L19,22L5,22Z"/></svg>"##;
const BUILTIN: crate::clipart_ui::Builtin = &[("Animals", "cat", CAT_SVG)];

struct Host {
    scene: Scene,
    ed: EditorState,
    status: String,
    undo: Vec<Scene>,
    clip: ClipState,
}

impl Default for Host {
    fn default() -> Host {
        // Never touch the real user library in tests.
        let dir = std::env::temp_dir().join(format!("base-ui-clip-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        Host { scene: Scene::default(), ed: EditorState::default(), status: String::new(), undo: vec![], clip: ClipState::with_builtin(base_core::clipart::Library::open(dir), BUILTIN) }
    }
}

impl ClipHost for Host {
    fn clip(&self) -> &ClipState {
        &self.clip
    }
    fn clip_mut(&mut self) -> &mut ClipState {
        &mut self.clip
    }
}

impl EditorHost for Host {
    fn ed(&self) -> &EditorState {
        &self.ed
    }
    fn ed_mut(&mut self) -> &mut EditorState {
        &mut self.ed
    }
    fn scene(&self) -> &Scene {
        &self.scene
    }
    fn scene_mut(&mut self) -> &mut Scene {
        &mut self.scene
    }
    fn layer_visible(&self, layer: usize) -> bool {
        layer != 9
    }
    fn checkpoint(&mut self) {
        self.undo.push(self.scene.clone());
    }
    fn touch(&mut self) {}
    fn undo(&mut self) {}
    fn redo(&mut self) {}
    fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    fn can_redo(&self) -> bool {
        false
    }
    fn drop_checkpoint(&mut self) {
        self.undo.pop();
    }
    fn open_shape_generators(&mut self) {}
    fn open_online_clipart_search(&mut self, _ctx: &egui::Context) {}
    fn say(&mut self, msg: String) {
        self.status = msg;
    }
    fn said(&self) -> &str {
        &self.status
    }
    fn work_size(&self) -> (f64, f64) {
        (200.0, 100.0)
    }
    fn units(&self) -> base_core::units::Units {
        base_core::units::Units::Mm
    }
    fn workspace_fill(&self) -> egui::Color32 {
        egui::Color32::DARK_GRAY
    }
    fn bed_fill(&self) -> egui::Color32 {
        egui::Color32::WHITE
    }
    fn shape_color(&self, layer: usize) -> Option<egui::Color32> {
        self.layer_visible(layer).then_some(egui::Color32::BLACK)
    }
    fn layer_color(&self, _layer: usize) -> egui::Color32 {
        egui::Color32::BLACK
    }
}

fn host_with_two_rects() -> (Host, u64, u64) {
    let mut h = Host::default();
    let a = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(0.0, 0.0));
    let b = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(20.0, 0.0));
    (h, a, b)
}

#[test]
fn commands_select_group_move_copy_and_delete() {
    let (mut h, a, b) = host_with_two_rects();
    h.select_all();
    assert_eq!(h.ed.sel, vec![a, b]);
    h.group_selection();
    assert_eq!(h.scene.group_of(a).len(), 2);
    h.transform_selection(Xf::translate(5.0, 0.0));
    assert!((h.sel_bounds().unwrap().min.x - 5.0).abs() < 1e-9);
    h.duplicate();
    assert_eq!(h.scene.shapes.len(), 4);
    assert_ne!(h.scene.shapes[2].group, h.scene.shapes[0].group, "pasted groups are new groups");
    h.delete_selection();
    assert_eq!(h.scene.shapes.len(), 2);
    assert!(h.ed.sel.is_empty());
}

#[test]
fn locked_shapes_survive_delete_and_a_hidden_layer_cannot_be_picked() {
    let (mut h, a, b) = host_with_two_rects();
    h.scene.shape_mut(a).unwrap().locked = true;
    h.ed.sel = vec![a, b];
    h.delete_selection();
    assert_eq!(h.scene.shapes.len(), 1);
    assert_eq!(h.ed.sel, vec![a]);
    h.scene.shape_mut(a).unwrap().layer = 9;
    assert_eq!(h.hit_shape(base_core::geom::Pt::new(5.0, 5.0)), None);
}

#[test]
fn node_editing_bakes_the_transform_and_removes_nodes() {
    let mut h = Host::default();
    let id = h.scene.add(0, Kind::Ellipse { w: 20.0, h: 10.0 }, Xf::translate(5.0, 5.0));
    h.ed.sel = vec![id];
    h.nodes_prepare();
    let nodes = |h: &Host| match &h.scene.shape(id).unwrap().kind {
        Kind::Bezier(cs) => cs[0].nodes.len(),
        _ => 0,
    };
    assert_eq!(nodes(&h), 4);
    h.ed.node_sel = vec![(id, 0, 0)];
    h.delete_nodes();
    assert_eq!(nodes(&h), 3);
    h.nodes_insert(Some(((id, 0, 1), 0.5)));
    assert_eq!(nodes(&h), 4);
    h.ed.sel.clear();
    h.start_node_edit();
    assert!(h.status.contains("Select a shape first"));
}

#[test]
fn the_canvas_draws_and_a_shape_is_created_by_the_tool_state() {
    let (mut h, _, _) = host_with_two_rects();
    h.ed.sel = vec![1];
    let ctx = egui::Context::default();
    for tool in [Tool::Select, Tool::Node, Tool::Rect, Tool::Pen] {
        h.ed.tool = tool;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::no_frame().show(ui, |ui| h.canvas(ui));
        });
        out.textures_delta.clear();
    }
    assert!(h.ed.canvas_avail.0 > 50.0, "the canvas knows its size");
    // The pen finishes a line into a shape on the active layer.
    h.ed.pen_pts = vec![base_core::geom::Pt::new(0.0, 0.0), base_core::geom::Pt::new(10.0, 10.0)];
    let before = h.scene.shapes.len();
    h.finish_pen(false);
    assert_eq!(h.scene.shapes.len(), before + 1);
}

#[test]
fn placing_a_picture_uses_the_active_layer_scales_centres_groups_and_can_be_undone() {
    let mut h = Host::default();
    h.ed.active_layer = 5;
    h.clip.size_mm = 30.0;
    h.place_clip(CAT_SVG, "cat");
    assert!(!h.ed.sel.is_empty());
    assert!(h.scene.shapes.iter().all(|s| s.layer == 5));
    let b = h.scene.bounds_of(&h.ed.sel).unwrap();
    assert!((b.width().max(b.height()) - 30.0).abs() < 1e-6);
    assert!(b.center().dist(base_core::geom::Pt::new(100.0, 50.0)) < 1e-6, "the middle of the 200 x 100 work area");
    assert_eq!(h.undo.len(), 1);
    // A broken picture changes nothing and leaves no undo step behind.
    h.place_clip(b"<svg", "broken");
    assert_eq!(h.undo.len(), 1);
    assert!(h.status.contains("Import failed"), "{}", h.status);
}

#[test]
fn user_pictures_mix_with_shipped_ones_per_category() {
    let mut h = Host::default();
    let svg = String::from_utf8_lossy(CAT_SVG).to_string();
    h.clip.lib.add(&svg, "my cat", "Animals", "https://x.org/cat.svg", "CC0", "me").unwrap();
    h.clip.lib.add(&svg, "my dog", "Pets", "https://x.org/dog.svg", "MIT", "me").unwrap();
    h.clip.reload();
    let cats = h.clip.categories();
    assert!(cats.contains(&"Animals".to_string()) && cats.contains(&"Pets".to_string()));
    assert_eq!(h.clip.user_svgs.len(), 2);
    h.clip.lib.set_category("my-dog", "Animals").unwrap();
    assert!(h.clip.lib.items.iter().all(|c| c.category == "Animals"));
    assert!(h.clip.user_categories().contains(&"Animals".to_string()));
}

#[test]
fn the_gallery_panel_draws() {
    let mut h = Host::default();
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        egui::CentralPanel::no_frame().show(ui, |ui| h.clipart_panel(ui));
    });
    out.textures_delta.clear();
}

#[test]
fn the_shared_bars_and_the_properties_sections_draw() {
    let (mut h, a, _) = host_with_two_rects();
    let ctx = egui::Context::default();
    let mut shown = vec![];
    for sel in [vec![], vec![a]] {
        h.ed.sel = sel;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            control_name(&h, ui);
            grid_toggles(&mut h, ui);
            selection_size(&h, ui);
            zoom_and_cursor(&h, ui);
            shown.push(properties_sections(&mut h, ui));
        });
        out.textures_delta.clear();
    }
    // A frame may run in more than one pass: the first is without a selection, the last with one.
    assert!(!shown[0] && *shown.last().unwrap(), "the sections need a selection: {shown:?}");
}

#[test]
fn auto_join_makes_one_closed_path_of_separate_lines_and_close_only_closes_what_is_near() {
    use base_core::geom::{Polyline, Pt};
    let mut h = Host::default();
    let line = |a: (f64, f64), b: (f64, f64)| Kind::Path(vec![Polyline::new(vec![Pt::new(a.0, a.1), Pt::new(b.0, b.1)], false)]);
    let mut ids = vec![];
    for (a, b) in [((0.0, 0.0), (20.0, 0.0)), ((20.0, 0.0), (20.0, 10.0)), ((20.0, 10.0), (0.0, 10.0)), ((0.0, 10.0), (0.05, 0.0))] {
        ids.push(h.scene.add(0, line(a, b), Xf::IDENTITY));
    }
    h.ed.sel = ids.clone();
    // A tolerance smaller than the gap of 0.05: the lines join into one open path.
    EditorShapeOps::join_selected(&mut h, 0.01, true);
    assert_eq!(h.scene.shapes.len(), 1, "{}", h.status);
    let open = match &h.scene.shapes[0].kind {
        Kind::Bezier(cs) => !cs[0].closed,
        _ => false,
    };
    assert!(open, "the gap is wider than the tolerance");
    // Close it with a tolerance that is wide enough.
    h.ed.sel = vec![h.scene.shapes[0].id];
    EditorShapeOps::join_selected(&mut h, 0.1, false);
    assert!(matches!(&h.scene.shapes[0].kind, Kind::Bezier(cs) if cs[0].closed));
    // Nothing to do on a closed path: said so, the scene is as it was.
    let before = h.scene.shapes.len();
    h.ed.sel = vec![h.scene.shapes[0].id];
    EditorShapeOps::join_selected(&mut h, 0.1, true);
    assert_eq!(h.scene.shapes.len(), before);
    assert!(h.status.contains("Nothing"), "{}", h.status);
}

#[test]
fn dogbone_corners_are_added_to_the_selected_shapes_and_a_shape_without_corners_says_so() {
    use base_core::geom::{Polyline, Pt};
    let mut h = Host::default();
    let slot = h.scene.add(0, Kind::Rect { w: 30.0, h: 10.0 }, Xf::translate(5.0, 5.0));
    h.ed.sel = vec![slot];
    EditorShapeOps::dogbone_selected(&mut h, 1.5, false);
    assert_eq!(h.scene.shapes.len(), 1);
    assert!(matches!(&h.scene.shapes[0].kind, Kind::Path(p) if p.len() == 1 && p[0].pts.len() > 4), "{}", h.status);
    assert!(h.status.contains("Notches added"), "{}", h.status);
    // A circle has no sharp corner.
    let round = Polyline::new((0..64).map(|i| { let a = i as f64 / 64.0 * std::f64::consts::TAU; Pt::new(50.0 + 5.0 * a.cos(), 50.0 + 5.0 * a.sin()) }).collect(), true);
    let id = h.scene.add(0, Kind::Path(vec![round]), Xf::IDENTITY);
    h.ed.sel = vec![id];
    let before = h.scene.shapes.len();
    EditorShapeOps::dogbone_selected(&mut h, 1.5, true);
    assert_eq!(h.scene.shapes.len(), before);
    assert!(h.status.contains("No sharp corners"), "{}", h.status);
}
