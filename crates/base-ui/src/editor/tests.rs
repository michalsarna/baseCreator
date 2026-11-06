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
    calibration: Option<([base_core::geom::Pt; 4], f32)>,
    cam: Option<base_core::camimg::CameraSource>,
    changed: usize,
    frames: u64,
}

impl Default for Host {
    fn default() -> Host {
        // Never touch the real user library in tests.
        let dir = std::env::temp_dir().join(format!("base-ui-clip-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        Host { scene: Scene::default(), ed: EditorState::default(), status: String::new(), undo: vec![], clip: ClipState::with_builtin(base_core::clipart::Library::open(dir), BUILTIN), calibration: None, cam: None, changed: 0, frames: 0 }
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
    fn overlay_calibration(&self) -> Option<([base_core::geom::Pt; 4], f32)> {
        self.calibration
    }
    fn overlay_changed(&mut self) {
        self.changed += 1;
    }
    fn overlay_camera(&mut self) -> Option<&mut base_core::camimg::CameraSource> {
        self.cam.as_mut()
    }
    fn live_camera(&mut self, _ctx: &egui::Context) -> Option<u64> {
        self.frames += 1;
        Some(self.frames)
    }
    fn background_camera_image(&self) -> Option<(egui::ColorImage, String)> {
        self.cam.as_ref()?;
        Some((egui::ColorImage::from_rgba_unmultiplied([4, 2], &[255u8; 32]), "cam".to_string()))
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

/// The negative of a picture is in Properties of every program: it can be undone and a locked picture stays as it is.
#[test]
fn the_negative_of_the_selected_picture_is_in_the_properties_and_can_be_undone() {
    let mut h = Host::default();
    let im = base_core::image::ImageData { name: "p".into(), px_w: 2, px_h: 2, gray: vec![0; 4], w: 10.0, h: 10.0, invert: false };
    let id = h.scene.add(0, Kind::Image(im), Xf::IDENTITY);
    h.ed.sel = vec![id];
    let negative = |s: &Scene| matches!(s.shape(id).map(|s| &s.kind), Some(Kind::Image(m)) if m.invert);
    super::props::set_negative(&mut h, id, true);
    assert!(negative(&h.scene));
    assert_eq!(h.undo.len(), 1, "one step of undo");
    assert!(!negative(&h.undo[0]), "the step keeps the picture as it was");
    h.scene.shape_mut(id).unwrap().locked = true;
    h.undo.clear();
    super::props::set_negative(&mut h, id, false);
    assert!(negative(&h.scene), "a locked picture does not change");
    assert!(h.undo.is_empty());
    // The section draws the option for a picture.
    let ctx = egui::Context::default();
    let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
        assert!(properties_sections(&mut h, ui));
    });
    out.textures_delta.clear();
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

/// A program that keeps the corners with the device: LightCreator. Without kept corners it is the case of the other programs.
#[test]
fn the_corners_kept_for_the_device_are_used_for_a_new_picture_and_win_over_the_reference_points() {
    use base_core::camimg::RefPoint;
    use base_core::geom::Pt;
    let ctx = egui::Context::default();
    let img = || egui::ColorImage::from_rgba_unmultiplied([4, 2], &[255u8; 32]);
    let kept = [Pt::new(1.0, 2.0), Pt::new(30.0, 2.0), Pt::new(30.0, 20.0), Pt::new(1.0, 20.0)];
    let mut h = Host::default();
    h.calibration = Some((kept, 0.25));
    h.set_background(&ctx, img(), "photo");
    let b = h.ed.background.as_ref().unwrap();
    assert_eq!((b.corners, b.opacity), (kept, 0.25), "the corners and the opacity of the device");
    // The live camera too, although the points would put the picture elsewhere.
    h.ed.background = None;
    let mut cam = base_core::camimg::CameraSource::url("cam", "http://x", 0);
    cam.reference = (0..4).map(|i| RefPoint { px: ([0.0, 4.0, 4.0, 0.0][i], [0.0, 0.0, 2.0, 2.0][i]), mm: ([5.0, 9.0, 9.0, 5.0][i], [5.0, 5.0, 9.0, 9.0][i]) }).collect();
    h.cam = Some(cam);
    h.ed.background_live = true;
    h.background_tick(&ctx);
    assert_eq!(h.ed.background.as_ref().unwrap().corners, kept);
    // Without kept corners the points do it.
    h.calibration = None;
    h.ed.background = None;
    h.background_tick(&ctx);
    assert!((h.ed.background.as_ref().unwrap().corners[2].x - 9.0).abs() < 1e-6);
    assert_eq!(h.changed, 0, "a frame of the camera is not a change by hand");
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

#[test]
fn the_insert_menu_chooses_the_same_tools_as_the_palette() {
    use crate::menu::{std_entries, Entry, StdAct};
    let tools: Vec<Tool> = std_entries::insert_tools().into_iter().filter_map(|e| if let Entry::Item(StdAct::Tool(t), _, _) = e { Some(t) } else { None }).collect();
    assert_eq!(tools, [Tool::Rect, Tool::Ellipse, Tool::Triangle, Tool::Star, Tool::Polygon, Tool::Heart, Tool::Spiral, Tool::Line, Tool::Pen, Tool::Text]);
    let mut h = Host::default();
    h.ed.pen_pts.push(base_core::geom::Pt { x: 1.0, y: 1.0 });
    StdAct::Tool(Tool::Polygon).run(&mut h);
    assert_eq!(h.ed.tool, Tool::Polygon);
    assert!(h.ed.show_shape_popup && h.ed.pen_pts.is_empty(), "like a click on the palette: the polygon form opens and a half-drawn line is dropped");
    assert_eq!(StdAct::Tool(Tool::Polygon).checked(&h), Some(true));
    assert_eq!(StdAct::Tool(Tool::Star).checked(&h), Some(false));
}

impl crate::layer_bar::LayerBarHost for Host {
    fn layer_label(&self, layer: usize) -> String {
        if layer == 0 { "A very long name of the first layer".to_string() } else { format!("Layer {layer}") }
    }
    fn open_layer_options(&mut self, _layer: usize) {}
}

#[test]
fn the_names_of_the_layers_with_shapes_can_be_shown_beside_the_colours() {
    let mut h = Host::default();
    h.scene.add(0, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(0.0, 0.0));
    h.scene.add(3, Kind::Rect { w: 10.0, h: 10.0 }, Xf::translate(20.0, 0.0));
    let ctx = egui::Context::default();
    let mut input = egui::RawInput::default();
    input.screen_rect = Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 60.0)));
    let texts = |h: &mut Host| {
        let mut out = ctx.run_ui(input.clone(), |ui| crate::layer_bar::layer_bar(h, ui));
        out.textures_delta.clear();
        out.shapes.iter().filter_map(|s| if let egui::epaint::Shape::Text(t) = &s.shape { Some(t.galley.text().to_string()) } else { None }).collect::<Vec<_>>()
    };
    assert!(!h.ed.grid_prefs.layer_names, "off by default");
    let plain = texts(&mut h);
    assert!(!plain.iter().any(|t| t.starts_with("Layer ") || t.starts_with("A very")), "only the colours: {plain:?}");
    h.ed.grid_prefs.layer_names = true;
    texts(&mut h);
    let shown = texts(&mut h);
    assert!(shown.iter().any(|t| t == "Layer 3"), "{shown:?}");
    assert!(shown.iter().any(|t| t.ends_with('…') && t.starts_with("A very long")), "a long name is cut: {shown:?}");
    assert!(!shown.iter().any(|t| t == "Layer 1"), "a layer without shapes has no name: {shown:?}");
}


/// Escape does the editor's work and stays in the input for a question or a pick that is open too; the other keys are used up.
#[test]
fn escape_stays_for_what_else_is_open() {
    let ctx = egui::Context::default();
    let (mut h, a, _) = host_with_two_rects();
    h.ed.sel = vec![a];
    let key = |k: egui::Key| egui::RawInput {
        events: vec![egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }],
        ..Default::default()
    };
    let mut seen = false;
    let mut out = ctx.run_ui(key(egui::Key::Escape), |ui| {
        super::plain_keys(&mut h, ui.ctx(), true);
        seen = ui.input(|i| i.key_pressed(egui::Key::Escape));
    });
    out.textures_delta.clear();
    assert!(h.ed.sel.is_empty(), "the selection is cleared");
    assert!(seen, "and whatever comes later in the frame still sees the key");
    let mut out = ctx.run_ui(key(egui::Key::R), |ui| {
        super::plain_keys(&mut h, ui.ctx(), true);
        seen = ui.input(|i| i.key_pressed(egui::Key::R));
    });
    out.textures_delta.clear();
    assert_eq!(h.ed.tool, Tool::Rect);
    assert!(!seen, "a letter of a tool is used up");
}

/// The texts of the polygon window are found by the shared table alone, in the languages of the table and of the files.
#[test]
fn the_new_rows_of_the_table_are_found() {
    use crate::i18n::{tr_in, Lang};
    for text in ["Swap the light and the dark parts of the picture", "Load preset", "Number of sides", "Number of turns", "Polygon", "Spiral", "Arrange", "Save G-code", "Drag on the work area to draw it."] {
        for lang in [Lang::Pl, Lang::Es] {
            assert_ne!(tr_in(lang, text), text, "{text} in {lang:?}");
        }
    }
}

/// The width of a window over a few frames: it must not change on its own.
fn window_widths(ctx: &egui::Context, frames: usize, mut draw: impl FnMut(&egui::Context)) -> Vec<f32> {
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1800.0, 1100.0));
    (0..frames)
        .map(|_| {
            let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| draw(&ui.ctx().clone()));
            out.textures_delta.clear();
            // The window opens in the middle of the screen.
            ctx.layer_id_at(screen.center()).and_then(|l| ctx.memory(|m| m.area_rect(l.id))).map_or(0.0, |r| r.width())
        })
        .collect()
}

/// The image dialog in Polish, whose "Rotate and flip" row is wider than its column: the window once grew by that much every frame
/// up to the width of the screen. The language is the same for the whole program, so this runs in a process of its own (below).
#[test]
#[ignore = "run by the_image_dialog_keeps_its_width_in_a_longer_language in a process of its own"]
fn image_dialog_width_in_polish() {
    if std::env::var_os("BASE_UI_ALONE").is_none() {
        return;
    }
    crate::i18n::set_lang(crate::i18n::Lang::Pl);
    let mut h = Host::default();
    let im = base_core::image::ImageData { name: "1793909638446.jpeg".into(), px_w: 96, px_h: 102, gray: vec![128; 96 * 102], w: 188.0, h: 200.0, invert: false };
    h.ed.img_dlg = Some(ImgDlg::new(None, im));
    let ctx = egui::Context::default();
    let w = window_widths(&ctx, 30, |ctx| h.image_dialogs(ctx));
    assert!(w[3] > 500.0 && w[3] < 1200.0 && w.windows(2).skip(3).all(|p| p[0] == p[1]), "{w:?}");
}

#[test]
fn the_image_dialog_keeps_its_width_in_a_longer_language() {
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "editor::tests::image_dialog_width_in_polish", "--ignored", "--nocapture"])
        .env("BASE_UI_ALONE", "1")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string() + String::from_utf8_lossy(&out.stderr).as_ref();
    assert!(out.status.success() && text.contains("1 passed"), "{text}");
}

#[test]
fn the_image_dialog_keeps_its_width() {
    let mut h = Host::default();
    let im = base_core::image::ImageData { name: "a picture with a rather long file name of many words.jpeg".into(), px_w: 40, px_h: 30, gray: vec![128; 1200], w: 40.0, h: 30.0, invert: false };
    h.ed.img_dlg = Some(ImgDlg::new(None, im));
    let ctx = egui::Context::default();
    let w = window_widths(&ctx, 30, |ctx| h.image_dialogs(ctx));
    assert!(w[3] > 500.0 && w[3] < 1200.0 && w.windows(2).skip(3).all(|p| p[0] == p[1]), "{w:?}");
}


/// The list of fonts of the text tool: open it with a click, then the arrows go down and up through it and change the font of
/// the text at once; the list shows ten fonts.
#[test]
fn the_arrows_change_the_font_in_the_open_list_and_it_shows_ten() {
    let fams = base_core::text::families();
    if fams.len() < 12 {
        return; // a machine with almost no fonts
    }
    let mut h = Host::default();
    let mut t = base_core::text::TextData::default();
    t.family = fams[0].clone();
    let id = h.scene.add(0, Kind::Text(t), Xf::translate(0.0, 0.0));
    h.ed.sel = vec![id];
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 900.0));
    let mut frame = |h: &mut Host, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
            egui::CentralPanel::default().show_inside(ui, |ui| super::text_ui::section(h, ui));
        });
        out.textures_delta.clear();
    };
    let family = |h: &Host| match &h.scene.shape(id).unwrap().kind {
        Kind::Text(t) => t.family.clone(),
        _ => unreachable!(),
    };
    frame(&mut h, vec![]);
    frame(&mut h, vec![]);
    // The button of the list: the first clickable widget about 190 points wide (with its arrow) under the text field.
    let button = ctx.viewport(|v| v.prev_pass.widgets.layers().flat_map(|(_, w)| w.iter().filter(|w| w.sense.senses_click()).map(|w| w.rect)).find(|r| r.width() > 185.0 && r.width() < 240.0)).expect("the font button");
    let at = button.center();
    frame(&mut h, vec![egui::Event::PointerMoved(at)]);
    frame(&mut h, vec![egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }, egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }]);
    frame(&mut h, vec![]);
    frame(&mut h, vec![egui::Event::PointerMoved(egui::pos2(490.0, 890.0))]);
    let key = |k: egui::Key| vec![egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }];
    frame(&mut h, key(egui::Key::ArrowDown));
    assert_eq!(family(&h), fams[1], "down: the next font");
    frame(&mut h, key(egui::Key::ArrowDown));
    frame(&mut h, key(egui::Key::ArrowUp));
    assert_eq!(family(&h), fams[1], "up: back one");
    // Ten names of the list are inside the popup at once, not more.
    let names: Vec<egui::Rect> = ctx.viewport(|v| {
        v.prev_pass.widgets.layers().filter(|(l, _)| l.order != egui::Order::Background).flat_map(|(_, w)| w.iter().map(|w| w.rect)).collect()
    });
    let row = names.iter().filter(|r| r.height() > 10.0 && r.height() < 30.0 && r.width() > 100.0).map(|r| r.height()).fold(0.0, f32::max);
    let layer = ctx.viewport(|v| v.prev_pass.widgets.layers().map(|(l, _)| *l).find(|l| l.order != egui::Order::Background)).expect("the popup layer");
    let popup = ctx.memory(|m| m.area_rect(layer.id)).expect("the popup's place");
    assert!(row > 0.0, "the list is open");
    let seen = popup.height() / row;
    assert!((10.5..13.5).contains(&seen), "about ten names and the search field: {seen} rows of {row}");
}
