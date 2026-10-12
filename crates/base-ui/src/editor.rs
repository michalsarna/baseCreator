//! The parts of an editor that do not depend on the application: tools, the view, drags in progress, node addresses,
//! grid preferences and the geometry of fitting the work area into a window.
use base_core::bezier::Node;
use base_core::geom::{Pt, Rect, Xf};
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

impl Default for GridPrefs {
    fn default() -> Self {
        GridPrefs { main_color: [0xc4, 0xc4, 0xcc, 0xff], minor_on: false, minor_mm: 2.0, minor_color: [0xe6, 0xe6, 0xea, 0xff], bed_color: None, workspace_color: None, line_width: 1.0 }
    }
}
