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
    let r = ui.interact(rect, Id::unique(id), Sense::click());
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
        let drag = ui.interact(rect, Id::unique("title_drag"), Sense::click_and_drag());
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
    ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, Id::unique("window_outline")))
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
        let id = Id::unique("edge_grab");
        if ctx.input(|i| i.pointer.primary_pressed()) && matches!(dir, D::East | D::South | D::SouthEast) {
            let grab = egui::vec2(screen.right() - p.x, screen.bottom() - p.y);
            ctx.data_mut(|d| d.insert_temp(id, (matches!(dir, D::East | D::SouthEast), matches!(dir, D::South | D::SouthEast), grab)));
        }
    }
}

/// Continue a resize started on the right or bottom edge (macOS).
#[cfg(target_os = "macos")]
pub fn window_edge_drag(ctx: &egui::Context) {
    let id = Id::unique("edge_grab");
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

/// Is the pointer over a window, a menu or another layer above the panels (and no button held)? The edge of a resizable panel
/// must not light up and take the drag then: the person is pointing at the window that covers it.
pub fn pointer_over_window(ctx: &egui::Context) -> bool {
    if ctx.input(|i| i.pointer.any_down()) {
        return false;
    }
    ctx.pointer_hover_pos().and_then(|p| ctx.layer_id_at(p)).is_some_and(|l| l.order != egui::Order::Background)
}

/// Like [`popup`], with icon buttons (or other small widgets) in the title line, next to the close button. They are added right
/// to left: the first one is the one at the right.
pub fn popup_with<R>(ctx: &egui::Context, id: &str, title: &str, size: [f32; 2], open: &mut bool, tools: impl FnOnce(&mut egui::Ui), add: impl FnOnce(&mut egui::Ui) -> R) -> Option<R> {
    popup_at(ctx, id, title, size, None, open, tools, add).map(|(r, _)| r)
}

/// [`popup_with`] that opens at `pos` (its top-left corner) when it is given, and also returns the rectangle of the window.
pub fn popup_at<R>(ctx: &egui::Context, id: &str, title: &str, size: [f32; 2], pos: Option<Pos2>, open: &mut bool, tools: impl FnOnce(&mut egui::Ui), add: impl FnOnce(&mut egui::Ui) -> R) -> Option<(R, Rect)> {
    if !*open {
        return None;
    }
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin { left: 10, right: 10, top: 6, bottom: 8 });
    let pos = pos.unwrap_or_else(|| ctx.content_rect().center() - egui::vec2(size[0], size[1]) / 2.0);
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
    r.and_then(|r| r.inner.map(|i| (i, r.response.rect)))
}

/// A window that shrinks to the picture it shows: the picture is fitted to the window, so a window of another shape than the
/// picture has an empty strip beside it or under it. The drawing code of the picture reports that strip in `slack`; once
/// the mouse is let go the window is made smaller by it. (A window of egui cannot be resized from code: it is opened again
/// under a new id, at the same place, with the new size.)
#[derive(Clone, Debug, Default)]
pub struct FitWindow {
    generation: u32,
    pos: Option<Pos2>,
    size: Option<[f32; 2]>,
    /// The empty width and height beside and under the picture, set while it is drawn.
    pub slack: egui::Vec2,
}

/// An empty strip thinner than this is left alone (rounding, the border of the picture).
const SLACK_MIN: f32 = 3.0;
/// The smallest window the fitting makes.
const FIT_MIN: [f32; 2] = [240.0, 160.0];

impl FitWindow {
    /// The id of the window: the same as long as it keeps its size.
    pub fn id(&self, base: &str) -> String {
        format!("{base}_{}", self.generation)
    }

    /// Where the window opens, if it was moved by a fitting.
    pub fn pos(&self) -> Option<Pos2> {
        self.pos
    }

    /// The size the window opens with: the fitted one, else `default`.
    pub fn size_or(&self, default: [f32; 2]) -> [f32; 2] {
        self.size.unwrap_or(default)
    }

    fn wanted(&self, ctx: &egui::Context, size: egui::Vec2) -> Option<egui::Vec2> {
        if ctx.input(|i| i.pointer.any_down()) || (self.slack.x < SLACK_MIN && self.slack.y < SLACK_MIN) {
            return None;
        }
        let (dx, dy) = (self.slack.x.max(0.0), self.slack.y.max(0.0));
        let w = egui::vec2((size.x - dx).max(FIT_MIN[0]), (size.y - dy).max(FIT_MIN[1]));
        ((w - size).abs().max_elem() >= SLACK_MIN).then_some(w)
    }

    /// Call after the window was shown, with its rectangle: when it is bigger than the picture needs, it is opened again
    /// smaller at the next frame.
    pub fn settle(&mut self, ctx: &egui::Context, rect: Rect) {
        if let Some(w) = self.wanted(ctx, rect.size()) {
            self.generation += 1;
            self.pos = Some(rect.min);
            self.size = Some([w.x, w.y]);
            self.slack = egui::Vec2::ZERO;
            ctx.request_repaint();
        }
    }

    /// For a window of its own (a viewport): the inner size it should take now, once the mouse is let go.
    pub fn viewport_size(&mut self, ctx: &egui::Context, inner: egui::Vec2) -> Option<egui::Vec2> {
        let w = self.wanted(ctx, inner)?;
        self.slack = egui::Vec2::ZERO;
        Some(w)
    }
}

/// A pop-up window of the common style that shrinks to its picture (see [`FitWindow`]).
pub fn popup_fit<R>(ctx: &egui::Context, id: &str, title: &str, size: [f32; 2], open: &mut bool, fit: &mut FitWindow, add: impl FnOnce(&mut egui::Ui, &mut FitWindow) -> R) -> Option<R> {
    let (id, size, pos) = (fit.id(id), fit.size_or(size), fit.pos());
    let r = popup_at(ctx, &id, title, size, pos, open, |_| {}, |ui| add(ui, fit));
    r.map(|(r, rect)| {
        fit.settle(ctx, rect);
        r
    })
}

/// The scale at which a picture of `size` fills what is left of the window (up as well as down), leaving `under` points of
/// height for what stands under it, and the empty width and height that remain beside and under it.
pub fn fit_picture(ui: &egui::Ui, size: egui::Vec2, under: f32) -> (f32, egui::Vec2) {
    let room = egui::vec2(ui.available_width().max(60.0), (ui.available_height() - under).max(60.0));
    let k = (room.x / size.x).min(room.y / size.y);
    (k, room - size * k)
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
            if let Some(r) = ctx.memory(|m| m.area_rect(egui::Id::unique(Some("t")))) {
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

    #[test]
    fn a_fit_window_shrinks_to_its_picture_and_then_stays() {
        let ctx = egui::Context::default();
        let mut fit = FitWindow::default();
        let mut open = true;
        let mut frame = |fit: &mut FitWindow| {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1200.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                popup_fit(ui.ctx(), "pic", "Test", [900.0, 700.0], &mut open, fit, |ui, fit| {
                    let size = egui::vec2(320.0, 180.0);
                    let (k, slack) = fit_picture(ui, size, 4.0);
                    fit.slack = slack;
                    ui.allocate_exact_size(size * k, egui::Sense::hover());
                });
            });
            out.textures_delta.clear();
        };
        for _ in 0..10 {
            frame(&mut fit);
        }
        assert!(fit.generation >= 1, "the window was never fitted");
        // The picture is 16:9: the window is no longer the 900 x 700 it was opened with.
        let last = fit.size.expect("a fitted size");
        assert!(last[1] < 600.0 && last[0] / last[1] < 2.2, "{last:?}");
        let g = fit.generation;
        for _ in 0..6 {
            frame(&mut fit);
        }
        assert!(fit.generation <= g + 1, "the window keeps being resized: {g} -> {}", fit.generation);
    }
}
