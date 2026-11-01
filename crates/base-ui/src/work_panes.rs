//! The central area of a program divided into up to three panes, side by side or one above the other. Every pane has a bar of tabs
//! (no list to choose from): a tab is a drawing, the preview, the G-code of a project as text, the analysis of a G-code file or the
//! camera. A tab is dragged to another pane (or inside its bar to another place), or sent there from its menu (right click). The last
//! tab of a pane closed closes the pane. Nothing is opened twice: every tab is in one pane (or in a window of its own) only.
//!
//! One drawing is always the active one (it is what the tools, the menus and the keys work on); a drawing in another tab is shown
//! for looking only, with a zoom of its own, and a click makes it the active one.
//!
//! This module knows the layout and the tabs; what a tab shows and what closing it means is up to the program ([`PaneHost`]), as
//! with `EditorHost` and `TabHost`. With one pane (the default) the layout is the bar of tabs above the work area.
use crate::editor::View;
use crate::i18n::{tr, trf};
use crate::theme;
use egui::{Rect, Sense};
use serde::{Deserialize, Serialize};

/// The most panes at once.
pub const MAX_PANES: usize = 3;
/// The least size of a pane along the split, in points.
pub const MIN_PANE: f32 = 160.0;
const GAP: f32 = 6.0;

/// How the panes are placed.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Default, Hash)]
pub enum Dir {
    /// Next to each other.
    #[default]
    Beside,
    /// One above the other.
    Stacked,
}

/// What a tab shows.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize, Hash)]
pub enum Content {
    /// A drawing; the number is its recovery slot, which does not change while it is open.
    Doc(usize),
    /// The preview of the active drawing.
    Preview,
    /// The G-code of a project as text that can be edited; the number is the slot of the project.
    GcodeText(usize),
    /// The analysis of a G-code file.
    Gcode,
    Camera,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pane {
    pub tabs: Vec<Content>,
    /// The tab that is shown.
    #[serde(default)]
    pub sel: usize,
    /// The share of the room; the shares are compared with each other.
    pub weight: f32,
    /// The zoom and the shift of the other drawing that is shown (not saved).
    #[serde(skip)]
    pub view: Option<View>,
}

impl Pane {
    fn new(tabs: Vec<Content>) -> Pane {
        Pane { tabs, sel: 0, weight: 1.0, view: None }
    }

    /// The tab that is shown.
    pub fn shown(&self) -> Option<Content> {
        self.tabs.get(self.sel).copied()
    }
}

/// A tab taken out into a window of its own (on another monitor, for example). It stays until it is sent back or closed.
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
    /// The tabs that live in windows of their own.
    #[serde(default)]
    pub windows: Vec<Window>,
    /// The pane that was used last (a new tab goes there).
    #[serde(skip)]
    pub focus: usize,
    /// The active drawing the last time the layout looked, to show its tab when it changes.
    #[serde(skip)]
    seen_active: Option<usize>,
}

impl Default for Panes {
    fn default() -> Panes {
        Panes { dir: Dir::Beside, panes: vec![Pane::new(vec![])], windows: vec![], focus: 0, seen_active: None }
    }
}

impl Panes {
    /// One pane only.
    pub fn is_single(&self) -> bool {
        self.panes.len() <= 1
    }

    /// Every tab in the panes, pane by pane.
    pub fn tabs(&self) -> Vec<Content> {
        self.panes.iter().flat_map(|p| p.tabs.iter().copied()).collect()
    }

    /// The tabs of each pane.
    pub fn layout(&self) -> Vec<Vec<Content>> {
        self.panes.iter().map(|p| p.tabs.clone()).collect()
    }

    /// Where a tab is: the pane and the place in its bar (a tab in a window is not in a pane).
    pub fn locate(&self, c: Content) -> Option<(usize, usize)> {
        self.panes.iter().enumerate().find_map(|(i, p)| p.tabs.iter().position(|t| *t == c).map(|j| (i, j)))
    }

    /// Is the tab open anywhere: in a pane or in a window?
    pub fn has(&self, c: Content) -> bool {
        self.locate(c).is_some() || self.windows.iter().any(|w| w.content == c)
    }

    /// Show the tab (in its pane) and make its pane the one used last. `false` when the tab is not in a pane.
    pub fn select(&mut self, c: Content) -> bool {
        match self.locate(c) {
            Some((i, j)) => {
                if self.panes[i].sel != j {
                    self.panes[i].sel = j;
                    self.panes[i].view = None;
                }
                self.focus = i;
                true
            }
            None => false,
        }
    }

    /// Panes without tabs go (the others close up); a layout always has a pane.
    fn drop_empty_panes(&mut self) {
        let mut focus = self.focus;
        let mut i = 0;
        self.panes.retain(|p| {
            let keep = !p.tabs.is_empty();
            if !keep && i < self.focus {
                focus = focus.saturating_sub(1);
            }
            i += 1;
            keep
        });
        if self.panes.is_empty() {
            self.panes.push(Pane::new(vec![]));
        }
        self.focus = focus.min(self.panes.len() - 1);
        for p in &mut self.panes {
            p.sel = p.sel.min(p.tabs.len().saturating_sub(1));
        }
        if self.panes.iter().any(|p| !(p.weight.is_finite() && p.weight > 0.0)) {
            self.equalize();
        }
    }

    fn take_from_panes(&mut self, c: Content) {
        for p in &mut self.panes {
            if let Some(j) = p.tabs.iter().position(|t| *t == c) {
                p.tabs.remove(j);
                if p.sel > j || p.sel >= p.tabs.len() {
                    p.sel = p.sel.saturating_sub(1);
                }
                p.view = None;
            }
        }
    }

    /// Take a tab away from the panes and the windows (it was closed). A pane that has no tab left goes away and the others close up.
    pub fn remove(&mut self, c: Content) {
        self.take_from_panes(c);
        self.windows.retain(|w| w.content != c);
        let before = self.panes.len();
        self.drop_empty_panes();
        if self.panes.len() != before {
            self.equalize();
        }
    }

    /// Move a tab to the pane `to` (a number past the last pane makes a new one when there is room), at the place `at` of its bar (the
    /// end when `None`). The tab is shown there. `false` when it did not move.
    pub fn move_to(&mut self, c: Content, to: usize, at: Option<usize>) -> bool {
        let Some((from, j)) = self.locate(c) else { return false };
        if to >= self.panes.len() && (self.panes.len() >= MAX_PANES || self.panes[from].tabs.len() < 2) {
            // Three panes at most; a pane that holds only this tab cannot be split off from itself.
            return false;
        }
        if from == to {
            // Inside the bar: another place.
            let n = self.panes[from].tabs.len();
            let at = at.unwrap_or(n - 1).min(n - 1);
            let t = self.panes[from].tabs.remove(j);
            self.panes[from].tabs.insert(at, t);
            self.panes[from].sel = at;
            self.focus = from;
            return true;
        }
        self.take_from_panes(c);
        let target = if to >= self.panes.len() {
            self.panes.push(Pane::new(vec![]));
            self.panes.len() - 1
        } else {
            to
        };
        let n = self.panes[target].tabs.len();
        let at = at.unwrap_or(n).min(n);
        self.panes[target].tabs.insert(at, c);
        self.panes[target].sel = at;
        self.panes[target].view = None;
        // The pane that gave the tab may be empty now: it goes, and the numbers shift.
        self.drop_empty_panes();
        self.equalize();
        if let Some((i, _)) = self.locate(c) {
            self.focus = i;
        }
        true
    }

    /// Put the view in two panes: the last tab of the first pane goes to a new pane. `false` when there is no room or the first pane has
    /// only one tab (nothing to put in the other pane).
    pub fn split_off_one(&mut self) -> bool {
        let Some(first) = self.panes.first() else { return false };
        if self.panes.len() >= MAX_PANES || first.tabs.len() < 2 {
            return false;
        }
        let last = first.tabs[first.tabs.len() - 1];
        self.move_to(last, self.panes.len(), None)
    }

    /// Put all the tabs in one pane again.
    pub fn merge_all(&mut self) {
        let all = self.tabs();
        let shown = self.panes.get(self.focus).and_then(|p| p.shown());
        self.panes = vec![Pane::new(all)];
        self.focus = 0;
        if let Some(c) = shown {
            self.select(c);
        }
    }

    /// Take a tab out into a window of its own.
    pub fn pop_out(&mut self, c: Content) {
        if let Some((i, _)) = self.locate(c) {
            let view = self.panes[i].view.take();
            self.take_from_panes(c);
            self.drop_empty_panes();
            self.windows.push(Window { content: c, pos: None, size: [640.0, 480.0], view });
        }
    }

    /// Bring the tab of window `i` back into the pane used last (or just close the window).
    pub fn window_back(&mut self, i: usize, as_tab: bool) {
        if i < self.windows.len() {
            let w = self.windows.remove(i);
            if as_tab {
                let at = self.focus.min(self.panes.len() - 1);
                self.panes[at].tabs.push(w.content);
                self.panes[at].sel = self.panes[at].tabs.len() - 1;
            }
        }
    }

    /// Make the layout right after the world changed. `available` is every tab there is (open drawings, the preview, the camera…):
    /// a tab that is gone leaves, a new one comes into the pane used last and is shown, twice the same tab is once, a pane without tabs
    /// goes. `active` is the slot of the active drawing: when it changed, its tab is shown.
    pub fn normalize(&mut self, available: &[Content], active: usize) {
        let mut seen: Vec<Content> = vec![];
        for p in &mut self.panes {
            let mut j = 0;
            let mut sel = p.sel;
            p.tabs.retain(|t| {
                let keep = available.contains(t) && !seen.contains(t);
                if keep {
                    seen.push(*t);
                } else if j < p.sel {
                    sel = sel.saturating_sub(1);
                }
                j += 1;
                keep
            });
            p.sel = sel;
        }
        self.windows.retain(|w| {
            let keep = available.contains(&w.content) && !seen.contains(&w.content);
            if keep {
                seen.push(w.content);
            }
            keep
        });
        if self.panes.is_empty() {
            self.panes.push(Pane::new(vec![]));
        }
        let focus = self.focus.min(self.panes.len() - 1);
        let mut added = None;
        for c in available {
            if !seen.contains(c) {
                self.panes[focus].tabs.push(*c);
                added = Some(*c);
            }
        }
        let before = self.panes.len();
        self.drop_empty_panes();
        if self.panes.len() != before {
            self.equalize();
        }
        if let Some(c) = added {
            self.select(c);
        }
        if self.seen_active != Some(active) {
            if self.seen_active.is_some() {
                self.select(Content::Doc(active));
            }
            self.seen_active = Some(active);
        }
    }

    pub fn equalize(&mut self) {
        for p in &mut self.panes {
            p.weight = 1.0;
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
        let mut p = serde_json::from_str::<Panes>(s).ok().filter(|p| !p.panes.is_empty() && p.panes.len() <= MAX_PANES).unwrap_or_default();
        if p.panes.iter().any(|x| !(x.weight.is_finite() && x.weight > 0.0)) {
            p.equalize();
        }
        p
    }
}

/// What the program does for the panes.
pub trait PaneHost {
    /// The title of a tab.
    fn tab_title(&self, content: Content) -> String;
    /// Has the tab unsaved work (it is marked with a dot)?
    fn tab_dirty(&self, _content: Content) -> bool {
        false
    }
    /// Draw what the tab shows. `view` is the zoom and the shift of the pane for a drawing that is not the active one.
    fn draw_pane(&mut self, ui: &mut egui::Ui, content: Content, view: &mut Option<View>) -> PaneEvent;
    /// Can this tab be taken out into a window of its own?
    fn can_pop_out(&self, _content: Content) -> bool {
        false
    }
}

/// What happened in the panes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PaneEvent {
    #[default]
    Nothing,
    /// A drawing's tab (or the drawing shown for looking) was clicked: make it the active one.
    Activate(usize),
    /// The cross of a tab (or its menu): close it, asking about unsaved work first.
    Close(Content),
    /// The "+" of the pane `n`: a new drawing there.
    NewTab(usize),
}

/// A tab that is being dragged.
#[derive(Clone, Copy, Debug)]
struct Drag(Content);

impl Panes {
    /// Draw the panes in the rest of `ui`: for each its bar of tabs and what the shown tab shows.
    pub fn show<H: PaneHost>(&mut self, h: &mut H, ui: &mut egui::Ui) -> PaneEvent {
        let area = ui.available_rect_before_wrap();
        ui.allocate_rect(area, Sense::hover());
        let rects = self.rects(area, MIN_PANE);
        let bar_h = crate::doc_tabs::tab_height() + 4.0;
        let mut event = PaneEvent::Nothing;
        let mut moves: Vec<(Content, usize, Option<usize>)> = vec![];
        let mut pops: Vec<Content> = vec![];
        let n_panes = self.panes.len();
        for i in 0..n_panes {
            let r = rects[i];
            let bar = Rect::from_min_size(r.min, egui::vec2(r.width(), bar_h));
            let body = Rect::from_min_max(egui::pos2(r.left(), r.top() + bar_h), r.max);
            ui.painter().rect_filled(bar, 0.0, theme::panel());
            // The bar: the tabs, then the "+".
            let mut bui = ui.new_child(egui::UiBuilder::new().max_rect(bar.shrink2(egui::vec2(4.0, 2.0))).layout(egui::Layout::left_to_right(egui::Align::Max)));
            let mut rects_of_tabs: Vec<Rect> = vec![];
            let tabs = self.panes[i].tabs.clone();
            let sel = self.panes[i].sel;
            egui::ScrollArea::horizontal().id_salt(("pane_tabs", i)).scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).show(&mut bui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    for (j, c) in tabs.iter().enumerate() {
                        let (resp, closed) = crate::doc_tabs::tab_widget(ui, &(i, c), &h.tab_title(*c), h.tab_dirty(*c), j == sel, Sense::click_and_drag());
                        rects_of_tabs.push(resp.rect);
                        resp.dnd_set_drag_payload(Drag(*c));
                        if closed || resp.middle_clicked() {
                            event = PaneEvent::Close(*c);
                        } else if resp.clicked() {
                            self.panes[i].sel = j;
                            self.panes[i].view = None;
                            self.focus = i;
                            if let Content::Doc(slot) = c {
                                event = PaneEvent::Activate(*slot);
                            }
                        }
                        resp.context_menu(|ui| {
                            for target in 0..n_panes {
                                if target != i && ui.button(trf("Move to pane {}", &[&(target + 1)])).clicked() {
                                    moves.push((*c, target, None));
                                    ui.close();
                                }
                            }
                            // One more pane, when there is room and the pane has another tab to stay with.
                            if n_panes < MAX_PANES && tabs.len() > 1 && ui.button(trf("Move to pane {}", &[&(n_panes + 1)])).clicked() {
                                moves.push((*c, n_panes, None));
                                ui.close();
                            }
                            if h.can_pop_out(*c) {
                                ui.separator();
                                if ui.button(tr("Open in a separate window")).clicked() {
                                    pops.push(*c);
                                    ui.close();
                                }
                            }
                            ui.separator();
                            if ui.button(tr("Close tab")).clicked() {
                                event = PaneEvent::Close(*c);
                                ui.close();
                            }
                        });
                    }
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(crate::doc_tabs::tab_height(), crate::doc_tabs::tab_height()), Sense::click());
                    if resp.hovered() {
                        ui.painter().rect_filled(rect, 5.0, theme::panel_dark());
                    }
                    ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "+", egui::FontId::proportional(18.0), theme::text());
                    if resp.on_hover_text(tr("New tab")).clicked() {
                        self.focus = i;
                        event = PaneEvent::NewTab(i);
                    }
                });
            });
            // Where a dragged tab lands: on this bar (at the place of the pointer) or on the body (at the end).
            let bar_zone = ui.interact(bar, ui.make_persistent_id(("pane_bar_zone", i)), Sense::hover());
            let body_zone = ui.interact(body, ui.make_persistent_id(("pane_body_zone", i)), Sense::hover());
            let at_pointer = |zone: &egui::Response| {
                let x = zone.ctx.pointer_latest_pos().map_or(f32::MAX, |p| p.x);
                rects_of_tabs.iter().filter(|t| t.center().x < x).count()
            };
            if bar_zone.dnd_hover_payload::<Drag>().is_some() {
                let k = at_pointer(&bar_zone);
                let x = if k < rects_of_tabs.len() { rects_of_tabs[k].left() - 1.0 } else { rects_of_tabs.last().map_or(bar.left() + 4.0, |t| t.right() + 1.0) };
                ui.painter().line_segment([egui::pos2(x, bar.top() + 3.0), egui::pos2(x, bar.bottom() - 1.0)], egui::Stroke::new(2.0, theme::accent()));
            }
            if let Some(d) = bar_zone.dnd_release_payload::<Drag>() {
                moves.push((d.0, i, Some(at_pointer(&bar_zone))));
            } else if let Some(d) = body_zone.dnd_release_payload::<Drag>() {
                moves.push((d.0, i, None));
            }
            if body_zone.dnd_hover_payload::<Drag>().is_some() {
                ui.painter().rect_stroke(body.shrink(1.0), 0.0, egui::Stroke::new(2.0, theme::accent()), egui::StrokeKind::Inside);
            }
            // The body: what the shown tab shows.
            let mut cui = ui.new_child(egui::UiBuilder::new().max_rect(body));
            cui.set_clip_rect(body.intersect(ui.clip_rect()));
            let sel = self.panes[i].sel.min(self.panes[i].tabs.len().saturating_sub(1));
            if let Some(c) = self.panes[i].tabs.get(sel).copied() {
                let mut view = self.panes[i].view.take();
                let e = h.draw_pane(&mut cui, c, &mut view);
                self.panes[i].view = view;
                if e != PaneEvent::Nothing {
                    event = e;
                    self.focus = i;
                }
            }
        }
        // The dividers.
        let mut dragged = None;
        for i in 0..self.panes.len().saturating_sub(1) {
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
        for (c, to, at) in moves {
            self.move_to(c, to, at);
        }
        for c in pops {
            self.pop_out(c);
        }
        event
    }
}

impl Panes {
    /// Draw the windows of the tabs that were taken out (call it every frame, outside the panels). Closing a window, or its "Back to
    /// the main window" button, puts the tab back. Remembers where the window is.
    pub fn show_windows<H: PaneHost>(&mut self, h: &mut H, ctx: &egui::Context) -> PaneEvent {
        let mut event = PaneEvent::Nothing;
        let mut back: Option<usize> = None;
        for i in 0..self.windows.len() {
            let content = self.windows[i].content;
            let mut builder = egui::ViewportBuilder::default().with_title(h.tab_title(content)).with_inner_size(self.windows[i].size);
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
    use Content::*;

    fn area() -> Rect {
        Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(906.0, 400.0))
    }

    fn with(tabs: &[Content]) -> Panes {
        let mut p = Panes::default();
        p.normalize(tabs, 0);
        p
    }

    #[test]
    fn new_tabs_come_into_the_pane_used_last_and_are_shown_and_gone_ones_leave() {
        let mut p = with(&[Doc(0), Preview]);
        assert_eq!(p.layout(), vec![vec![Doc(0), Preview]]);
        assert_eq!(p.panes[0].shown(), Some(Preview), "the tab that came last is shown");
        p.normalize(&[Doc(0), Preview, Camera], 0);
        assert_eq!(p.panes[0].shown(), Some(Camera));
        // The preview is closed: it leaves and a neighbour is shown.
        p.normalize(&[Doc(0), Camera], 0);
        assert_eq!(p.layout(), vec![vec![Doc(0), Camera]]);
        assert_eq!(p.panes[0].shown(), Some(Camera));
        // Nothing is twice.
        p.panes[0].tabs.push(Doc(0));
        p.normalize(&[Doc(0), Camera], 0);
        assert_eq!(p.tabs(), vec![Doc(0), Camera]);
    }

    #[test]
    fn a_tab_goes_to_another_pane_and_the_last_tab_of_a_pane_closes_the_pane() {
        let mut p = with(&[Doc(0), Doc(1), Preview]);
        assert!(p.move_to(Preview, 1, None));
        assert_eq!(p.layout(), vec![vec![Doc(0), Doc(1)], vec![Preview]]);
        assert_eq!(p.focus, 1);
        // The other pane has one tab only: it cannot be split further from itself, but it can go to the first pane.
        assert!(!p.move_to(Preview, 2, None), "a pane that holds only this tab does not make another");
        assert!(p.move_to(Preview, 0, Some(1)));
        assert_eq!(p.layout(), vec![vec![Doc(0), Preview, Doc(1)]], "the pane that was left empty is gone");
        // A third pane, and no fourth.
        assert!(p.move_to(Doc(1), 1, None) && p.move_to(Preview, 2, None));
        assert_eq!(p.layout().len(), 3);
        p.panes[0].tabs.push(Camera);
        assert!(!p.move_to(Camera, 3, None), "three panes at most");
        // Closing the last tab of a pane closes the pane and the others close up.
        p.remove(Doc(1));
        assert_eq!(p.layout(), vec![vec![Doc(0), Camera], vec![Preview]]);
    }

    #[test]
    fn a_tab_is_dragged_to_a_place_inside_its_bar() {
        let mut p = with(&[Doc(0), Doc(1), Preview]);
        assert!(p.move_to(Doc(0), 0, Some(2)));
        assert_eq!(p.layout(), vec![vec![Doc(1), Preview, Doc(0)]]);
        assert_eq!(p.panes[0].shown(), Some(Doc(0)));
        assert!(p.move_to(Doc(0), 0, Some(0)));
        assert_eq!(p.layout()[0][0], Doc(0));
    }

    #[test]
    fn the_view_is_split_in_two_and_put_together_again() {
        let mut p = with(&[Doc(0)]);
        assert!(!p.split_off_one(), "one tab: nothing to put in the other pane");
        let mut p = with(&[Doc(0), Preview]);
        assert!(p.split_off_one());
        assert_eq!(p.layout(), vec![vec![Doc(0)], vec![Preview]]);
        p.merge_all();
        assert_eq!(p.layout(), vec![vec![Doc(0), Preview]]);
    }

    #[test]
    fn the_tab_of_the_drawing_that_became_active_is_shown() {
        let mut p = with(&[Doc(0), Doc(1), Preview]);
        p.normalize(&[Doc(0), Doc(1), Preview], 0);
        assert_eq!(p.panes[0].shown(), Some(Preview), "the preview was shown last");
        p.normalize(&[Doc(0), Doc(1), Preview], 1);
        assert_eq!(p.panes[0].shown(), Some(Doc(1)), "the active drawing changed: its tab is shown");
        p.move_to(Doc(0), 1, None);
        p.normalize(&[Doc(0), Doc(1), Preview], 0);
        assert_eq!(p.panes[1].shown(), Some(Doc(0)));
        assert_eq!(p.focus, 1);
    }

    #[test]
    fn a_divider_moves_two_panes_and_keeps_the_least_size() {
        let mut p = with(&[Doc(0), Preview]);
        p.split_off_one();
        p.drag(0, 100.0, area(), MIN_PANE);
        let r = p.rects(area(), MIN_PANE);
        assert!((r[0].width() - 550.0).abs() < 0.5, "{r:?}");
        p.drag(0, 5000.0, area(), MIN_PANE);
        let r = p.rects(area(), MIN_PANE);
        assert!((r[1].width() - MIN_PANE).abs() < 0.5 && (r[0].width() + r[1].width() + GAP - 906.0).abs() < 0.5);
        p.equalize();
        assert!(p.panes.iter().all(|x| x.weight == 1.0));
        p.dir = Dir::Stacked;
        let r = p.rects(area(), 10.0);
        assert!(r.iter().all(|r| r.width() == 906.0));
    }

    #[test]
    fn the_layout_is_saved_and_read_back_and_nonsense_gives_one_pane() {
        let mut p = with(&[Doc(0), Doc(1), Preview]);
        p.dir = Dir::Stacked;
        p.move_to(Preview, 1, None);
        p.drag(0, 40.0, area(), MIN_PANE);
        let q = Panes::from_json(&p.to_json());
        assert_eq!(q.dir, Dir::Stacked);
        assert_eq!(q.layout(), p.layout());
        assert!((q.panes[0].weight - p.panes[0].weight).abs() < 1e-3);
        assert!(Panes::from_json("").is_single() && Panes::from_json("{\"dir\":1}").is_single() && Panes::from_json("{\"dir\":\"Beside\",\"panes\":[]}").is_single());
        // A saved layout that names a drawing that is not open any more is mended when it meets the real tabs.
        let mut q = Panes::from_json(&p.to_json());
        q.normalize(&[Doc(1), Preview], 1);
        assert!(!q.has(Doc(0)));
        assert!(q.has(Doc(1)) && q.has(Preview));
    }

    #[test]
    fn a_tab_goes_out_into_a_window_and_comes_back() {
        let mut p = with(&[Doc(0), Preview, Camera]);
        p.pop_out(Preview);
        assert_eq!(p.layout(), vec![vec![Doc(0), Camera]]);
        assert!(p.has(Preview));
        // It is not put into a pane again while it lives in a window.
        p.normalize(&[Doc(0), Preview, Camera], 0);
        assert_eq!(p.layout(), vec![vec![Doc(0), Camera]]);
        p.window_back(0, true);
        assert!(p.locate(Preview).is_some() && p.windows.is_empty());
        p.pop_out(Camera);
        p.normalize(&[Doc(0), Preview], 0);
        assert!(p.windows.is_empty(), "a window of a tab that is gone goes too");
        let q = Panes::from_json(&Panes { windows: vec![Window { content: Preview, pos: Some([2000.0, 100.0]), size: [640.0, 480.0], view: None }], ..Panes::default() }.to_json());
        assert_eq!(q.windows[0].pos, Some([2000.0, 100.0]));
    }

    struct Host {
        drew: Vec<Content>,
        click: Option<usize>,
    }

    impl PaneHost for Host {
        fn tab_title(&self, c: Content) -> String {
            format!("{c:?}")
        }
        fn draw_pane(&mut self, ui: &mut egui::Ui, content: Content, _view: &mut Option<View>) -> PaneEvent {
            self.drew.push(content);
            ui.label("x");
            self.click.take().map(PaneEvent::Activate).unwrap_or_default()
        }
    }

    #[test]
    fn every_pane_draws_its_shown_tab_once_and_a_click_on_a_drawing_is_reported() {
        let ctx = egui::Context::default();
        let mut h = Host { drew: vec![], click: None };
        let mut p = with(&[Doc(0), Preview, Camera]);
        ctx.run_ui(egui::RawInput::default(), |ui| {
            p.show(&mut h, ui);
        })
        .textures_delta
        .clear();
        assert_eq!(h.drew, [Camera], "one pane: the tab that is shown");
        p.move_to(Preview, 1, None);
        p.move_to(Doc(0), 2, None);
        h.drew.clear();
        h.click = Some(0);
        let mut got = PaneEvent::Nothing;
        ctx.run_ui(egui::RawInput::default(), |ui| {
            got = p.show(&mut h, ui);
        })
        .textures_delta
        .clear();
        assert_eq!(h.drew.len(), 3);
        assert!(h.drew.contains(&Camera) && h.drew.contains(&Preview) && h.drew.contains(&Doc(0)));
        assert_eq!(got, PaneEvent::Activate(0));
    }
}
