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

#[test]
fn arrays_copy_the_selection_in_a_grid_and_round_a_circle() {
    let mut h = Host::default();
    let id = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(0.0, 0.0));
    h.ed.sel = vec![id];
    EditorDesign::grid_array(&mut h, 3, 2, 20.0, 15.0);
    assert_eq!(h.scene.shapes.len(), 6, "{}", h.status);
    assert_eq!(h.ed.sel.len(), 6, "originals and copies stay selected");
    let b = h.scene.bounds_of(&h.ed.sel.clone()).unwrap();
    assert!((b.max.x - 50.0).abs() < 1e-9 && (b.max.y - 25.0).abs() < 1e-9, "{b:?}");
    // A circle of four copies of one square, turned: the box of all of them is centred on the square's own centre... the square's centre is the centre of the circle.
    let mut h = Host::default();
    let id = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(0.0, 0.0));
    h.ed.sel = vec![id];
    EditorDesign::circle_array(&mut h, 4, 360.0, true);
    assert_eq!(h.scene.shapes.len(), 4);
    // Too many copies are refused and change nothing.
    EditorDesign::grid_array(&mut h, 200, 200, 1.0, 1.0);
    assert_eq!(h.scene.shapes.len(), 4);
    assert!(h.status.contains("2000"), "{}", h.status);
}

#[test]
fn copies_go_along_a_path_text_bends_and_lines_are_trimmed() {
    use base_core::geom::{Polyline, Pt};
    let mut h = Host::default();
    let sq = h.scene.add(0, Kind::Rect { w: 4.0, h: 4.0 }, Xf::translate(-2.0, -2.0));
    let path = h.scene.add(0, Kind::Path(vec![Polyline::new(vec![Pt::new(0.0, 0.0), Pt::new(40.0, 0.0)], false)]), Xf::IDENTITY);
    h.ed.sel = vec![sq, path];
    EditorDesign::copy_along_path(&mut h, 10.0, false);
    // Five copies (0, 10, 20, 30, 40) and the path; the original square became the first copy.
    assert_eq!(h.scene.shapes.len(), 6, "{}", h.status);
    // Trim: a line through a square keeps the part inside.
    let mut h = Host::default();
    let line = h.scene.add(0, Kind::Path(vec![Polyline::new(vec![Pt::new(-10.0, 5.0), Pt::new(30.0, 5.0)], false)]), Xf::IDENTITY);
    let region = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::IDENTITY);
    h.ed.sel = vec![line, region];
    EditorDesign::trim_by_shape(&mut h, true);
    let len: f64 = h.scene.shapes.iter().filter(|s| matches!(s.kind, Kind::Path(_))).flat_map(|s| s.polys()).map(|p| p.length()).sum();
    assert!((len - 10.0).abs() < 1e-6, "{len} {}", h.status);
    // Nothing selected for a path: said so.
    h.ed.sel.clear();
    EditorDesign::copy_along_path(&mut h, 10.0, false);
    assert!(h.status.contains("path"), "{}", h.status);
}

#[test]
fn duplicates_go_shapes_break_apart_and_simplify_and_a_nest_fits_the_area() {
    use base_core::geom::{Polyline, Pt};
    let mut h = Host::default();
    let a = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(5.0, 5.0));
    let b = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(5.0, 5.0));
    let c = h.scene.add(1, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(5.0, 5.0));
    h.ed.sel = vec![a, b, c];
    EditorDesign::remove_duplicates(&mut h);
    assert_eq!(h.scene.shapes.len(), 2, "{}: the same shape on another layer stays", h.status);
    EditorDesign::remove_duplicates(&mut h);
    assert!(h.status.contains("no duplicates"), "{}", h.status);
    // A shape of two outlines breaks into two shapes.
    let two = Kind::Path(vec![Polyline::new(vec![Pt::new(0.0, 0.0), Pt::new(5.0, 0.0), Pt::new(5.0, 5.0)], true), Polyline::new(vec![Pt::new(20.0, 0.0), Pt::new(25.0, 0.0), Pt::new(25.0, 5.0)], true)]);
    let id = h.scene.add(0, two, Xf::IDENTITY);
    h.ed.sel = vec![id];
    EditorDesign::break_apart(&mut h);
    assert_eq!(h.ed.sel.len(), 2);
    // Simplify a line with points on it.
    let line = Kind::Path(vec![Polyline::new((0..=20).map(|i| Pt::new(i as f64, 0.0)).collect(), false)]);
    let id = h.scene.add(0, line, Xf::IDENTITY);
    h.ed.sel = vec![id];
    EditorDesign::simplify_selected(&mut h, 0.01);
    assert!(h.status.contains("21 points became 2"), "{}", h.status);
    // Nest three squares on a small area: they fit in a row and move there.
    let mut h = Host::default();
    let ids: Vec<u64> = (0..3).map(|i| h.scene.add(0, Kind::Rect { w: 20.0, h: 20.0 }, Xf::translate(100.0 * i as f64, 50.0))).collect();
    h.ed.sel = ids.clone();
    EditorDesign::nest_selected(&mut h, (70.0, 30.0), 2.0, false);
    let b = h.scene.bounds_of(&ids).unwrap();
    assert!(b.min.x.abs() < 1e-9 && b.min.y.abs() < 1e-9 && b.max.x <= 70.0 + 1e-9 && b.max.y <= 30.0 + 1e-9, "{b:?} {}", h.status);
    EditorDesign::nest_selected(&mut h, (30.0, 30.0), 2.0, false);
    assert!(h.status.contains("do not fit"), "{}", h.status);
}

#[test]
fn serial_text_counts_up_and_a_code_is_put_on_the_page() {
    let mut h = Host::default();
    let t = h.scene.add(0, Kind::Text(base_core::text::TextData { text: "SN-{n}".into(), ..Default::default() }), Xf::translate(10.0, 10.0));
    h.ed.sel = vec![t];
    let texts = base_core::design::serial_texts("SN-{n}", 1, 1, 4, 3);
    EditorDesign::serial_text(&mut h, "SN-{n}", &texts, 2, 30.0, 15.0);
    assert_eq!(h.scene.shapes.len(), 4, "{}", h.status);
    let got: Vec<String> = h.scene.shapes.iter().filter_map(|s| if let Kind::Text(d) = &s.kind { Some(d.text.clone()) } else { None }).collect();
    assert_eq!(got, vec!["SN-001", "SN-002", "SN-003", "SN-004"]);
    // QR and Code 39 become path shapes; a bad Code 39 text is said.
    let mut h = Host::default();
    EditorDesign::make_code(&mut h, CodeKind::Qr, "hello", 30.0, 0.0);
    assert!(matches!(&h.scene.shapes[0].kind, Kind::Path(p) if !p.is_empty()), "{}", h.status);
    EditorDesign::make_code(&mut h, CodeKind::Code39, "ab-12", 0.5, 10.0);
    assert_eq!(h.scene.shapes.len(), 2);
    EditorDesign::make_code(&mut h, CodeKind::Code39, "a_b", 0.5, 10.0);
    assert_eq!(h.scene.shapes.len(), 2);
    assert!(h.status.contains("Code 39"), "{}", h.status);
}

#[test]
fn the_measure_window_reports_the_box_area_and_the_gap_of_two_shapes() {
    let mut h = Host::default();
    assert!(EditorDesign::measure_lines(&h)[0].contains("Select"));
    let a = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(0.0, 0.0));
    let b = h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(30.0, 0.0));
    h.ed.sel = vec![a, b];
    let lines = EditorDesign::measure_lines(&h).join("\n");
    assert!(lines.contains("Width 40.000  Height 10.000"), "{lines}");
    assert!(lines.contains("Area 200.000"), "{lines}");
    assert!(lines.contains("Gap between the boxes: 20.000"), "{lines}");
    assert!(lines.contains("Between the centres: 30.000"), "{lines}");
}

#[test]
fn a_background_picture_goes_under_the_drawing_with_its_corners_on_the_work_area_and_can_be_fitted_and_removed() {
    use base_core::geom::Pt;
    let ctx = egui::Context::default();
    let mut h = Host::default();
    let img = || egui::ColorImage::from_rgba_unmultiplied([4, 2], &[255u8; 32]);
    h.set_background(&ctx, img(), "plan.png");
    let (w, hh) = h.work_size();
    let b = h.ed.background.as_ref().unwrap();
    assert_eq!((b.name.as_str(), b.size, b.visible, b.edit), ("plan.png", (4, 2), true, false));
    assert_eq!(b.corners, [Pt::new(0.0, 0.0), Pt::new(w, 0.0), Pt::new(w, hh), Pt::new(0.0, hh)]);
    // Moved corners are kept for a picture of the same size (a new frame of a camera), and fitted again on request.
    h.ed.background.as_mut().unwrap().corners[2] = Pt::new(5.0, 6.0);
    h.set_background(&ctx, img(), "plan2.png");
    assert_eq!(h.ed.background.as_ref().unwrap().corners[2], Pt::new(5.0, 6.0));
    h.fit_background();
    assert_eq!(h.ed.background.as_ref().unwrap().corners[2], Pt::new(w, hh));
    // It is drawn (with the handles when they can be dragged) and the window opens and closes.
    h.ed.background.as_mut().unwrap().edit = true;
    h.ed.show_background = true;
    for _ in 0..2 {
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| h.canvas(ui));
            h.background_window(ui.ctx());
        });
        out.textures_delta.clear();
    }
    assert!(h.ed.show_background);
}

#[test]
fn the_perspective_map_puts_the_middle_of_the_picture_in_the_middle_of_a_skewed_quad() {
    use base_core::geom::Pt;
    let square = [Pt::new(0.0, 0.0), Pt::new(10.0, 0.0), Pt::new(10.0, 10.0), Pt::new(0.0, 10.0)];
    let m = super::background_ui::quad_map(&square, 0.5, 0.25);
    assert!((m.x - 5.0).abs() < 1e-9 && (m.y - 2.5).abs() < 1e-9, "{m:?}");
    // Corners go to the corners of any quad.
    let quad = [Pt::new(1.0, 1.0), Pt::new(9.0, 2.0), Pt::new(8.0, 9.0), Pt::new(0.0, 7.0)];
    for ((u, v), c) in [((0.0, 0.0), quad[0]), ((1.0, 0.0), quad[1]), ((1.0, 1.0), quad[2]), ((0.0, 1.0), quad[3])] {
        let p = super::background_ui::quad_map(&quad, u, v);
        assert!((p.x - c.x).abs() < 1e-9 && (p.y - c.y).abs() < 1e-9);
    }
}
