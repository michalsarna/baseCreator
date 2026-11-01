//! The central area of a program divided into up to three panes, side by side or one above the other. One drawing is always the
//! active one (it is what the tools, the menus and the keys work on); the other panes only show something: another open
//! drawing (to look at, with a zoom of its own), the preview, the analysis of a G-code file or the camera. A click in a pane
//! with another drawing makes that drawing the active one.
//!
//! This module knows the layout and what each pane is meant to show; what a pane draws is up to the program ([`PaneHost`]),
//! as with `EditorHost` and `TabHost`. With one pane (the default) nothing is drawn around the content: the program looks and
//! works as it did before the panes existed.
use crate::editor::View;
use crate::i18n::tr;
use crate::theme;
use egui::{Rect, Sense};
use serde::{Deserialize, Serialize};

/// The most panes at once.
pub const MAX_PANES: usize = 3;
/// The least size of a pane along the split, in points.
pub const MIN_PANE: f32 = 160.0;
const GAP: f32 = 6.0;
const HEADER_H: f32 = 24.0;

/// How the panes are placed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default, Hash)]
pub enum Dir {
    /// Next to each other.
    #[default]
    Beside,
    /// One above the other.
    Stacked,
}

/// What a pane shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Hash)]
pub enum Content {
    /// The active drawing: the one that is edited.
    Active,
    /// Another open drawing, for looking only; the number is its recovery slot, which does not change while it is open.
    Doc(usize),
    Preview,
    Gcode,
    Camera,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pane {
    pub content: Content,
    /// The share of the room; the shares are compared with each other.
    pub weight: f32,
    /// The zoom and the shift of a pane that shows another drawing (not saved).
    #[serde(skip)]
    pub view: Option<View>,
}

impl Pane {
    fn new(content: Content) -> Pane {
        Pane { content, weight: 1.0, view: None }
    }
}

/// A pane taken out into a window of its own (on another monitor, for example). It stays until it is sent back or closed.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Window {
    pub content: Content,
    /// Where the window was last seen (the top-left corner of its inside) and how big it was.
    pub pos: Option<[f32; 2]>,
    pub size: [f32; 2],
    #[serde(skip)]
    pub view: Option<View>,
}

/// The layout: what is where. Saved in the settings of the program.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Panes {
    pub dir: Dir,
    pub panes: Vec<Pane>,
    /// The panes that live in windows of their own.
    #[serde(default)]
    pub windows: Vec<Window>,
}

impl Default for Panes {
    fn default() -> Panes {
        Panes { dir: Dir::Beside, panes: vec![Pane::new(Content::Active)], windows: vec![] }
    }
}

impl Panes {
    /// Only the active drawing, as if there were no panes.
    pub fn is_single(&self) -> bool {
        self.panes.len() <= 1
    }

    pub fn contents(&self) -> Vec<Content> {
        self.panes.iter().map(|p| p.content).collect()
    }

    pub fn shows(&self, c: Content) -> bool {
        self.panes.iter().any(|p| p.content == c)
    }

    /// Add a pane that shows `c` (after the last one); `false` when there are already [`MAX_PANES`] or `c` is on show.
    pub fn split(&mut self, c: Content) -> bool {
        if self.panes.len() >= MAX_PANES || self.shows(c) {
            return false;
        }
        self.panes.push(Pane::new(c));
        self.equalize();
        true
    }

    /// Take a pane away (the last one stays, and the active drawing is always on show).
    pub fn close(&mut self, i: usize) {
        if self.panes.len() > 1 && i < self.panes.len() {
            self.panes.remove(i);
            self.equalize();
            if !self.shows(Content::Active) {
                self.panes[0].content = Content::Active;
            }
        }
    }

    /// Show something else in a pane. The active drawing stays on show: choosing it elsewhere swaps the two panes.
    pub fn set_content(&mut self, i: usize, c: Content) {
        if i >= self.panes.len() || self.panes[i].content == c {
            return;
        }
        if let Some(j) = self.panes.iter().position(|p| p.content == c) {
            let old = self.panes[i].content;
            self.panes[j].content = old;
            self.panes[j].view = None;
        }
        self.panes[i].content = c;
        self.panes[i].view = None;
    }

    pub fn equalize(&mut self) {
        for p in &mut self.panes {
            p.weight = 1.0;
        }
    }

    /// The drawing `now` became the active one (the tab bar or a click in a pane): the pane that showed it as another drawing
    /// now shows the one that was active, `was`.
    pub fn activated(&mut self, was: usize, now: usize) {
        for p in &mut self.panes {
            if p.content == Content::Doc(now) {
                p.content = Content::Doc(was);
                p.view = None;
            }
        }
    }

    /// Make the layout right after the world changed: a drawing that is not open any more, a camera that is not there, two
    /// panes with the same content, no pane with the active drawing. `docs` are the slots of all the open drawings,
    /// `active` the slot of the active one.
    pub fn normalize(&mut self, docs: &[usize], active: usize, has_camera: bool) {
        let mut seen: Vec<Content> = vec![];
        for p in &mut self.panes {
            let bad = match p.content {
                Content::Doc(s) => s == active || !docs.contains(&s),
                Content::Camera => !has_camera,
                _ => false,
            };
            if bad {
                p.content = Content::Active;
            }
            if seen.contains(&p.content) {
                p.content = Content::Active;
            }
            seen.push(p.content);
        }
        // Only the first pane with the active drawing keeps it; the others are dropped.
        let mut first = true;
        self.panes.retain(|p| {
            if p.content != Content::Active {
                return true;
            }
            let keep = first;
            first = false;
            keep
        });
        if self.panes.is_empty() {
            self.panes.push(Pane::new(Content::Active));
        }
        if !self.shows(Content::Active) {
            self.panes[0].content = Content::Active;
        }
        self.panes.truncate(MAX_PANES);
        if self.panes.iter().any(|p| !(p.weight.is_finite() && p.weight > 0.0)) {
            self.equalize();
        }
        // A window shows what is not in a pane, and never the active drawing.
        let on_show = self.contents();
        let mut seen: Vec<Content> = vec![];
        self.windows.retain(|w| {
            let ok = match w.content {
                Content::Active => false,
                Content::Doc(s) => s != active && docs.contains(&s),
                Content::Camera => has_camera,
                _ => true,
            } && !on_show.contains(&w.content)
                && !seen.contains(&w.content);
            seen.push(w.content);
            ok
        });
    }

    /// Take the pane `i` out into a window (the active drawing cannot go).
    pub fn pop_out(&mut self, i: usize) {
        if i < self.panes.len() && self.panes[i].content != Content::Active {
            let c = self.panes[i].content;
            let size = [640.0, 480.0];
            self.windows.push(Window { content: c, pos: None, size, view: self.panes[i].view.take() });
            self.close(i);
        }
    }

    /// Bring the window `i` back as a pane (when there is room), or just close it.
    pub fn window_back(&mut self, i: usize, as_pane: bool) {
        if i < self.windows.len() {
            let w = self.windows.remove(i);
            if as_pane && self.split(w.content) {
                if let Some(p) = self.panes.last_mut() {
                    p.view = w.view;
                }
            }
        }
    }

    /// The rectangles of the panes inside `area`, separated by gaps for the dividers; every pane is at least `min` wide (or
    /// high) when the area allows it.
    pub fn rects(&self, area: Rect, min: f32) -> Vec<Rect> {
        let n = self.panes.len();
        let along = if self.dir == Dir::Beside { area.width() } else { area.height() };
        let room = (along - GAP * (n.saturating_sub(1)) as f32).max(0.0);
        let total: f32 = self.panes.iter().map(|p| p.weight).sum::<f32>().max(1e-6);
        let min = min.min(room / n.max(1) as f32);
        let mut sizes: Vec<f32> = self.panes.iter().map(|p| room * p.weight / total).collect();
        // Raise the small ones to the minimum, taking it from the big ones.
        for _ in 0..n {
            let short: f32 = sizes.iter().map(|s| (min - s).max(0.0)).sum();
            if short <= 0.0 {
                break;
            }
            let big: f32 = sizes.iter().map(|s| (s - min).max(0.0)).sum();
            for s in &mut sizes {
                if *s < min {
                    *s = min;
                } else if big > 0.0 {
                    *s -= (*s - min) / big * short;
                }
            }
        }
        let mut pos = if self.dir == Dir::Beside { area.left() } else { area.top() };
        sizes
            .iter()
            .map(|s| {
                let r = if self.dir == Dir::Beside { Rect::from_min_max(egui::pos2(pos, area.top()), egui::pos2(pos + s, area.bottom())) } else { Rect::from_min_max(egui::pos2(area.left(), pos), egui::pos2(area.right(), pos + s)) };
                pos += s + GAP;
                r
            })
            .collect()
    }

    /// Move the divider after pane `i` by `delta` points inside `area`.
    pub fn drag(&mut self, i: usize, delta: f32, area: Rect, min: f32) {
        if i + 1 >= self.panes.len() {
            return;
        }
        let rects = self.rects(area, min);
        let size = |r: &Rect| if self.dir == Dir::Beside { r.width() } else { r.height() };
        let (a, b) = (size(&rects[i]), size(&rects[i + 1]));
        let min = min.min((a + b) / 2.0);
        let delta = delta.clamp(min - a, b - min);
        // The weights become the sizes, so that the other panes keep theirs.
        for (p, r) in self.panes.iter_mut().zip(&rects) {
            p.weight = size(r);
        }
        self.panes[i].weight = a + delta;
        self.panes[i + 1].weight = b - delta;
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// The saved layout, or the single pane when there is none or it cannot be read.
    pub fn from_json(s: &str) -> Panes {
        serde_json::from_str::<Panes>(s).ok().filter(|p| !p.panes.is_empty() && p.panes.len() <= MAX_PANES).unwrap_or_default()
    }
}

/// The name of a kind of content, for the choice in the bar of a pane. `Doc` is named by the program (the title of the drawing).
pub fn content_name(c: Content) -> String {
    match c {
        Content::Active => tr("Active project").to_string(),
        Content::Doc(_) => String::new(),
        Content::Preview => tr("Preview").to_string(),
        Content::Gcode => tr("G-code analysis").to_string(),
        Content::Camera => tr("Camera").to_string(),
    }
}

/// What the program does for the panes.
pub trait PaneHost {
    /// The choices for a pane: the active drawing, the other open drawings (`Doc(slot)`), and what else the program can show.
    fn pane_choices(&self) -> Vec<(Content, String)>;
    /// Draw the content of a pane. `Active` is what the central area showed before the panes existed.
    fn draw_pane(&mut self, ui: &mut egui::Ui, content: Content, view: &mut Option<View>) -> PaneEvent;
    /// Can this content be taken out into a window of its own (a program may refuse for a kind of content)?
    fn can_pop_out(&self, _content: Content) -> bool {
        false
    }
}

/// What happened in a pane.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PaneEvent {
    #[default]
    Nothing,
    /// The pane with another drawing was clicked: make it the active one.
    Activate(usize),
}

/// What the person did in the bar of a pane.
enum Bar {
    Choose(usize, Content),
    Close(usize),
    PopOut(usize),
}

impl Panes {
    /// Draw the panes in the rest of `ui`. With one pane there is no bar and no divider.
    pub fn show<H: PaneHost>(&mut self, h: &mut H, ui: &mut egui::Ui) -> PaneEvent {
        if self.is_single() {
            let mut view = None;
            return h.draw_pane(ui, Content::Active, &mut view);
        }
        let area = ui.available_rect_before_wrap();
        ui.allocate_rect(area, Sense::hover());
        let rects = self.rects(area, MIN_PANE);
        let choices = h.pane_choices();
        let (mut event, mut bar) = (PaneEvent::Nothing, None);
        for i in 0..self.panes.len() {
            let r = rects[i];
            let head = Rect::from_min_size(r.min, egui::vec2(r.width(), HEADER_H));
            let body = Rect::from_min_max(egui::pos2(r.left(), r.top() + HEADER_H), r.max);
            let content = self.panes[i].content;
            let mut hui = ui.new_child(egui::UiBuilder::new().max_rect(head).layout(egui::Layout::left_to_right(egui::Align::Center)));
            hui.painter().rect_filled(head, 0.0, theme::panel());
            let label = choices.iter().find(|c| c.0 == content).map(|c| c.1.clone()).unwrap_or_default();
            egui::ComboBox::from_id_salt(("pane_content", i)).selected_text(label).width((head.width() - 70.0).max(40.0)).show_ui(&mut hui, |ui| {
                for (c, name) in &choices {
                    if ui.selectable_label(*c == content, name).clicked() {
                        bar = Some(Bar::Choose(i, *c));
                    }
                }
            });
            if h.can_pop_out(content) && hui.small_button("⧉").on_hover_text(tr("Open in a separate window")).clicked() {
                bar = Some(Bar::PopOut(i));
            }
            if hui.small_button("×").on_hover_text(tr("Close pane")).clicked() {
                bar = Some(Bar::Close(i));
            }
            let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(body));
            bui.set_clip_rect(body.intersect(ui.clip_rect()));
            let mut view = self.panes[i].view.take();
            let e = h.draw_pane(&mut bui, content, &mut view);
            self.panes[i].view = view;
            if e != PaneEvent::Nothing {
                event = e;
            }
        }
        // The dividers.
        let mut dragged = None;
        for i in 0..self.panes.len() - 1 {
            let gap = if self.dir == Dir::Beside { Rect::from_min_max(egui::pos2(rects[i].right(), area.top()), egui::pos2(rects[i + 1].left(), area.bottom())) } else { Rect::from_min_max(egui::pos2(area.left(), rects[i].bottom()), egui::pos2(area.right(), rects[i + 1].top())) };
            let resp = ui.interact(gap, ui.make_persistent_id(("pane_divider", i)), Sense::click_and_drag());
            let colour = if resp.hovered() || resp.dragged() { theme::accent() } else { theme::border() };
            ui.painter().rect_filled(gap, 0.0, colour);
            if resp.hovered() || resp.dragged() {
                ui.ctx().set_cursor_icon(if self.dir == Dir::Beside { egui::CursorIcon::ResizeHorizontal } else { egui::CursorIcon::ResizeVertical });
            }
            if resp.double_clicked() {
                self.equalize();
            } else if resp.dragged() {
                dragged = Some((i, if self.dir == Dir::Beside { resp.drag_delta().x } else { resp.drag_delta().y }));
            }
        }
        if let Some((i, d)) = dragged {
            self.drag(i, d, area, MIN_PANE);
        }
        match bar {
            Some(Bar::Choose(i, c)) => self.set_content(i, c),
            Some(Bar::Close(i)) => self.close(i),
            Some(Bar::PopOut(i)) => self.pop_out(i),
            None => {}
        }
        event
    }
}

impl Panes {
    /// Draw the windows of the panes that were taken out (call it every frame, outside the panels). Closing a window, or its
    /// "Back to the main window" button, puts the pane back. Remembers where the window is.
    pub fn show_windows<H: PaneHost>(&mut self, h: &mut H, ctx: &egui::Context) -> PaneEvent {
        let mut event = PaneEvent::Nothing;
        let mut back: Option<usize> = None;
        let choices = h.pane_choices();
        for i in 0..self.windows.len() {
            let content = self.windows[i].content;
            let title = choices.iter().find(|c| c.0 == content).map(|c| c.1.clone()).unwrap_or_default();
            let mut builder = egui::ViewportBuilder::default().with_title(title).with_inner_size(self.windows[i].size);
            if let Some(p) = self.windows[i].pos {
                builder = builder.with_position(p);
            }
            let mut view = self.windows[i].view.take();
            let mut seen: Option<([f32; 2], [f32; 2])> = None;
            let id = egui::ViewportId::from_hash_of(("work_pane_window", content));
            ctx.show_viewport_immediate(id, builder, |ctx, _class| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    ui.horizontal(|ui| {
                        if ui.button(tr("Back to the main window")).clicked() {
                            back = Some(i);
                        }
                    });
                    ui.separator();
                    let e = h.draw_pane(ui, content, &mut view);
                    if e != PaneEvent::Nothing {
                        event = e;
                    }
                });
                if ctx.input(|i| i.viewport().close_requested()) {
                    back = Some(i);
                }
                if let Some(r) = ctx.input(|i| i.viewport().inner_rect) {
                    seen = Some(([r.min.x, r.min.y], [r.width(), r.height()]));
                }
            });
            self.windows[i].view = view;
            if let Some((pos, size)) = seen {
                self.windows[i].pos = Some(pos);
                self.windows[i].size = size;
            }
        }
        if let Some(i) = back {
            self.window_back(i, true);
        }
        event
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn area() -> Rect {
        Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(906.0, 400.0))
    }

    #[test]
    fn the_default_is_one_pane_with_the_active_drawing() {
        let p = Panes::default();
        assert!(p.is_single() && p.contents() == [Content::Active]);
    }

    #[test]
    fn a_split_shares_the_room_and_keeps_the_gaps() {
        let mut p = Panes::default();
        assert!(p.split(Content::Doc(3)) && p.split(Content::Preview));
        assert!(!p.split(Content::Gcode), "three panes at most");
        let r = p.rects(area(), MIN_PANE);
        assert_eq!(r.len(), 3);
        assert!((r[0].width() - 298.0).abs() < 0.01 && (r[1].left() - r[0].right() - GAP).abs() < 0.01);
        assert!((r[2].right() - 906.0).abs() < 0.01);
        p.dir = Dir::Stacked;
        let r = p.rects(area(), 10.0);
        assert!(r.iter().all(|r| r.width() == 906.0) && (r[0].height() - (400.0 - 2.0 * GAP) / 3.0).abs() < 0.01);
    }

    #[test]
    fn a_divider_moves_two_panes_and_keeps_the_least_size() {
        let mut p = Panes::default();
        p.split(Content::Preview);
        p.drag(0, 100.0, area(), MIN_PANE);
        let r = p.rects(area(), MIN_PANE);
        assert!((r[0].width() - 550.0).abs() < 0.5, "{r:?}");
        p.drag(0, 5000.0, area(), MIN_PANE);
        let r = p.rects(area(), MIN_PANE);
        assert!((r[1].width() - MIN_PANE).abs() < 0.5 && (r[0].width() + r[1].width() + GAP - 906.0).abs() < 0.5);
        p.drag(0, -5000.0, area(), MIN_PANE);
        assert!((p.rects(area(), MIN_PANE)[0].width() - MIN_PANE).abs() < 0.5);
        // The other panes keep their size when a divider moves.
        let mut p = Panes::default();
        p.split(Content::Doc(1));
        p.split(Content::Preview);
        let before = p.rects(area(), MIN_PANE)[2].width();
        p.drag(0, 30.0, area(), MIN_PANE);
        assert!((p.rects(area(), MIN_PANE)[2].width() - before).abs() < 0.5);
        p.equalize();
        assert!(p.panes.iter().all(|x| x.weight == 1.0));
    }

    #[test]
    fn the_active_drawing_is_always_on_show() {
        let mut p = Panes::default();
        p.split(Content::Doc(1));
        p.close(0);
        assert!(p.is_single() && p.contents() == [Content::Active], "the last pane became the active drawing");
        p.split(Content::Preview);
        // Choosing the active drawing in another pane swaps the two.
        p.set_content(1, Content::Active);
        assert_eq!(p.contents(), [Content::Preview, Content::Active]);
        p.close(1);
        assert_eq!(p.contents(), [Content::Active]);
    }

    #[test]
    fn a_click_on_another_drawing_swaps_it_with_the_active_one() {
        let mut p = Panes::default();
        p.split(Content::Doc(2));
        p.activated(0, 2);
        assert_eq!(p.contents(), [Content::Active, Content::Doc(0)]);
    }

    #[test]
    fn a_layout_that_no_longer_fits_is_mended() {
        let mut p = Panes { dir: Dir::Beside, panes: vec![Pane::new(Content::Doc(7)), Pane::new(Content::Camera), Pane::new(Content::Doc(1)), Pane::new(Content::Doc(1))], windows: vec![] };
        p.normalize(&[0, 1], 0, false);
        // Doc 7 is not open, the camera is not there, doc 1 is shown twice, and three panes are the most.
        assert_eq!(p.contents(), [Content::Active, Content::Doc(1)]);
        let mut p = Panes { dir: Dir::Stacked, panes: vec![Pane::new(Content::Doc(1)), Pane::new(Content::Preview)], windows: vec![] };
        p.normalize(&[0, 1], 1, true);
        assert_eq!(p.contents(), [Content::Active, Content::Preview], "the drawing that became the active one is not shown as another");
        let mut p = Panes { dir: Dir::Beside, panes: vec![Pane::new(Content::Preview), Pane::new(Content::Gcode)], windows: vec![] };
        p.normalize(&[0], 0, true);
        assert!(p.shows(Content::Active));
        p.panes[0].weight = f32::NAN;
        p.normalize(&[0], 0, true);
        assert!(p.panes.iter().all(|x| x.weight == 1.0));
    }

    #[test]
    fn the_layout_is_saved_and_read_back_and_nonsense_gives_one_pane() {
        let mut p = Panes::default();
        p.dir = Dir::Stacked;
        p.split(Content::Doc(4));
        p.split(Content::Gcode);
        p.drag(0, 40.0, area(), MIN_PANE);
        let q = Panes::from_json(&p.to_json());
        assert_eq!(q.dir, Dir::Stacked);
        assert_eq!(q.contents(), p.contents());
        assert!((q.panes[0].weight - p.panes[0].weight).abs() < 1e-3);
        assert!(Panes::from_json("").is_single() && Panes::from_json("{\"dir\":1}").is_single() && Panes::from_json("{\"dir\":\"Beside\",\"panes\":[]}").is_single());
    }

    #[test]
    fn a_pane_goes_out_into_a_window_and_comes_back() {
        let mut p = Panes::default();
        p.split(Content::Preview);
        p.pop_out(0);
        assert!(p.panes.len() == 2 && p.windows.is_empty(), "the active drawing never leaves");
        p.pop_out(1);
        assert_eq!(p.contents(), [Content::Active]);
        assert_eq!(p.windows.len(), 1);
        p.windows[0].pos = Some([2000.0, 100.0]);
        // The window is saved with its place, and a window whose content is in a pane or gone is dropped.
        let mut q = Panes::from_json(&p.to_json());
        assert_eq!(q.windows[0].pos, Some([2000.0, 100.0]));
        q.split(Content::Preview);
        q.normalize(&[0], 0, true);
        assert!(q.windows.is_empty(), "the preview is in a pane already");
        p.window_back(0, true);
        assert_eq!(p.contents(), [Content::Active, Content::Preview]);
        p.pop_out(1);
        p.window_back(0, false);
        assert_eq!(p.contents(), [Content::Active]);
        assert!(Panes::from_json("{\"dir\":\"Beside\",\"panes\":[{\"content\":\"Active\",\"weight\":1.0}]}").windows.is_empty(), "a layout saved before the windows existed is read");
    }

    struct Host {
        drew: Vec<Content>,
        click: Option<usize>,
    }

    impl PaneHost for Host {
        fn pane_choices(&self) -> Vec<(Content, String)> {
            vec![(Content::Active, "This".into()), (Content::Doc(1), "Other".into()), (Content::Preview, "Preview".into())]
        }
        fn draw_pane(&mut self, ui: &mut egui::Ui, content: Content, _view: &mut Option<View>) -> PaneEvent {
            self.drew.push(content);
            ui.label("x");
            self.click.take().map(PaneEvent::Activate).unwrap_or_default()
        }
    }

    #[test]
    fn one_pane_draws_only_the_content_and_several_draw_each_pane_once() {
        let ctx = egui::Context::default();
        let mut h = Host { drew: vec![], click: None };
        let mut p = Panes::default();
        ctx.run_ui(egui::RawInput::default(), |ui| {
            p.show(&mut h, ui);
        })
        .textures_delta
        .clear();
        assert_eq!(h.drew, [Content::Active]);
        p.split(Content::Doc(1));
        p.split(Content::Preview);
        h.drew.clear();
        h.click = Some(1);
        let mut got = PaneEvent::Nothing;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            got = p.show(&mut h, ui);
        })
        .textures_delta
        .clear();
        assert_eq!(h.drew, [Content::Active, Content::Doc(1), Content::Preview]);
        assert_eq!(got, PaneEvent::Activate(1));
    }
}
