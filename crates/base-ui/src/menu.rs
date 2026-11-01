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
