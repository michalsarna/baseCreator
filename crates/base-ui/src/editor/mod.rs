//! The parts of an editor that do not depend on the application: tools, the view, drags in progress, node addresses,
//! grid preferences and the geometry of fitting the work area into a window.
mod bars;
mod view_only;
mod canvas;
mod commands;
mod context_menu;
mod host;
mod keys;
mod image_ui;
mod style_ui;
mod prefs;
mod props;
mod background_ui;
mod design_ui;
mod shape_ops;
pub mod ruler;
pub mod text_ui;

pub use bars::{arrange_bar, dims_button, palette_button_size, tool_button, tool_palette};
pub use canvas::{CanvasView, EditorCanvas};
pub use commands::EditorCommands;
pub use context_menu::{add_as_zones, context_menu};
pub use host::EditorHost;
pub use keys::{hold_space, plain_keys};
pub use shape_ops::{EditorShapeOps, Exchange};
pub use background_ui::{background_section, Background, EditorBackground};
pub use design_ui::{CodeKind, DesignCmd, DesignState, EditorDesign};
pub use image_ui::{is_image_file, EditorImages, ImgDlg, TraceDlg, IMAGE_EXTENSIONS};
pub use style_ui::{EditorStyleTrace, StyleDlg};
pub use view_only::{show_other_doc, Look};
pub use bars::{control_name, grid_toggles, select_tool, selection_size, zoom_and_cursor};
pub use prefs::view_options_window;
pub use props::{layer_section, properties_sections, set_layer};

use base_core::bezier::Node;
use base_core::geom::{Pt, Rect, Xf};
use base_core::scene::Shape;
use egui::Key;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Tool {
    Select,
    Node,
    Rect,
    Ellipse,
    Triangle,
    Star,
    Polygon,
    Heart,
    Spiral,
    Line,
    Pen,
    Text,
    Pan,
    Zoom,
    ZoomOut,
    /// Measure the distance between two points, or the length of a straight segment that is clicked.
    Measure,
}

impl Tool {
    pub const ALL: [(Tool, &'static str, Key); 16] = [
        (Tool::Select, "Select (V)", Key::V),
        (Tool::Node, "Node edit (N)", Key::N),
        (Tool::Rect, "Rectangle (R)", Key::R),
        (Tool::Ellipse, "Ellipse (E)", Key::E),
        (Tool::Triangle, "Triangle (Y)", Key::Y),
        (Tool::Star, "Star (S)", Key::S),
        (Tool::Polygon, "Polygon (G)", Key::G),
        (Tool::Heart, "Heart (K)", Key::K),
        (Tool::Spiral, "Spiral (I)", Key::I),
        (Tool::Line, "Line (L)", Key::L),
        (Tool::Pen, "Polyline / pen (P)", Key::P),
        (Tool::Text, "Text (T)", Key::T),
        (Tool::Pan, "Pan (H)", Key::H),
        (Tool::Zoom, "Zoom in (Z)", Key::Z),
        (Tool::ZoomOut, "Zoom out (X)", Key::X),
        (Tool::Measure, "Ruler (M)", Key::M),
    ];

    /// Selecting and editing tools, kept apart from the tools that insert new objects.
    pub fn is_edit(self) -> bool {
        matches!(self, Tool::Select | Tool::Node)
    }

    /// The tools that insert new objects (the middle group of the palette; the *Insert* menu lists them).
    pub fn is_insert(self) -> bool {
        !self.is_edit() && !self.is_nav()
    }

    /// View navigation tools, kept apart from the drawing and editing tools.
    pub fn is_nav(self) -> bool {
        matches!(self, Tool::Pan | Tool::Zoom | Tool::ZoomOut | Tool::Measure)
    }
}

/// The measurement of the ruler: from `a` to `b` (while `b` is not set, to the pointer). `segment` says that it is the length of a
/// straight segment that was clicked; `picked` is a curve, a circle or an ellipse that was clicked.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Measure {
    pub a: Option<Pt>,
    pub b: Option<Pt>,
    pub segment: bool,
    /// The button is held since `a`: the end follows the pointer and is set where it is let go.
    pub dragging: bool,
    pub picked: Option<Picked>,
}

/// A curve, a circle or an ellipse measured by the ruler.
#[derive(Clone, Debug, PartialEq)]
pub enum Picked {
    /// A curved segment: its points, its length, the circle it lies on when it is an arc (centre, radius), and the point clicked.
    Curve { pts: Vec<Pt>, len: f64, arc: Option<(Pt, f64)>, at: Pt },
    /// A circle or an ellipse: its centre, its two semi-axes as vectors from the centre, and the point clicked on it.
    Ellipse { c: Pt, ax: Pt, ay: Pt, at: Pt },
}

impl Measure {
    /// The line measured, with the pointer `live` as the end while it is not set.
    pub fn line(&self, live: Option<Pt>) -> Option<(Pt, Pt)> {
        Some((self.a?, self.b.or(live)?))
    }
}

/// The far corner of a shape being drawn from `start`: with Shift held, as wide as high (a circle of the ellipse tool, a square of
/// the rectangle, the even shapes of the others) on the side the pointer is.
pub fn keep_square(tool: Tool, start: Pt, end: Pt, shift: bool) -> Pt {
    if !shift || !matches!(tool, Tool::Rect | Tool::Ellipse | Tool::Triangle | Tool::Star | Tool::Polygon | Tool::Heart | Tool::Spiral) {
        return end;
    }
    let s = (end.x - start.x).abs().max((end.y - start.y).abs());
    Pt::new(start.x + s * (end.x - start.x).signum(), start.y + s * (end.y - start.y).signum())
}

/// The words of a measurement: the length, the differences in X and Y (as the coordinates of the work area count them) and the angle
/// from the X axis as it is seen (counter-clockwise).
pub fn measure_text(a: Pt, b: Pt, units: base_core::units::Units, machine: base_core::units::Units) -> String {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = dx.hypot(dy);
    let angle = if len > 1e-9 { (-dy).atan2(dx).to_degrees() } else { 0.0 };
    let f = |v: f64| crate::units_ui::fmt_len_both(units, machine, v, 2);
    format!("{}   ΔX {}   ΔY {}   α {:.1}°", f(len), f(dx), f(dy), angle)
}

#[derive(Clone, Debug)]
pub struct View {
    pub zoom: f32, // screen px per mm
    pub pan: egui::Vec2,
    pub need_fit: bool,
    /// Keep the work area fitted to the window as it is resized. Zooming or panning by hand turns this off.
    pub auto_fit: bool,
}

/// A node address: shape id, contour index, node index.
pub type NodeRef = (u64, usize, usize);

#[derive(Clone)]
pub enum Drag {
    Move { start: Pt, orig: Vec<(u64, Xf)> },
    Scale { anchor: Pt, grab: Pt, handle: usize, orig: Vec<(u64, Xf)> },
    /// Turn about the middle of the selection; `from` is the angle of the point that was grabbed.
    Rotate { centre: Pt, from: f64, orig: Vec<(u64, Xf)> },
    /// Slant along an edge: `horizontal` shears x by the distance from the opposite edge `anchor`.
    Skew { horizontal: bool, anchor: Pt, grab: Pt, orig: Vec<(u64, Xf)> },
    Marquee { start: Pt },
    Create { start: Pt },
    NodeMove { start: Pt, orig: Vec<(NodeRef, Node)> },
    HandleMove { node: NodeRef, out: bool },
    NodeMarquee { start: Pt },
}

/// 8 resize handles: TL, T, TR, R, BR, B, BL, L.
pub fn handle_pts(b: &Rect) -> [Pt; 8] {
    let (l, r, t, bt) = (b.min.x, b.max.x, b.min.y, b.max.y);
    let (cx, cy) = (b.center().x, b.center().y);
    [Pt::new(l, t), Pt::new(cx, t), Pt::new(r, t), Pt::new(r, cy), Pt::new(r, bt), Pt::new(cx, bt), Pt::new(l, bt), Pt::new(l, cy)]
}

/// Zoom (pixels per mm) and pan that centre a `bed` (mm) in an area of `avail` pixels with `margin` pixels around.
pub fn fit_view(avail: (f32, f32), bed: (f64, f64), margin: f32) -> (f32, egui::Vec2) {
    let z = ((avail.0 - 2.0 * margin) / bed.0 as f32).min((avail.1 - 2.0 * margin) / bed.1 as f32).max(0.05);
    (z, egui::vec2((avail.0 - bed.0 as f32 * z) / 2.0, (avail.1 - bed.1 as f32 * z) / 2.0))
}

/// Zoom and pan that centre the region `min` / `size` (mm) in an area of `avail` pixels with `margin` around it.
pub fn fit_region(avail: (f32, f32), min: (f64, f64), size: (f64, f64), margin: f32) -> (f32, egui::Vec2) {
    let z = ((avail.0 - 2.0 * margin) / size.0 as f32).min((avail.1 - 2.0 * margin) / size.1 as f32).clamp(0.05, 200.0);
    let (cx, cy) = (min.0 as f32 + size.0 as f32 / 2.0, min.1 as f32 + size.1 as f32 / 2.0);
    (z, egui::vec2(avail.0 / 2.0 - cx * z, avail.1 / 2.0 - cy * z))
}

/// The colour of the shapes of a layer that makes no output (not cut, milled or drawn): faded far towards the work area `bed`, so
/// they stand apart from the ones that are made and still show.
pub fn faded_colour(c: egui::Color32, bed: egui::Color32) -> egui::Color32 {
    let mix = |a: u8, b: u8| (a as f32 * 0.3 + b as f32 * 0.7).round() as u8;
    egui::Color32::from_rgb(mix(c.r(), bed.r()), mix(c.g(), bed.g()), mix(c.b(), bed.b()))
}

impl GridPrefs {
    /// The units the program shows lengths in: the chosen ones, or those of the device.
    pub fn display_units(&self, device: base_core::units::Units) -> base_core::units::Units {
        self.units.unwrap_or(device)
    }
}

pub fn nice_step(min_mm: f64) -> f64 {
    for m in [1.0, 2.0, 5.0] .iter().cycle().zip(0..).map(|(m, i)| m * 10f64.powi(i / 3 - 2)) {
        if m >= min_mm {
            return m;
        }
    }
    1000.0
}

/// Appearance of the work-area grids. The main grid's switch and spacing live in `App::show_grid` / `App::grid`
/// because snapping uses them too.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridPrefs {
    pub main_color: [u8; 4],
    pub minor_on: bool,
    /// Spacing of the secondary grid in mm. Always smaller than the main spacing.
    pub minor_mm: f64,
    pub minor_color: [u8; 4],
    /// Colour of the work-area sheet; `None` picks one that suits the colour scheme.
    #[serde(default)]
    pub bed_color: Option<[u8; 4]>,
    /// Colour of the space around the work area; `None` takes the one of the colour scheme.
    #[serde(default)]
    pub workspace_color: Option<[u8; 4]>,
    /// The units the program shows lengths in (the properties, the ruler, the dimensions, the previews); `None` takes those of the
    /// device. The G-code is always in the units of the device.
    #[serde(default)]
    pub units: Option<base_core::units::Units>,
    /// Colour of the work area (the material, the paper) in the preview of a job; `None` takes the one of the drawing.
    #[serde(default)]
    pub preview_color: Option<[u8; 4]>,
    /// Thickness of the outlines drawn in the editor, in points. Previews use a little less.
    #[serde(default = "default_line_width")]
    pub line_width: f32,
    /// Show the name of a layer that has shapes beside its colour in the row of layers.
    #[serde(default)]
    pub layer_names: bool,
    /// After a G-code file is saved, open it in the analysis of G-code.
    #[serde(default)]
    pub gcode_preview_after_save: bool,
    /// Show the name of the variable beside the label of a field that has one (see `widgets::var_label`).
    #[serde(default)]
    pub variable_names: bool,
    /// The colours and the size of the letters of the console.
    #[serde(default)]
    pub console: crate::console_ui::ConsoleLook,
}

fn default_line_width() -> f32 {
    1.0
}

impl GridPrefs {
    /// Fill colour of the work area: your choice, or very light grey in the dark schemes and white in the light ones.
    pub fn bed_fill(&self, dark_scheme: bool) -> egui::Color32 {
        match self.bed_color {
            Some(c) => egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
            None if dark_scheme => egui::Color32::from_rgb(0xea, 0xea, 0xed),
            None => egui::Color32::WHITE,
        }
    }

    /// Colour of the work area in the preview of a job: your choice there, or the one of the drawing.
    pub fn preview_fill(&self, dark_scheme: bool) -> egui::Color32 {
        match self.preview_color {
            Some(c) => egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3]),
            None => self.bed_fill(dark_scheme),
        }
    }

    /// Colour of the space around the work area: your choice, or the one of the colour scheme.
    pub fn workspace_fill(&self) -> egui::Color32 {
        self.workspace_color.map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])).unwrap_or_else(crate::theme::workspace)
    }
}

impl Default for GridPrefs {
    fn default() -> Self {
        GridPrefs { main_color: [0xc4, 0xc4, 0xcc, 0xff], minor_on: false, minor_mm: 2.0, minor_color: [0xe6, 0xe6, 0xea, 0xff], bed_color: None, workspace_color: None, units: None, preview_color: None, line_width: 1.0, layer_names: false, gcode_preview_after_save: false, variable_names: false, console: Default::default() }
    }
}

/// The state of an editor: what is selected, the tool, the view, the grid and snapping, and the shape being drawn.
/// An application keeps one and gives access to it through `EditorHost`.
pub struct EditorState {
    /// What a new text starts as: the settings of the text changed last.
    pub text_default: base_core::text::TextData,
    /// A text was just placed: the field to type in takes the keyboard.
    pub focus_text: bool,
    /// The search in the list of fonts.
    pub font_filter: String,
    /// When a text was last changed, for one undo step per burst of editing.
    pub text_changed_at: f64,
    pub sel: Vec<u64>,
    /// The ghosts of the other side of a two-sided job that are selected (their ids are those of the other side, never mixed with
    /// `sel`): they are a fixed reference to align and snap to, and are never moved.
    pub ghost_sel: Vec<u64>,
    pub node_sel: Vec<NodeRef>,
    pub active_seg: Option<NodeRef>,
    pub tool: Tool,
    /// The layer (or similar group) that new shapes go to.
    pub active_layer: usize,
    pub view: View,
    pub drag: Option<Drag>,
    pub pen_pts: Vec<Pt>,
    /// The selection handles turn and slant (true) instead of resizing (false); a click on a handle swaps them.
    pub rotate_mode: bool,
    /// The Properties tab keeps the proportions when the width or the height is typed.
    pub lock_aspect: bool,
    /// The angle typed in the Properties tab.
    pub rot_input: f64,
    pub snap: bool,
    /// Snap a moved selection to the edges and centres of the other objects (guides are drawn while it holds).
    pub snap_objects: bool,
    /// Boxes of the objects a drag can snap to (taken when the drag starts) and the guide lines now shown.
    pub snap_targets: Option<Vec<Rect>>,
    pub snap_guides: (Option<f64>, Option<f64>),
    pub grid: f64,
    pub show_grid: bool,
    pub grid_prefs: GridPrefs,
    pub polygon_sides: u32,
    pub spiral_turns: u32,
    /// Radius (mm) for rounding corners.
    pub fillet_radius: f64,
    /// The small windows of the offset and of rounding the corners of whole shapes, and what the offset window holds.
    pub show_offset: bool,
    /// The small window that joins paths and closes them (Auto-join, Close path with tolerance), and its tolerance (mm).
    pub show_join: bool,
    /// The window of the dogbone / T-bone corners, and its radius (mm).
    pub show_bones: bool,
    pub bone_radius: f64,
    /// The design commands (arrays, text on a path, serial text, codes, nesting, measuring) and what their windows hold.
    pub design: DesignState,
    /// A picture under the drawing (a photo, a plan, the camera) and its window.
    pub background: Option<Background>,
    pub show_background: bool,
    /// The picture under the drawing follows the camera.
    pub background_live: bool,
    pub background_seq: u64,
    /// The picture of a project that was opened, to be shown in the next frame.
    pub background_pending: Option<base_core::overlay_data::OverlayData>,
    pub join_tol: f64,
    pub show_round: bool,
    pub offset_dist: f64,
    pub offset_keep: bool,
    /// The small window with the number of sides (polygon) or turns (spiral).
    pub show_shape_popup: bool,
    /// The last polygon or spiral drawn: tool, shape id and the box it was drawn in. The popup edits it.
    pub last_param: Option<(Tool, u64, f64, f64)>,
    /// The node-editing bar is shown while the canvas, the bar or the tool bars were the last thing clicked.
    pub node_bar_active: bool,
    /// Fit the view to all objects on the next frame.
    pub fit_all_req: bool,
    pub cursor_mm: Option<Pt>,
    /// Size in pixels of the canvas as last drawn.
    pub canvas_avail: (f32, f32),
    /// Top-left corner of the work area on screen, for tests and overlays.
    pub canvas_origin: egui::Pos2,
    /// Textures of the pictures in the scene, by shape id and whether they are inverted.
    pub image_tex: std::collections::HashMap<(u64, bool), egui::TextureHandle>,
    /// The tool to return to when Space (temporary pan) is released.
    pub space_prev: Option<Tool>,
    /// When Escape was pressed last: a second press soon after goes back to the select tool.
    pub esc_time: f64,
    /// What the ruler shows over the drawing.
    pub measure: Measure,
    /// The dimensions of the drawing over it, as on a technical drawing (the button next to the toolpaths).
    pub show_dims: bool,
    /// The menu of the node bar is open by a right click on a node or a segment.
    pub node_menu: bool,
    /// What was copied (shapes with their groups), ready to paste.
    pub clipboard: Vec<Shape>,
    /// The picture that was in the clipboard of the system when shapes were last copied: a paste takes the shapes while it is
    /// still there, and a picture copied later (from another program) when it changed.
    pub clipboard_image_at_copy: Option<u64>,
    /// The import / adjust dialog of a picture, and the trace dialog.
    pub img_dlg: Option<ImgDlg>,
    /// A file with Illustrator layers that was read and waits for the answer how to put its paths on the layers.
    pub drawing_choice: Option<(std::path::PathBuf, Vec<base_core::svg::Outline>)>,
    pub trace_dlg: Option<TraceDlg>,
    /// The style trace window of a picture.
    pub style_dlg: Option<StyleDlg>,
}

/// What of the editor belongs to one open drawing (a tab): the selection, the layer new shapes go to and the view.
/// `EditorState::swap_doc_view` exchanges it with the live one when another tab is shown.
pub struct DocView {
    pub sel: Vec<u64>,
    pub node_sel: Vec<NodeRef>,
    pub active_seg: Option<NodeRef>,
    pub active_layer: usize,
    pub view: View,
    pub background: Option<Background>,
    pub background_live: bool,
    pub background_pending: Option<base_core::overlay_data::OverlayData>,
}

impl Default for DocView {
    fn default() -> Self {
        DocView { sel: vec![], node_sel: vec![], active_seg: None, active_layer: 0, view: View { zoom: 2.0, pan: egui::vec2(40.0, 40.0), need_fit: true, auto_fit: true }, background: None, background_live: false, background_pending: None }
    }
}

impl EditorState {
    /// Put the drawing-bound part of the editor into `other` and take what `other` held. Whatever refers to the shapes of the
    /// drawing that was shown (a drag in progress, the guides, the textures of its pictures, the picture dialogs) is dropped.
    pub fn swap_doc_view(&mut self, other: &mut DocView) {
        std::mem::swap(&mut self.sel, &mut other.sel);
        self.ghost_sel.clear();
        std::mem::swap(&mut self.node_sel, &mut other.node_sel);
        std::mem::swap(&mut self.active_seg, &mut other.active_seg);
        std::mem::swap(&mut self.active_layer, &mut other.active_layer);
        std::mem::swap(&mut self.view, &mut other.view);
        std::mem::swap(&mut self.background, &mut other.background);
        std::mem::swap(&mut self.background_live, &mut other.background_live);
        std::mem::swap(&mut self.background_pending, &mut other.background_pending);
        self.background_seq = 0;
        self.drag = None;
        self.pen_pts.clear();
        self.measure = Measure::default();
        self.snap_targets = None;
        self.snap_guides = (None, None);
        self.last_param = None;
        self.show_shape_popup = false;
        self.img_dlg = None;
        self.drawing_choice = None;
        self.trace_dlg = None;
        self.style_dlg = None;
        self.image_tex.clear();
    }
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            sel: vec![],
            ghost_sel: vec![],
            node_sel: vec![],
            active_seg: None,
            tool: Tool::Select,
            active_layer: 0,
            view: View { zoom: 2.0, pan: egui::vec2(40.0, 40.0), need_fit: true, auto_fit: true },
            drag: None,
            pen_pts: vec![],
            rotate_mode: false,
            lock_aspect: true,
            rot_input: 15.0,
            snap: false,
            snap_objects: false,
            snap_targets: None,
            snap_guides: (None, None),
            grid: 10.0,
            show_grid: true,
            grid_prefs: GridPrefs::default(),
            text_default: Default::default(),
            focus_text: false,
            font_filter: String::new(),
            text_changed_at: 0.0,
            polygon_sides: 6,
            spiral_turns: 3,
            fillet_radius: 2.0,
            show_offset: false,
            show_join: false,
            show_bones: false,
            bone_radius: 1.5,
            design: DesignState::default(),
            background: None,
            show_background: false,
            background_live: false,
            background_seq: 0,
            background_pending: None,
            join_tol: 0.1,
            show_round: false,
            offset_dist: -1.0,
            offset_keep: true,
            show_shape_popup: false,
            last_param: None,
            node_bar_active: true,
            fit_all_req: false,
            cursor_mm: None,
            canvas_avail: (900.0, 600.0),
            canvas_origin: egui::Pos2::ZERO,
            image_tex: Default::default(),
            space_prev: None,
            esc_time: -10.0,
            measure: Measure::default(),
            show_dims: false,
            node_menu: false,
            clipboard: vec![],
            clipboard_image_at_copy: None,
            img_dlg: None,
            drawing_choice: None,
            trace_dlg: None,
            style_dlg: None,
        }
    }
}

impl EditorState {
    /// Keep the secondary grid finer than the main grid.
    pub fn clamp_grid(&mut self) {
        self.grid = self.grid.clamp(0.1, 1000.0);
        let max = (self.grid * 0.9).max(0.05);
        self.grid_prefs.minor_mm = self.grid_prefs.minor_mm.clamp(0.05, max);
    }
}

impl Default for EditorState {
    fn default() -> Self {
        EditorState::new()
    }
}

#[cfg(test)]
mod tests;

/// The zoom of the drawing as the status bar shows it, in percent.
pub fn zoom_percent(zoom: f32) -> f32 {
    zoom / 2.0 * 100.0
}

/// The zoom (in percent) above which the secondary grid is drawn.
pub const MINOR_GRID_FROM: f32 = 200.0;

/// Is the secondary grid drawn at this zoom? Only closer than 200 %.
pub fn minor_grid_seen(zoom: f32) -> bool {
    zoom_percent(zoom) > MINOR_GRID_FROM
}

#[cfg(test)]
mod minor_grid_tests {
    #[test]
    fn the_secondary_grid_is_seen_only_closer_than_200_percent() {
        assert!(!super::minor_grid_seen(2.0), "100 %");
        assert!(!super::minor_grid_seen(4.0), "200 % itself");
        assert!(super::minor_grid_seen(4.02), "201 %");
        assert_eq!(super::zoom_percent(2.72).round(), 136.0);
    }
}
