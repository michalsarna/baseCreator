//! The controls of an animated preview (a toolpath drawn as the machine would do it): back to the start, play and
//! pause, a slider for the progress, the speed and repeat. The same controls are used by every program.
use crate::i18n::tr;
use crate::widgets::{icon_button, toggle_icon};
use egui::{Slider, Ui};

/// The speeds of the animation, as multiples of the basic one.
pub const SPEEDS: [f32; 7] = [0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 25.0];
/// How long the whole job takes to draw at the basic speed (seconds).
pub const BASIC_SECONDS: f32 = 20.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Playback {
    pub playing: bool,
    /// Index in `SPEEDS`.
    pub speed: usize,
    pub repeat: bool,
}

impl Default for Playback {
    fn default() -> Playback {
        Playback { playing: false, speed: 2, repeat: false }
    }
}

impl Playback {
    /// Move `progress` (0 to 1) on by `dt` seconds. Returns true when it moved.
    pub fn step(&mut self, progress: &mut f32, dt: f32) -> bool {
        if !self.playing {
            return false;
        }
        if *progress >= 1.0 {
            if self.repeat {
                *progress = 0.0;
            } else {
                self.playing = false;
                return false;
            }
        }
        *progress += dt * SPEEDS[self.speed.min(SPEEDS.len() - 1)] / BASIC_SECONDS;
        if *progress >= 1.0 {
            *progress = 1.0;
            if !self.repeat {
                self.playing = false;
            }
        }
        true
    }

    /// The row of controls; also advances the animation while it plays. Returns true when the progress changed.
    pub fn show(&mut self, ui: &mut Ui, progress: &mut f32) -> bool {
        let before = *progress;
        if icon_button(ui, "rotate-ccw", &tr("Back to the start"), true, 22.0) {
            *progress = 0.0;
        }
        let (icon, tip) = if self.playing { ("pause", tr("Pause")) } else { ("play", tr("Play")) };
        if icon_button(ui, icon, &tip, true, 22.0) {
            if !self.playing && *progress >= 1.0 {
                *progress = 0.0;
            }
            self.playing = !self.playing;
        }
        ui.spacing_mut().slider_width = 260.0;
        ui.add(Slider::new(progress, 0.0..=1.0).show_value(false));
        egui::ComboBox::from_id_salt("playback_speed")
            .width(60.0)
            .selected_text(format!("{}×", SPEEDS[self.speed.min(SPEEDS.len() - 1)]))
            .show_ui(ui, |ui| {
                for (i, s) in SPEEDS.iter().enumerate() {
                    ui.selectable_value(&mut self.speed, i, format!("{s}×"));
                }
            })
            .response
            .on_hover_text(tr("Animation speed"));
        if toggle_icon(ui, "repeat", &tr("Repeat"), self.repeat) {
            self.repeat = !self.repeat;
        }
        let dt = ui.input(|i| i.stable_dt).min(0.1);
        if self.step(progress, dt) {
            ui.ctx().request_repaint();
        }
        (*progress - before).abs() > f32::EPSILON
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_icons_of_the_controls_exist() {
        for name in ["rotate-ccw", "play", "pause", "repeat"] {
            assert!(crate::icons::image(name, 16.0, egui::Color32::WHITE).is_some(), "{name}");
        }
    }

    /// The play button really is clickable and the animation moves while the frames come.
    #[test]
    fn clicking_play_starts_the_animation_and_it_moves() {
        let ctx = egui::Context::default();
        let mut pb = Playback::default();
        let mut progress = 1.0f32;
        let mut start = egui::Pos2::ZERO;
        let mut t = 0.0;
        let mut frame = |ctx: &egui::Context, events: Vec<egui::Event>, pb: &mut Playback, progress: &mut f32, start: &mut egui::Pos2| {
            t += 0.05;
            let input = egui::RawInput { events, time: Some(t), predicted_dt: 0.05, ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        *start = ui.cursor().min;
                        pb.show(ui, progress);
                    });
                });
            });
            out.textures_delta.clear();
        };
        frame(&ctx, vec![], &mut pb, &mut progress, &mut start);
        // The second button: the first is 22 wide with a little space after it.
        let play = start + egui::vec2(22.0 + 2.0 + ctx.global_style().spacing.item_spacing.x + 11.0, 11.0);
        let click = |pressed| vec![egui::Event::PointerMoved(play), egui::Event::PointerButton { pos: play, button: egui::PointerButton::Primary, pressed, modifiers: Default::default() }];
        frame(&ctx, click(true), &mut pb, &mut progress, &mut start);
        frame(&ctx, click(false), &mut pb, &mut progress, &mut start);
        assert!(pb.playing, "the click on play started it");
        let after_click = progress;
        assert!(after_click < 1.0, "it started from the beginning: {after_click}");
        for _ in 0..5 {
            frame(&ctx, vec![], &mut pb, &mut progress, &mut start);
        }
        assert!(progress > after_click, "it moves: {progress} after {after_click}");
    }

    #[test]
    fn it_runs_to_the_end_and_stops_or_repeats() {
        let mut p = Playback { playing: true, speed: 2, repeat: false };
        let mut x = 0.0;
        assert!(p.step(&mut x, BASIC_SECONDS / 2.0));
        assert!((x - 0.5).abs() < 1e-6);
        p.step(&mut x, BASIC_SECONDS);
        assert!(x == 1.0 && !p.playing);
        assert!(!p.step(&mut x, 1.0), "stopped");
        let mut r = Playback { playing: true, speed: 2, repeat: true };
        let mut y = 1.0;
        r.step(&mut y, 1.0);
        assert!(r.playing && y < 0.2, "starts again from the beginning: {y}");
        let mut fast = Playback { playing: true, speed: 6, repeat: false };
        let mut z = 0.0;
        fast.step(&mut z, 0.5);
        assert!((z - 12.5 / BASIC_SECONDS).abs() < 1e-6);
    }
}
