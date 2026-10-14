//! The controls of an animated preview (a toolpath drawn as the machine would do it): back to the start, play and
//! pause, a slider for the progress, the speed and repeat. The same controls are used by every program.
use crate::i18n::tr;
use crate::widgets::icon_button;
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
        if icon_button(ui, "rotate-ccw", &tr("Back to the start"), false, 22.0) {
            *progress = 0.0;
        }
        let (icon, tip) = if self.playing { ("pause", tr("Pause")) } else { ("play", tr("Play")) };
        if icon_button(ui, icon, &tip, self.playing, 22.0) {
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
        if icon_button(ui, "repeat", &tr("Repeat"), self.repeat, 22.0) {
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
