//! The parts of an editor that do not depend on the application: tools, the view, drags in progress, node addresses,
//! grid preferences and the geometry of fitting the work area into a window.
mod bars;
mod canvas;
mod commands;
mod context_menu;
mod host;
mod image_ui;
mod prefs;
mod props;
mod shape_ops;

pub use bars::{arrange_bar, tool_button, tool_palette};
pub use canvas::{CanvasView, EditorCanvas};
pub use commands::EditorCommands;
pub use context_menu::context_menu;
pub use host::EditorHost;
pub use shape_ops::EditorShapeOps;
pub use image_ui::{is_image_file, EditorImages, ImgDlg, TraceDlg, IMAGE_EXTENSIONS};
pub use bars::{control_name, grid_toggles, selection_size, zoom_and_cursor};
pub use prefs::view_options_window;
pub use props::properties_sections;

use base_core::bezier::Node;
use base_core::geom::{Pt, Rect, Xf};
use base_core::scene::Shape;
use egui::Key;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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
}

impl Tool {
    pub const ALL: [(Tool, &'static str, Key); 15] = [
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
    ];

    /// Selecting and editing tools, kept apart from the tools that insert new objects.
    pub fn is_edit(self) -> bool {
        matches!(self, Tool::Select | Tool::Node)
    }

    /// View navigation tools, kept apart from the drawing and editing tools.
    pub fn is_nav(self) -> bool {
        matches!(self, Tool::Pan | Tool::Zoom | Tool::ZoomOut)
    }
}

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
    /// Thickness of the outlines drawn in the editor, in points. Previews use a little less.
    #[serde(default = "default_line_width")]
    pub line_width: f32,
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

    /// Colour of the space around the work area: your choice, or the one of the colour scheme.
    pub fn workspace_fill(&self) -> egui::Color32 {
        self.workspace_color.map(|c| egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])).unwrap_or_else(crate::theme::workspace)
    }
}

impl Default for GridPrefs {
    fn default() -> Self {
        GridPrefs { main_color: [0xc4, 0xc4, 0xcc, 0xff], minor_on: false, minor_mm: 2.0, minor_color: [0xe6, 0xe6, 0xea, 0xff], bed_color: None, workspace_color: None, line_width: 1.0 }
    }
}

/// The state of an editor: what is selected, the tool, the view, the grid and snapping, and the shape being drawn.
/// An application keeps one and gives access to it through `EditorHost`.
pub struct EditorState {
    pub sel: Vec<u64>,
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
    /// What was copied (shapes with their groups), ready to paste.
    pub clipboard: Vec<Shape>,
    /// The import / adjust dialog of a picture, and the trace dialog.
    pub img_dlg: Option<ImgDlg>,
    pub trace_dlg: Option<TraceDlg>,
}

impl EditorState {
    pub fn new() -> EditorState {
        EditorState {
            sel: vec![],
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
            polygon_sides: 6,
            spiral_turns: 3,
            fillet_radius: 2.0,
            show_offset: false,
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
            clipboard: vec![],
            img_dlg: None,
            trace_dlg: None,
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
