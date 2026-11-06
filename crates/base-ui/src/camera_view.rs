//! The camera view of every program: the three ways of showing it (a window of its own, a tab of the side panel, a tab of the
//! work area), the picture that zooms and moves, and the window of its own. The programs keep the state (the zoom, the shift, the
//! fit of the window) and the parts that are theirs (the overlay button, the recording, the working time) and draw them in the
//! same places, so the view is the same in every program.
use crate::i18n::tr;
use crate::window::FitWindow;
use base_core::camimg::{CameraSource, CameraView};
use egui::{Rect, Vec2};
use std::time::{Duration, Instant};

/// How long the camera window must keep its size to count as let go.
pub const STILL: Duration = Duration::from_millis(500);
/// The smallest camera window (the same as its minimum inner size).
pub const MIN: [f32; 2] = [320.0, 240.0];
/// The most the picture is zoomed in.
pub const MAX_ZOOM: f32 = 8.0;

/// How the camera view is shown.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StreamMode {
    /// A window of the operating system of its own.
    Detached,
    /// A tab of the side panel.
    Tab,
    /// A tab of the work area, next to the drawings.
    Workspace,
}

impl StreamMode {
    /// As it is kept in `CameraView::mode` and `CameraSource::view_mode`.
    pub fn from_index(i: u8) -> StreamMode {
        match i {
            1 => StreamMode::Detached,
            2 => StreamMode::Tab,
            3 => StreamMode::Workspace,
            // 0 was a window inside the program: it is a tab of the side panel now.
            _ => StreamMode::Tab,
        }
    }
    pub fn index(self) -> u8 {
        match self {
            StreamMode::Detached => 1,
            StreamMode::Tab => 2,
            StreamMode::Workspace => 3,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            StreamMode::Detached => "Separate window",
            StreamMode::Tab => "Side tab",
            StreamMode::Workspace => "Tab in the work area",
        }
    }
    /// The name of the mode, translated.
    pub fn tip(self) -> String {
        tr(self.label()).to_string()
    }
    /// How the view of camera `cam` opens: its own way, or the device's.
    pub fn of(view: &CameraView, cam: Option<&CameraSource>) -> StreamMode {
        StreamMode::from_index(cam.and_then(|c| c.view_mode).unwrap_or(view.mode))
    }
}

/// The three ways, with their icons.
pub const MODE_ICONS: [(StreamMode, &str); 3] = [(StreamMode::Detached, "square-arrow-out-up-right"), (StreamMode::Tab, "panel-right"), (StreamMode::Workspace, "panel-top")];

/// The three buttons of the ways of showing the view; the one picked, when one was.
pub fn mode_buttons(ui: &mut egui::Ui, now: StreamMode) -> Option<StreamMode> {
    let mut pick = None;
    for (m, icon) in MODE_ICONS {
        if crate::widgets::toggle_icon(ui, icon, &m.tip(), now == m) {
            pick = Some(m);
        }
    }
    pick.filter(|m| *m != now)
}

/// The picture of the camera, fitted to the space (up as well as down) and zoomed and moved by the wheel, a drag, the zoom bar and a
/// double click; "waiting" while there is none yet. `under` is a guess of the height of what stands under it before it was measured.
/// Returns the place of the picture and the width left over, for [`FitWindow::measure`] after what stands under it was drawn.
pub fn picture(ui: &mut egui::Ui, tex: Option<&egui::TextureHandle>, zoom: &mut f32, pan: &mut Vec2, fit: &mut FitWindow, under: f32) -> Option<(Rect, f32)> {
    let Some(t) = tex else {
        fit.slack = Vec2::ZERO;
        ui.add_space(30.0);
        ui.label(tr("Waiting for the picture…"));
        ui.add_space(30.0);
        return None;
    };
    let sz = t.size_vec2();
    // 1.0 is the picture fitted to the space. What stands under it is left out of its height: the last frame measured it.
    let (k, slack_x) = crate::window::fit_picture(ui, sz, fit.under(under));
    let (resp, painter) = ui.allocate_painter(sz * k, egui::Sense::click_and_drag());
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll != 0.0 {
            *zoom = (*zoom * (scroll * 0.0015).exp()).clamp(1.0, MAX_ZOOM);
        }
    }
    if resp.dragged() {
        *pan += resp.drag_delta();
    }
    // The zoom bar of every picture, in the lower left corner, and a double click: the whole picture again.
    let k = match crate::zoom_bar::overlay(ui, resp.rect, *zoom, &tr("Scroll to zoom, drag to move, double click to see the whole picture")) {
        Some(crate::zoom_bar::ZoomCmd::In) => Some(crate::zoom_bar::STEP),
        Some(crate::zoom_bar::ZoomCmd::Out) => Some(1.0 / crate::zoom_bar::STEP),
        Some(crate::zoom_bar::ZoomCmd::Whole) => {
            *zoom = 1.0;
            None
        }
        None => None,
    };
    if let Some(k) = k {
        let z = (*zoom * k).clamp(1.0, MAX_ZOOM);
        *pan *= z / *zoom;
        *zoom = z;
    }
    if resp.double_clicked() {
        *zoom = 1.0;
    }
    // The picture cannot be pushed out of its frame.
    let room = resp.rect.size() * (*zoom - 1.0) / 2.0;
    *pan = pan.clamp(-room, room);
    let img = Rect::from_center_size(resp.rect.center() + *pan, resp.rect.size() * *zoom);
    painter.with_clip_rect(resp.rect).image(t.id(), img, Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), egui::Color32::WHITE);
    Some((resp.rect, slack_x))
}

/// Is the window of its own being resized (its size changed a moment ago)? The picture stands still then.
pub fn resizing(win: &Option<([f32; 2], Instant)>) -> bool {
    win.is_some_and(|(_, t)| t.elapsed() < STILL)
}

/// What the window of its own did in this frame.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct WindowSeen {
    /// The program cannot open windows of its own (it draws them inside its window): show the view another way.
    pub embedded: bool,
    pub close: bool,
    /// The place and the inner size of the window, rounded.
    pub place: Option<([f32; 2], [f32; 2])>,
}

/// The window of its own of the camera view: the same bar, edges and square corners as the main window ([`crate::window::own_window`]),
/// opened where `saved` says. `add` draws the view in it.
pub fn window(ctx: &egui::Context, id: egui::ViewportId, title: &str, logo: Option<&egui::TextureHandle>, saved: &CameraView, mut add: impl FnMut(&mut egui::Ui)) -> WindowSeen {
    let mut builder = crate::window::own_window_builder(title).with_inner_size(saved.win_size.unwrap_or([720.0, 580.0])).with_min_inner_size(MIN);
    if let Some(p) = saved.win_pos {
        builder = builder.with_position(p);
    }
    let mut seen = WindowSeen::default();
    ctx.show_viewport_immediate(id, builder, |ui, class| {
        if class == egui::ViewportClass::EmbeddedWindow {
            seen.embedded = true;
            return;
        }
        if crate::window::own_window(ui, title, logo, egui::vec2(MIN[0], MIN[1])) {
            seen.close = true;
        }
        egui::CentralPanel::default().show(ui, |ui| add(ui));
        ui.input(|i| {
            let vp = i.viewport();
            if let (Some(o), Some(inner)) = (vp.outer_rect, vp.inner_rect) {
                seen.place = Some(([o.min.x.round(), o.min.y.round()], [inner.width().round(), inner.height().round()]));
            }
            if vp.close_requested() {
                seen.close = true;
            }
        });
    });
    seen
}

/// Note the size of the window of its own; once it has stood still for a while (the edge was let go) and is bigger than its picture
/// needs, give back the empty strip: the window is cut to the scaled picture.
pub fn settle(ctx: &egui::Context, id: egui::ViewportId, win: &mut Option<([f32; 2], Instant)>, fit: &FitWindow, size: [f32; 2]) {
    match *win {
        Some((s, _)) if s == size => {}
        _ => *win = Some((size, Instant::now())),
    }
    if resizing(win) {
        ctx.request_repaint_after(STILL);
    } else if let Some(w) = fit.fitted(egui::vec2(size[0], size[1]), MIN) {
        ctx.send_viewport_cmd_to(id, egui::ViewportCommand::InnerSize(w));
    }
}

/// Keep where the window of its own is in `view` (for the next time it opens); true when it changed. Not while a button is held (the
/// window is being moved or resized) and not for a window that is too small to be one.
pub fn keep_place(ctx: &egui::Context, view: &mut CameraView, place: Option<([f32; 2], [f32; 2])>) -> bool {
    let Some((pos, size)) = place else { return false };
    if size[0] >= 100.0 && size[1] >= 100.0 && (view.win_pos != Some(pos) || view.win_size != Some(size)) && !ctx.input(|i| i.pointer.any_down()) {
        view.win_pos = Some(pos);
        view.win_size = Some(size);
        return true;
    }
    false
}

/// Keep the way of showing and the zoom: the zoom and the way with the device, the way also with camera `active` (each camera opens
/// its own way). True when something changed.
pub fn keep_mode(view: &mut CameraView, cameras: &mut [CameraSource], active: usize, mode: StreamMode, zoom: f32) -> bool {
    let m = mode.index();
    let new_cam = active < cameras.len() && cameras[active].view_mode != Some(m);
    if new_cam || view.mode != m || (view.zoom - zoom).abs() > 0.005 {
        view.mode = m;
        view.zoom = zoom;
        if let Some(c) = cameras.get_mut(active) {
            c.view_mode = Some(m);
        }
        return true;
    }
    false
}

/// In the options of the view: where the view of each camera opens. The camera and the way picked, when one was.
pub fn mode_options(ui: &mut egui::Ui, view: &CameraView, cameras: &[CameraSource]) -> Option<(usize, StreamMode)> {
    if cameras.is_empty() {
        return None;
    }
    ui.add_space(8.0);
    ui.label(egui::RichText::new(tr("Camera view")).strong());
    let mut pick = None;
    egui::Grid::new("camera_modes").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        for (i, c) in cameras.iter().enumerate() {
            let now = StreamMode::of(view, Some(c));
            ui.label(&c.name);
            ui.horizontal(|ui| {
                for (m, icon) in MODE_ICONS {
                    if crate::widgets::toggle_icon(ui, icon, &m.tip(), now == m) {
                        pick = Some((i, m));
                    }
                }
            });
            ui.end_row();
        }
    });
    pick
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ways_keep_their_numbers_and_the_old_floating_window_is_a_side_tab() {
        for m in [StreamMode::Detached, StreamMode::Tab, StreamMode::Workspace] {
            assert_eq!(StreamMode::from_index(m.index()), m);
        }
        assert_eq!(StreamMode::from_index(0), StreamMode::Tab);
        let view = CameraView { mode: 3, ..Default::default() };
        let mut cam = CameraSource::url("c", "http://x", 0);
        assert_eq!(StreamMode::of(&view, Some(&cam)), StreamMode::Workspace, "the device's way");
        cam.view_mode = Some(1);
        assert_eq!(StreamMode::of(&view, Some(&cam)), StreamMode::Detached, "the camera's own way");
    }

    #[test]
    fn the_way_and_the_zoom_are_kept_with_the_device_and_the_camera() {
        let mut view = CameraView::default();
        let mut cams = vec![CameraSource::url("a", "http://a", 0), CameraSource::url("b", "http://b", 0)];
        assert!(keep_mode(&mut view, &mut cams, 1, StreamMode::Detached, 2.0));
        assert_eq!((view.mode, view.zoom, cams[1].view_mode, cams[0].view_mode), (1, 2.0, Some(1), None));
        assert!(!keep_mode(&mut view, &mut cams, 1, StreamMode::Detached, 2.0), "nothing changed");
    }

    #[test]
    fn the_picture_zooms_with_the_bar_and_cannot_leave_its_frame() {
        let ctx = egui::Context::default();
        let tex = ctx.load_texture("t", egui::ColorImage::new([40, 30], vec![egui::Color32::GRAY; 1200]), Default::default());
        let (mut zoom, mut pan, mut fit) = (1.0f32, Vec2::new(500.0, 0.0), FitWindow::default());
        let mut frame = |events: Vec<egui::Event>, zoom: &mut f32, pan: &mut Vec2, fit: &mut FitWindow| {
            let mut got = None;
            let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0))), events, ..Default::default() }, |ui| got = picture(ui, Some(&tex), zoom, pan, fit, 40.0));
            out.textures_delta.clear();
            got
        };
        let (rect, _) = frame(vec![], &mut zoom, &mut pan, &mut fit).expect("a picture");
        assert_eq!(pan, Vec2::ZERO, "at the whole picture there is no room to move");
        // The zoom-in button is the first one of the bar in the lower left corner of the picture.
        let at = rect.left_bottom() + egui::vec2(6.0 + 2.0 + 13.0, -36.0 + 16.0);
        frame(vec![egui::Event::PointerMoved(at)], &mut zoom, &mut pan, &mut fit);
        let press = |down| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
        frame(vec![press(true), press(false)], &mut zoom, &mut pan, &mut fit);
        assert!((zoom - crate::zoom_bar::STEP).abs() < 1e-6, "zoom {zoom}");
        assert!(frame(vec![], &mut zoom, &mut pan, &mut fit).is_some());
        let mut none = 1.0;
        let mut out = ctx.run_ui(Default::default(), |ui| assert!(picture(ui, None, &mut none, &mut pan, &mut fit, 40.0).is_none()));
        out.textures_delta.clear();
    }
}
