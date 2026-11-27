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
    /// The text of a two-sided job, made the way that was chosen (one file with or without a stop, or a file of one side). Save
    /// G-code then asks how to save, as the button of the program does, while the text has no edits by hand.
    pub sides: Option<crate::two_sided::SaveSides>,
    /// The stages of the text (its start, sides, layers, shapes, stops, end) and the length of the text they were found in.
    stages: Vec<Stage>,
    stages_of: Option<usize>,
    /// The line to bring into view (a stage was clicked), and the line that stays marked after it.
    goto: Option<usize>,
    marked: Option<usize>,
    /// The first line in view: its stage is lit in the list.
    top_line: usize,
}

/// What a stage of the G-code is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StageKind {
    /// The first line of the file.
    Start,
    /// Side A or side B (`true`) of a two-sided job.
    Side(bool),
    /// A layer, by its name.
    Layer(String),
    /// An element of a job done one element after the other, by its number.
    Element(usize),
    /// A shape (an object, a line, a fill, a contour…), by its number in the file.
    Shape(usize),
    /// A stop (M0, M1): a pen to change, the material to turn over.
    Stop,
    /// The end of the job.
    End,
}

/// A stage of the G-code: where it starts (the line, from 0) and what the comment there says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    pub line: usize,
    pub kind: StageKind,
    pub detail: String,
}

impl Stage {
    /// The title in the list and beside the text.
    pub fn title(&self) -> String {
        match &self.kind {
            StageKind::Start => tr("Start").to_string(),
            StageKind::Side(b) => tr(if *b { "Side B" } else { "Side A" }).to_string(),
            StageKind::Layer(name) => format!("{}: {name}", tr("Layer")),
            StageKind::Element(n) => crate::i18n::trf("Element {}", &[n]),
            StageKind::Shape(n) => crate::i18n::trf("Shape {}", &[n]),
            StageKind::Stop => tr("Stop").to_string(),
            StageKind::End => tr("End of the job").to_string(),
        }
    }

    /// How far it is indented in the list.
    fn depth(&self) -> usize {
        match self.kind {
            StageKind::Start | StageKind::Side(_) | StageKind::End => 0,
            StageKind::Element(_) | StageKind::Layer(_) | StageKind::Stop => 1,
            StageKind::Shape(_) => 2,
        }
    }

    /// Is it named beside its line in the text too? (The other stages are plain to see in their comment.)
    fn tagged(&self) -> bool {
        matches!(self.kind, StageKind::Shape(_) | StageKind::Element(_) | StageKind::Stop)
    }
}

/// The comment of a line: `; …` or `( … )`, and the code before it.
fn comment_of(line: &str) -> (&str, Option<&str>) {
    let t = line.trim();
    let semi = t.find(';');
    let paren = t.find('(');
    match (semi, paren) {
        (Some(s), p) if p.is_none_or(|p| s < p) => (t[..s].trim(), Some(t[s + 1..].trim())),
        (_, Some(p)) => {
            let end = t[p..].find(')').map_or(t.len(), |e| p + e);
            (t[..p].trim(), Some(t[p + 1..end].trim()))
        }
        _ => (t, None),
    }
}

/// Does the comment start something (`Fill start: 162 lines`, `object 1 of 2 start: rectangle`, `Clearing of the pocket start`)?
fn starts_something(c: &str) -> bool {
    let c = c.to_ascii_lowercase();
    c.contains(" start:") || c.ends_with(" start") || c.contains(" start again")
}

/// Does the comment end something (`Line 1 end`, `; offset fill end`)?
fn ends_something(c: &str) -> bool {
    c.to_ascii_lowercase().ends_with(" end")
}

/// The stages of a G-code text, from the comments the programs write: its start, the sides, the layers, the elements, every shape
/// (a comment that starts something while nothing else is started: what starts inside it, a line of a fill, a path of an object, is
/// a part of it), the stops and the end. The first line is always the start.
pub fn stages(text: &str) -> Vec<Stage> {
    let mut out = vec![];
    let (mut shapes, mut open) = (0usize, 0usize);
    for (i, raw) in text.lines().enumerate() {
        let (code, comment) = comment_of(raw);
        let c = comment.unwrap_or("");
        let lower = c.to_ascii_lowercase();
        let word = code.split_whitespace().next().unwrap_or("").to_ascii_uppercase();
        let mut add = |kind: StageKind| out.push(Stage { line: i, kind, detail: c.to_string() });
        if i == 0 {
            add(StageKind::Start);
            continue;
        }
        if matches!(word.as_str(), "M0" | "M00" | "M1" | "M01") {
            add(StageKind::Stop);
            continue;
        }
        if comment.is_none() {
            continue;
        }
        if lower == "side a" || lower.starts_with("side a:") || lower == "side b" || lower.starts_with("side b:") {
            open = 0;
            add(StageKind::Side(lower.starts_with("side b")));
        } else if let Some(name) = c.strip_prefix("Layer:") {
            open = 0;
            add(StageKind::Layer(name.trim().to_string()));
        } else if raw.trim_start().starts_with(';') && lower.starts_with("layer ") {
            // LightCreator: `; layer Cut (Line) speed 20 mm/s power 80%`.
            open = 0;
            let rest = &c["layer ".len()..];
            add(StageKind::Layer(rest.find(" (").map_or(rest, |e| &rest[..e]).trim().to_string()));
        } else if lower.starts_with("element ") && starts_something(c) {
            open = 0;
            let n = lower["element ".len()..].split_whitespace().next().and_then(|n| n.parse().ok()).unwrap_or(0);
            add(StageKind::Element(n));
        } else if lower.starts_with("end of layer") || lower.starts_with("element ") && ends_something(c) {
            open = 0;
        } else if lower == "end of the job" {
            open = 0;
            add(StageKind::End);
        } else if starts_something(c) {
            if open == 0 {
                shapes += 1;
                add(StageKind::Shape(shapes));
            }
            open += 1;
        } else if ends_something(c) {
            open = open.saturating_sub(1);
        }
    }
    out
}

/// What the person asked for.
#[derive(Debug, PartialEq, Eq)]
pub enum TextAction {
    None,
    /// Make the G-code of the project again (the edits are lost).
    MakeAgain,
    /// Show the toolpaths of this text in the analysis of G-code.
    Analyse,
    /// Save the G-code of a two-sided job: the program asks how (`two_sided::save_window`).
    SaveSides,
}

impl GcodeText {
    pub fn new(slot: usize, name: &str, text: String) -> GcodeText {
        GcodeText { slot, name: name.to_string(), original: text.clone(), text, confirm_again: false, saved_as: None, stamp: 0, waiting: None, sides: None, stages: vec![], stages_of: None, goto: None, marked: None, top_line: 0 }
    }

    /// Does Save G-code ask how to save (a text of a two-sided job without edits by hand), instead of writing the text as it is?
    pub fn asks_how_to_save(&self) -> bool {
        self.sides.is_some() && !self.dirty()
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
        self.stages_of = None;
    }

    /// The stages of the text, found again when it changed.
    pub fn stages(&mut self) -> &[Stage] {
        if self.stages_of != Some(self.text.len()) {
            self.stages = stages(&self.text);
            self.stages_of = Some(self.text.len());
        }
        &self.stages
    }

    /// Bring line `line` (from 0) into view, put the cursor at its start and mark it.
    pub fn go_to(&mut self, line: usize) {
        self.goto = Some(line);
        self.marked = Some(line);
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
            if g.asks_how_to_save() {
                // A two-sided job is saved the way the person chooses, as with the button of the program.
                action = TextAction::SaveSides;
            } else if let Some(Err(e)) = g.save_dialog() {
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
    let panel = if g.stages().len() > 1 { (ui.available_width() * 0.3).clamp(140.0, 240.0) } else { 0.0 };
    let full = ui.available_size();
    ui.horizontal_top(|ui| {
        let gap = if panel > 0.0 { ui.spacing().item_spacing.x } else { 0.0 };
        ui.allocate_ui_with_layout(egui::vec2((full.x - panel - gap).max(80.0), full.y), egui::Layout::top_down(egui::Align::Min), |ui| text_area(ui, g));
        if panel > 0.0 {
            ui.allocate_ui_with_layout(egui::vec2(panel, full.y), egui::Layout::top_down(egui::Align::Min), |ui| stages_list(ui, g));
        }
    });
    action
}

/// The text with the numbers of its lines on the left (only shown, never in the file) and the names of its shapes beside them.
fn text_area(ui: &mut egui::Ui, g: &mut GcodeText) {
    let font = egui::TextStyle::Monospace.resolve(ui.style());
    // Lines are never wrapped: a line of the text is a row on the screen, next to its number.
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
        let job = highlight(text.as_str(), font.clone(), f32::INFINITY);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let lines = g.text.lines().count().max(1);
    let digit = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
    let gutter = ((lines.to_string().len().max(3) as f32) * digit + 14.0).min(120.0);
    let goto = g.goto.take();
    let edit_id = ui.make_persistent_id(("gcode_text_edit", g.slot));
    let mut scroll = egui::ScrollArea::both().id_salt(("gcode_text_scroll", g.slot)).auto_shrink([false, false]);
    if let Some(line) = goto {
        // A line is a row (never wrapped): the line with three more above it at the top.
        let row = ui.fonts_mut(|f| f.row_height(&font));
        scroll = scroll.vertical_scroll_offset(line.saturating_sub(3) as f32 * row).horizontal_scroll_offset(0.0);
    }
    scroll.show(ui, |ui| {
        if let Some(line) = goto {
            // The cursor at the start of the line.
            let at: usize = g.text.split_inclusive('\n').take(line).map(|l| l.chars().count()).sum();
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), edit_id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at))));
            state.store(ui.ctx(), edit_id);
        }
        let margin = egui::Margin { left: gutter as i8, right: 4, top: 2, bottom: 2 };
        let out = egui::TextEdit::multiline(&mut g.text).id(edit_id).font(egui::TextStyle::Monospace).code_editor().margin(margin).desired_width(f32::INFINITY).desired_rows(30).layouter(&mut layouter).show(ui);
        if out.response.changed() {
            g.confirm_again = false;
        }
        let rows = &out.galley.rows;
        let pos = out.galley_pos.to_vec2();
        let clip = ui.clip_rect();
        let painter = ui.painter().with_clip_rect(clip);
        // Only the rows in view are numbered.
        let first = rows.partition_point(|r| r.rect().translate(pos).max.y < clip.min.y);
        g.top_line = first;
        let dim = theme::text_dim();
        let small = FontId::monospace((font.size * 0.85).max(8.0));
        let tag_from = g.stages.partition_point(|s| s.line < first);
        let mut tags = g.stages[tag_from..].iter().filter(|s| s.tagged()).peekable();
        for (i, row) in rows.iter().enumerate().skip(first) {
            let r = row.rect().translate(pos);
            if r.min.y > clip.max.y {
                break;
            }
            if g.marked == Some(i) {
                painter.rect_filled(egui::Rect::from_min_max(egui::pos2(clip.min.x, r.min.y), egui::pos2(clip.max.x, r.max.y)), 0.0, theme::accent().gamma_multiply(0.15));
            }
            painter.text(egui::pos2(out.galley_pos.x - 6.0, r.center().y), egui::Align2::RIGHT_CENTER, (i + 1).to_string(), small.clone(), dim);
            while tags.peek().is_some_and(|s| s.line < i) {
                tags.next();
            }
            if let Some(s) = tags.peek().filter(|s| s.line == i) {
                let galley = painter.layout_no_wrap(s.title(), small.clone(), theme::accent());
                let at = egui::pos2(r.max.x + 16.0, r.center().y - galley.size().y / 2.0);
                painter.rect_filled(egui::Rect::from_min_size(at, galley.size()).expand2(egui::vec2(5.0, 1.0)), 4.0, theme::accent().gamma_multiply(0.15));
                painter.galley(at, galley, theme::accent());
            }
        }
        // A line over the numbers, as in an editor of code.
        let x = out.galley_pos.x - 3.0;
        painter.line_segment([egui::pos2(x, out.response.rect.min.y), egui::pos2(x, out.response.rect.max.y)], egui::Stroke::new(1.0, dim.gamma_multiply(0.3)));
    });
}

/// The stages of the G-code, one small box each: a click brings its line into view. The stage of the first line in view is lit.
fn stages_list(ui: &mut egui::Ui, g: &mut GcodeText) {
    ui.label(RichText::new(tr("Stages")).strong());
    let current = g.stages.partition_point(|s| s.line <= g.top_line).saturating_sub(1);
    let row_h = 36.0;
    let mut clicked = None;
    egui::ScrollArea::vertical().id_salt(("gcode_text_stages", g.slot)).auto_shrink([false, false]).show_rows(ui, row_h, g.stages.len(), |ui, range| {
        for i in range {
            let s = &g.stages[i];
            let width = ui.available_width();
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, row_h), egui::Sense::click());
            let indent = s.depth() as f32 * 10.0;
            let card = egui::Rect::from_min_max(egui::pos2(rect.min.x + indent, rect.min.y + 1.0), egui::pos2(rect.max.x - 2.0, rect.max.y - 1.0));
            let lit = i == current;
            let fill = if lit { theme::accent().gamma_multiply(0.25) } else if resp.hovered() { theme::text().gamma_multiply(0.08) } else { theme::text().gamma_multiply(0.04) };
            let p = ui.painter().with_clip_rect(card.intersect(ui.clip_rect()));
            p.rect_filled(card, 4.0, fill);
            if lit {
                p.rect_stroke(card, 4.0, egui::Stroke::new(1.0, theme::accent()), egui::StrokeKind::Inside);
            }
            let body = egui::TextStyle::Body.resolve(ui.style());
            let small = egui::TextStyle::Small.resolve(ui.style());
            p.text(egui::pos2(card.min.x + 6.0, card.min.y + 3.0), egui::Align2::LEFT_TOP, s.title(), FontId::new(small.size * 1.1, body.family.clone()), theme::text());
            p.text(egui::pos2(card.max.x - 6.0, card.min.y + 3.0), egui::Align2::RIGHT_TOP, (s.line + 1).to_string(), small.clone(), theme::text_dim());
            p.text(egui::pos2(card.min.x + 6.0, card.max.y - 3.0), egui::Align2::LEFT_BOTTOM, &s.detail, small, theme::text_dim());
            if resp.on_hover_text(crate::i18n::trf("Go to line {}", &[&(s.line + 1)])).clicked() {
                clicked = Some(s.line);
            }
        }
    });
    if let Some(line) = clicked {
        g.go_to(line);
        ui.ctx().request_repaint();
    }
}

/// "123 lines".
fn trf_lines(text: &str) -> String {
    crate::i18n::trf("{} lines", &[&text.lines().count()])
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    /// The text of a two-sided job is saved the way the person chooses; once it is edited by hand, as it is.
    #[test]
    fn the_text_of_a_two_sided_job_asks_how_to_save_until_it_is_edited() {
        let mut g = super::GcodeText::new(0, "job.nc", "G21\n".to_string());
        assert!(!g.asks_how_to_save(), "an ordinary text is written as it is");
        g.sides = Some(crate::two_sided::SaveSides::Separate);
        assert!(g.asks_how_to_save());
        g.text.push_str("G0 X1\n");
        assert!(!g.asks_how_to_save(), "the edits are saved, not made again");
    }

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

    /// A text of PenCreator: the start, side A, two layers, every line and fill a shape (the lines of a fill are a part of it), a stop
    /// to change the pen, side B after the stop to turn the material over, the end.
    #[test]
    fn the_stages_are_found_in_the_comments_of_the_programs() {
        let text = "(PenCreator: t)\nG21 G90 G17\n(Side A)\n(Layer: Black pen)\n(Line 1 of 2 start: closed, 4.0 mm)\nG1 X1\n(Line 1 end)\n(Line 2 of 2 start: open, 1.0 mm)\nG1 X2\n(Line 2 end)\n(End of layer Black pen)\n(Layer: Red pen)\nM0 (Change the pen to: Red pen)\n(Fill start: 2 lines)\n(Line 1 of 2 start: open, 1.0 mm)\nG1 X3\n(Line 1 end)\n(Fill end)\nM0 (Turn the material over and resume to do side B)\n(Side B: the material is turned over side to side)\n(Layer: Black pen)\n(Clearing of the pocket start)\nG1 X4\n(End of the job)\nM30\n";
        let st = stages(text);
        let kinds: Vec<(usize, StageKind)> = st.iter().map(|s| (s.line, s.kind.clone())).collect();
        assert_eq!(
            kinds,
            vec![
                (0, StageKind::Start),
                (2, StageKind::Side(false)),
                (3, StageKind::Layer("Black pen".into())),
                (4, StageKind::Shape(1)),
                (7, StageKind::Shape(2)),
                (11, StageKind::Layer("Red pen".into())),
                (12, StageKind::Stop),
                (13, StageKind::Shape(3)),
                (18, StageKind::Stop),
                (19, StageKind::Side(true)),
                (20, StageKind::Layer("Black pen".into())),
                (21, StageKind::Shape(4)),
                (23, StageKind::End),
            ]
        );
        assert_eq!(st[3].detail, "Line 1 of 2 start: closed, 4.0 mm");
        assert_eq!(st[6].detail, "Change the pen to: Red pen");
        // LightCreator writes `;` comments: an object is the shape, its paths are parts of it; one element after the other.
        let lc = "; LightCreator job - Dummy\nG21\n; job: 1 layer(s) to burn, 2 object(s)\n; own start of the job\n; element 1 of 2 start: 1 object(s)\n; layer Cut (Line) speed 20 mm/s power 80%\n; object 1 of 2 start: rectangle\n; path 1 of 1 start: closed, 4.0 mm\nG1 X1 S800\n; path 1 end\n; object 1 end\n; end of layer Cut\n; element 1 end\n; element 2 of 2 start: 1 object(s)\n; layer Cut (Line) speed 20 mm/s power 80%\n; object 2 of 2 start: ellipse\n; end of the job\n";
        let kinds: Vec<(usize, StageKind)> = stages(lc).into_iter().map(|s| (s.line, s.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                (0, StageKind::Start),
                (4, StageKind::Element(1)),
                (5, StageKind::Layer("Cut".into())),
                (6, StageKind::Shape(1)),
                (13, StageKind::Element(2)),
                (14, StageKind::Layer("Cut".into())),
                (15, StageKind::Shape(2)),
                (16, StageKind::End),
            ]
        );
        assert_eq!(stages(""), vec![]);
        assert_eq!(stages("G21\n").len(), 1, "the start only");
    }

    /// The stages follow the text when it changes; a click on one brings its line into view.
    #[test]
    fn a_stage_brings_its_line_into_view() {
        let mut text = String::from("(PenCreator: t)\nG21\n(Layer: a)\n");
        for i in 0..300 {
            text.push_str(&format!("G1 X{i}\n"));
        }
        text.push_str("(Line 1 of 1 start: open, 1 mm)\nG1 X1\n(End of the job)\n");
        let mut g = GcodeText::new(2, "p.nc", text);
        assert_eq!(g.stages().len(), 4);
        let at = g.stages()[2].line;
        assert_eq!(at, 303);
        let ctx = egui::Context::default();
        let size = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 400.0))), ..Default::default() };
        let frame = |g: &mut GcodeText| {
            let mut out = ctx.run_ui(size.clone(), |ui| {
                show(ui, g);
            });
            out.textures_delta.clear();
        };
        frame(&mut g);
        assert_eq!(g.top_line, 0);
        g.go_to(at);
        frame(&mut g);
        frame(&mut g);
        assert!(g.top_line > 250 && g.top_line <= at, "the line is in view: {}", g.top_line);
        // The text changed: its stages are found again.
        g.text.push_str("(Fill start: 2 lines)\n");
        assert_eq!(g.stages().len(), 5);
    }

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
