//! The Console tab: a live view of everything that crosses the link to the machine, and a line to send a command.
//! The same in every program that talks to a machine.
use crate::i18n::tr;
use crate::link::{ConsoleLine, Dir};
use crate::theme;
use egui::{Color32, FontId, Key, Modifiers, RichText, Ui};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::ops::{Deref, DerefMut};
use std::time::{Duration, Instant};

/// How many lines are kept; when there are more, the oldest quarter goes.
const KEEP: usize = 4000;

/// How many commands of the user are remembered for the Up key.
const KEEP_COMMANDS: usize = 200;
/// A command that has not shown up in what was sent after this long never will (the job was stopped, the link was lost).
const AWAIT_LIMIT: Duration = Duration::from_secs(60);

/// How the console looks (*Settings → View options…*). A colour that is `None` follows the colour scheme.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ConsoleLook {
    pub font_size: f32,
    /// What the program sends.
    pub program: [u8; 3],
    /// What the user sends from the console.
    pub user: [u8; 3],
    /// What the machine answers.
    pub reply: [u8; 3],
    /// The messages of the program.
    pub message: Option<[u8; 3]>,
    pub background: Option<[u8; 3]>,
}

impl Default for ConsoleLook {
    fn default() -> ConsoleLook {
        ConsoleLook { font_size: 11.0, program: [0x3b, 0x82, 0xd8], user: [0xd9, 0x77, 0x06], reply: [0x2e, 0xa0, 0x4f], message: None, background: None }
    }
}

impl ConsoleLook {
    pub const FONT_SIZES: std::ops::RangeInclusive<f32> = 8.0..=24.0;

    fn colour(c: [u8; 3]) -> Color32 {
        Color32::from_rgb(c[0], c[1], c[2])
    }

    pub fn background_colour(&self) -> Color32 {
        self.background.map(Self::colour).unwrap_or_else(theme::console_bg)
    }

    pub fn message_colour(&self) -> Color32 {
        self.message.map(Self::colour).unwrap_or_else(theme::text_dim)
    }

    fn line(&self, dir: Dir) -> (&'static str, Color32) {
        match dir {
            Dir::Tx => ("> ", Self::colour(self.program)),
            Dir::User => ("> ", Self::colour(self.user)),
            Dir::Rx => ("< ", Self::colour(self.reply)),
            Dir::Info => ("# ", self.message_colour()),
        }
    }
}

/// The lines of the console and what the tab remembers: whether the status polls are shown, whether it follows the
/// newest line, the command being typed and the commands the user sent.
pub struct ConsoleState {
    lines: Vec<ConsoleLine>,
    pub polls: bool,
    pub autoscroll: bool,
    pub input: String,
    /// The commands the user sent, the oldest first; the Up key goes back through them.
    history: Vec<String>,
    /// The place in `history` the input was taken from.
    recalled: Option<usize>,
    /// What was typed before the Up key was pressed; the Down key brings it back.
    draft: String,
    /// What the user sent and the link has not reported yet, as it will be reported.
    awaited: VecDeque<(String, Instant)>,
}

impl Default for ConsoleState {
    fn default() -> ConsoleState {
        ConsoleState { lines: vec![], polls: false, autoscroll: true, input: String::new(), history: vec![], recalled: None, draft: String::new(), awaited: VecDeque::new() }
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
    /// Add a line; the console does not grow without end. A line that the user sent from the console gets the colour of the user.
    pub fn push_capped(&mut self, mut l: ConsoleLine) {
        self.awaited.retain(|(_, sent)| sent.elapsed() < AWAIT_LIMIT);
        if l.dir == Dir::Tx && !l.poll {
            if let Some(i) = self.awaited.iter().position(|(text, _)| *text == l.text) {
                self.awaited.remove(i);
                l.dir = Dir::User;
            }
        }
        self.lines.push(l);
        if self.lines.len() > KEEP {
            self.lines.drain(..KEEP / 4);
        }
    }

    /// Remember a command the user sends: for the Up key, and to tell it from what the program sends.
    fn remember(&mut self, command: &str) {
        if self.history.last().map(String::as_str) != Some(command) {
            self.history.push(command.to_string());
            if self.history.len() > KEEP_COMMANDS {
                self.history.remove(0);
            }
        }
        self.awaited.push_back((base_core::gcode_out::ascii(command), Instant::now()));
        self.recalled = None;
        self.draft.clear();
    }

    /// The Up key: the command before the one shown. Returns whether the input changed.
    fn recall_older(&mut self) -> bool {
        let at = match self.recalled {
            Some(0) => return false,
            Some(i) => i - 1,
            None if self.history.is_empty() => return false,
            None => {
                self.draft = self.input.clone();
                self.history.len() - 1
            }
        };
        self.recalled = Some(at);
        self.input = self.history[at].clone();
        true
    }

    /// The Down key: the command after the one shown, and at the end what was being typed. Returns whether the input changed.
    fn recall_newer(&mut self) -> bool {
        let Some(i) = self.recalled else { return false };
        if i + 1 < self.history.len() {
            self.recalled = Some(i + 1);
            self.input = self.history[i + 1].clone();
        } else {
            self.recalled = None;
            self.input = std::mem::take(&mut self.draft);
        }
        true
    }

    /// The tab. Returns the command to send when the user pressed Send (or Enter) and `can_send` is true.
    pub fn show(&mut self, ui: &mut Ui, can_send: bool, look: &ConsoleLook) -> Option<String> {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.polls, tr("Status polls"));
            ui.checkbox(&mut self.autoscroll, tr("Auto-scroll"));
            if ui.button(tr("Clear")).clicked() {
                self.lines.clear();
            }
        });
        let show_polls = self.polls;
        let rows: Vec<&ConsoleLine> = self.lines.iter().filter(|l| show_polls || !l.poll).collect();
        let size = look.font_size.clamp(*ConsoleLook::FONT_SIZES.start(), *ConsoleLook::FONT_SIZES.end());
        let row_h = ui.fonts_mut(|f| f.row_height(&FontId::monospace(size)));
        let input_h = 34.0;
        let h = (ui.available_height() - input_h).max(80.0);
        egui::Frame::new().fill(look.background_colour()).stroke(egui::Stroke::new(1.0, theme::border())).inner_margin(egui::Margin::same(4)).show(ui, |ui| {
            egui::ScrollArea::vertical().id_salt("console").max_height(h - 10.0).auto_shrink([false, false]).stick_to_bottom(self.autoscroll).show_rows(ui, row_h, rows.len(), |ui, range| {
                for l in &rows[range] {
                    let (prefix, color) = look.line(l.dir);
                    ui.label(RichText::new(format!("{prefix}{}", l.text)).monospace().size(size).color(color));
                }
            });
        });
        let mut out = None;
        ui.horizontal(|ui| {
            let id = egui::Id::unique("console_input");
            // Taken before the field sees them, or it would move its cursor.
            let mut recalled = false;
            if ui.memory(|m| m.has_focus(id)) {
                if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowUp)) {
                    recalled = self.recall_older();
                } else if ui.input_mut(|i| i.consume_key(Modifiers::NONE, Key::ArrowDown)) {
                    recalled = self.recall_newer();
                }
            }
            let r = ui.add(egui::TextEdit::singleline(&mut self.input).id(id).desired_width((ui.available_width() - 110.0).max(60.0)).hint_text(tr("send G-code / $ command")));
            if recalled {
                let mut state = egui::TextEdit::load_state(ui.ctx(), id).unwrap_or_default();
                let end = egui::text::CCursor::new(self.input.chars().count());
                state.cursor.set_char_range(Some(egui::text::CCursorRange::one(end)));
                state.store(ui.ctx(), id);
            }
            let send = ui.button(tr("Send")).clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)));
            if send && can_send && !self.input.trim().is_empty() {
                let command = self.input.trim().to_string();
                self.remember(&command);
                out = Some(command);
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
                sent = c.show(ui, false, &ConsoleLook::default());
            });
        });
        out.textures_delta.clear();
        assert!(sent.is_none() && c.input == "$H", "nothing is sent without a link");
    }

    fn tx(text: &str, poll: bool) -> ConsoleLine {
        ConsoleLine { dir: Dir::Tx, text: text.into(), poll }
    }

    #[test]
    fn the_keys_go_through_the_commands_of_the_user_only() {
        let mut c = ConsoleState::default();
        assert!(!c.recall_older(), "nothing to go back to");
        c.push_capped(tx("G0 X1", false));
        assert!(!c.recall_older(), "what the program sent is not in the list");
        c.remember("$H");
        c.remember("G0 X5");
        c.remember("G0 X5");
        c.input = "half typed".into();
        assert!(c.recall_older());
        assert_eq!(c.input, "G0 X5", "the newest first, a repeat only once");
        assert!(c.recall_older());
        assert_eq!(c.input, "$H");
        assert!(!c.recall_older(), "the oldest stays");
        assert_eq!(c.input, "$H");
        assert!(c.recall_newer());
        assert_eq!(c.input, "G0 X5");
        assert!(c.recall_newer());
        assert_eq!(c.input, "half typed", "what was being typed comes back");
        assert!(!c.recall_newer());
    }

    #[test]
    fn a_line_of_the_user_is_told_from_the_lines_of_the_program() {
        let mut c = ConsoleState::default();
        c.remember("G0 X1");
        c.remember("(żółw)");
        c.push_capped(tx("G1 X2", false));
        c.push_capped(tx("G0 X1", false));
        c.push_capped(tx("G0 X1", false));
        c.push_capped(tx(&base_core::gcode_out::ascii("(żółw)"), false));
        c.push_capped(tx("?", true));
        let dirs: Vec<Dir> = c.iter().map(|l| l.dir).collect();
        assert_eq!(dirs, [Dir::Tx, Dir::User, Dir::Tx, Dir::User, Dir::Tx], "only once for each command, and as the link writes it");
        // A command that was never sent does not colour a later line of the program.
        c.remember("M3");
        c.awaited[0].1 = Instant::now() - AWAIT_LIMIT - Duration::from_secs(1);
        c.push_capped(tx("M3", false));
        assert_eq!(c.last().unwrap().dir, Dir::Tx);
    }

    #[test]
    fn the_arrow_keys_work_in_the_field_and_leave_the_cursor_at_the_end() {
        let mut c = ConsoleState::default();
        c.remember("$H");
        c.remember("G0 X5");
        let ctx = egui::Context::default();
        let id = egui::Id::unique("console_input");
        let frame = |c: &mut ConsoleState, key: Option<Key>| {
            let mut raw = egui::RawInput::default();
            if let Some(key) = key {
                raw.events.push(egui::Event::Key { key, physical_key: None, pressed: true, repeat: false, modifiers: Modifiers::NONE });
            }
            let mut out = ctx.run_ui(raw, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    c.show(ui, true, &ConsoleLook::default());
                });
            });
            out.textures_delta.clear();
        };
        frame(&mut c, None);
        frame(&mut c, Some(Key::ArrowUp));
        assert_eq!(c.input, "", "without the focus the key does nothing");
        ctx.memory_mut(|m| m.request_focus(id));
        frame(&mut c, None);
        frame(&mut c, Some(Key::ArrowUp));
        assert_eq!(c.input, "G0 X5");
        frame(&mut c, Some(Key::ArrowUp));
        assert_eq!(c.input, "$H");
        let at = egui::TextEdit::load_state(&ctx, id).and_then(|s| s.cursor.char_range()).map(|r| r.primary.index.0);
        assert_eq!(at, Some(2));
        frame(&mut c, Some(Key::ArrowDown));
        assert_eq!(c.input, "G0 X5");
        assert!(ctx.memory(|m| m.has_focus(id)), "the field keeps the focus");
    }

    #[test]
    fn an_old_settings_file_gets_the_look_of_before() {
        let look: ConsoleLook = serde_json::from_slice(b"{}").unwrap();
        assert_eq!(look, ConsoleLook::default());
        assert_eq!(look.font_size, 11.0);
        assert_eq!(look.program, [0x3b, 0x82, 0xd8]);
        assert_ne!(look.user, look.program);
    }
}
