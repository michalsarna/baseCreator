//! The zoom buttons of every picture that can be zoomed (the import, trace and preview windows, the G-code viewer, the camera):
//! zoom in, zoom out and the whole picture, with the zoom, on a small bar in the lower left corner of the picture.
use crate::i18n::tr;
use crate::theme;
use egui::RichText;

/// What a button of the bar asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomCmd {
    In,
    Out,
    /// Back to the whole picture (the first view).
    Whole,
    /// Fit the view to all the objects (where the picture has objects: the previews).
    FitObjects,
}

/// How much one press of a button zooms.
pub const STEP: f32 = 1.5;

/// The bar over the lower left corner of `frame` (the picture). `zoom` is shown as `×1.0` (1 = the whole picture); `hint` says
/// what the mouse does in this picture and shows when the pointer is over the bar.
pub fn overlay(ui: &mut egui::Ui, frame: egui::Rect, zoom: f32, hint: &str) -> Option<ZoomCmd> {
    overlay_with(ui, frame, zoom, hint, false)
}

/// The bar with a fourth button, **Fit all objects**, for a picture of objects (the previews of a job).
pub fn overlay_with(ui: &mut egui::Ui, frame: egui::Rect, zoom: f32, hint: &str, fit_objects: bool) -> Option<ZoomCmd> {
    let bar = egui::Rect::from_min_size(frame.left_bottom() + egui::vec2(6.0, -36.0), egui::vec2((frame.width() - 12.0).max(10.0), 32.0));
    let mut bar_ui = ui.new_child(egui::UiBuilder::new().max_rect(bar).layout(egui::Layout::left_to_right(egui::Align::Center)));
    bar_ui.set_clip_rect(frame.intersect(ui.clip_rect()));
    let mut cmd = None;
    let fill = ui.visuals().window_fill();
    egui::Frame::new().fill(fill).corner_radius(4.0).inner_margin(2.0).show(&mut bar_ui, |ui| {
        ui.horizontal(|ui| {
            for (c, icon, tip) in [(ZoomCmd::In, "zoom-in", "Zoom in"), (ZoomCmd::Out, "zoom-out", "Zoom out"), (ZoomCmd::Whole, "scan", "Reset view"), (ZoomCmd::FitObjects, "expand", "Fit all objects")] {
                if c == ZoomCmd::FitObjects && !fit_objects {
                    continue;
                }
                if crate::widgets::icon_button(ui, icon, &tr(tip), true, 26.0) {
                    cmd = Some(c);
                }
            }
            ui.label(RichText::new(format!("×{:.1}", zoom)).color(theme::text_dim()));
        })
        .response
        .on_hover_text(hint);
    });
    cmd
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three buttons lie in the lower left corner of the picture, in this order, and each gives its command.
    #[test]
    fn the_bar_sits_in_the_lower_left_corner_and_its_buttons_answer() {
        let ctx = egui::Context::default();
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        let frame = egui::Rect::from_min_max(egui::pos2(100.0, 50.0), egui::pos2(700.0, 550.0));
        let mut run = |events: Vec<egui::Event>| {
            let mut got = None;
            let mut out = ctx.run_ui(egui::RawInput { screen_rect: Some(screen), events, ..Default::default() }, |ui| got = overlay(ui, frame, 2.0, "hint"));
            out.textures_delta.clear();
            got
        };
        run(vec![]);
        let mut buttons: Vec<egui::Rect> = ctx.viewport(|v| v.prev_pass.widgets.layers().flat_map(|(_, w)| w.iter().filter(|w| w.sense.senses_click() && (w.rect.width() - w.rect.height()).abs() < 1.0).map(|w| w.rect)).collect());
        buttons.sort_by(|a, b| a.left().total_cmp(&b.left()));
        assert_eq!(buttons.len(), 3, "{buttons:?}");
        assert!(buttons.iter().all(|b| frame.contains_rect(*b) && b.left() < frame.left() + 120.0 && b.bottom() > frame.bottom() - 40.0), "{buttons:?}");
        for (b, want) in buttons.iter().zip([ZoomCmd::In, ZoomCmd::Out, ZoomCmd::Whole]) {
            let at = b.center();
            run(vec![egui::Event::PointerMoved(at)]);
            let press = |down| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
            assert_eq!(run(vec![press(true), press(false)]), Some(want));
        }
    }
}

#[cfg(test)]
mod fit_tests {
    use super::*;

    /// A preview's bar has a fourth button that fits all the objects.
    #[test]
    fn the_bar_of_a_preview_also_fits_all_the_objects() {
        let ctx = egui::Context::default();
        let frame = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(600.0, 400.0));
        let mut run = |events: Vec<egui::Event>| {
            let mut got = None;
            let mut out = ctx.run_ui(egui::RawInput { events, ..Default::default() }, |ui| got = overlay_with(ui, frame, 1.0, "hint", true));
            out.textures_delta.clear();
            got
        };
        run(vec![]);
        let mut buttons: Vec<egui::Rect> = ctx.viewport(|v| v.prev_pass.widgets.layers().flat_map(|(_, w)| w.iter().filter(|w| w.sense.senses_click() && (w.rect.width() - w.rect.height()).abs() < 1.0).map(|w| w.rect)).collect());
        buttons.sort_by(|a, b| a.left().total_cmp(&b.left()));
        assert_eq!(buttons.len(), 4);
        let at = buttons[3].center();
        run(vec![egui::Event::PointerMoved(at)]);
        let press = |down| egui::Event::PointerButton { pos: at, button: egui::PointerButton::Primary, pressed: down, modifiers: Default::default() };
        assert_eq!(run(vec![press(true), press(false)]), Some(ZoomCmd::FitObjects));
    }
}
