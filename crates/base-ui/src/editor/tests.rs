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
    /// The picture in the clipboard of the system, as the tests set it (never the real one).
    clip_img: Option<crate::clip_image::ClipImage>,
    /// The sheets a nesting opened, when the test lets it open any.
    sheets: Option<Vec<Vec<base_core::scene::Shape>>>,
}

impl Default for Host {
    fn default() -> Host {
        // Never touch the real user library in tests.
        let dir = std::env::temp_dir().join(format!("base-ui-clip-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        Host { scene: Scene::default(), ed: EditorState::default(), status: String::new(), undo: vec![], clip: ClipState::with_builtin(base_core::clipart::Library::open(dir), BUILTIN), calibration: None, cam: None, changed: 0, frames: 0, clip_img: None, sheets: None }
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
    fn can_add_sheets(&self) -> bool {
        self.sheets.is_some()
    }
    fn add_sheets(&mut self, sheets: Vec<Vec<base_core::scene::Shape>>) -> usize {
        let n = sheets.len();
        self.sheets.get_or_insert_default().extend(sheets);
        n
    }
    fn said(&self) -> &str {
        &self.status
    }
    fn work_size(&self) -> (f64, f64) {
        (200.0, 100.0)
    }
    fn clipboard_image(&self) -> Option<crate::clip_image::ClipImage> {
        self.clip_img.clone()
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
fn a_nest_by_the_real_outlines_keeps_groups_together_and_leaves_what_does_not_fit() {
    let mut h = Host::default();
    let ids: Vec<u64> = (0..4).map(|i| h.scene.add(0, Kind::Ellipse { w: 20.0, h: 20.0 }, Xf::translate(200.0 + 50.0 * i as f64, 150.0))).collect();
    // The last two are one part.
    let g = Some(99);
    for id in &ids[2..] {
        h.scene.shape_mut(*id).unwrap().group = g;
    }
    h.ed.sel = ids.clone();
    let gap = |h: &Host| h.scene.shape(ids[3]).unwrap().xf.e - h.scene.shape(ids[2]).unwrap().xf.e;
    let before = gap(&h);
    EditorDesign::nest_true_shape(&mut h, (100.0, 100.0), 2.0, 4, true);
    let b = h.scene.bounds_of(&ids).unwrap();
    assert!(b.min.x >= -1e-6 && b.min.y >= -1e-6 && b.max.x <= 100.0 + 1e-6 && b.max.y <= 100.0 + 1e-6, "{b:?} {}", h.status);
    assert!(h.status.contains("3 shapes nested"), "{}", h.status);
    // The group moved as one (a quarter turn may swap the axis of the distance).
    let (a, c) = (h.scene.shape(ids[2]).unwrap().bounds().unwrap().center(), h.scene.shape(ids[3]).unwrap().bounds().unwrap().center());
    assert!((a.dist(c) - before).abs() < 1e-6);
    // A tiny area: some stay where they are.
    let mut h2 = Host::default();
    let ids: Vec<u64> = (0..3).map(|i| h2.scene.add(0, Kind::Rect { w: 20.0, h: 20.0 }, Xf::translate(100.0 * i as f64, 50.0))).collect();
    h2.ed.sel = ids;
    EditorDesign::nest_true_shape(&mut h2, (50.0, 25.0), 2.0, 1, true);
    assert!(h2.status.contains("did not fit"), "{}", h2.status);
    assert_eq!(h2.scene.shapes.len(), 3, "without sheets nothing is taken away");
    // With sheets: what does not fit goes on new ones, nothing is lost and every sheet holds what fits.
    let mut h3 = Host { sheets: Some(vec![]), ..Host::default() };
    let ids: Vec<u64> = (0..5).map(|i| h3.scene.add(0, Kind::Rect { w: 20.0, h: 20.0 }, Xf::translate(100.0 * i as f64, 50.0))).collect();
    h3.ed.sel = ids;
    EditorDesign::nest_true_shape(&mut h3, (50.0, 25.0), 2.0, 1, true);
    let sheets = h3.sheets.clone().unwrap();
    let on_sheets: usize = sheets.iter().map(|s| s.len()).sum();
    assert!(!sheets.is_empty() && on_sheets + h3.scene.shapes.len() == 5, "{} {sheets:?}", h3.status);
    assert!(h3.status.contains("new sheet"), "{}", h3.status);
    for s in sheets.iter().flatten() {
        let b = s.bounds().unwrap();
        assert!(b.min.x >= -1e-6 && b.max.x <= 50.0 + 1e-6 && b.max.y <= 25.0 + 1e-6, "{b:?}");
    }
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
            egui::CentralPanel::default().show(ui, |ui| super::text_ui::section(h, ui));
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

/// The picture of the import window zooms and moves as in the trace window: the zoom button, a drag, a double click.
#[test]
fn the_picture_of_the_import_window_zooms_and_moves() {
    let mut h = Host::default();
    let im = base_core::image::ImageData { name: "p.png".into(), px_w: 80, px_h: 60, gray: vec![128; 4800], w: 80.0, h: 60.0, invert: false };
    h.ed.img_dlg = Some(ImgDlg::new(None, im));
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0));
    let mut t = 0.0;
    let mut frame = |h: &mut Host, events: Vec<egui::Event>| {
        t += 0.05;
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), time: Some(t), events, ..Default::default() }, |ui| h.image_dialogs(&ui.ctx().clone()));
        out.textures_delta.clear();
    };
    let press = |at: egui::Pos2, down: bool| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
    for _ in 0..3 {
        frame(&mut h, vec![]);
    }
    // The zoom-in button: the first square clickable widget of about 26 points in the window.
    let button = ctx.viewport(|v| v.prev_pass.widgets.layers().filter(|(l, _)| l.order != egui::Order::Background).flat_map(|(_, w)| w.iter().filter(|w| w.sense.senses_click()).map(|w| w.rect)).find(|r| (r.width() - r.height()).abs() < 4.0 && r.width() > 20.0 && r.width() < 34.0)).expect("the zoom-in button");
    frame(&mut h, vec![egui::Event::PointerMoved(button.center())]);
    frame(&mut h, vec![press(button.center(), true), press(button.center(), false)]);
    frame(&mut h, vec![]);
    assert!((h.ed.img_dlg.as_ref().unwrap().view.zoom - 1.5).abs() < 1e-6, "zoomed in by the button");
    // A drag in the picture (above the buttons) moves it.
    let a = button.center() + egui::vec2(150.0, -200.0);
    frame(&mut h, vec![egui::Event::PointerMoved(a)]);
    frame(&mut h, vec![press(a, true)]);
    for i in 1..=5 {
        frame(&mut h, vec![egui::Event::PointerMoved(a + egui::vec2(8.0 * i as f32, 4.0 * i as f32))]);
    }
    frame(&mut h, vec![press(a + egui::vec2(40.0, 20.0), false)]);
    let pan = h.ed.img_dlg.as_ref().unwrap().view.pan;
    assert!(pan.x > 10.0 && pan.y > 5.0, "moved by the drag: {pan:?}");
    // A double click shows the whole picture again.
    let b = a + egui::vec2(40.0, 20.0);
    frame(&mut h, vec![press(b, true), press(b, false)]);
    frame(&mut h, vec![press(b, true), press(b, false)]);
    frame(&mut h, vec![]);
    assert_eq!(h.ed.img_dlg.as_ref().unwrap().view, super::image_ui::PictureView::default());
}

fn clip(w: u32, h: u32, v: u8) -> crate::clip_image::ClipImage {
    crate::clip_image::ClipImage { w, h, rgba: [v, v, v, 255].repeat((w * h) as usize) }
}

/// A picture in the clipboard of the system opens the import window on Paste; shapes copied in the program after it are pasted
/// as shapes, and a picture that came later wins again.
#[test]
fn paste_opens_the_import_window_for_a_picture_and_keeps_the_copied_shapes() {
    let (mut h, a, _) = host_with_two_rects();
    h.clip_img = Some(clip(40, 20, 0));
    h.paste();
    let d = h.ed.img_dlg.take().expect("the import window");
    assert_eq!((d.base.px_w, d.base.px_h), (40, 20));
    assert!((d.base.w - 50.0).abs() < 1e-9, "the longer side is half the shorter side of the work area");
    // Shapes copied now are pasted as shapes while the same picture stays in the clipboard.
    h.ed.sel = vec![a];
    h.copy();
    let n = h.scene.shapes.len();
    h.paste();
    assert!(h.ed.img_dlg.is_none() && h.scene.shapes.len() == n + 1, "the shapes");
    // Another picture comes into the clipboard: it is pasted.
    h.clip_img = Some(clip(10, 30, 200));
    h.paste();
    assert_eq!(h.ed.img_dlg.as_ref().map(|d| (d.base.px_w, d.base.px_h)), Some((10, 30)));
    // Nothing in the clipboard of the system: the shapes as always.
    h.ed.img_dlg = None;
    h.clip_img = None;
    h.paste();
    assert!(h.ed.img_dlg.is_none() && h.scene.shapes.len() == n + 2);
}

/// Ctrl/Cmd+V over the canvas arrives as a paste event: a picture opens the import window, text pastes the copied shapes.
#[test]
fn a_paste_event_over_the_canvas_pastes_a_picture_or_the_shapes() {
    let (mut h, a, _) = host_with_two_rects();
    let ctx = egui::Context::default();
    let mut frame = |h: &mut Host, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| {
            egui::CentralPanel::no_frame().show(ui, |ui| h.canvas(ui));
        });
        out.textures_delta.clear();
    };
    frame(&mut h, vec![]);
    let img = egui::ColorImage::new([6, 4], vec![egui::Color32::BLACK; 24]);
    frame(&mut h, vec![egui::Event::PasteImage(std::sync::Arc::new(img))]);
    assert_eq!(h.ed.img_dlg.as_ref().map(|d| (d.base.px_w, d.base.px_h, d.base.gray[0])), Some((6, 4, 0)));
    h.ed.img_dlg = None;
    h.ed.sel = vec![a];
    h.copy();
    let n = h.scene.shapes.len();
    frame(&mut h, vec![egui::Event::Paste("some text".into())]);
    assert_eq!(h.scene.shapes.len(), n + 1, "the copied shapes");
}

fn open_path(h: &mut Host, pts: &[(f64, f64)]) -> u64 {
    let c = base_core::bezier::Contour { nodes: pts.iter().map(|(x, y)| base_core::bezier::Node::corner(base_core::geom::Pt::new(*x, *y))).collect(), closed: false };
    h.scene.add(0, Kind::Bezier(vec![c]), Xf::IDENTITY)
}

fn contours(h: &Host, id: u64) -> Vec<base_core::bezier::Contour> {
    match h.scene.shape(id).map(|s| &s.kind) {
        Some(Kind::Bezier(cs)) => cs.clone(),
        _ => vec![],
    }
}

/// The letters of the node tool act on the selected nodes, and do not switch to the tools that have the same letters.
#[test]
fn the_keys_of_the_node_tool_edit_the_nodes_and_keep_the_tool() {
    let mut h = Host::default();
    let id = open_path(&mut h, &[(0.0, 0.0), (10.0, 0.0), (20.0, 0.0), (30.0, 0.0)]);
    h.ed.sel = vec![id];
    h.ed.tool = Tool::Node;
    let ctx = egui::Context::default();
    let press = |h: &mut Host, k: egui::Key| {
        let ev = vec![egui::Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: Default::default() }];
        let mut out = ctx.run_ui(egui::RawInput { events: ev, ..Default::default() }, |ui| super::keys::plain_keys(h, ui.ctx(), true));
        out.textures_delta.clear();
    };
    h.ed.node_sel = vec![(id, 0, 0)];
    press(&mut h, egui::Key::S);
    assert!(contours(&h, id)[0].seg_is_curve(0), "S: a curve");
    h.ed.node_sel = vec![(id, 0, 0)];
    press(&mut h, egui::Key::L);
    assert!(!contours(&h, id)[0].seg_is_curve(0), "L: a line");
    h.ed.node_sel = vec![(id, 0, 0)];
    press(&mut h, egui::Key::I);
    assert_eq!(contours(&h, id)[0].nodes.len(), 5, "I: a node inserted");
    h.ed.node_sel = vec![(id, 0, 1)];
    press(&mut h, egui::Key::D);
    assert_eq!(contours(&h, id)[0].nodes.len(), 4, "D: deleted");
    h.ed.node_sel = vec![(id, 0, 1)];
    press(&mut h, egui::Key::C);
    press(&mut h, egui::Key::R);
    h.ed.node_sel = vec![(id, 0, 2)];
    press(&mut h, egui::Key::B);
    assert_eq!(contours(&h, id).len(), 2, "B: cut in two");
    assert_eq!(h.ed.tool, Tool::Node, "none of the letters switched the tool");
    h.ed.tool = Tool::Select;
    press(&mut h, egui::Key::R);
    assert_eq!(h.ed.tool, Tool::Rect, "outside the node tool R is still the rectangle");
}

/// A node dropped onto another one: the end of a path on its other end closes it, an end of another path joins the two.
#[test]
fn a_node_dropped_on_another_joins_them() {
    let mut h = Host::default();
    let a = open_path(&mut h, &[(0.0, 0.0), (10.0, 0.0), (10.0, 10.0), (0.2, 0.1)]);
    let b = open_path(&mut h, &[(50.0, 0.0), (60.0, 0.0)]);
    let c = open_path(&mut h, &[(60.1, 0.1), (70.0, 10.0)]);
    h.ed.sel = vec![a, b, c];
    assert!(h.join_dropped_node((a, 0, 3), 0.5));
    assert!(contours(&h, a)[0].closed && contours(&h, a)[0].nodes.len() == 3);
    assert!(h.join_dropped_node((b, 0, 1), 0.5));
    assert_eq!(contours(&h, b)[0].nodes.len(), 3, "the two paths are one");
    assert!(h.scene.shape(c).is_none() && !h.ed.sel.contains(&c), "the emptied shape is gone");
    assert!(!h.join_dropped_node((b, 0, 0), 0.5), "nothing near");
}

#[test]
fn an_svg_from_lightburn_brings_its_picture_and_every_export_format_is_written() {
    use crate::editor::{EditorImages, Exchange};
    let fixture = concat!(env!("CARGO_MANIFEST_DIR"), "/../base-core/tests/fixtures/interchange/zdjecie.svg");
    let mut h = Host::default();
    h.import_drawing_path(std::path::PathBuf::from(fixture));
    assert_eq!(h.scene.shapes.len(), 1, "{}", h.said());
    assert!(h.scene.shapes[0].is_image());
    assert_eq!(h.ed.sel.len(), 1);
    h.scene.add(0, Kind::Rect { w: 10.0, h: 20.0 }, Xf::translate(5.0, 5.0));
    let names = |l: usize| format!("L{l}");
    let colors = |_: usize| [0, 0, 0];
    let cut = |_: usize| None;
    let info = base_core::lightburn::LayerInfo { name: &names, color: &colors, cut: &cut };
    for f in Exchange::ALL {
        let ext = if f == Exchange::LightBurn { "lbrn" } else { "x" };
        let text = f.write(&h.scene, &[], (400.0, 400.0), std::path::Path::new(&format!("a.{ext}")), &info);
        let ok = match f {
            Exchange::Dxf => text.contains("AC1021"),
            Exchange::DxfLegacy => text.contains("AC1009"),
            Exchange::Illustrator => text.starts_with("%!PS-Adobe") && !text.contains("Bitmap"),
            Exchange::LightBurn => text.contains("<LightBurnProject") && text.contains("FormatVersion=\"0\"") && text.contains("Type=\"Bitmap\""),
            Exchange::MillMage => text.contains("<MillMageProject") && text.contains("LayerIndex") && text.contains("Type=\"Bitmap\""),
        };
        assert!(ok, "{f:?}");
    }
}

#[test]
fn the_ruler_takes_nodes_and_a_clicked_straight_segment_and_says_the_length() {
    use base_core::geom::Pt;
    let mut h = Host::default();
    h.scene.add(0, Kind::Rect { w: 30.0, h: 40.0 }, Xf::translate(10.0, 10.0));
    h.scene.add(0, Kind::Ellipse { w: 20.0, h: 20.0 }, Xf::translate(100.0, 10.0));
    h.ed.view.zoom = 2.0;
    h.ed.view.pan = egui::vec2(0.0, 0.0);
    let o = egui::Pos2::ZERO;
    // Near a corner of the rectangle the point is the corner itself; far from everything it is where the pointer is.
    assert_eq!(h.measure_point(o, h.w2s(o, Pt::new(11.0, 10.5))), Pt::new(10.0, 10.0));
    assert_eq!(h.measure_point(o, h.w2s(o, Pt::new(70.0, 70.0))), Pt::new(70.0, 70.0));
    // A click on the top edge gives that edge; on the round ellipse nothing straight.
    let edge = h.measure_pick(o, h.w2s(o, Pt::new(25.0, 10.5))).expect("the top edge");
    assert!(edge.segment && (edge.a.unwrap().dist(edge.b.unwrap()) - 30.0).abs() < 1e-9);
    let round = h.measure_pick(o, h.w2s(o, Pt::new(110.0, 10.0))).expect("the circle");
    assert!(matches!(round.picked, Some(super::Picked::Ellipse { .. })), "the circle by its radius");
    // The middle of the top edge is a point of the ruler too.
    assert_eq!(h.measure_point(o, h.w2s(o, Pt::new(25.5, 10.5))), Pt::new(25.0, 10.0));
    let text = super::measure_text(Pt::new(0.0, 0.0), Pt::new(30.0, -40.0), base_core::units::Units::Mm, base_core::units::Units::Mm);
    let both = super::measure_text(Pt::new(0.0, 0.0), Pt::new(25.4, 0.0), base_core::units::Units::Mm, base_core::units::Units::Inch);
    assert!(both.starts_with("25.40 mm [1.000 in]"), "the units of the machine beside: {both}");
    assert!(text.starts_with("50") && text.contains("ΔX 30") && text.contains("ΔY -40") && text.contains("α 53.1°"), "{text}");
    // The tool is in the palette with its key, and Escape forgets the measurement.
    assert!(Tool::ALL.iter().any(|(t, l, k)| *t == Tool::Measure && *l == "Ruler (M)" && *k == egui::Key::M));
    h.ed.measure = super::Measure { a: Some(Pt::new(0.0, 0.0)), ..Default::default() };
    assert_eq!(h.ed.measure.line(Some(Pt::new(3.0, 4.0))).map(|(a, b)| a.dist(b)), Some(5.0), "to the pointer while the end is not set");
}

#[test]
fn a_right_click_with_a_tool_goes_back_to_selecting_and_several_selected_objects_have_frames() {
    let (mut h, a, b) = host_with_two_rects();
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let mut frame = |h: &mut Host, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
            egui::CentralPanel::no_frame().show(ui, |ui| h.canvas(ui));
        });
        out.textures_delta.clear();
        out.shapes
    };
    let at = egui::pos2(400.0, 300.0);
    for tool in [Tool::Rect, Tool::Pen, Tool::Node, Tool::Measure] {
        super::select_tool(&mut h.ed, tool);
        h.ed.sel = vec![a];
        frame(&mut h, vec![egui::Event::PointerMoved(at)]);
        let right = |down: bool| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed: down, modifiers: Default::default() };
        frame(&mut h, vec![right(true), right(false)]);
        assert_eq!(h.ed.tool, Tool::Select, "{tool:?}: back to the select tool");
        assert!(h.ed.sel.is_empty(), "{tool:?}: as a double Escape");
    }
    // Two selected objects: each has a dashed frame of its own besides the frame of the selection.
    h.ed.sel = vec![a, b];
    let shapes = frame(&mut h, vec![]);
    let dashes = shapes.iter().filter(|s| matches!(&s.shape, egui::Shape::LineSegment { .. })).count();
    h.ed.sel = vec![a];
    let single = frame(&mut h, vec![]).iter().filter(|s| matches!(&s.shape, egui::Shape::LineSegment { .. })).count();
    assert!(dashes > single + 8, "dashes around both objects: {dashes} against {single}");
}

#[test]
fn a_layer_that_is_not_made_is_faded_towards_the_work_area() {
    let c = super::faded_colour(egui::Color32::BLACK, egui::Color32::WHITE);
    assert!(c.r() > 150 && c.r() == c.g(), "{c:?}");
    assert_eq!(super::faded_colour(egui::Color32::WHITE, egui::Color32::WHITE), egui::Color32::WHITE);
}

#[test]
fn a_right_click_on_a_node_opens_the_menu_of_the_node_bar_and_elsewhere_goes_back_to_selecting() {
    use base_core::geom::Pt;
    let mut h = Host::default();
    let id = h.scene.add(0, Kind::Rect { w: 40.0, h: 30.0 }, Xf::translate(20.0, 20.0));
    h.ed.sel = vec![id];
    super::select_tool(&mut h.ed, Tool::Node);
    let ctx = egui::Context::default();
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
    let mut frame = |h: &mut Host, events: Vec<egui::Event>| {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| {
            egui::CentralPanel::no_frame().show(ui, |ui| h.canvas(ui));
        });
        out.textures_delta.clear();
    };
    frame(&mut h, vec![]);
    frame(&mut h, vec![]);
    let right = |at: egui::Pos2, down: bool| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Secondary, pressed: down, modifiers: Default::default() };
    // The first corner of the rectangle (a rectangle in the node tool is made a path of four corners).
    let corner = h.w2s(h.ed.canvas_origin, Pt::new(20.0, 20.0));
    frame(&mut h, vec![egui::Event::PointerMoved(corner)]);
    frame(&mut h, vec![right(corner, true), right(corner, false)]);
    frame(&mut h, vec![]);
    assert_eq!(h.ed.tool, Tool::Node, "the menu, not the select tool");
    assert!(h.ed.node_menu && h.ed.node_sel.len() == 1, "the node is taken and its menu is open");
    // A right click away from the shape closes the menu; the next one goes back to the select tool.
    let away = (corner + egui::vec2(260.0, 200.0)).min(egui::pos2(780.0, 580.0));
    for _ in 0..2 {
        frame(&mut h, vec![egui::Event::PointerMoved(away), right(away, true), right(away, false)]);
        frame(&mut h, vec![]);
    }
    assert_eq!(h.ed.tool, Tool::Select);
}

#[test]
fn shift_draws_a_circle_with_the_ellipse_tool_and_a_square_with_the_rectangle() {
    use base_core::geom::Pt;
    let start = Pt::new(10.0, 10.0);
    assert_eq!(super::keep_square(Tool::Ellipse, start, Pt::new(40.0, 20.0), true), Pt::new(40.0, 40.0));
    assert_eq!(super::keep_square(Tool::Rect, start, Pt::new(0.0, -20.0), true), Pt::new(-20.0, -20.0), "towards the pointer");
    assert_eq!(super::keep_square(Tool::Ellipse, start, Pt::new(40.0, 20.0), false), Pt::new(40.0, 20.0));
    assert_eq!(super::keep_square(Tool::Line, start, Pt::new(40.0, 20.0), true), Pt::new(40.0, 20.0), "a line is not squared");
}

/// The style trace window of a picture: it opens from the menu, the lines come from the thread, the window keeps its size,
/// and Trace puts one path in the place of the picture on a layer without shapes.
#[test]
fn the_style_trace_window_makes_the_lines_and_puts_them_on_an_empty_layer() {
    use crate::editor::EditorStyleTrace;
    use crate::menu::StdAct;
    let mut h = Host::default();
    // A dark disc on white, 40 x 30 mm, put at (10, 20).
    let gray = (0..60 * 45).map(|i| { let (x, y) = ((i % 60) as f64, (i / 60) as f64); if (x - 30.0).hypot(y - 22.0) < 15.0 { 20 } else { 255 } }).collect();
    let im = base_core::image::ImageData { name: "disc.png".into(), px_w: 60, px_h: 45, gray, w: 40.0, h: 30.0, invert: false };
    h.scene.add(2, Kind::Rect { w: 5.0, h: 5.0 }, Xf::IDENTITY);
    let pic = h.scene.add(0, Kind::Image(im), Xf::translate(10.0, 20.0));
    h.ed.sel = vec![pic];
    assert!(StdAct::StyleTrace.enabled(&h));
    StdAct::StyleTrace.run(&mut h);
    let d = h.ed.style_dlg.as_ref().expect("the window opened");
    assert_eq!(d.layer, 1, "the first layer with no shapes, not the picture's (0) nor the one with the rectangle (2)");
    let ctx = egui::Context::default();
    let mut heights = vec![];
    let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1800.0, 1100.0));
    for _ in 0..400 {
        let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), ..Default::default() }, |ui| h.image_dialogs(&ui.ctx().clone()));
        out.textures_delta.clear();
        heights.push(ctx.memory(|m| m.area_rect(egui::Id::unique(Some("w_style_trace")))).map_or(0.0, |r| r.height()));
        if h.ed.style_dlg.as_ref().is_some_and(|d| d.ready().is_some()) && heights.len() > 40 {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    let n = heights.len();
    assert!(heights[n - 1] > 300.0 && heights[n - 1] < 1000.0 && heights[n - 1] == heights[5], "the window grew: {heights:?}");
    let d = h.ed.style_dlg.take().unwrap();
    let out = d.ready().cloned().expect("the result came from the thread");
    assert!(!out.lines.is_empty());
    h.apply_style_trace(&d, &out);
    let s = h.scene.shapes.last().unwrap();
    assert_eq!((s.layer, s.xf), (1, Xf::translate(10.0, 20.0)));
    assert!(matches!(&s.kind, Kind::Path(lines) if lines.len() == out.lines.len()));
    assert_eq!(h.ed.sel, vec![s.id]);
    assert_eq!(h.scene.shapes.len(), 3, "the picture is kept");
}

/// The style trace window in Polish, with the longest labels, keeps its size (runs in a process of its own: the language is
/// the same for the whole program).
#[test]
#[ignore = "run by the_style_trace_window_keeps_its_size_in_a_longer_language in a process of its own"]
fn style_trace_window_in_polish() {
    use crate::editor::EditorStyleTrace;
    if std::env::var_os("BASE_UI_ALONE").is_none() {
        return;
    }
    crate::i18n::set_lang(crate::i18n::Lang::Pl);
    let mut h = Host::default();
    let im = base_core::image::ImageData { name: "a.png".into(), px_w: 40, px_h: 30, gray: vec![90; 1200], w: 40.0, h: 30.0, invert: false };
    let pic = h.scene.add(0, Kind::Image(im), Xf::IDENTITY);
    h.ed.sel = vec![pic];
    h.open_style_trace();
    // The style with the most settings and the longest names.
    h.ed.style_dlg.as_mut().unwrap().params = base_core::style_trace::Style::FlowLines.defaults();
    let ctx = egui::Context::default();
    let w = window_widths(&ctx, 30, |ctx| h.image_dialogs(ctx));
    assert!(w[3] > 500.0 && w[3] < 1300.0 && w.windows(2).skip(3).all(|p| p[0] == p[1]), "{w:?}");
}

#[test]
fn the_style_trace_window_keeps_its_size_in_a_longer_language() {
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "editor::tests::style_trace_window_in_polish", "--ignored", "--nocapture"])
        .env("BASE_UI_ALONE", "1")
        .output()
        .unwrap();
    let text = String::from_utf8_lossy(&out.stdout).to_string() + String::from_utf8_lossy(&out.stderr).as_ref();
    assert!(out.status.success() && text.contains("1 passed"), "{text}");
}
