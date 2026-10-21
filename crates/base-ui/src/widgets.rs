//! Small widgets of the interface: icon buttons, toggles for windows and the hints of a scrolled icon strip.
use crate::{icons, theme};
use egui::{Color32, RichText};

/// Double chevrons at the top and / or bottom edge of a scrolled icon strip when more icons lie beyond it.
pub fn scroll_hints<R>(ui: &egui::Ui, out: &egui::scroll_area::ScrollAreaOutput<R>) {
    let view = out.inner_rect;
    let (up, down) = (out.state.clamped_offset().y > 1.0, out.state.clamped_offset().y + view.height() < out.content_size.y - 1.0);
    for (show, name, top) in [(up, "chevrons-up", true), (down, "chevrons-down", false)] {
        if !show {
            continue;
        }
        let h = 16.0;
        let rect = if top {
            egui::Rect::from_min_size(view.left_top(), egui::vec2(view.width(), h))
        } else {
            egui::Rect::from_min_size(egui::pos2(view.left(), view.bottom() - h), egui::vec2(view.width(), h))
        };
        let painter = ui.painter().with_clip_rect(view);
        painter.rect_filled(rect, 0.0, theme::panel().gamma_multiply(0.92));
        icons::paint(ui, egui::Rect::from_center_size(rect.center(), egui::vec2(h - 2.0, h - 2.0)), name, theme::accent());
    }
}

/// Text button for a window that can be open or closed: light blue while it is open.
pub fn window_toggle(ui: &mut egui::Ui, text: &str, tip: &str, open: bool, enabled: bool) -> bool {
    let btn = if open {
        egui::Button::new(RichText::new(text).color(Color32::WHITE)).fill(Color32::from_rgb(0x3f, 0xa9, 0xf5))
    } else {
        egui::Button::new(text)
    };
    ui.add_enabled(enabled, btn).on_hover_text(tip).clicked()
}

/// An icon button that stays highlighted while `on`.
pub fn toggle_icon(ui: &mut egui::Ui, icon: &str, tip: &str, on: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(28.0, 28.0), egui::Sense::click());
    if on {
        ui.painter().rect_filled(rect, 6.0, theme::accent().gamma_multiply(0.3));
    } else if resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, theme::panel_dark());
    }
    icons::paint(ui, rect.shrink(5.6), icon, if on { theme::accent() } else { theme::text() });
    resp.on_hover_text(tip).clicked()
}

/// A square icon-only button with a tooltip; greyed out and inert when `on` is false.
pub fn icon_button(ui: &mut egui::Ui, icon: &str, tip: &str, on: bool, size: f32) -> bool {
    icon_button_ink(ui, icon, tip, on, size, theme::text())
}

pub fn icon_button_ink(ui: &mut egui::Ui, icon: &str, tip: &str, on: bool, size: f32, color: Color32) -> bool {
    let sense = if on { egui::Sense::click() } else { egui::Sense::hover() };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), sense);
    if on && resp.hovered() {
        ui.painter().rect_filled(rect, 6.0, theme::panel_dark());
    }
    let ink = if on { color } else { color.gamma_multiply(0.35) };
    icons::paint(ui, rect.shrink(size * 0.2), icon, ink);
    let resp = resp.on_hover_text(tip);
    ui.add_space(2.0);
    on && resp.clicked()
}


/// A small heading with an icon, above a group of controls (the sections of the Properties tab).
pub fn section_title(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.horizontal(|ui| {
        ui.add(icons::slot(Some(icon), theme::text_dim()));
        ui.label(RichText::new(title).strong().color(theme::text_dim()));
    });
    ui.add_space(2.0);
}

/// Buttons of one size in rows of three, as a table: (label, tooltip, icon). Returns the index of the clicked one.
pub fn button_grid(ui: &mut egui::Ui, id: &str, cells: &[(&str, &str, &str)]) -> Option<usize> {
    let mut clicked = None;
    let bw = (ui.available_width() - 12.0) / 3.0;
    egui::Grid::new(id).num_columns(3).spacing([6.0, 6.0]).show(ui, |ui| {
        for (i, (label, tip, icon)) in cells.iter().enumerate() {
            let btn = egui::Button::image_and_text(icons::slot(Some(icon), theme::text()), *label).truncate();
            if ui.add_sized([bw, 24.0], btn).on_hover_text(*tip).clicked() {
                clicked = Some(i);
            }
            if i % 3 == 2 {
                ui.end_row();
            }
        }
    });
    clicked
}

/// A row of widgets in the middle of the available width. The width of the row is measured while it is drawn and used
/// for the next frame (the first frame has the row at the left), so any widgets work, not only text buttons.
pub fn centered_row(ui: &mut egui::Ui, id_salt: &str, add: impl FnOnce(&mut egui::Ui)) {
    let id = ui.scope_id().with(("centered_row", id_salt));
    let known: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(0.0);
    let pad = if known > 0.0 { ((ui.available_width() - known) / 2.0).max(0.0) } else { 0.0 };
    let mut start = 0.0;
    let mut end = 0.0;
    ui.horizontal(|ui| {
        ui.add_space(pad);
        start = ui.cursor().min.x;
        add(ui);
        end = ui.min_rect().max.x;
    });
    let width = (end - start).max(0.0);
    if (width - known).abs() > 0.5 {
        ui.data_mut(|d| d.insert_temp(id, width));
        ui.ctx().request_repaint();
    }
}

#[cfg(test)]
mod centered_row_tests {
    use super::*;

    #[test]
    fn the_row_moves_to_the_middle_once_its_width_is_known() {
        let ctx = egui::Context::default();
        let mut left = vec![];
        for _ in 0..3 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    centered_row(ui, "t", |ui| {
                        let r = ui.button("A button");
                        left.push(r.rect.min.x);
                    });
                });
            });
            out.textures_delta.clear();
        }
        assert!(left[2] > left[0] + 10.0, "{left:?}");
        assert!((left[2] - left[1]).abs() < 1.0 || left[1] > left[0], "{left:?}");
    }
}

/// Which edge of a panel is the one that is dragged to resize it.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Edge {
    Left,
    Right,
    Top,
    Bottom,
}

/// The stroke of a line that can be dragged: thin and dim at rest, red and thicker under the pointer or while it is dragged.
pub fn drag_line_stroke(hot: bool) -> egui::Stroke {
    if hot {
        egui::Stroke::new(3.0, Color32::from_rgb(0xe0, 0x30, 0x30))
    } else {
        egui::Stroke::new(1.0, theme::border())
    }
}

/// Lights up the dragged edge of a resizable panel (red and thicker) while the pointer is on it or drags it, so it is
/// clear that the line moves. Call it with the rectangle of the panel after it was shown.
pub fn panel_edge_highlight(ctx: &egui::Context, panel: egui::Rect, edge: Edge) {
    let id = egui::Id::unique(("panel_edge_hot", edge as u8));
    let line = match edge {
        Edge::Left => [panel.left_top(), panel.left_bottom()],
        Edge::Right => [panel.right_top(), panel.right_bottom()],
        Edge::Top => [panel.left_top(), panel.right_top()],
        Edge::Bottom => [panel.left_bottom(), panel.right_bottom()],
    };
    let (pointer, down) = ctx.input(|i| (i.pointer.hover_pos().or(i.pointer.latest_pos()), i.pointer.primary_down()));
    let near = pointer.is_some_and(|p| {
        let (across, along) = match edge {
            Edge::Left | Edge::Right => ((p.x - line[0].x).abs(), p.y >= panel.top() && p.y <= panel.bottom()),
            Edge::Top | Edge::Bottom => ((p.y - line[0].y).abs(), p.x >= panel.left() && p.x <= panel.right()),
        };
        across <= 5.0 && along
    });
    let was: bool = ctx.data(|d| d.get_temp(id)).unwrap_or(false);
    // Once the drag has begun on the edge it stays lit until the button is released, wherever the pointer goes.
    let hot = near || (was && down);
    ctx.data_mut(|d| d.insert_temp(id, hot));
    if hot {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::unique("panel_edge_line")));
        painter.line_segment(line, drag_line_stroke(true));
    }
}

/// A horizontal line between two parts of a tab (a list above, its properties below) that is dragged up and down. The
/// height of the part above is kept in `height`; the line is red and thicker under the pointer.
pub fn horizontal_splitter(ui: &mut egui::Ui, id_salt: &str, height: &mut f32, min: f32, max: f32) {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 9.0), egui::Sense::drag());
    if resp.dragged() {
        *height = (*height + resp.drag_delta().y).clamp(min, max.max(min));
    }
    let hot = resp.hovered() || resp.dragged();
    if hot {
        ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeVertical);
    }
    let _ = id_salt;
    let y = rect.center().y;
    ui.painter().line_segment([egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)], drag_line_stroke(hot));
}

/// A scrolled list with a draggable line under it (the part below keeps the rest of the tab). The height of the list is
/// kept for the session; the line is red and thicker under the pointer.
pub fn list_with_splitter<R>(ui: &mut egui::Ui, id_salt: &str, default_height: f32, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    let id = ui.scope_id().with(("list_height", id_salt));
    let mut h: f32 = ui.data(|d| d.get_temp(id)).unwrap_or(default_height);
    let max = (ui.ctx().content_rect().height() - 260.0).max(120.0);
    h = h.clamp(60.0, max);
    let out = egui::ScrollArea::vertical().id_salt(id_salt).max_height(h).auto_shrink([false, true]).show(ui, add).inner;
    horizontal_splitter(ui, id_salt, &mut h, 60.0, max);
    ui.data_mut(|d| d.insert_temp(id, h));
    out
}

/// `h:mm:ss` for a time in seconds.
pub fn fmt_time(s: f64) -> String {
    let s = if s.is_finite() && s > 0.0 { s as u64 } else { 0 };
    format!("{}:{:02}:{:02}", s / 3600, s / 60 % 60, s % 60)
}
