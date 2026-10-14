//! The Console tab: a live view of everything that crosses the link to the machine, and a line to send a command.
//! The same in every program that talks to a machine.
use crate::i18n::tr;
use crate::link::{ConsoleLine, Dir};
use crate::theme;
use egui::{Color32, RichText, Ui};
use std::ops::{Deref, DerefMut};

/// How many lines are kept; when there are more, the oldest quarter goes.
const KEEP: usize = 4000;

/// The lines of the console and what the tab remembers: whether the status polls are shown, whether it follows the
/// newest line, and the command being typed.
pub struct ConsoleState {
    lines: Vec<ConsoleLine>,
    pub polls: bool,
    pub autoscroll: bool,
    pub input: String,
}

impl Default for ConsoleState {
    fn default() -> ConsoleState {
        ConsoleState { lines: vec![], polls: false, autoscroll: true, input: String::new() }
    }
}

impl Deref for ConsoleState {
    type Target = Vec<ConsoleLine>;
    fn deref(&self) -> &Vec<ConsoleLine> {
        &self.lines
    }
}

impl DerefMut for ConsoleState {
    fn deref_mut(&mut self) -> &mut Vec<ConsoleLine> {
        &mut self.lines
    }
}

impl ConsoleState {
    /// Add a line; the console does not grow without end.
    pub fn push_capped(&mut self, l: ConsoleLine) {
        self.lines.push(l);
        if self.lines.len() > KEEP {
            self.lines.drain(..KEEP / 4);
        }
    }

    /// The tab. Returns the command to send when the user pressed Send (or Enter) and `can_send` is true.
    pub fn show(&mut self, ui: &mut Ui, can_send: bool) -> Option<String> {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.polls, tr("Status polls"));
            ui.checkbox(&mut self.autoscroll, tr("Auto-scroll"));
            if ui.button(tr("Clear")).clicked() {
                self.lines.clear();
            }
        });
        let accent_tx = Color32::from_rgb(0x3b, 0x82, 0xd8);
        let accent_rx = Color32::from_rgb(0x2e, 0xa0, 0x4f);
        let show_polls = self.polls;
        let rows: Vec<&ConsoleLine> = self.lines.iter().filter(|l| show_polls || !l.poll).collect();
        let row_h = 14.0;
        let input_h = 34.0;
        let h = (ui.available_height() - input_h).max(80.0);
        egui::Frame::new().fill(theme::console_bg()).stroke(egui::Stroke::new(1.0, theme::border())).inner_margin(egui::Margin::same(4)).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("console").max_height(h - 10.0).auto_shrink([false, false]).stick_to_bottom(self.autoscroll).show_rows(ui, row_h, rows.len(), |ui, range| {
                for l in &rows[range] {
                    let (prefix, color) = match l.dir {
                        Dir::Tx => ("> ", accent_tx),
                        Dir::Rx => ("< ", accent_rx),
                        Dir::Info => ("# ", theme::text_dim()),
                    };
                    ui.label(RichText::new(format!("{prefix}{}", l.text)).monospace().size(11.0).color(color));
                }
            });
        });
        let mut out = None;
        ui.horizontal(|ui| {
            let r = ui.add(egui::TextEdit::singleline(&mut self.input).desired_width((ui.available_width() - 110.0).max(60.0)).hint_text(tr("send G-code / $ command")));
            let send = ui.button(tr("Send")).clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if send && can_send && !self.input.trim().is_empty() {
                out = Some(self.input.trim().to_string());
                self.input.clear();
                r.request_focus();
            }
        });
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_keeps_the_newest_lines_and_sends_what_was_typed() {
        let mut c = ConsoleState::default();
        for i in 0..(KEEP + 10) {
            c.push_capped(ConsoleLine::info(format!("line {i}")));
        }
        assert!(c.len() <= KEEP && c.len() > KEEP / 2);
        assert_eq!(c.last().unwrap().text, format!("line {}", KEEP + 9));
        c.input = "$H".into();
        let ctx = egui::Context::default();
        let mut sent = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                sent = c.show(ui, false);
            });
        });
        out.textures_delta.clear();
        assert!(sent.is_none() && c.input == "$H", "nothing is sent without a link");
    }
}
