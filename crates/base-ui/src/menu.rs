//! Menus. One definition of the entries serves the in-window menu bar (`menu_bar`) and the native macOS menu bar
//! (`native_menu`). The application chooses its own action type `A` and lists its menus; the entries of the
//! standard editing commands (`StdAct`) are provided here so that every application offers them in the same way.
use crate::editor::{EditorCommands, EditorHost};
use crate::i18n::tr;
use crate::{icons, theme};
use std::path::PathBuf;

/// One line of a menu.
#[derive(Clone)]
pub enum Entry<A> {
    /// Action, English label, accelerator in muda syntax (`CmdOrCtrl+O`) and display text (`Ctrl+O`).
    Item(A, &'static str, Option<(&'static str, &'static str)>),
    Sep,
    Sub(&'static str, Vec<Entry<A>>),
    /// The "Open recent" submenu, filled from the list of recently used files.
    Recent,
}

impl<A> Entry<A> {
    /// The same entry with its actions turned into another type.
    pub fn map<B>(self, f: &impl Fn(A) -> B) -> Entry<B> {
        match self {
            Entry::Item(a, label, accel) => Entry::Item(f(a), label, accel),
            Entry::Sep => Entry::Sep,
            Entry::Sub(title, children) => Entry::Sub(title, children.into_iter().map(|e| e.map(f)).collect()),
            Entry::Recent => Entry::Recent,
        }
    }
}

/// The top-level menus: English title and entries.
pub type Menus<A> = Vec<(&'static str, Vec<Entry<A>>)>;

pub const fn key(accel: &'static str, shown: &'static str) -> Option<(&'static str, &'static str)> {
    Some((accel, shown))
}

/// What a menu needs from the application.
pub trait MenuHost {
    type Act: Copy + PartialEq + std::fmt::Debug;
    fn menus(&self) -> Menus<Self::Act>;
    fn act_enabled(&self, _a: Self::Act) -> bool {
        true
    }
    /// `Some(on)` for entries that are switches.
    fn act_checked(&self, _a: Self::Act) -> Option<bool> {
        None
    }
    /// The icon of an action (a name from `icons`).
    fn act_icon(&self, _a: Self::Act) -> Option<&'static str> {
        None
    }
    /// The recently used files, and the actions that open one of them or clear the list.
    fn recent_files(&self) -> Vec<PathBuf> {
        vec![]
    }
    fn open_recent_act(&self, _i: usize) -> Option<Self::Act> {
        None
    }
    fn clear_recent_act(&self) -> Option<Self::Act> {
        None
    }
    /// Carry out the action the user chose.
    fn run_act(&mut self, ctx: &egui::Context, a: Self::Act);
}

/// The in-window menu bar.
pub fn menu_bar<H: MenuHost>(host: &mut H, ui: &mut egui::Ui) {
    let mut pending = None;
    egui::MenuBar::new().ui(ui, |ui| {
        for (title, entries) in host.menus() {
            let icon = icons::menu_title_icon(title).and_then(|n| icons::image(n, 15.0, theme::text()));
            match icon {
                Some(img) => ui.menu_button((img, tr(title)), |ui| menu_entries(host, ui, &entries, &mut pending)),
                None => ui.menu_button(tr(title), |ui| menu_entries(host, ui, &entries, &mut pending)),
            };
        }
    });
    if let Some(a) = pending {
        let ctx = ui.ctx().clone();
        host.run_act(&ctx, a);
    }
}

/// The entries of one menu or submenu; the action the user clicked is left in `pending`.
pub fn menu_entries<H: MenuHost>(host: &H, ui: &mut egui::Ui, entries: &[Entry<H::Act>], pending: &mut Option<H::Act>) {
    for e in entries {
        match e {
            Entry::Sep => {
                ui.separator();
            }
            Entry::Sub(title, children) => {
                let ink = theme::text();
                ui.menu_button((icons::slot(icons::submenu_icon(title), ink), tr(title)), |ui| menu_entries(host, ui, children, pending));
            }
            Entry::Recent => {
                let ink = theme::text();
                let recent = host.recent_files();
                ui.menu_button((icons::slot(Some("history"), ink), tr("Open recent")), |ui| {
                    if recent.is_empty() {
                        ui.add_enabled(false, egui::Button::new(tr("(empty)")));
                    }
                    for (i, p) in recent.iter().enumerate() {
                        let name = p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string());
                        let btn = egui::Button::image_and_text(icons::slot(Some("file"), ink), name);
                        if ui.add(btn).on_hover_text(p.display().to_string()).clicked() {
                            *pending = host.open_recent_act(i);
                            ui.close();
                        }
                    }
                    if !recent.is_empty() {
                        ui.separator();
                        let btn = egui::Button::image_and_text(icons::slot(Some("trash"), ink), tr("Clear list"));
                        if ui.add(btn).clicked() {
                            *pending = host.clear_recent_act();
                            ui.close();
                        }
                    }
                });
            }
            Entry::Item(act, label, accel) => {
                let enabled = host.act_enabled(*act);
                let ink = if enabled { theme::text() } else { theme::text().gamma_multiply(0.4) };
                // A tick marks switched-on options; other entries show their own icon.
                let (icon, ink) = match host.act_checked(*act) {
                    Some(true) => (Some("check"), ink),
                    // Switched off: the entry's own icon, faded, so every entry has one.
                    Some(false) => (host.act_icon(*act), ink.gamma_multiply(0.45)),
                    None => (host.act_icon(*act), ink),
                };
                let mut btn = egui::Button::image_and_text(icons::slot(icon, ink), tr(label));
                if let Some((_, shown)) = accel {
                    btn = btn.shortcut_text(*shown);
                }
                if ui.add_enabled(enabled, btn).clicked() {
                    *pending = Some(*act);
                    ui.close();
                }
            }
        }
    }
}

/// The editing commands every application offers, with the same labels, accelerators and icons.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum StdAct {
    Undo,
    Redo,
    Copy,
    Paste,
    Duplicate,
    Delete,
    SelectAll,
    Align(u8),
    CenterOnBed,
    CenterEachOther,
    FlipH,
    FlipV,
    RotCw,
    RotCcw,
    ToFront,
    ToBack,
    ToPath,
    EditNodes,
    Group,
    Ungroup,
    Lock,
    Unlock,
    ToggleGrid,
    ToggleSnap,
    ToggleSnapObjects,
    FitBed,
    FitAll,
    ZoomIn,
    ZoomOut,
    ToggleAutoFit,
    ImportImage,
    AdjustImage,
    TraceImage,
    BoolUnion,
    BoolIntersect,
    BoolSubtract,
    BoolXor,
    OffsetShape,
    RoundCorners,
    ToCurves,
    AutoJoin,
    ClosePath,
    DogboneCorners,
    ExportSelectionSvg,
    ExportSelectionDxf,
    GridArray,
    CircleArray,
    CopyAlongPath,
    TextOnPath,
    TrimByShape,
    RemoveDuplicates,
    BreakApart,
    SimplifyShapes,
    SerialText,
    MakeCode,
    NestShapes,
    MeasureShapes,
    BackgroundPicture,
    /// Choose a tool of the palette (the *Insert* menu lists the ones that insert objects).
    Tool(crate::editor::Tool),
    /// Divide the central area: beside or stacked (a second pane is added when there is only one).
    SplitView(crate::work_panes::Dir),
    /// Back to the active drawing alone.
    SingleView,
}

use Entry::{Item, Sep};

impl StdAct {
    /// Carry the command out on an editor.
    pub fn run<H: EditorHost>(self, h: &mut H) {
        match self {
            StdAct::Undo => h.undo(),
            StdAct::Redo => h.redo(),
            StdAct::Copy => h.copy(),
            StdAct::Paste => h.paste(),
            StdAct::Duplicate => h.duplicate(),
            StdAct::Delete => h.delete_selection(),
            StdAct::SelectAll => h.select_all(),
            StdAct::Align(m) => h.align(m),
            StdAct::CenterOnBed => h.center_on_bed(),
            StdAct::CenterEachOther => h.center_on_each_other(),
            StdAct::FlipH => h.flip(true),
            StdAct::FlipV => h.flip(false),
            StdAct::RotCw => h.rotate_sel(90.0),
            StdAct::RotCcw => h.rotate_sel(-90.0),
            StdAct::ToFront => h.reorder(true),
            StdAct::ToBack => h.reorder(false),
            StdAct::ToPath => h.to_path(),
            StdAct::EditNodes => h.start_node_edit(),
            StdAct::Group => h.group_selection(),
            StdAct::Ungroup => h.ungroup_selection(),
            StdAct::Lock => h.lock_selection(true),
            StdAct::Unlock => h.lock_selection(false),
            StdAct::ToggleGrid => h.ed_mut().show_grid = !h.ed().show_grid,
            StdAct::ToggleSnap => h.ed_mut().snap = !h.ed().snap,
            StdAct::ToggleSnapObjects => h.ed_mut().snap_objects = !h.ed().snap_objects,
            StdAct::FitBed => {
                let v = &mut h.ed_mut().view;
                v.need_fit = true;
                v.auto_fit = true;
            }
            StdAct::FitAll => {
                h.ed_mut().view.auto_fit = false;
                h.ed_mut().fit_all_req = true;
            }
            StdAct::ZoomIn => h.zoom_about_centre(1.5),
            StdAct::ZoomOut => h.zoom_about_centre(1.0 / 1.5),
            StdAct::ImportImage => crate::editor::EditorImages::import_image_dialog(h),
            StdAct::AdjustImage => crate::editor::EditorImages::open_adjust(h),
            StdAct::TraceImage => crate::editor::EditorImages::open_trace(h),
            StdAct::BoolUnion => crate::editor::EditorShapeOps::bool_op(h, base_core::ops::BoolOp::Union),
            StdAct::BoolIntersect => crate::editor::EditorShapeOps::bool_op(h, base_core::ops::BoolOp::Intersect),
            StdAct::BoolSubtract => crate::editor::EditorShapeOps::bool_op(h, base_core::ops::BoolOp::Difference),
            StdAct::BoolXor => crate::editor::EditorShapeOps::bool_op(h, base_core::ops::BoolOp::Xor),
            StdAct::OffsetShape => h.ed_mut().show_offset = true,
            StdAct::RoundCorners => h.ed_mut().show_round = true,
            StdAct::ToCurves => crate::editor::EditorShapeOps::to_curves(h),
            StdAct::AutoJoin | StdAct::ClosePath => h.ed_mut().show_join = true,
            StdAct::DogboneCorners => {
                let r = h.bone_radius_default();
                h.ed_mut().bone_radius = r;
                h.ed_mut().show_bones = true;
            }
            StdAct::GridArray => h.ed_mut().design.open = Some(crate::editor::DesignCmd::Grid),
            StdAct::CircleArray => h.ed_mut().design.open = Some(crate::editor::DesignCmd::Circle),
            StdAct::CopyAlongPath => h.ed_mut().design.open = Some(crate::editor::DesignCmd::AlongPath),
            StdAct::TextOnPath => h.ed_mut().design.open = Some(crate::editor::DesignCmd::TextPath),
            StdAct::TrimByShape => h.ed_mut().design.open = Some(crate::editor::DesignCmd::Trim),
            StdAct::RemoveDuplicates => crate::editor::EditorDesign::remove_duplicates(h),
            StdAct::BreakApart => crate::editor::EditorDesign::break_apart(h),
            StdAct::SimplifyShapes => h.ed_mut().design.open = Some(crate::editor::DesignCmd::Simplify),
            StdAct::SerialText => h.ed_mut().design.open = Some(crate::editor::DesignCmd::Serial),
            StdAct::MakeCode => h.ed_mut().design.open = Some(crate::editor::DesignCmd::Code),
            StdAct::NestShapes => {
                let (w, hh) = h.work_size();
                h.ed_mut().design.area = (w, hh);
                h.ed_mut().design.open = Some(crate::editor::DesignCmd::Nest);
            }
            StdAct::MeasureShapes => h.ed_mut().design.open = Some(crate::editor::DesignCmd::Measure),
            StdAct::BackgroundPicture => h.ed_mut().show_background = true,
            StdAct::Tool(t) => crate::editor::select_tool(h.ed_mut(), t),
            StdAct::SplitView(dir) => {
                if let Some(p) = h.panes_mut() {
                    p.dir = dir;
                    if p.is_single() {
                        p.split_off_one();
                    }
                }
            }
            StdAct::SingleView => {
                if let Some(p) = h.panes_mut() {
                    p.merge_all();
                }
            }
            StdAct::ExportSelectionSvg => crate::editor::EditorShapeOps::export_selection(h, false),
            StdAct::ExportSelectionDxf => crate::editor::EditorShapeOps::export_selection(h, true),
            StdAct::ToggleAutoFit => {
                let v = &mut h.ed_mut().view;
                v.auto_fit = !v.auto_fit;
                if v.auto_fit {
                    v.need_fit = true;
                }
            }
        }
    }

    pub fn enabled<H: EditorHost>(self, h: &H) -> bool {
        match self {
            StdAct::Undo => h.can_undo(),
            StdAct::Redo => h.can_redo(),
            StdAct::CenterEachOther => h.ed().sel.len() >= 2,
            StdAct::AdjustImage | StdAct::TraceImage => h.ed().sel.first().and_then(|id| h.scene().shape(*id)).is_some_and(|s| matches!(s.kind, base_core::scene::Kind::Image(_))),
            _ => true,
        }
    }

    /// `Some(on)` for the commands that are switches.
    pub fn checked<H: EditorHost>(self, h: &H) -> Option<bool> {
        match self {
            StdAct::ToggleGrid => Some(h.ed().show_grid),
            StdAct::ToggleSnap => Some(h.ed().snap),
            StdAct::ToggleSnapObjects => Some(h.ed().snap_objects),
            StdAct::ToggleAutoFit => Some(h.ed().view.auto_fit),
            StdAct::Tool(t) => Some(h.ed().tool == t),
            StdAct::SplitView(_) | StdAct::SingleView => None,
            _ => None,
        }
    }

    pub fn icon(self) -> &'static str {
        match self {
            StdAct::Undo => "undo-2",
            StdAct::Redo => "redo-2",
            StdAct::Copy => "copy",
            StdAct::Paste => "clipboard-paste",
            StdAct::Duplicate => "copy-plus",
            StdAct::Delete => "trash",
            StdAct::SelectAll => "lasso-select",
            StdAct::Align(0) => "align-start-vertical",
            StdAct::Align(1) => "align-center-vertical",
            StdAct::Align(2) => "align-end-vertical",
            StdAct::Align(3) => "align-start-horizontal",
            StdAct::Align(4) => "align-center-horizontal",
            StdAct::Align(_) => "align-end-horizontal",
            StdAct::CenterOnBed => "focus",
            StdAct::CenterEachOther => "target",
            StdAct::FlipH => "flip-horizontal",
            StdAct::FlipV => "flip-vertical",
            StdAct::RotCw => "rotate-cw",
            StdAct::RotCcw => "rotate-ccw",
            StdAct::ToFront => "bring-to-front",
            StdAct::ToBack => "send-to-back",
            StdAct::ToPath | StdAct::EditNodes => "spline",
            StdAct::Group => "group",
            StdAct::Ungroup => "ungroup",
            StdAct::Lock => "lock",
            StdAct::Unlock => "lock-open",
            StdAct::ToggleGrid => "grid-3x3",
            StdAct::ToggleSnap => "magnet",
            StdAct::ToggleSnapObjects => "target",
            StdAct::FitBed | StdAct::ToggleAutoFit => "scan",
            StdAct::FitAll => "expand",
            StdAct::ZoomIn => "zoom-in",
            StdAct::ZoomOut => "zoom-out",
            StdAct::ImportImage => "image",
            StdAct::AdjustImage => "sliders-horizontal",
            StdAct::TraceImage => "portrait",
            StdAct::BoolUnion => "squares-unite",
            StdAct::BoolIntersect => "squares-intersect",
            StdAct::BoolSubtract => "squares-subtract",
            StdAct::BoolXor => "squares-exclude",
            StdAct::OffsetShape => "maximize",
            StdAct::RoundCorners => "square-round-corner",
            StdAct::ToCurves => "spline",
            StdAct::AutoJoin | StdAct::ClosePath => "route",
            StdAct::DogboneCorners => "square-round-corner",
            StdAct::ExportSelectionSvg | StdAct::ExportSelectionDxf => "file-output",
            StdAct::GridArray => "grid-3x3",
            StdAct::CircleArray => "rotate-cw",
            StdAct::CopyAlongPath => "copy-plus",
            StdAct::TextOnPath => "type",
            StdAct::TrimByShape => "squares-subtract",
            StdAct::RemoveDuplicates => "copy",
            StdAct::BreakApart => "ungroup",
            StdAct::SimplifyShapes => "spline",
            StdAct::SerialText => "type",
            StdAct::MakeCode => "grid-2x2",
            StdAct::NestShapes => "layers",
            StdAct::MeasureShapes => "move",
            StdAct::BackgroundPicture => "image",
            StdAct::Tool(t) => crate::icons::tool_icon(t),
            StdAct::SplitView(crate::work_panes::Dir::Beside) => "panel-right",
            StdAct::SplitView(crate::work_panes::Dir::Stacked) => "panel-top",
            StdAct::SingleView => "square",
        }
    }
}

/// The entries of the standard commands, in groups the application strings together with its own entries.
pub mod std_entries {
    use super::*;

    /// Undo, redo, copy, paste, duplicate, delete, select all.
    pub fn edit() -> Vec<Entry<StdAct>> {
        vec![
            Item(StdAct::Undo, "Undo", key("CmdOrCtrl+Z", "Ctrl+Z")),
            Item(StdAct::Redo, "Redo", key("CmdOrCtrl+Shift+Z", "Ctrl+Y")),
            Sep,
            Item(StdAct::Copy, "Copy", key("CmdOrCtrl+C", "Ctrl+C")),
            Item(StdAct::Paste, "Paste", key("CmdOrCtrl+V", "Ctrl+V")),
            Item(StdAct::Duplicate, "Duplicate", key("CmdOrCtrl+D", "Ctrl+D")),
            Item(StdAct::Delete, "Delete", key("Backspace", "Del")),
            Item(StdAct::SelectAll, "Select all", key("CmdOrCtrl+A", "Ctrl+A")),
        ]
    }
    /// The six alignments.
    pub fn align() -> Vec<Entry<StdAct>> {
        ["Align left", "Align centre (H)", "Align right", "Align top", "Align centre (V)", "Align bottom"]
            .into_iter()
            .enumerate()
            .map(|(i, n)| Item(StdAct::Align(i as u8), n, None))
            .collect()
    }
    /// Centre on the bed, on each other, flip, rotate.
    pub fn place() -> Vec<Entry<StdAct>> {
        vec![
            Item(StdAct::CenterOnBed, "Centre on bed", None),
            Item(StdAct::CenterEachOther, "Centre on each other", None),
            Item(StdAct::FlipH, "Flip horizontal", None),
            Item(StdAct::FlipV, "Flip vertical", None),
            Item(StdAct::RotCw, "Rotate 90° CW", None),
            Item(StdAct::RotCcw, "Rotate 90° CCW", None),
        ]
    }
    pub fn group() -> Vec<Entry<StdAct>> {
        vec![
            Item(StdAct::Group, "Group", key("CmdOrCtrl+G", "Ctrl+G")),
            Item(StdAct::Ungroup, "Ungroup", key("CmdOrCtrl+Shift+G", "Ctrl+Shift+G")),
            Item(StdAct::Lock, "Lock", key("CmdOrCtrl+L", "Ctrl+L")),
            Item(StdAct::Unlock, "Unlock", key("CmdOrCtrl+Shift+L", "Ctrl+Shift+L")),
        ]
    }
    pub fn order() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::ToFront, "Bring to front", None), Item(StdAct::ToBack, "Send to back", None)]
    }
    /// Edit nodes, convert to path.
    pub fn nodes() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::EditNodes, "Edit nodes", key("N", "N")), Item(StdAct::ToPath, "Convert to path", None)]
    }
    /// Grid and snapping switches.
    pub fn grid() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::ToggleGrid, "Grid", None), Item(StdAct::ToggleSnap, "Snap to grid", None), Item(StdAct::ToggleSnapObjects, "Snap to objects", None)]
    }
    /// Import a picture (for the Import menu).
    pub fn import_image() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::ImportImage, "Import image…", None)]
    }
    /// Adjust and trace a picture that is on the page.
    pub fn image_tools() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::AdjustImage, "Adjust image…", None), Item(StdAct::TraceImage, "Trace image…", None)]
    }
    /// The operations on whole shapes: boolean operations, offset, rounding the corners, conversion to curves.
    pub fn shape_ops() -> Vec<Entry<StdAct>> {
        vec![
            Item(StdAct::BoolUnion, "Union", None),
            Item(StdAct::BoolIntersect, "Intersection", None),
            Item(StdAct::BoolSubtract, "Subtract", None),
            Item(StdAct::BoolXor, "Exclusive or", None),
            Sep,
            Item(StdAct::OffsetShape, "Offset shape…", None),
            Item(StdAct::RoundCorners, "Round corners…", None),
            Item(StdAct::ToCurves, "Convert to curves", None),
            Sep,
            Item(StdAct::AutoJoin, "Auto-join paths…", None),
            Item(StdAct::ClosePath, "Close path with tolerance…", None),
            Item(StdAct::DogboneCorners, "Dogbone / T-bone corners…", None),
        ]
    }
    /// Arrays, copies along a path, text on a path, trimming, duplicates, simplifying, serial text, codes, nesting and measuring.
    pub fn design_tools() -> Vec<Entry<StdAct>> {
        vec![
            Item(StdAct::GridArray, "Grid array…", None),
            Item(StdAct::CircleArray, "Circular array…", None),
            Item(StdAct::CopyAlongPath, "Copy along path…", None),
            Item(StdAct::TextOnPath, "Text on path…", None),
            Sep,
            Item(StdAct::TrimByShape, "Trim by shape…", None),
            Item(StdAct::RemoveDuplicates, "Remove duplicates", None),
            Item(StdAct::BreakApart, "Break apart", None),
            Item(StdAct::SimplifyShapes, "Simplify…", None),
            Sep,
            Item(StdAct::SerialText, "Serial text…", None),
            Item(StdAct::MakeCode, "Barcode / QR code…", None),
            Item(StdAct::NestShapes, "Nest shapes…", None),
            Item(StdAct::MeasureShapes, "Measure…", None),
        ]
    }
    /// The tools of the palette that insert objects: shapes, lines and text. The application adds its generators and clipart.
    pub fn insert_tools() -> Vec<Entry<StdAct>> {
        crate::editor::Tool::ALL.into_iter().filter(|t| t.0.is_insert()).map(|(t, label, _)| Item(StdAct::Tool(t), label, None)).collect()
    }
    /// The picture under the drawing.
    pub fn background() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::BackgroundPicture, "Background picture…", None)]
    }
    /// Write the selected shapes (all when nothing is selected) as SVG or DXF.
    pub fn export_selection() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::ExportSelectionSvg, "Export selection as SVG…", None), Item(StdAct::ExportSelectionDxf, "Export selection as DXF…", None)]
    }
    /// The division of the central area into panes.
    pub fn split_view() -> Vec<Entry<StdAct>> {
        vec![Item(StdAct::SplitView(crate::work_panes::Dir::Beside), "Split view: side by side", None), Item(StdAct::SplitView(crate::work_panes::Dir::Stacked), "Split view: one above the other", None), Item(StdAct::SingleView, "Single view", None)]
    }
    /// Zoom and fit.
    pub fn zoom() -> Vec<Entry<StdAct>> {
        vec![
            Item(StdAct::ZoomIn, "Zoom in", key("CmdOrCtrl+=", "Ctrl++")),
            Item(StdAct::ZoomOut, "Zoom out", key("CmdOrCtrl+-", "Ctrl+-")),
            Item(StdAct::FitBed, "Fit bed to window", key("CmdOrCtrl+Shift+0", "Ctrl+Shift+0")),
            Item(StdAct::FitAll, "Fit all objects", key("CmdOrCtrl+Shift+9", "Ctrl+Shift+9")),
            Item(StdAct::ToggleAutoFit, "Auto-fit work area to window", None),
        ]
    }
}

/// What differs between the menus of the programs. The layout is the same in all of them (see [`standard_menus`]):
///
/// **File · Edit · Insert · Arrange · View · Machine · (the menu of the process) · Settings · Help**
///
/// and every program puts only its own entries into the places below.
pub struct MenuParts<A> {
    pub new: A,
    pub open: A,
    pub save: A,
    pub save_as: A,
    pub quit: A,
    pub about: A,
    pub view_gcode: A,
    pub generate: A,
    pub import_plugin: A,
    pub online_clipart: A,
    pub plugins: A,
    pub view_options: A,
    pub program_options: A,
    pub set_lang: fn(crate::i18n::Lang) -> A,
    pub set_scheme: fn(crate::theme::Scheme) -> A,
    pub github: A,
    pub download_backup: A,
    pub restore_backup: A,
    pub check_updates: A,
    /// File, after *Save as…* (e.g. *Project setup…*).
    pub file_extra: Vec<Entry<A>>,
    /// The importers of the program; *Import image…* comes before them, the importers of plugins after them.
    pub import: Vec<Entry<A>>,
    /// The exports of the program, *Export G-code…* first; the export of the selection comes after them.
    pub export: Vec<Entry<A>>,
    /// File, after *View G-code file…* (e.g. *Print…*).
    pub output_extra: Vec<Entry<A>>,
    /// View, after the grid, the zoom and the split view: what the program shows besides the drawing.
    pub view_extra: Vec<Entry<A>>,
    /// View has *Background picture…* (a program that has an overlay of its own leaves it out).
    pub background: bool,
    /// The machine: its settings first, then, after a line, what runs on it.
    pub machine: Vec<Entry<A>>,
    /// The menu of the process (Laser, Milling, Plotter): the library, the preview of the job, the export of the G-code.
    pub process: (&'static str, Vec<Entry<A>>),
    /// Settings, after *Program options…* (nothing in the programs now; the libraries are in the menu of the process).
    pub settings_extra: Vec<Entry<A>>,
    /// Help, first (e.g. *Quick guide*).
    pub help_first: Vec<Entry<A>>,
    /// Help, after the backup (e.g. *Export diagnostics…*).
    pub help_extra: Vec<Entry<A>>,
}

/// The menus of a program: the same layout and the same standard entries in every program, with the program's own entries in their
/// places. The macOS app menu (About, Quit) is added by the native menu; elsewhere *Quit* ends File and *About* ends Help.
pub fn standard_menus<A: Copy + From<StdAct>>(p: MenuParts<A>) -> Menus<A> {
    use Entry::{Item, Recent, Sep, Sub};
    let std = |e: Vec<Entry<StdAct>>| -> Vec<Entry<A>> { e.into_iter().map(|e| e.map(&A::from)).collect() };
    let mac = cfg!(target_os = "macos");
    let mut file = vec![Item(p.new, "New", key("CmdOrCtrl+N", "Ctrl+N")), Item(p.open, "Open…", key("CmdOrCtrl+O", "Ctrl+O")), Recent, Sep, Item(p.save, "Save", key("CmdOrCtrl+S", "Ctrl+S")), Item(p.save_as, "Save as…", key("CmdOrCtrl+Shift+S", "Ctrl+Shift+S"))];
    file.extend(p.file_extra);
    file.push(Sep);
    let import = [std(std_entries::import_image()), p.import, vec![Sep, Item(p.import_plugin, crate::plugins_ui::MENU_IMPORT, None), Item(p.generate, crate::plugins_ui::MENU_GENERATE, None)]].concat();
    file.push(Sub("Import", import));
    file.push(Sub("Export", [p.export, vec![Sep], std(std_entries::export_selection())].concat()));
    file.push(Item(p.view_gcode, crate::gcode_view_ui::MENU_VIEW, None));
    file.extend(p.output_extra);
    if !mac {
        file.push(Sep);
        file.push(Item(p.quit, "Quit", key("CmdOrCtrl+Q", "Ctrl+Q")));
    }
    let arrange = [
        std(std_entries::align()),
        vec![Sep],
        std(std_entries::place()),
        vec![Sep],
        std(std_entries::group()),
        vec![Sep],
        std(std_entries::order()),
        vec![Sep],
        std(std_entries::nodes()),
        vec![Sep],
        std(std_entries::shape_ops()),
        vec![Sep],
        std(std_entries::design_tools()),
        vec![Sep],
        std(std_entries::image_tools()),
    ]
    .concat();
    let mut view = [std(std_entries::grid()), std(std_entries::zoom()), vec![Sep], std(std_entries::split_view())].concat();
    if p.background {
        view.push(Sep);
        view.extend(std(std_entries::background()));
    }
    if !p.view_extra.is_empty() {
        view.push(Sep);
        view.extend(p.view_extra);
    }
    let mut settings = vec![Item(p.view_options, "View options…", None), Item(p.program_options, "Program options…", None)];
    settings.extend(p.settings_extra);
    let (set_lang, set_scheme) = (p.set_lang, p.set_scheme);
    settings.extend([
        Sep,
        Item(p.plugins, crate::plugins_ui::MENU_PLUGINS, None),
        Sep,
        Sub("Language", crate::i18n::Lang::ALL.iter().map(|l| Item(set_lang(*l), l.name(), None)).collect()),
        Sub("Colour scheme", crate::theme::Scheme::available().iter().map(|s| Item(set_scheme(*s), s.label(), None)).collect()),
    ]);
    let mut help = p.help_first;
    help.extend([Item(p.github, "Project page on GitHub", None), Sep, Item(p.download_backup, crate::backup_ui::DOWNLOAD, None), Item(p.restore_backup, crate::backup_ui::RESTORE, None)]);
    help.extend(p.help_extra);
    help.extend([Sep, Item(p.check_updates, "Check for updates…", None)]);
    if !mac {
        help.push(Sep);
        help.push(Item(p.about, "About", None));
    }
    vec![
        ("File", file),
        ("Edit", std(std_entries::edit())),
        // The tools of the left palette that insert objects, and the other places shapes come from.
        ("Insert", [std(std_entries::insert_tools()), vec![Sep, Item(p.generate, crate::plugins_ui::MENU_GENERATE, None), Item(p.online_clipart, "Online clipart…", None)]].concat()),
        ("Arrange", arrange),
        ("View", view),
        ("Machine", p.machine),
        p.process,
        ("Settings", settings),
        ("Help", help),
    ]
}

/// The key of an accelerator (`CmdOrCtrl+Shift+S`): the modifiers and the key. `None` for one without Ctrl/Cmd (a plain letter, Del, F1:
/// the programs handle those themselves, they depend on what is being edited) and for a key this does not know.
pub fn parse_accel(accel: &str) -> Option<(egui::Modifiers, egui::Key)> {
    let mut parts: Vec<&str> = accel.split('+').collect();
    // `CmdOrCtrl+=` and `CmdOrCtrl++`: the last part can be empty when the key is "+".
    let last = match parts.pop()? {
        "" => "+",
        k => k,
    };
    let mut mods = egui::Modifiers::NONE;
    for m in parts.iter().filter(|m| !m.is_empty()) {
        match *m {
            "CmdOrCtrl" => mods = mods | egui::Modifiers::COMMAND,
            "Shift" => mods = mods | egui::Modifiers::SHIFT,
            "Alt" => mods = mods | egui::Modifiers::ALT,
            _ => return None,
        }
    }
    if !mods.command {
        return None;
    }
    let key = match last {
        "=" => egui::Key::Equals,
        "-" => egui::Key::Minus,
        "+" => egui::Key::Plus,
        k => egui::Key::from_name(k)?,
    };
    Some((mods, key))
}

/// The action of the menu entry whose Ctrl/Cmd key was just pressed (the key is taken, so nothing else acts on it too). For the menu bar
/// in the window (Windows, Linux, macOS without the native bar): the native bar of macOS has the keys itself. `enabled` leaves out an
/// entry that cannot be used now. Nothing is taken while a text field has the keyboard.
pub fn shortcut_act<A: Copy>(ctx: &egui::Context, menus: &Menus<A>, enabled: impl Fn(A) -> bool) -> Option<A> {
    if ctx.egui_wants_keyboard_input() {
        return None;
    }
    fn find<A: Copy>(ctx: &egui::Context, entries: &[Entry<A>], enabled: &impl Fn(A) -> bool) -> Option<A> {
        for e in entries {
            match e {
                Entry::Item(a, _, Some((accel, _))) => {
                    if let Some((mods, key)) = parse_accel(accel) {
                        if enabled(*a) && take_exact(ctx, mods, key) {
                            return Some(*a);
                        }
                    }
                }
                Entry::Sub(_, children) => {
                    if let Some(a) = find(ctx, children, enabled) {
                        return Some(a);
                    }
                }
                _ => {}
            }
        }
        None
    }
    menus.iter().find_map(|(_, entries)| find(ctx, entries, &enabled))
}

/// Take the press of `key` with exactly these modifiers. (egui's `consume_key` lets Ctrl+G match Ctrl+Shift+G too, so Group would win
/// over Ungroup.)
fn take_exact(ctx: &egui::Context, mods: egui::Modifiers, key: egui::Key) -> bool {
    let same = |m: &egui::Modifiers| m.command == mods.command && m.shift == mods.shift && m.alt == mods.alt;
    let here = ctx.input(|i| i.events.iter().any(|e| matches!(e, egui::Event::Key { key: k, pressed: true, modifiers, .. } if *k == key && same(modifiers))));
    here && ctx.input_mut(|i| i.consume_key(mods, key))
}

/// Pairs of entries that have the same key (for a test of each program: a key belongs to one command). The same command twice (in two
/// menus, under the same label) is not a clash.
pub fn key_clashes<A>(menus: &Menus<A>) -> Vec<(String, &'static str, &'static str)> {
    fn keys<A>(entries: &[Entry<A>], out: &mut Vec<(&'static str, &'static str)>) {
        for e in entries {
            match e {
                Entry::Item(_, label, Some((accel, _))) => out.push((accel, label)),
                Entry::Sub(_, children) => keys(children, out),
                _ => {}
            }
        }
    }
    let mut all = vec![];
    for (_, entries) in menus {
        keys(entries, &mut all);
    }
    let mut out = vec![];
    for (i, (k, label)) in all.iter().enumerate() {
        for (k2, l2) in &all[i + 1..] {
            if k2 == k && l2 != label {
                out.push((k.to_string(), *label, *l2));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn all_std_entries() -> Vec<Entry<StdAct>> {
        [std_entries::edit(), std_entries::align(), std_entries::place(), std_entries::group(), std_entries::order(), std_entries::nodes(), std_entries::grid(), std_entries::zoom(), std_entries::import_image(), std_entries::image_tools(), std_entries::shape_ops(), std_entries::insert_tools(), std_entries::split_view()].concat()
    }

    #[test]
    fn every_standard_command_has_an_entry_an_icon_that_exists_and_a_translation() {
        let have: std::collections::HashSet<&str> = icons::all_names().into_iter().collect();
        let mut seen = vec![];
        for e in all_std_entries() {
            if let Entry::Item(a, label, _) = e {
                assert!(have.contains(a.icon()), "icon of {a:?}");
                assert!(!seen.contains(&a), "{a:?} twice");
                seen.push(a);
                assert!(!label.is_empty());
            }
        }
        // 6 alignments and every other command once.
        assert_eq!(seen.iter().filter(|a| matches!(a, StdAct::Align(_))).count(), 6);
        assert!(seen.contains(&StdAct::Undo) && seen.contains(&StdAct::ZoomOut) && seen.contains(&StdAct::ToggleAutoFit));
    }

    #[derive(Clone, Copy, PartialEq, Debug)]
    enum T {
        Std(StdAct),
        App(u8),
        Lang,
        Scheme,
    }
    impl From<StdAct> for T {
        fn from(a: StdAct) -> T {
            T::Std(a)
        }
    }

    fn parts() -> MenuParts<T> {
        MenuParts {
            new: T::App(1),
            open: T::App(2),
            save: T::App(3),
            save_as: T::App(4),
            quit: T::App(5),
            about: T::App(6),
            view_gcode: T::App(7),
            generate: T::App(8),
            import_plugin: T::App(9),
            online_clipart: T::App(10),
            plugins: T::App(11),
            view_options: T::App(12),
            program_options: T::App(13),
            set_lang: |_| T::Lang,
            set_scheme: |_| T::Scheme,
            github: T::App(14),
            download_backup: T::App(15),
            restore_backup: T::App(16),
            check_updates: T::App(17),
            file_extra: vec![],
            import: vec![Item(T::App(20), "Import DXF…", None)],
            export: vec![Item(T::App(21), "Export G-code…", None)],
            output_extra: vec![],
            view_extra: vec![],
            background: true,
            machine: vec![Item(T::App(22), "Device settings…", None)],
            process: ("Milling", vec![Item(T::App(23), "Export G-code…", key("CmdOrCtrl+E", "Ctrl+E"))]),
            settings_extra: vec![],
            help_first: vec![],
            help_extra: vec![],
        }
    }

    #[test]
    fn the_standard_menus_have_one_layout_and_no_key_twice() {
        let m = standard_menus(parts());
        let titles: Vec<&str> = m.iter().map(|(t, _)| *t).collect();
        assert_eq!(titles, ["File", "Edit", "Insert", "Arrange", "View", "Machine", "Milling", "Settings", "Help"]);
        assert!(key_clashes(&m).is_empty(), "{:?}", key_clashes(&m));
        // A clash is found.
        let mut p = parts();
        p.process.1.push(Item(T::App(24), "Something", key("CmdOrCtrl+Shift+G", "Ctrl+Shift+G")));
        let clashes = key_clashes(&standard_menus(p));
        assert_eq!(clashes, vec![("CmdOrCtrl+Shift+G".to_string(), "Ungroup", "Something")]);
    }

    #[test]
    fn accelerators_are_read_and_a_pressed_key_gives_its_entry_once() {
        assert_eq!(parse_accel("CmdOrCtrl+Shift+S"), Some((egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::S)));
        assert_eq!(parse_accel("CmdOrCtrl+="), Some((egui::Modifiers::COMMAND, egui::Key::Equals)));
        assert_eq!(parse_accel("CmdOrCtrl+-"), Some((egui::Modifiers::COMMAND, egui::Key::Minus)));
        assert_eq!(parse_accel("CmdOrCtrl+Shift+0"), Some((egui::Modifiers::COMMAND | egui::Modifiers::SHIFT, egui::Key::Num0)));
        assert_eq!(parse_accel("Backspace"), None, "the programs keep the plain keys");
        assert_eq!(parse_accel("F1"), None);
        let m = standard_menus(parts());
        let ctx = egui::Context::default();
        let press = |key: egui::Key, modifiers: egui::Modifiers| egui::RawInput { events: vec![egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers }], ..Default::default() };
        let mut got = vec![];
        for (key, mods) in [(egui::Key::E, egui::Modifiers::COMMAND), (egui::Key::G, egui::Modifiers::COMMAND | egui::Modifiers::SHIFT), (egui::Key::G, egui::Modifiers::COMMAND), (egui::Key::L, egui::Modifiers::NONE)] {
            ctx.run_ui(press(key, mods), |ui| got.push(shortcut_act(ui.ctx(), &m, |_| true))).textures_delta.clear();
        }
        assert_eq!(got, vec![Some(T::App(23)), Some(T::Std(StdAct::Ungroup)), Some(T::Std(StdAct::Group)), None], "Ctrl+Shift+G is Ungroup, not Group");
        // An entry that cannot be used now leaves the key alone.
        let mut none = None;
        ctx.run_ui(press(egui::Key::E, egui::Modifiers::COMMAND), |ui| none = shortcut_act(ui.ctx(), &m, |a| a != T::App(23))).textures_delta.clear();
        assert_eq!(none, None);
    }

    #[test]
    fn mapping_entries_keeps_their_shape() {
        let e = Entry::Sub("Sub", vec![Entry::Item(1u8, "One", None), Entry::Sep, Entry::Recent]);
        match e.map(&|a: u8| a as u32 + 10) {
            Entry::Sub(t, c) => {
                assert_eq!(t, "Sub");
                assert!(matches!(c[0], Entry::Item(11, "One", None)) && matches!(c[1], Entry::Sep) && matches!(c[2], Entry::Recent));
            }
            _ => panic!("a submenu stays a submenu"),
        }
    }
}
