//! The G-code of a project as text: made for the project (with its postprocessors) and shown in a tab, coloured, and open to edits. The
//! person can change it by hand, write it to a file, make it again from the project or look at its toolpaths in the analysis of
//! G-code. Closing the tab with edits asks whether to save them.
use crate::i18n::tr;
use crate::theme;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, RichText};
use std::path::PathBuf;

/// The G-code of one project.
pub struct GcodeText {
    /// The slot of the project it was made for.
    pub slot: usize,
    /// The name of the file it would be saved as, and its title in the tab.
    pub name: String,
    pub text: String,
    /// What was made (or last saved): the text is changed when it differs.
    original: String,
    /// Asked to make it again while there are edits: the second click confirms.
    confirm_again: bool,
    /// Where it was last written.
    pub saved_as: Option<PathBuf>,
    /// What the project looked like (its G-code settings, notes, postprocessors, saves) when the text was made: the program makes the
    /// text again, when it has no edits, as soon as this differs from what the project is now.
    pub stamp: u64,
    /// The stamp that is waiting for the project to stay as it is for a moment, and since when.
    waiting: Option<(u64, std::time::Instant)>,
}

/// What the person asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum TextAction {
    None,
    /// Make the G-code of the project again (the edits are lost).
    MakeAgain,
    /// Show the toolpaths of this text in the analysis of G-code.
    Analyse,
}

impl GcodeText {
    pub fn new(slot: usize, name: &str, text: String) -> GcodeText {
        GcodeText { slot, name: name.to_string(), original: text.clone(), text, confirm_again: false, saved_as: None, stamp: 0, waiting: None }
    }

    /// Is it time to make the text again? It is when the project (`stamp`) differs from the one the text was made for, has no edits by hand,
    /// and has stayed the same for `wait` (typing a note is not a reason to make the program at every key).
    pub fn due(&mut self, stamp: u64, wait: std::time::Duration) -> bool {
        if self.stamp == stamp || self.dirty() {
            self.waiting = None;
            return false;
        }
        if wait.is_zero() {
            return true;
        }
        match self.waiting {
            Some((s, since)) if s == stamp => since.elapsed() >= wait,
            _ => {
                self.waiting = Some((stamp, std::time::Instant::now()));
                false
            }
        }
    }

    /// Has the text been changed by hand since it was made or saved?
    pub fn dirty(&self) -> bool {
        self.text != self.original
    }

    /// Put a freshly made text in (the edits are gone).
    pub fn replace(&mut self, text: String) {
        self.original = text.clone();
        self.text = text;
        self.confirm_again = false;
    }

    /// Write the text to `path`; the text is not "changed" afterwards.
    pub fn save_to(&mut self, path: PathBuf) -> Result<(), String> {
        std::fs::write(&path, &self.text).map_err(|e| e.to_string())?;
        self.original = self.text.clone();
        self.saved_as = Some(path);
        Ok(())
    }

    /// Ask where to write it and write it. `None` when the person cancelled.
    pub fn save_dialog(&mut self) -> Option<Result<PathBuf, String>> {
        let p = rfd::FileDialog::new().add_filter("G-code", &["nc", "gcode", "ngc"]).set_file_name(self.name.clone()).save_file()?;
        Some(self.save_to(p.clone()).map(|()| p))
    }
}

/// The colour of a word of G-code, by its letter: motion and other G words, M words, the axes, the speeds and tools, line numbers.
fn word_colour(letter: char) -> Color32 {
    let dark = theme::text().r() > 128;
    let (g, m, axis, speed, number) = if dark {
        (Color32::from_rgb(0x7a, 0xb8, 0xff), Color32::from_rgb(0xd7, 0x8a, 0xe6), Color32::from_rgb(0xe8, 0xb0, 0x5c), Color32::from_rgb(0x8c, 0xd0, 0x7a), Color32::from_gray(0x80))
    } else {
        (Color32::from_rgb(0x1a, 0x5f, 0xb4), Color32::from_rgb(0x8a, 0x2b, 0xb0), Color32::from_rgb(0xb0, 0x62, 0x00), Color32::from_rgb(0x2a, 0x7a, 0x20), Color32::from_gray(0x90))
    };
    match letter.to_ascii_uppercase() {
        'G' => g,
        'M' => m,
        'X' | 'Y' | 'Z' | 'A' | 'B' | 'C' | 'I' | 'J' | 'K' | 'R' => axis,
        'F' | 'S' | 'T' | 'P' => speed,
        'N' => number,
        _ => theme::text(),
    }
}

/// The text laid out with the colours: comments (`;` to the end of the line and `( … )`) dim, words by their letter.
pub fn highlight(text: &str, font: FontId, wrap: f32) -> LayoutJob {
    let plain = theme::text();
    let dim = theme::text_dim();
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let mut add = |s: &str, colour: Color32| {
        if !s.is_empty() {
            job.append(s, 0.0, TextFormat { font_id: font.clone(), color: colour, ..Default::default() });
        }
    };
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let _ = n;
        let chars: Vec<(usize, char)> = line.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            let (at, c) = chars[i];
            if c == ';' {
                add(&line[at..], dim);
                break;
            } else if c == '(' {
                let end = line[at..].find(')').map_or(line.len(), |e| at + e + 1);
                add(&line[at..end], dim);
                while i < chars.len() && chars[i].0 < end {
                    i += 1;
                }
            } else if c.is_ascii_alphabetic() {
                // A word: the letter and the number that follows it.
                let start = at;
                i += 1;
                while i < chars.len() && (chars[i].1.is_ascii_digit() || chars[i].1 == '.' || chars[i].1 == '-' || chars[i].1 == '+') {
                    i += 1;
                }
                let end = if i < chars.len() { chars[i].0 } else { line.len() };
                add(&line[start..end], word_colour(c));
            } else {
                let start = at;
                i += 1;
                while i < chars.len() && !(chars[i].1 == ';' || chars[i].1 == '(' || chars[i].1.is_ascii_alphabetic()) {
                    i += 1;
                }
                let end = if i < chars.len() { chars[i].0 } else { line.len() };
                add(&line[start..end], plain);
            }
        }
    }
    job
}

/// The tab: the bar of buttons and the text. Returns what the person asked for.
pub fn show(ui: &mut egui::Ui, g: &mut GcodeText) -> TextAction {
    let mut action = TextAction::None;
    ui.horizontal(|ui| {
        if ui.button(tr("Save G-code")).on_hover_text(tr("Write the text to a file")).clicked() {
            if let Some(Err(e)) = g.save_dialog() {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::unique("gcode_text_error"), e));
            }
        }
        let again = if g.confirm_again { tr("Lose the edits?") } else { tr("Make again") };
        let green = egui::Button::new(RichText::new(again).color(Color32::WHITE)).fill(crate::window::SAVE_GREEN);
        if ui.add(green).on_hover_text(tr("Make the G-code of the project again; the edits are lost")).clicked() {
            if g.dirty() && !g.confirm_again {
                g.confirm_again = true;
            } else {
                action = TextAction::MakeAgain;
            }
        }
        if ui.button(tr("Analyse")).on_hover_text(tr("Show the toolpaths of this text in the analysis of G-code")).clicked() {
            action = TextAction::Analyse;
        }
        if g.dirty() {
            ui.label(RichText::new(tr("Changed by hand")).small().color(theme::accent()));
        }
        ui.label(RichText::new(trf_lines(&g.text)).small().color(theme::text_dim()));
    });
    if let Some(e) = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::unique("gcode_text_error"))) {
        ui.label(RichText::new(crate::i18n::trf("Could not save: {}", &[&e])).small().color(Color32::from_rgb(0xc0, 0x30, 0x30)));
    }
    ui.separator();
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, wrap: f32| {
        let job = highlight(text.as_str(), font.clone(), wrap);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    egui::ScrollArea::both().id_salt(("gcode_text_scroll", g.slot)).auto_shrink([false, false]).show(ui, |ui| {
        let before = g.text.len();
        ui.add(egui::TextEdit::multiline(&mut g.text).font(egui::TextStyle::Monospace).code_editor().desired_width(f32::INFINITY).desired_rows(30).layouter(&mut layouter));
        if g.text.len() != before {
            g.confirm_again = false;
        }
    });
    action
}

/// "123 lines".
fn trf_lines(text: &str) -> String {
    crate::i18n::trf("{} lines", &[&text.lines().count()])
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    #[test]
    fn the_text_is_made_again_when_the_project_changed_stayed_so_for_a_moment_and_has_no_edits() {
        let mut g = GcodeText::new(0, "a.nc", "G21\n".into());
        g.stamp = 1;
        assert!(!g.due(1, Duration::ZERO), "the same project");
        assert!(g.due(2, Duration::ZERO), "no waiting asked for");
        // With a wait the first sight only starts the clock, and a newer change starts it again.
        assert!(!g.due(2, Duration::from_millis(30)));
        std::thread::sleep(Duration::from_millis(40));
        assert!(g.due(2, Duration::from_millis(30)));
        assert!(!g.due(3, Duration::from_millis(30)), "a newer change restarts the wait");
        // Edits by hand are never thrown away.
        g.text.push_str("(by hand)\n");
        assert!(!g.due(4, Duration::ZERO));
    }

    use super::*;

    #[test]
    fn the_text_is_changed_when_it_differs_from_what_was_made_and_saving_or_making_again_clears_it() {
        let mut g = GcodeText::new(3, "part.nc", "G21\nG0 X1\n".to_string());
        assert!(!g.dirty());
        g.text.push_str("M30\n");
        assert!(g.dirty());
        let dir = std::env::temp_dir().join(format!("gcode-text-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.nc");
        g.save_to(p.clone()).unwrap();
        assert!(!g.dirty() && g.saved_as.as_deref() == Some(p.as_path()));
        assert_eq!(std::fs::read_to_string(&p).unwrap(), "G21\nG0 X1\nM30\n");
        g.text.clear();
        assert!(g.dirty());
        g.replace("G21\n".to_string());
        assert!(!g.dirty());
        assert!(g.save_to(dir.join("no/such/dir/a.nc")).is_err());
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_words_comments_and_numbers_get_their_own_colours_and_the_text_is_kept() {
        let src = "N10 G1 X10.5 Y-3 F300 ; cut\n(a comment) M3 S12000\n";
        let job = highlight(src, FontId::monospace(12.0), 1000.0);
        assert_eq!(job.text, src, "colouring never changes the text");
        let colour_of = |word: &str| job.sections.iter().find(|s| job.text[(s.byte_range.start.0 as usize)..(s.byte_range.end.0 as usize)].contains(word)).map(|s| s.format.color);
        assert_eq!(colour_of("G1"), Some(word_colour('G')));
        assert_eq!(colour_of("X10.5"), Some(word_colour('X')));
        assert_eq!(colour_of("F300"), Some(word_colour('F')));
        assert_eq!(colour_of("M3"), Some(word_colour('M')));
        assert_eq!(colour_of("N10"), Some(word_colour('N')));
        assert_eq!(colour_of("; cut"), Some(theme::text_dim()), "a comment to the end of the line");
        assert_eq!(colour_of("(a comment)"), Some(theme::text_dim()));
        assert_ne!(word_colour('G'), word_colour('M'));
        let empty = highlight("", FontId::monospace(12.0), 100.0);
        assert!(empty.text.is_empty());
        assert_eq!(highlight("é(ü) G1 X1\n", FontId::monospace(12.0), 100.0).text, "é(ü) G1 X1\n", "non-ASCII text is kept");
    }

    #[test]
    fn the_tab_draws_and_asks_twice_before_the_edits_are_thrown_away() {
        let ctx = egui::Context::default();
        let mut g = GcodeText::new(1, "p.nc", "G21\nG0 X1\n".to_string());
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            assert_eq!(show(ui, &mut g), TextAction::None);
        });
        out.textures_delta.clear();
        g.text.push('x');
        g.confirm_again = true;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            show(ui, &mut g);
        });
        out.textures_delta.clear();
        assert!(g.dirty());
    }
}
