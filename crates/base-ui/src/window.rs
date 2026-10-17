//! The application draws its own window frame: no system title bar and no rounded corners. A flat bar on top
//! carries the icon, the title and the minimise / maximise / close buttons, and doubles as the drag handle.
//! Also the pop-up windows and the coloured buttons every window uses.
use crate::i18n::tr;
use crate::{icons, theme};
use egui::{Color32, CursorIcon, Id, Pos2, Rect, Sense, Stroke, ViewportCommand};

const BAR_H: f32 = 32.0;
const BTN_W: f32 = 46.0;
/// Width of the invisible strip along the window edge that starts a resize.
const EDGE: f32 = 5.0;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Glyph {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// One flat button of the title bar, drawn with a few lines so it needs no font or icon.
fn caption_button(ui: &mut egui::Ui, rect: Rect, id: &str, glyph: Glyph, tip: &str) -> bool {
    let r = ui.interact(rect, Id::new(id), Sense::click());
    let close = glyph == Glyph::Close;
    if r.hovered() {
        let fill = if close { Color32::from_rgb(0xe8, 0x11, 0x23) } else { theme::text().gamma_multiply(0.12) };
        ui.painter().rect_filled(rect, 0.0, fill);
    }
    let ink = if close && r.hovered() { Color32::WHITE } else { theme::text() };
    let st = Stroke::new(1.2, ink);
    let c = rect.center();
    let h = 5.0;
    let p = ui.painter();
    match glyph {
        Glyph::Minimize => {
            p.line_segment([Pos2::new(c.x - h, c.y), Pos2::new(c.x + h, c.y)], st);
        }
        Glyph::Maximize => {
            p.rect_stroke(Rect::from_center_size(c, egui::vec2(2.0 * h, 2.0 * h)), 0.0, st, egui::StrokeKind::Middle);
        }
        Glyph::Restore => {
            let back = Rect::from_center_size(c + egui::vec2(1.5, -1.5), egui::vec2(2.0 * h - 2.0, 2.0 * h - 2.0));
            let front = Rect::from_center_size(c + egui::vec2(-1.0, 1.0), egui::vec2(2.0 * h - 2.0, 2.0 * h - 2.0));
            p.rect_stroke(back, 0.0, st, egui::StrokeKind::Middle);
            p.rect_filled(front, 0.0, if close && r.hovered() { Color32::TRANSPARENT } else { theme::panel() });
            p.rect_stroke(front, 0.0, st, egui::StrokeKind::Middle);
        }
        Glyph::Close => {
            p.line_segment([Pos2::new(c.x - h, c.y - h), Pos2::new(c.x + h, c.y + h)], st);
            p.line_segment([Pos2::new(c.x - h, c.y + h), Pos2::new(c.x + h, c.y - h)], st);
        }
    }
    r.on_hover_text(tip).clicked()
}

/// The bar across the top of the window. Returns true when the close button was clicked.
pub fn title_bar(ui: &mut egui::Ui, title: &str, logo: Option<&egui::TextureHandle>) -> bool {
    let ctx = ui.ctx().clone();
    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
    let frame = egui::Frame::new().fill(theme::panel_dark()).stroke(Stroke::new(1.0, theme::border()));
    let mut close = false;
    egui::Panel::top("title_bar").frame(frame).exact_size(BAR_H).show(ui, |ui| {
        let rect = ui.max_rect();
        let drag = ui.interact(rect, Id::new("title_drag"), Sense::click_and_drag());
        if drag.double_clicked() {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        } else if drag.drag_started_by(egui::PointerButton::Primary) {
            ctx.send_viewport_cmd(ViewportCommand::StartDrag);
        }
        // Icon and title.
        let mut x = rect.left() + 10.0;
        if let Some(logo) = logo {
            let img = Rect::from_min_size(Pos2::new(x, rect.center().y - 9.0), egui::vec2(18.0, 18.0));
            ui.painter().image(logo.id(), img, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)), Color32::WHITE);
            x += 26.0;
        }
        ui.painter().text(Pos2::new(x, rect.center().y), egui::Align2::LEFT_CENTER, title, egui::FontId::proportional(13.0), theme::text());
        // Buttons on the right.
        let btn = |n: usize| Rect::from_min_size(Pos2::new(rect.right() - BTN_W * (n as f32 + 1.0), rect.top()), egui::vec2(BTN_W, rect.height()));
        if caption_button(ui, btn(0), "cap_close", Glyph::Close, &tr("Close")) {
            close = true;
        }
        let (g, tip) = if maximized { (Glyph::Restore, tr("Restore")) } else { (Glyph::Maximize, tr("Maximise")) };
        if caption_button(ui, btn(1), "cap_max", g, &tip) {
            ctx.send_viewport_cmd(ViewportCommand::Maximized(!maximized));
        }
        if caption_button(ui, btn(2), "cap_min", Glyph::Minimize, &tr("Minimise")) {
            ctx.send_viewport_cmd(ViewportCommand::Minimized(true));
        }
    });
    close
}

/// Resizing by the window edges, which the missing system frame no longer offers.
pub fn window_edges(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false) || i.viewport().fullscreen.unwrap_or(false)) {
        return;
    }
    let screen = ctx.content_rect();
    // Thin outline so the flat window stays visible against the desktop.
    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::new("window_outline")))
        .rect_stroke(screen, 0.0, Stroke::new(1.0, theme::border()), egui::StrokeKind::Inside);
    let Some(p) = ctx.input(|i| i.pointer.hover_pos()) else { return };
    let (l, r, t, b) = (p.x - screen.left() < EDGE, screen.right() - p.x < EDGE, p.y - screen.top() < EDGE, screen.bottom() - p.y < EDGE);
    use egui::ResizeDirection as D;
    let dir = match (l, r, t, b) {
        (true, _, true, _) => Some(D::NorthWest),
        (_, true, true, _) => Some(D::NorthEast),
        (true, _, _, true) => Some(D::SouthWest),
        (_, true, _, true) => Some(D::SouthEast),
        (true, ..) => Some(D::West),
        (_, true, ..) => Some(D::East),
        (_, _, true, _) => Some(D::North),
        (_, _, _, true) => Some(D::South),
        _ => None,
    };
    let Some(dir) = dir else { return };
    ctx.set_cursor_icon(match dir {
        D::North | D::South => CursorIcon::ResizeVertical,
        D::East | D::West => CursorIcon::ResizeHorizontal,
        D::NorthWest | D::SouthEast => CursorIcon::ResizeNwSe,
        D::NorthEast | D::SouthWest => CursorIcon::ResizeNeSw,
    });
    #[cfg(not(target_os = "macos"))]
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(ViewportCommand::BeginResize(dir));
    }
    // macOS cannot start a system resize on a frameless window: grow or shrink from the right and bottom
    // edges by following the pointer instead.
    #[cfg(target_os = "macos")]
    {
        let id = Id::new("edge_grab");
        if ctx.input(|i| i.pointer.primary_pressed()) && matches!(dir, D::East | D::South | D::SouthEast) {
            let grab = egui::vec2(screen.right() - p.x, screen.bottom() - p.y);
            ctx.data_mut(|d| d.insert_temp(id, (matches!(dir, D::East | D::SouthEast), matches!(dir, D::South | D::SouthEast), grab)));
        }
    }
}

/// Continue a resize started on the right or bottom edge (macOS).
#[cfg(target_os = "macos")]
pub fn window_edge_drag(ctx: &egui::Context) {
    let id = Id::new("edge_grab");
    let Some((east, south, grab)) = ctx.data(|d| d.get_temp::<(bool, bool, egui::Vec2)>(id)) else { return };
    let (down, pos) = ctx.input(|i| (i.pointer.primary_down(), i.pointer.latest_pos()));
    let (Some(p), true) = (pos, down) else {
        ctx.data_mut(|d| d.remove_temp::<(bool, bool, egui::Vec2)>(id));
        return;
    };
    let size = ctx.content_rect().size();
    let w = if east { p.x + grab.x } else { size.x };
    let h = if south { p.y + grab.y } else { size.y };
    ctx.send_viewport_cmd(ViewportCommand::InnerSize(egui::vec2(w.max(900.0), h.max(560.0))));
}

/// A slim title line for floating tool windows (preview, camera): small text and a small close button. `extra`
/// adds widgets next to the close button, right-aligned.
pub fn mini_title(ui: &mut egui::Ui, title: &str, open: &mut bool, extra: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.y = 0.0;
        ui.label(egui::RichText::new(title).size(13.0).strong().color(theme::text()));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), Sense::click());
            if resp.hovered() {
                ui.painter().rect_filled(rect, 3.0, theme::text().gamma_multiply(0.15));
            }
            icons::paint(ui, rect.shrink(3.0), "x", theme::text());
            if resp.on_hover_text(tr("Close")).clicked() {
                *open = false;
            }
            extra(ui);
        });
    });
    ui.separator();
}

/// The red used for everything that deletes.
pub const DELETE_RED: egui::Color32 = egui::Color32::from_rgb(0xc0, 0x30, 0x30);

/// A "Delete" button: a bin icon and the text, both red.
pub fn delete_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    delete_button_enabled(ui, true, text)
}

pub fn delete_button_enabled(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    let ink = if enabled { DELETE_RED } else { DELETE_RED.gamma_multiply(0.4) };
    ui.add_enabled(enabled, egui::Button::image_and_text(icons::slot(Some("trash"), ink), egui::RichText::new(text).color(ink)))
}

pub const SAVE_GREEN: egui::Color32 = egui::Color32::from_rgb(0x2e, 0xa0, 0x4f);
/// The icon of a new project on the tool bar: a lighter blue than the import buttons, to read on dark schemes too.
pub const NEW_BLUE: egui::Color32 = egui::Color32::from_rgb(0x3b, 0x93, 0xf5);
/// The icon of opening a project on the tool bar.
pub const OPEN_ORANGE: egui::Color32 = egui::Color32::from_rgb(0xf0, 0x8c, 0x1e);
pub const IMPORT_BLUE: egui::Color32 = egui::Color32::from_rgb(0x2f, 0x6f, 0xc0);

fn filled_button(ui: &mut egui::Ui, enabled: bool, text: egui::RichText, fill: egui::Color32) -> egui::Response {
    ui.add_enabled(enabled, egui::Button::new(text.color(egui::Color32::WHITE)).fill(fill))
}

/// A green button: everything that saves.
pub fn save_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    save_button_enabled(ui, true, text)
}

pub fn save_button_enabled(ui: &mut egui::Ui, enabled: bool, text: &str) -> egui::Response {
    filled_button(ui, enabled, egui::RichText::new(text), SAVE_GREEN)
}

/// A blue button: everything that imports or reads from a file.
pub fn import_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    filled_button(ui, true, egui::RichText::new(text), IMPORT_BLUE)
}

/// A small red button with only the bin icon, for lists; `tip` says what it deletes.
pub fn delete_icon(ui: &mut egui::Ui, tip: &str) -> egui::Response {
    ui.add(egui::Button::image(icons::slot(Some("trash"), egui::Color32::WHITE)).fill(DELETE_RED)).on_hover_text(tip)
}

/// The size a widget may take to fill what is left of a window or a panel: everything that is available, less the spacing
/// of the layout that follows every widget. A widget of the full `available_size` makes the window grow by that spacing on
/// every frame, up to the size of the screen.
pub fn fill_size(ui: &egui::Ui) -> egui::Vec2 {
    let s = ui.spacing().item_spacing;
    (ui.available_size_before_wrap() - egui::vec2(0.0, s.y)).max(egui::vec2(1.0, 1.0))
}

/// A pop-up window in the common style: slim title line, close button, movable and resizable both ways.
/// `size` is the starting size; `open` is cleared by the close button.
pub fn popup<R>(ctx: &egui::Context, id: &str, title: &str, size: [f32; 2], open: &mut bool, add: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    popup_with(ctx, id, title, size, open, |_| {}, add)
}

/// Like [`popup`], with icon buttons (or other small widgets) in the title line, next to the close button. They are added right
/// to left: the first one is the one at the right.
pub fn popup_with<R>(ctx: &egui::Context, id: &str, title: &str, size: [f32; 2], open: &mut bool, tools: impl FnOnce(&mut egui::Ui), add: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    if !*open {
        return None;
    }
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 8 });
    let pos = ctx.content_rect().center() - egui::vec2(size[0], size[1]) / 2.0;
    let mut keep = true;
    let r = egui::Window::new(id)
        .title_bar(false)
        .frame(frame)
        .collapsible(false)
        .default_size(size)
        .default_pos(pos)
        .resizable(true)
        .show(ctx, |ui| {
            mini_title(ui, title, &mut keep, tools);
            let r = add(ui);
            // Take the rest of the size the window was given, so it keeps a height taller than its content.
            ui.allocate_space(ui.available_size_before_wrap().max(egui::Vec2::ZERO));
            r
        });
    if !keep {
        *open = false;
    }
    r.and_then(|r| r.inner)
}

#[cfg(test)]
mod size_tests {
    use super::*;

    fn heights(content: impl Fn(&mut egui::Ui)) -> Vec<f32> {
        let ctx = egui::Context::default();
        let mut out = vec![];
        let mut open = true;
        for _ in 0..8 {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
            let mut o = ctx.run_ui(input, |ui| {
                popup(ui.ctx(), "t", "Test", [600.0, 400.0], &mut open, |ui| content(ui));
            });
            o.textures_delta.clear();
            if let Some(r) = ctx.memory(|m| m.area_rect(egui::Id::new(Some("t")))) {
                out.push(r.height());
            }
        }
        out
    }

    #[test]
    fn a_popup_keeps_its_size_with_little_in_it_or_with_a_widget_that_fills_it() {
        let small = heights(|ui| {
            ui.label("hello");
        });
        assert!((small[7] - small[2]).abs() < 1.0 && small[7] > 350.0, "{small:?}");
        let filling = heights(|ui| {
            let (r, _) = ui.allocate_exact_size(fill_size(ui), egui::Sense::hover());
            ui.painter().rect_filled(r, 0.0, egui::Color32::RED);
        });
        assert!((filling[7] - filling[2]).abs() < 1.0, "{filling:?}");
        let list = heights(|ui| {
            egui::ScrollArea::vertical().max_height(fill_size(ui).y).auto_shrink([false, false]).show(ui, |ui| {
                for i in 0..100 {
                    ui.label(format!("{i}"));
                }
            });
        });
        assert!((list[7] - list[2]).abs() < 1.0, "{list:?}");
    }
}
