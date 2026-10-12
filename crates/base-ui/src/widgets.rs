//! Small widgets of the interface: icon buttons, toggles for windows and the hints of a scrolled icon strip.
use crate::{icons, theme};
use egui::{Color32, RichText};

/// Double chevrons at the top and / or bottom edge of a scrolled icon strip when more icons lie beyond it.
pub fn scroll_hints<R>(ui: &egui::Ui, out: &egui::scroll_area::ScrollAreaOutput<R>) {
    let view = out.inner_rect;
    let (up, down) = (out.state.offset.y > 1.0, out.state.offset.y + view.height() < out.content_size.y - 1.0);
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

