//! Tests of the editor against a minimal host (a scene and an editor state, nothing else).
use super::*;
use base_core::geom::Xf;
use base_core::scene::{Kind, Scene};

#[derive(Default)]
struct Host {
    scene: Scene,
    ed: EditorState,
    status: String,
    undo: Vec<Scene>,
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
