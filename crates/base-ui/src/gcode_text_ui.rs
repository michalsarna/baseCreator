//! The G-code of a project as text: made for the project (with its postprocessors) and shown in a tab, coloured, and open to edits. The
//! person can change it by hand, write it to a file, make it again from the project or look at its toolpaths in the analysis of
//! G-code. Closing the tab with edits asks whether to save them. Its blocks (a side, a layer, a shape, a pass) fold to their first line,
//! a shape can be previewed alone, and its colours, typeface and size are those of the scheme and of the *G-code editor* part of Program
//! options.
use crate::i18n::tr;
use crate::theme;
use egui::text::{LayoutJob, TextFormat};
use egui::{Color32, FontId, RichText};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
    /// The last line in view.
    bottom_line: usize,
    /// The text as last drawn and how far below the top of the scrolled area it starts: a stage brings its line exactly to the top
    /// (the height of a row from the font is a little off: many thousand lines down the text was far from its line).
    drawn: Option<(std::sync::Arc<egui::Galley>, f32)>,
    /// Where the text was drawn on the screen (its first row) and the width of the column of the arrows, at the last frame.
    drawn_at: (egui::Pos2, f32),
    /// The blocks that can be folded (found with the stages) and the first lines of the folded ones.
    folds: Vec<Fold>,
    folded: BTreeSet<usize>,
    /// A shape (or a layer, a side) shown alone, in its own window: its title and the viewer.
    preview: Option<(String, crate::gcode_view_ui::GcodeViewer)>,
}

/// A block of lines that folds to its first line: a side, an element, a layer, a pass of a layer, a shape, a pass of a shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fold {
    /// Its first line (from 0), which stays in view when it is folded.
    pub start: usize,
    /// Its last line.
    pub end: usize,
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
    scan(text).0
}

/// The blocks of a G-code text that fold, by their first line. A block goes on to the line before the next block of its own kind or
/// of a bigger one (a layer ends where the next layer, element or side starts), or to the comment that ends it (`end of layer`, the end
/// of a shape); the passes (`pass 1/2` of LightCreator, `Pass 1 of 3` of PenCreator, `Depth 1 of 7` of MillCreator) are blocks inside a
/// layer or inside a shape.
pub fn folds(text: &str) -> Vec<Fold> {
    scan(text).1
}

/// Is the comment the start of a pass: `pass 1/2`, `Pass 1 of 3`, `Depth 2 of 7: 4 mm`?
fn starts_pass(lower: &str) -> bool {
    let Some(rest) = lower.strip_prefix("pass ").or_else(|| lower.strip_prefix("depth ")) else { return false };
    let n = rest.split(['/', ' ']).next().unwrap_or("");
    !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) && (rest.contains('/') || rest.contains(" of "))
}

/// The open blocks, each with its first line and its level (0 a side, 1 an element, 2 a layer, 3 a pass of a layer, 4 a shape, 5 a
/// pass of a shape). Closing at a level ends the blocks of that level and the smaller ones at `end`.
struct Blocks {
    open: Vec<(usize, u8)>,
    done: Vec<Fold>,
}

impl Blocks {
    fn close(&mut self, level: u8, end: usize) {
        while self.open.last().is_some_and(|&(_, l)| l >= level) {
            let (start, _) = self.open.pop().unwrap_or_default();
            if end > start {
                self.done.push(Fold { start, end });
            }
        }
    }

    fn start(&mut self, level: u8, line: usize) {
        self.close(level, line.saturating_sub(1));
        self.open.push((line, level));
    }
}

/// The stages and the blocks of a text, found in one reading.
fn scan(text: &str) -> (Vec<Stage>, Vec<Fold>) {
    let mut out = vec![];
    let mut blocks = Blocks { open: vec![], done: vec![] };
    let (mut shapes, mut open) = (0usize, 0usize);
    let mut last = 0;
    for (i, raw) in text.lines().enumerate() {
        last = i;
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
            blocks.start(0, i);
        } else if let Some(rest) = c.strip_prefix("Layer:") {
            // `Layer: Cut`, or LightCreator's `Layer: Cut (Line) speed 20 mm/s power 80%`: the name is before the brackets.
            open = 0;
            let rest = rest.trim();
            add(StageKind::Layer(rest.find(" (").map_or(rest, |e| &rest[..e]).trim().to_string()));
            blocks.start(2, i);
        } else if raw.trim_start().starts_with(';') && lower.starts_with("layer ") {
            // LightCreator: `; layer Cut (Line) speed 20 mm/s power 80%`.
            open = 0;
            let rest = &c["layer ".len()..];
            add(StageKind::Layer(rest.find(" (").map_or(rest, |e| &rest[..e]).trim().to_string()));
            blocks.start(2, i);
        } else if lower.starts_with("element ") && starts_something(c) {
            open = 0;
            let n = lower["element ".len()..].split_whitespace().next().and_then(|n| n.parse().ok()).unwrap_or(0);
            add(StageKind::Element(n));
            blocks.start(1, i);
        } else if lower.starts_with("end of layer") {
            // The passes and shapes in it end before it, the layer with it.
            open = 0;
            blocks.close(3, i.saturating_sub(1));
            blocks.close(2, i);
        } else if lower.starts_with("element ") && ends_something(c) {
            open = 0;
            blocks.close(2, i.saturating_sub(1));
            blocks.close(1, i);
        } else if lower == "end of the job" {
            open = 0;
            add(StageKind::End);
            blocks.close(0, i.saturating_sub(1));
        } else if starts_pass(&lower) {
            blocks.start(if open > 0 { 5 } else { 3 }, i);
        } else if starts_something(c) {
            if open == 0 {
                shapes += 1;
                add(StageKind::Shape(shapes));
                blocks.start(4, i);
            }
            open += 1;
        } else if ends_something(c) {
            open = open.saturating_sub(1);
            if open == 0 {
                blocks.close(4, i);
            }
        }
    }
    blocks.close(0, last);
    let mut folds = blocks.done;
    folds.sort_by_key(|f| f.start);
    (out, folds)
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
        GcodeText { slot, name: name.to_string(), original: text.clone(), text, confirm_again: false, saved_as: None, stamp: 0, waiting: None, sides: None, stages: vec![], stages_of: None, goto: Some(0), marked: None, top_line: 0, bottom_line: 0, drawn: None, drawn_at: (egui::Pos2::ZERO, 0.0), folds: vec![], folded: BTreeSet::new(), preview: None }
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
        self.folded.clear();
    }

    /// The stages of the text, found again when it changed (with its blocks; a folded block that is gone is no longer folded).
    pub fn stages(&mut self) -> &[Stage] {
        if self.stages_of != Some(self.text.len()) {
            (self.stages, self.folds) = scan(&self.text);
            self.stages_of = Some(self.text.len());
            let folds = &self.folds;
            self.folded.retain(|l| folds.binary_search_by_key(l, |f| f.start).is_ok());
        }
        &self.stages
    }

    /// The block that starts at `line`, if one does.
    pub fn fold_at(&mut self, line: usize) -> Option<Fold> {
        self.stages();
        self.folds.binary_search_by_key(&line, |f| f.start).ok().map(|i| self.folds[i])
    }

    /// Is the block that starts at `line` folded?
    pub fn is_folded(&self, line: usize) -> bool {
        self.folded.contains(&line)
    }

    /// Fold the block that starts at `line`, or unfold it when it is folded.
    pub fn toggle_fold(&mut self, line: usize) {
        if !self.folded.remove(&line) && self.fold_at(line).is_some() {
            self.folded.insert(line);
        }
    }

    /// The lines out of view, as ranges of lines (first and last, merged and in order): all but the first line of every folded block.
    pub fn hidden(&self) -> Vec<(usize, usize)> {
        let mut out: Vec<(usize, usize)> = vec![];
        for f in self.folds.iter().filter(|f| self.folded.contains(&f.start)) {
            let (a, b) = (f.start + 1, f.end);
            match out.last_mut() {
                Some(last) if a <= last.1 + 1 => last.1 = last.1.max(b),
                _ => out.push((a, b)),
            }
        }
        out
    }

    /// The lines of the stage that starts at `line`: its block, or the lines to the next stage.
    fn stage_lines(&mut self, line: usize) -> std::ops::Range<usize> {
        if let Some(f) = self.fold_at(line) {
            return f.start..f.end + 1;
        }
        let next = self.stages.iter().map(|s| s.line).find(|&l| l > line).unwrap_or_else(|| self.text.lines().count());
        line..next
    }

    /// Show the stage that starts at `line` alone in a window, as if only it were made on the material.
    pub fn preview_stage(&mut self, line: usize) -> Result<(), String> {
        let lines = self.stage_lines(line);
        let title = self.stages.iter().find(|s| s.line == line).map_or_else(|| self.name.clone(), |s| format!("{} · {}", s.title(), self.name));
        let v = crate::gcode_view_ui::GcodeViewer::from_text_part(PathBuf::from(&title), &self.text, lines).map_err(|e| crate::i18n::trf("Open failed: {}", &[&e]))?;
        self.preview = Some((title, v));
        Ok(())
    }

    /// The window of a stage shown alone, while it is open.
    pub fn previewing(&self) -> Option<&str> {
        self.preview.as_ref().map(|(t, _)| t.as_str())
    }

    /// Show the start of the text (a new tab, or the G-code made again by the person): the text opens at its first line.
    pub fn to_start(&mut self) {
        self.goto = Some(0);
        self.marked = None;
    }

    /// Bring line `line` (from 0) into view, put the cursor at its start and mark it.
    pub fn go_to(&mut self, line: usize) {
        // A line inside folded blocks: they open.
        let folds = &self.folds;
        self.folded.retain(|&s| !folds.iter().any(|f| f.start == s && s < line && line <= f.end));
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

/// The colours of the editor, made from the colours of the scheme: its background (a deep shade of the panels in a dark scheme, a
/// light one in a light scheme), the edge with the numbers of the lines, the text, the comments and the words by their letter. The
/// words take the hue of the accent of the scheme and hues spread from it, made light or dark enough to read on the background.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Syntax {
    pub background: Color32,
    pub gutter: Color32,
    pub text: Color32,
    pub comment: Color32,
    /// G words (motion), M words, the axes (and arc centres), the speeds, powers and tools, the numbers of the lines (N).
    pub g: Color32,
    pub m: Color32,
    pub axis: Color32,
    pub speed: Color32,
    pub number: Color32,
}

fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// The relative luminance (WCAG).
fn luminance(c: Color32) -> f32 {
    let lin = |v: u8| {
        let v = v as f32 / 255.0;
        if v <= 0.040_45 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) }
    };
    0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
}

/// The contrast of two colours (WCAG): 1 for the same, 21 for black on white.
pub fn contrast(a: Color32, b: Color32) -> f32 {
    let (x, y) = (luminance(a), luminance(b));
    (x.max(y) + 0.05) / (x.min(y) + 0.05)
}

/// Hue, saturation and lightness, each 0…1.
fn hsl(c: Color32) -> (f32, f32, f32) {
    let (r, g, b) = (c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0);
    let (max, min) = (r.max(g).max(b), r.min(g).min(b));
    let l = (max + min) / 2.0;
    let d = max - min;
    if d < 1e-6 {
        return (0.0, 0.0, l);
    }
    let s = d / (1.0 - (2.0 * l - 1.0).abs());
    let h = if max == r { ((g - b) / d).rem_euclid(6.0) } else if max == g { (b - r) / d + 2.0 } else { (r - g) / d + 4.0 };
    (h / 6.0, s, l)
}

fn from_hsl(h: f32, s: f32, l: f32) -> Color32 {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = h.rem_euclid(1.0) * 6.0;
    let x = c * (1.0 - (hp.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    let u = |v: f32| ((v + m).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color32::from_rgb(u(r), u(g), u(b))
}

/// The colour made lighter (on a dark background) or darker (on a light one) until it has `ratio` of contrast with the background.
fn readable(c: Color32, background: Color32, ratio: f32) -> Color32 {
    let (h, s, mut l) = hsl(c);
    let up = luminance(background) < 0.2;
    let mut out = c;
    for _ in 0..60 {
        if contrast(out, background) >= ratio {
            break;
        }
        l = if up { (l + 0.02).min(1.0) } else { (l - 0.02).max(0.0) };
        out = from_hsl(h, s, l);
    }
    out
}

/// The colours of the editor for the colours of a scheme.
pub fn syntax_of(ink: &theme::Ink) -> Syntax {
    let background = if ink.dark { mix(ink.panel_dark, Color32::BLACK, 0.45) } else { mix(ink.panel, Color32::WHITE, 0.75) };
    let gutter = mix(background, ink.accent, if ink.dark { 0.07 } else { 0.05 });
    let (h, s, _) = hsl(ink.accent);
    // A grey accent (Batman) gives the words its hue all the same, with colour enough to tell them apart.
    let s = s.clamp(0.55, 0.85);
    let word = |turn: f32| readable(from_hsl(h + turn, s, if ink.dark { 0.68 } else { 0.36 }), background, 4.5);
    Syntax {
        background,
        gutter,
        text: readable(ink.text, background, 7.0),
        comment: readable(ink.text_dim, background, 3.5),
        g: word(0.0),
        m: word(0.2),
        speed: word(0.4),
        axis: word(0.6),
        number: readable(ink.text_dim, background, 3.0),
    }
}

/// The colours of the editor for the scheme in use.
pub fn syntax() -> Syntax {
    syntax_of(&theme::ink())
}

/// The colour of a word of G-code, by its letter: motion and other G words, M words, the axes, the speeds and tools, line numbers.
fn word_colour(letter: char, c: &Syntax) -> Color32 {
    match letter.to_ascii_uppercase() {
        'G' => c.g,
        'M' => c.m,
        'X' | 'Y' | 'Z' | 'A' | 'B' | 'C' | 'I' | 'J' | 'K' | 'R' => c.axis,
        'F' | 'S' | 'T' | 'P' => c.speed,
        'N' => c.number,
        _ => c.text,
    }
}

/// The text laid out with the colours: comments (`;` to the end of the line and `( … )`) dim, words by their letter.
pub fn highlight(text: &str, font: FontId, wrap: f32) -> LayoutJob {
    highlight_with(text, font, wrap, &syntax(), &[])
}

/// [`highlight`] with the colours `c` and the lines of `hidden` (ranges of lines, first and last, in order) out of view: they keep their
/// rows, with no height and no colour, so that a row is still a line of the text.
fn highlight_with(text: &str, font: FontId, wrap: f32, c: &Syntax, hidden: &[(usize, usize)]) -> LayoutJob {
    let (plain, dim) = (c.text, c.comment);
    let mut job = LayoutJob::default();
    job.wrap.max_width = wrap;
    let gone = TextFormat { font_id: font.clone(), color: Color32::TRANSPARENT, line_height: Some(0.0), ..Default::default() };
    let add = |job: &mut LayoutJob, s: &str, colour: Color32| {
        if !s.is_empty() {
            job.append(s, 0.0, TextFormat { font_id: font.clone(), color: colour, ..Default::default() });
        }
    };
    let mut hide = hidden.iter().peekable();
    let mut gone_from: Option<usize> = None;
    let mut at_byte = 0;
    for (n, line) in text.split_inclusive('\n').enumerate() {
        let line_start = at_byte;
        at_byte += line.len();
        while hide.peek().is_some_and(|h| h.1 < n) {
            hide.next();
        }
        if hide.peek().is_some_and(|h| h.0 <= n) {
            // The hidden lines go in as one piece.
            gone_from.get_or_insert(line_start);
            continue;
        }
        if let Some(from) = gone_from.take() {
            job.append(&text[from..line_start], 0.0, gone.clone());
        }
        let chars: Vec<(usize, char)> = line.char_indices().collect();
        let mut i = 0;
        while i < chars.len() {
            let (at, ch) = chars[i];
            if ch == ';' {
                add(&mut job, &line[at..], dim);
                break;
            } else if ch == '(' {
                let end = line[at..].find(')').map_or(line.len(), |e| at + e + 1);
                add(&mut job, &line[at..end], dim);
                while i < chars.len() && chars[i].0 < end {
                    i += 1;
                }
            } else if ch.is_ascii_alphabetic() {
                // A word: the letter and the number that follows it.
                let start = at;
                i += 1;
                while i < chars.len() && (chars[i].1.is_ascii_digit() || chars[i].1 == '.' || chars[i].1 == '-' || chars[i].1 == '+') {
                    i += 1;
                }
                let end = if i < chars.len() { chars[i].0 } else { line.len() };
                add(&mut job, &line[start..end], word_colour(ch, c));
            } else {
                let start = at;
                i += 1;
                while i < chars.len() && !(chars[i].1 == ';' || chars[i].1 == '(' || chars[i].1.is_ascii_alphabetic()) {
                    i += 1;
                }
                let end = if i < chars.len() { chars[i].0 } else { line.len() };
                add(&mut job, &line[start..end], plain);
            }
        }
    }
    if let Some(from) = gone_from {
        job.append(&text[from..], 0.0, gone);
    }
    job
}

/// The typeface and size of the G-code editor (the *G-code editor* part of Program options), the same in every program.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct EditorPrefs {
    /// An id of `fonts::EDITOR_FACES`.
    #[serde(default = "default_face")]
    pub face: String,
    /// In points; 0 is the size of the monospace text of the interface.
    #[serde(default)]
    pub size: f32,
}

fn default_face() -> String {
    crate::fonts::EDITOR_FACES[0].0.to_string()
}

impl Default for EditorPrefs {
    fn default() -> Self {
        EditorPrefs { face: default_face(), size: 0.0 }
    }
}

/// The file of the settings of the editor, in the shared folder.
pub const PREFS_FILE: &str = "gcode_editor.json";

/// The file (once a program said where with [`init`]) and the settings.
static PREFS: Mutex<Option<(Option<PathBuf>, EditorPrefs)>> = Mutex::new(None);

/// Read the settings of the editor from the shared folder `base`. Called once when the program starts.
pub fn init(base: &Path) {
    let file = base.join(PREFS_FILE);
    let p = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<EditorPrefs>(&t).ok()).unwrap_or_default();
    *PREFS.lock().unwrap_or_else(|e| e.into_inner()) = Some((Some(file), p));
}

/// The settings of the editor.
pub fn editor_prefs() -> EditorPrefs {
    PREFS.lock().unwrap_or_else(|e| e.into_inner()).as_ref().map(|(_, p)| p.clone()).unwrap_or_default()
}

/// Change the settings of the editor (and write them to the shared folder, when the program said where).
pub fn set_editor_prefs(p: &EditorPrefs) -> Result<(), String> {
    let mut st = PREFS.lock().unwrap_or_else(|e| e.into_inner());
    let file = st.as_ref().and_then(|(f, _)| f.clone());
    *st = Some((file.clone(), p.clone()));
    let Some(file) = file else { return Ok(()) };
    if let Some(dir) = file.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(file, serde_json::to_string_pretty(p).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}

/// The font of the editor: the typeface and size of the settings (the monospace font of the interface where the typefaces are not
/// installed, as in a test).
pub fn editor_font(ui: &egui::Ui) -> FontId {
    editor_font_of(ui, &editor_prefs())
}

/// The *G-code editor* part of Program options: a line, a title, the typeface and the size, and a sample in them. True when something
/// was changed.
pub fn options_section(ui: &mut egui::Ui) -> bool {
    let mut p = editor_prefs();
    let interface = egui::TextStyle::Monospace.resolve(ui.style()).size;
    crate::widgets::options_line(ui);
    ui.label(RichText::new(tr("G-code editor")).strong());
    ui.add_space(2.0);
    let mut changed = false;
    crate::widgets::options_grid(ui, "gcode_editor_options_grid", |ui| {
        ui.label(tr("Font"));
        let name = crate::fonts::EDITOR_FACES.iter().find(|f| f.0 == p.face).map_or(crate::fonts::EDITOR_FACES[0].1, |f| f.1);
        egui::ComboBox::from_id_salt("gcode_editor_face").selected_text(name).show_ui(ui, |ui| {
            for (id, label) in crate::fonts::EDITOR_FACES {
                let family = crate::fonts::editor_family(id);
                let shown = if ui.ctx().fonts(|f| f.families().contains(&family)) { RichText::new(label).family(family) } else { RichText::new(label) };
                if ui.selectable_label(p.face == id, shown).clicked() && p.face != id {
                    p.face = id.to_string();
                    changed = true;
                }
            }
        });
        ui.end_row();
        ui.label(tr("Font size"));
        let mut size = if p.size > 0.0 { p.size } else { interface };
        if ui.add(egui::DragValue::new(&mut size).range(6.0..=48.0).speed(0.25).max_decimals(1).suffix(" pt")).changed() {
            p.size = size;
            changed = true;
        }
        ui.end_row();
    });
    // A sample: a line of a program in the font and the colours of the editor.
    let font = editor_font_of(ui, &p);
    let c = syntax();
    let job = highlight_with("G1 X12.5 Y40 F1200 S800 ; Layer: Cut\n", font, f32::INFINITY, &c, &[]);
    egui::Frame::new().fill(c.background).corner_radius(4.0).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
        ui.label(job);
    });
    if changed {
        let _ = set_editor_prefs(&p);
    }
    changed
}

/// The font of settings `p` (the sample of Program options shows a choice before it is kept).
fn editor_font_of(ui: &egui::Ui, p: &EditorPrefs) -> FontId {
    let base = egui::TextStyle::Monospace.resolve(ui.style());
    let size = if p.size > 0.0 { p.size.clamp(6.0, 48.0) } else { base.size };
    let family = crate::fonts::editor_family(&p.face);
    if ui.ctx().fonts(|f| f.families().contains(&family)) { FontId::new(size, family) } else { FontId::new(size, base.family) }
}

/// The tab: the bar of buttons and the text. Returns what the person asked for.
pub fn show(ui: &mut egui::Ui, g: &mut GcodeText) -> TextAction {
    let mut action = TextAction::None;
    // A little room between the bar of the tabs and the buttons.
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        // A little room on the left, so the buttons do not touch the edge of the tab.
        ui.add_space(6.0);
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
    if let Some(e) = ui.ctx().data(|d| d.get_temp::<String>(egui::Id::unique("gcode_text_preview_error"))) {
        ui.label(RichText::new(e).small().color(Color32::from_rgb(0xc0, 0x30, 0x30)));
    }
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
    preview_window(ui.ctx(), g);
    action
}

/// The window of a stage shown alone: its moves on the material of the whole program, the moves through the air left out.
fn preview_window(ctx: &egui::Context, g: &mut GcodeText) {
    let Some((title, v)) = g.preview.take() else { return };
    let mut open = true;
    let mut viewer = Some(v);
    crate::window::popup(ctx, &format!("gcode_text_preview_{}", g.slot), &title, [640.0, 520.0], &mut open, |ui| {
        ui.set_min_size(egui::vec2(560.0, 420.0));
        if !crate::gcode_view_ui::panel(&mut viewer, ui) {
            viewer = None;
        }
    });
    if let (true, Some(v)) = (open, viewer) {
        g.preview = Some((title, v));
    }
}

/// The text with the numbers of its lines on the left (only shown, never in the file), an arrow by every block that folds, and the
/// names of its shapes beside them. A folded block shows its first line and how many lines it hides.
fn text_area(ui: &mut egui::Ui, g: &mut GcodeText) {
    let font = editor_font(ui);
    let colours = syntax();
    g.stages();
    let hidden = g.hidden();
    // Lines are never wrapped: a line of the text is a row on the screen, next to its number.
    let mut layouter = |ui: &egui::Ui, text: &dyn egui::TextBuffer, _wrap: f32| {
        let job = highlight_with(text.as_str(), font.clone(), f32::INFINITY, &colours, &hidden);
        ui.fonts_mut(|f| f.layout_job(job))
    };
    let lines = g.text.lines().count().max(1);
    let digit = ui.fonts_mut(|f| f.glyph_width(&font, '0'));
    let arrow_w = (font.size * 0.9).max(10.0);
    let gutter = ((lines.to_string().len().max(3) as f32) * digit + 14.0 + arrow_w).min(126.0);
    let goto = g.goto.take();
    let edit_id = ui.make_persistent_id(("gcode_text_edit", g.slot));
    let mut scroll = egui::ScrollArea::both().id_salt(("gcode_text_scroll", g.slot)).auto_shrink([false, false]);
    if let Some(line) = goto {
        // A line is a row (never wrapped): the line with three more above it at the top, where the text as last drawn has it.
        let above = line.saturating_sub(3);
        let y = match &g.drawn {
            _ if line == 0 => 0.0,
            Some((galley, dy)) if above < galley.rows.len() => dy + galley.rows[above].rect().min.y,
            _ => above as f32 * ui.fonts_mut(|f| f.row_height(&font)),
        };
        scroll = scroll.vertical_scroll_offset(y).horizontal_scroll_offset(0.0);
    }
    // The background of the scheme, also behind the numbers.
    let area = ui.available_rect_before_wrap();
    ui.painter().rect_filled(area, 0.0, colours.background);
    let mut toggle = None;
    scroll.show(ui, |ui| {
        let content_top = ui.max_rect().min.y;
        if let Some(line) = goto {
            // The cursor at the start of the line.
            let at: usize = g.text.split_inclusive('\n').take(line).map(|l| l.chars().count()).sum();
            let mut state = egui::text_edit::TextEditState::load(ui.ctx(), edit_id).unwrap_or_default();
            state.cursor.set_char_range(Some(egui::text::CCursorRange::one(egui::text::CCursor::new(at))));
            state.store(ui.ctx(), edit_id);
        }
        let margin = egui::Margin { left: gutter as i8, right: 4, top: 2, bottom: 2 };
        let out = egui::TextEdit::multiline(&mut g.text)
            .id(edit_id)
            .font(font.clone())
            .code_editor()
            .background_color(colours.background)
            .margin(margin)
            .desired_width(f32::INFINITY)
            .desired_rows(30)
            .layouter(&mut layouter)
            .show(ui);
        if out.response.changed() {
            g.confirm_again = false;
        }
        g.drawn = Some((out.galley.clone(), out.galley_pos.y - content_top));
        g.drawn_at = (out.galley_pos, arrow_w);
        let rows = &out.galley.rows;
        let pos = out.galley_pos.to_vec2();
        let clip = ui.clip_rect();
        let painter = ui.painter().with_clip_rect(clip);
        // The edge with the numbers, a shade of the background.
        let edge = egui::Rect::from_min_max(egui::pos2(out.response.rect.min.x, clip.min.y), egui::pos2(out.galley_pos.x - 3.0, clip.max.y));
        painter.rect_filled(edge, 0.0, colours.gutter);
        // Only the rows in view are numbered.
        let first = rows.partition_point(|r| r.rect().translate(pos).max.y < clip.min.y);
        g.top_line = first;
        g.bottom_line = first;
        let small = FontId::new((font.size * 0.85).max(8.0), font.family.clone());
        let tag_from = g.stages.partition_point(|s| s.line < first);
        let mut tags = g.stages[tag_from..].iter().filter(|s| s.tagged()).peekable();
        let fold_from = g.folds.partition_point(|f| f.start < first);
        let mut folds = g.folds[fold_from..].iter().peekable();
        let mut i = first;
        while i < rows.len() {
            // Rows of folded blocks have no height: the next one in view is after them.
            if let Some(&(_, end)) = hidden.iter().find(|h| h.0 <= i && i <= h.1) {
                i = end + 1;
                continue;
            }
            let r = rows[i].rect().translate(pos);
            if r.min.y > clip.max.y {
                break;
            }
            g.bottom_line = i;
            if g.marked == Some(i) {
                painter.rect_filled(egui::Rect::from_min_max(egui::pos2(clip.min.x, r.min.y), egui::pos2(clip.max.x, r.max.y)), 0.0, theme::accent().gamma_multiply(0.15));
            }
            painter.text(egui::pos2(out.galley_pos.x - 6.0 - arrow_w, r.center().y), egui::Align2::RIGHT_CENTER, (i + 1).to_string(), small.clone(), colours.number);
            let mut x = r.max.x + 16.0;
            while folds.peek().is_some_and(|f| f.start < i) {
                folds.next();
            }
            if let Some(f) = folds.peek().filter(|f| f.start == i) {
                // The arrow: down while the block is open, to the right while it is folded.
                let folded = g.folded.contains(&i);
                let c = egui::pos2(out.galley_pos.x - 4.0 - arrow_w / 2.0, r.center().y);
                let hit = egui::Rect::from_center_size(c, egui::vec2(arrow_w, r.height().max(8.0)));
                let resp = ui.interact(hit, edit_id.with(("fold", i)), egui::Sense::click()).on_hover_cursor(egui::CursorIcon::PointingHand);
                let ink = if resp.hovered() { colours.text } else { colours.number };
                let k = arrow_w * 0.22;
                let pts = if folded { vec![c + egui::vec2(-k * 0.6, -k), c + egui::vec2(k * 0.9, 0.0), c + egui::vec2(-k * 0.6, k)] } else { vec![c + egui::vec2(-k, -k * 0.6), c + egui::vec2(k, -k * 0.6), c + egui::vec2(0.0, k * 0.9)] };
                painter.add(egui::Shape::convex_polygon(pts, ink, egui::Stroke::NONE));
                if resp.on_hover_text(tr(if folded { "Unfold" } else { "Fold" })).clicked() {
                    toggle = Some(i);
                }
                if folded {
                    // How much is folded away; a click opens it.
                    let label = format!("⋯ {}", crate::i18n::trf("{} lines", &[&(f.end - f.start)]));
                    let galley = painter.layout_no_wrap(label, small.clone(), colours.comment);
                    let at = egui::pos2(x, r.center().y - galley.size().y / 2.0);
                    let pill = egui::Rect::from_min_size(at, galley.size()).expand2(egui::vec2(5.0, 1.0));
                    painter.rect_filled(pill, 4.0, colours.comment.gamma_multiply(0.18));
                    painter.galley(at, galley, colours.comment);
                    if ui.interact(pill, edit_id.with(("folded", i)), egui::Sense::click()).on_hover_text(tr("Unfold")).clicked() {
                        toggle = Some(i);
                    }
                    x = pill.max.x + 12.0;
                }
            }
            while tags.peek().is_some_and(|s| s.line < i) {
                tags.next();
            }
            if let Some(s) = tags.peek().filter(|s| s.line == i) {
                let galley = painter.layout_no_wrap(s.title(), small.clone(), theme::accent());
                let at = egui::pos2(x, r.center().y - galley.size().y / 2.0);
                painter.rect_filled(egui::Rect::from_min_size(at, galley.size()).expand2(egui::vec2(5.0, 1.0)), 4.0, theme::accent().gamma_multiply(0.15));
                painter.galley(at, galley, theme::accent());
            }
            i += 1;
        }
        // A line over the numbers, as in an editor of code.
        let x = out.galley_pos.x - 3.0;
        painter.line_segment([egui::pos2(x, out.response.rect.min.y), egui::pos2(x, out.response.rect.max.y)], egui::Stroke::new(1.0, colours.number.gamma_multiply(0.3)));
    });
    if let Some(line) = toggle {
        g.toggle_fold(line);
        ui.ctx().request_repaint();
    }
}

/// The stage lit in the list.
fn lit_stage(g: &GcodeText) -> usize {
    let at = g.marked.filter(|m| (g.top_line..=g.bottom_line).contains(m)).unwrap_or(g.top_line);
    g.stages.partition_point(|s| s.line <= at).saturating_sub(1)
}

/// The stages of the G-code, one small box each: a click brings its line into view. The stage of the first line in view is lit.
fn stages_list(ui: &mut egui::Ui, g: &mut GcodeText) {
    ui.label(RichText::new(tr("Stages")).strong());
    // The stage that was clicked while its line is in view (it is brought in below the top), else the stage of the first line in view.
    let current = lit_stage(g);
    let row_h = 36.0;
    let mut clicked = None;
    let mut fold = None;
    let mut preview = None;
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
            let resp = resp.on_hover_text(crate::i18n::trf("Go to line {}", &[&(s.line + 1)]));
            if resp.clicked() {
                clicked = Some(s.line);
            }
            // The right button: fold or unfold its lines, or show it alone.
            let folds = g.folds.binary_search_by_key(&s.line, |f| f.start).is_ok();
            let folded = g.folded.contains(&s.line);
            let alone = !matches!(s.kind, StageKind::Start | StageKind::Stop | StageKind::End);
            let line = s.line;
            if folds || alone {
                resp.context_menu(|ui| {
                    if folds && ui.button(tr(if folded { "Unfold" } else { "Fold" })).clicked() {
                        fold = Some(line);
                        ui.close();
                    }
                    if alone && ui.button(tr("Preview")).clicked() {
                        preview = Some(line);
                        ui.close();
                    }
                });
            }
        }
    });
    if let Some(line) = clicked {
        g.go_to(line);
        ui.ctx().request_repaint();
    }
    if let Some(line) = fold {
        g.toggle_fold(line);
        ui.ctx().request_repaint();
    }
    if let Some(line) = preview {
        // A stage without moves (only comments) says so over the text.
        let id = egui::Id::unique("gcode_text_preview_error");
        match g.preview_stage(line) {
            Ok(()) => ui.ctx().data_mut(|d| d.remove::<String>(id)),
            Err(e) => ui.ctx().data_mut(|d| {
                d.insert_temp(id, e);
            }),
        }
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
        let lc = "; LightCreator job - Dummy\nG21\n; job: 1 layer(s) to burn, 2 object(s)\n; own start of the job\n; element 1 of 2 start: 1 object(s)\n; Layer: Cut (Line) speed 20 mm/s power 80%\n; object 1 of 2 start: rectangle\n; path 1 of 1 start: closed, 4.0 mm\nG1 X1 S800\n; path 1 end\n; object 1 end\n; end of layer Cut\n; element 1 end\n; element 2 of 2 start: 1 object(s)\n; layer Cut (Line) speed 20 mm/s power 80%\n; object 2 of 2 start: ellipse\n; end of the job\n";
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
        assert!(g.top_line > 250 && g.top_line <= at && at <= g.bottom_line, "the line is in view: {}..{}", g.top_line, g.bottom_line);
        // The clicked stage is the lit one, not the stage of the lines above it at the top.
        assert_eq!(lit_stage(&g), 2);
        // Far down a long text the line is at the top all the same (the height of a row from the font was a little off).
        let mut long = String::from("(MillCreator: t)\nG21\n(Layer: a)\n");
        for i in 0..20000 {
            long.push_str(&format!("G1 X{i} Y{i}.25 Z-9.525\n"));
        }
        long.push_str("(Contour 1 of 1 start: 1 mm round)\nG1 X1\n");
        for i in 0..20000 {
            long.push_str(&format!("G1 X{i} Y{i}.5 Z-10\n"));
        }
        long.push_str("(End of the job)\n");
        let mut l = GcodeText::new(4, "m.nc", long);
        let far = l.stages()[2].line;
        frame(&mut l);
        assert_eq!(l.top_line, 0);
        l.go_to(far);
        frame(&mut l);
        frame(&mut l);
        assert!((far - 4..=far - 2).contains(&l.top_line), "the line with three above it at the top: {} for {far}", l.top_line);
        assert_eq!(lit_stage(&l), 2);
        // Made again by the person: the text opens at its first line.
        l.to_start();
        frame(&mut l);
        frame(&mut l);
        assert_eq!(l.top_line, 0);
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
        let c = syntax_of(&theme::ink_of(theme::Scheme::Dark));
        let job = highlight_with(src, FontId::monospace(12.0), 1000.0, &c, &[]);
        assert_eq!(job.text, src, "colouring never changes the text");
        let colour_of = |word: &str| job.sections.iter().find(|s| job.text[(s.byte_range.start.0 as usize)..(s.byte_range.end.0 as usize)].contains(word)).map(|s| s.format.color);
        assert_eq!(colour_of("G1"), Some(c.g));
        assert_eq!(colour_of("X10.5"), Some(c.axis));
        assert_eq!(colour_of("F300"), Some(c.speed));
        assert_eq!(colour_of("M3"), Some(c.m));
        assert_eq!(colour_of("N10"), Some(c.number));
        assert_eq!(colour_of("; cut"), Some(c.comment), "a comment to the end of the line");
        assert_eq!(colour_of("(a comment)"), Some(c.comment));
        let empty = highlight("", FontId::monospace(12.0), 100.0);
        assert!(empty.text.is_empty());
        assert_eq!(highlight("é(ü) G1 X1\n", FontId::monospace(12.0), 100.0).text, "é(ü) G1 X1\n", "non-ASCII text is kept");
    }

    /// The colours come from the scheme: in every scheme the background is its own shade, the words take the hue of its accent, and
    /// everything reads on the background and differs from the rest.
    #[test]
    fn the_colours_of_every_scheme_read_well_on_its_own_background() {
        let mut backgrounds = vec![];
        for scheme in theme::Scheme::EVERY {
            let ink = theme::ink_of(scheme);
            let c = syntax_of(&ink);
            let name = format!("{scheme:?}");
            assert_eq!(ink.dark, luminance(c.background) < 0.2, "{name}: a dark background in a dark scheme");
            for (what, colour, ratio) in [("text", c.text, 7.0), ("G", c.g, 4.5), ("M", c.m, 4.5), ("axes", c.axis, 4.5), ("speeds", c.speed, 4.5), ("comments", c.comment, 3.5), ("numbers", c.number, 3.0)] {
                assert!(contrast(colour, c.background) >= ratio, "{name}: {what} {colour:?} on {:?} has {:.2}", c.background, contrast(colour, c.background));
            }
            let words = [c.g, c.m, c.axis, c.speed];
            for a in 0..4 {
                for b in a + 1..4 {
                    let d = (words[a].r() as f32 - words[b].r() as f32).abs() + (words[a].g() as f32 - words[b].g() as f32).abs() + (words[a].b() as f32 - words[b].b() as f32).abs();
                    assert!(d > 60.0, "{name}: words {a} and {b} look alike: {:?} {:?}", words[a], words[b]);
                }
            }
            // G words have the hue of the accent (when the accent has a colour at all).
            let (ha, sa, _) = hsl(ink.accent);
            let (hg, _, _) = hsl(c.g);
            let turn = (ha - hg).abs().min(1.0 - (ha - hg).abs());
            assert!(sa < 0.15 || turn < 0.03, "{name}: G words in the hue of the accent");
            backgrounds.push(c.background);
        }
        backgrounds.sort_by_key(|c| (c.r(), c.g(), c.b()));
        backgrounds.dedup();
        assert!(backgrounds.len() > theme::Scheme::EVERY.len() / 2, "the schemes have their own backgrounds");
    }

    /// The blocks of the programs: LightCreator's layers with passes and objects, MillCreator's contours with their depths, PenCreator's
    /// lines with their passes. Folded, a block shows its first line; a line inside it brought into view opens it.
    #[test]
    fn the_blocks_of_the_programs_fold_to_their_first_line() {
        let lc = "; LightCreator job — t\nG21\n; Layer: C00 (Line) speed 20 mm/s power 60%\n; settings: 2 pass(es)\n; pass 1/2\n; object 1 of 2 start: rectangle\n; path 1 start\nG1 X1\n; path 1 end\n; object 1 end\n; object 2 of 2 start: ellipse\nG1 X2\n; object 2 end\n; pass 2/2\n; object 1 of 2 start: rectangle\nG1 X1\n; object 1 end\n; end of layer C00\n; Layer: C01 (Fill) speed 20 mm/s power 60%\n; fill start: 2 lines\nG1 X3\n; fill end\n; end of layer C01\n; End of the job\nM5\n";
        let f = folds(lc);
        let at = |l: usize| f.iter().find(|x| x.start == l).map(|x| x.end);
        assert_eq!(at(2), Some(17), "the layer to its end");
        assert_eq!(at(4), Some(12), "the first pass to the line before the second");
        assert_eq!(at(13), Some(16), "the second pass to the end of its object (the end of the layer closes it)");
        assert_eq!(at(5), Some(9), "an object to its end, with its path inside");
        assert_eq!(at(6), None, "a path is a part of its object, not a block");
        assert_eq!(at(10), Some(12));
        assert_eq!(at(18), Some(22));
        assert_eq!(at(19), Some(21), "a fill");
        let mill = "; MillCreator: t\nG21\n; Layer: Cut out\n; Contour 1 of 2 start: 10 mm round\n; Depth 1 of 2: 1 mm\nG1 Z-1\nG1 X5\n; Depth 2 of 2: 2 mm\nG1 Z-2\nG1 X5\n; Contour 1 end\n; Contour 2 of 2 start: 5 mm round\nG1 X9\n; Contour 2 end\n; End of the job\nM5\n";
        let f = folds(mill);
        let at = |l: usize| f.iter().find(|x| x.start == l).map(|x| x.end);
        assert_eq!(at(3), Some(10), "a contour to its end");
        assert_eq!((at(4), at(7)), (Some(6), Some(10)), "its depths, the last to the end of the contour");
        assert_eq!(at(2), Some(13), "the layer to the end of the job");
        let pen = "; PenCreator: t\n; Layer: Ink\n; Line 1 of 1 start: closed, 10 mm\n; Pass 1 of 2\nG1 X1\n; Pass 2 of 2\nG1 X0\n; Line 1 end\n";
        let f = folds(pen);
        let at = |l: usize| f.iter().find(|x| x.start == l).map(|x| x.end);
        assert_eq!((at(2), at(3), at(5)), (Some(7), Some(4), Some(7)));
        assert!(!starts_pass("depth by tone") && !starts_pass("pass the salt") && starts_pass("pass 3/4") && starts_pass("pass 2 of 3"));

        let mut g = GcodeText::new(0, "lc.nc", lc.to_string());
        assert!(g.hidden().is_empty());
        g.toggle_fold(4);
        g.toggle_fold(2);
        assert_eq!(g.hidden(), vec![(3, 17)], "the folded pass inside the folded layer is hidden with it");
        g.toggle_fold(2);
        assert_eq!(g.hidden(), vec![(5, 12)]);
        g.toggle_fold(7);
        assert!(!g.is_folded(7), "a line that starts no block does not fold");
        // A line inside a folded block brought into view: the block opens.
        g.go_to(11);
        assert!(g.hidden().is_empty());
        g.toggle_fold(18);
        // Made again: nothing is folded.
        g.replace(lc.to_string());
        assert!(g.hidden().is_empty());
        // An edit that takes a block away unfolds it.
        g.toggle_fold(19);
        g.text = g.text.replace("; fill start: 2 lines\n", "");
        g.stages();
        assert!(!g.is_folded(19));
    }

    /// The rows of the folded lines have no height (the text below moves up) and keep the text; the arrow by the first line folds the
    /// block with a click, the right button on its stage too, and a shape is shown alone.
    #[test]
    fn a_folded_block_takes_no_room_and_the_arrow_and_the_menu_fold_it() {
        let mut text = String::from("; PenCreator: t\nG21\n; Layer: Ink\n; Line 1 of 2 start: open, 5 mm\n");
        for i in 0..40 {
            text.push_str(&format!("G1 X{i} Y1\n"));
        }
        text.push_str("; Line 1 end\n; Line 2 of 2 start: open, 5 mm\nG0 X0 Y50\nM3 S1000\nG1 X10 Y50\nG1 X10 Y60\n; Line 2 end\n; End of the job\nM5\n");
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let c = syntax();
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let font = editor_font(ui);
            assert_eq!(font.family, crate::fonts::editor_family("noto"), "the typeface of the editor where the fonts are installed");
            let open = ui.fonts_mut(|f| f.layout_job(highlight_with(&text, font.clone(), f32::INFINITY, &c, &[])));
            let folded = ui.fonts_mut(|f| f.layout_job(highlight_with(&text, font.clone(), f32::INFINITY, &c, &[(4, 43)])));
            assert_eq!(folded.rows.len(), open.rows.len(), "a row is still a line");
            assert_eq!(folded.job.text, text);
            assert!(folded.rows[4..=43].iter().all(|r| r.rect().height() < 0.5), "the folded rows take no room");
            assert!(folded.size().y < open.size().y - 30.0 * open.rows[0].rect().height());
            assert!((folded.rows[44].rect().min.y - folded.rows[3].rect().max.y).abs() < 1.0, "the line after the block comes right after its first line");
        });
        out.textures_delta.clear();
        let mut g = GcodeText::new(5, "pen.nc", text.clone());
        let size = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(900.0, 600.0))), ..Default::default() };
        let frame = |g: &mut GcodeText, events: Vec<egui::Event>| {
            let mut input = size.clone();
            input.events = events;
            let mut out = ctx.run_ui(input, |ui| {
                show(ui, g);
            });
            out.textures_delta.clear();
        };
        frame(&mut g, vec![]);
        frame(&mut g, vec![]);
        // A click on the number of line 4 does not fold; a click on the arrow next to it folds its shape.
        let click = |g: &mut GcodeText, p: egui::Pos2| {
            frame(g, vec![egui::Event::PointerMoved(p)]);
            frame(g, vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: true, modifiers: Default::default() }]);
            frame(g, vec![egui::Event::PointerButton { pos: p, button: egui::PointerButton::Primary, pressed: false, modifiers: Default::default() }]);
        };
        let arrow_of = |g: &GcodeText, line: usize| {
            let (galley, _) = g.drawn.clone().expect("drawn");
            let (at, w) = g.drawn_at;
            egui::pos2(at.x - 4.0 - w / 2.0, at.y + galley.rows[line].rect().center().y)
        };
        let p = arrow_of(&g, 3);
        let number = p - egui::vec2(g.drawn_at.1 + 8.0, 0.0);
        click(&mut g, number);
        assert!(g.folded.is_empty(), "a click on the number does not fold");
        click(&mut g, p);
        let folded = g.is_folded(3);
        assert!(folded, "a click on the arrow of the shape folds it");
        frame(&mut g, vec![]);
        assert_eq!(g.hidden(), vec![(4, 44)]);
        // The text and its edits are untouched by folding.
        assert_eq!(g.text, text);
        // The second shape alone: its moves only, from where the pen is, on the bounds of the whole program.
        let second = g.stages().iter().find(|s| s.kind == StageKind::Shape(2)).map(|s| s.line).expect("shape 2");
        g.preview_stage(second).unwrap();
        let (title, v) = g.preview.as_ref().expect("a window");
        assert!(title.contains("pen.nc"));
        assert!(v.material && !v.show_rapid);
        assert_eq!(v.tp.segs.iter().filter(|s| s.cut).count(), 2, "the two drawn moves of the shape");
        assert_eq!(v.tp.min[0], 0.0);
        assert_eq!(v.tp.max[0], 39.0, "the bounds of the whole program");
        frame(&mut g, vec![]);
        assert!(g.previewing().is_some(), "the window stays open");
        // The window keeps the size it opened with, frame after frame (it grew to the height of the screen).
        let height = || ctx.memory(|m| m.layer_ids().find(|l| l.order != egui::Order::Background).and_then(|l| m.area_rect(l.id))).map(|r| r.height()).expect("the window is shown");
        let first = height();
        for _ in 0..8 {
            frame(&mut g, vec![]);
        }
        assert!((height() - first).abs() < 1.0 && first < 560.0, "the window does not grow: {first} → {}", height());
        assert!(g.preview_stage(0).is_err() || g.previewing().is_some());
    }

    #[test]
    fn the_settings_of_the_editor_are_kept_in_a_file_of_the_shared_folder() {
        let p = EditorPrefs { face: "jetbrains".into(), size: 15.5 };
        let text = serde_json::to_string(&p).unwrap();
        assert_eq!(serde_json::from_str::<EditorPrefs>(&text).unwrap(), p);
        assert_eq!(serde_json::from_str::<EditorPrefs>("{}").unwrap(), EditorPrefs::default(), "an empty or old file gives the default");
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            let f = editor_font_of(ui, &p);
            assert_eq!((f.size, f.family), (15.5, crate::fonts::editor_family("jetbrains")));
            assert_eq!(editor_font_of(ui, &EditorPrefs::default()).size, egui::TextStyle::Monospace.resolve(ui.style()).size, "0 is the size of the interface");
            for (id, _) in crate::fonts::EDITOR_FACES {
                assert!(ui.ctx().fonts(|f| f.families().contains(&crate::fonts::editor_family(id))), "{id} is installed");
            }
        });
        out.textures_delta.clear();
        let bare = egui::Context::default();
        let mut out = bare.run_ui(egui::RawInput::default(), |ui| {
            assert_eq!(editor_font_of(ui, &p).family, egui::FontFamily::Monospace, "without the fonts, the monospace font of the interface");
        });
        out.textures_delta.clear();
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
