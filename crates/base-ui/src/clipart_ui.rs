//! The clipart gallery: a side-panel tab with the pictures shipped with the program (blue dot) and those
//! you saved or downloaded (green dot), sorted into categories, plus a window to search the internet for more.
use crate::clip_net::{ClipNet, Reply, Req};
use crate::editor::EditorHost;
use crate::i18n::{tr, trf};
use crate::theme;
use crate::units_ui::drag_len;
use crate::window::{delete_button, popup, save_button, save_button_enabled};
use base_core::clipart::iconify::{self, Hit};
use base_core::clipart::{Clip, Library, DEFAULT_CATEGORY};
use base_core::geom::Pt;
use egui::{Color32, RichText};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

include!(concat!(env!("OUT_DIR"), "/clipart_files.rs"));

/// (category, name, SVG bytes) of pictures shipped with a program.
pub type Builtin = &'static [(&'static str, &'static str, &'static [u8])];

/// The translated title of the gallery's side panel tab.
pub fn tab_label() -> &'static str {
    tr("Clipart")
}

pub const BLUE: Color32 = Color32::from_rgb(0x3b, 0x82, 0xd8);
pub const GREEN: Color32 = Color32::from_rgb(0x2e, 0xa0, 0x4f);
const TILE: f32 = 70.0;

pub enum ClipDialog {
    Rename { id: String, name: String },
    Move { id: String, choice: String, new_cat: String },
    Info { id: String },
    Delete { id: String },
}

pub struct ClipState {
    pub lib: Library,
    /// The pictures that come with the program.
    pub builtin: Builtin,
    /// SVG bytes of the user's pictures, for thumbnails.
    pub user_svgs: HashMap<String, Arc<[u8]>>,
    pub filter: String,
    /// `None` shows every category.
    pub category: Option<String>,
    pub show_builtin: bool,
    pub show_user: bool,
    /// Size in millimetres of the longer side of a placed picture.
    pub size_mm: f64,
    pub dialog: Option<ClipDialog>,
    pub online: Option<OnlineDlg>,
    /// Categories folded up to their title line.
    pub collapsed: std::collections::HashSet<String>,
    /// When the index file was last read; another program sharing the folder may change it.
    stamp: Option<std::time::SystemTime>,
}

impl ClipState {
    /// The gallery over a library folder, with the pictures that ship with every program built on base-ui.
    pub fn new(lib: Library) -> ClipState {
        Self::with_builtin(lib, BUILTIN_CLIPART)
    }

    /// The same with another set of included pictures.
    pub fn with_builtin(lib: Library, builtin: Builtin) -> ClipState {
        let mut s = ClipState { lib, builtin, user_svgs: HashMap::new(), filter: String::new(), category: None, show_builtin: true, show_user: true, size_mm: 50.0, dialog: None, online: None, collapsed: Default::default(), stamp: None };
        s.stamp = s.index_time();
        s.reload();
        s
    }

    fn index_time(&self) -> Option<std::time::SystemTime> {
        std::fs::metadata(self.lib.dir().join("index.json")).and_then(|m| m.modified()).ok()
    }

    /// Pick up pictures that another program added to or removed from the shared gallery.
    pub fn sync(&mut self) {
        let t = self.index_time();
        if t != self.stamp {
            self.stamp = t;
            self.lib.refresh();
            self.reload();
        }
    }

    /// Read the pictures of the library from disk (for thumbnails).
    pub fn reload(&mut self) {
        self.user_svgs = self.lib.items.iter().filter_map(|c| self.lib.svg(&c.id).map(|s| (c.id.clone(), Arc::from(s.into_bytes().into_boxed_slice())))).collect();
    }

    /// All categories: those of the shipped pictures and those of the library, translated names first.
    pub fn categories(&self) -> Vec<String> {
        let mut v: Vec<String> = self.builtin.iter().map(|c| c.0.to_string()).collect();
        v.extend(self.lib.categories());
        v.sort_by_key(|s| tr_owned(s).to_lowercase());
        v.dedup();
        v
    }

    pub fn user_categories(&self) -> Vec<String> {
        let mut v = self.lib.categories();
        if !v.iter().any(|c| c == DEFAULT_CATEGORY) {
            v.push(DEFAULT_CATEGORY.to_string());
        }
        v.sort_by_key(|s| s.to_lowercase());
        v
    }
}

/// Translate a category name when it is one of the built-in English names.
pub fn tr_owned(s: &str) -> String {
    match s {
        "Animals" => tr("Animals"),
        "Nature" => tr("Nature"),
        "Celebrations" => tr("Celebrations"),
        "Symbols" => tr("Symbols"),
        "Shapes" => tr("Shapes"),
        "Tools" => tr("Tools"),
        "Food" => tr("Food"),
        "Objects" => tr("Objects"),
        "Music and fun" => tr("Music and fun"),
        "My clipart" => tr("My clipart"),
        other => return other.to_string(),
    }
    .to_string()
}

pub enum Source {
    Builtin(usize),
    User(String),
}

pub struct Tile {
    source: Source,
    name: String,
    category: String,
}

pub enum Action {
    Place(Vec<u8>, String),
    Dialog(ClipDialog),
}

fn light_tile(ui: &mut egui::Ui, size: f32) -> (egui::Rect, egui::Response) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click());
    ui.painter().rect_filled(rect, 6.0, Color32::from_rgb(0xea, 0xea, 0xed));
    if resp.hovered() {
        ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.0, theme::accent()), egui::StrokeKind::Inside);
    }
    (rect, resp)
}

// ---------------------------------------------------------------------------------------------------------
// Online search

pub struct OnlineDlg {
    pub query: String,
    pub hits: Vec<Hit>,
    pub thumbs: HashMap<String, Arc<[u8]>>,
    requested: HashSet<String>,
    pub sel: Option<usize>,
    pub name: String,
    pub choice: String,
    pub new_cat: String,
    pub url: String,
    pub status: String,
    pub busy: bool,
    /// The search the current results belong to; answers to older searches are ignored.
    sent_query: String,
    /// What a download from an address is going to be saved as.
    pending_url: Option<(String, String, String)>,
    pub net: ClipNet,
}

impl OnlineDlg {
    pub fn new(ctx: &egui::Context, base: &str) -> OnlineDlg {
        OnlineDlg {
            query: String::new(),
            hits: vec![],
            thumbs: HashMap::new(),
            requested: HashSet::new(),
            sel: None,
            name: String::new(),
            choice: DEFAULT_CATEGORY.to_string(),
            new_cat: String::new(),
            url: String::new(),
            status: String::new(),
            busy: false,
            sent_query: String::new(),
            pending_url: None,
            net: ClipNet::spawn(ctx.clone(), base),
        }
    }
}

/// The clipart gallery. The application keeps a `ClipState` and says where it is.
pub trait ClipHost: EditorHost {
    fn clip(&self) -> &ClipState;
    fn clip_mut(&mut self) -> &mut ClipState;
}

/// The gallery itself, for every `ClipHost`.
pub trait ClipGallery: ClipHost {

    /// Place a picture on the active layer at the middle of the work area.
    fn place_clip(&mut self, svg: &[u8], label: &str) {
        let (bw, bh) = self.work_size();
        self.checkpoint();
        let (layer, size) = (self.ed().active_layer, self.clip().size_mm);
        match base_core::clipart::import_clip(svg, self.scene_mut(), layer, size, Pt::new(bw / 2.0, bh / 2.0)) {
            Ok(ids) => {
                self.touch();
                self.ed_mut().sel = ids;
                self.say(trf("Placed {}", &[&label]));
            }
            Err(e) => {
                self.drop_checkpoint();
                self.say(trf("Import failed: {}", &[&e]));
            }
        }
    }

    fn clipart_panel(&mut self, ui: &mut egui::Ui) {
        self.clip_mut().sync();
        let units = self.units();
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut self.clip_mut().filter).hint_text(tr("Search")).desired_width(130.0));
            if ui.button(tr("Online…")).on_hover_text(tr("Search the internet for more pictures and save them here")).clicked() {
                let ctx = ui.ctx().clone();
                self.open_online_clipart(&ctx);
            }
            if ui.button(tr("Add file…")).on_hover_text(tr("Add SVG files from your computer to your clipart")).clicked() {
                self.add_clip_files();
            }
        });
        ui.horizontal(|ui| {
            let current = self.clip_mut().category.clone();
            egui::ComboBox::from_id_salt("clip_cat").width(150.0).selected_text(current.as_deref().map(tr_owned).unwrap_or_else(|| tr("All categories").to_string())).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.clip_mut().category, None, tr("All categories"));
                for c in self.clip_mut().categories() {
                    ui.selectable_value(&mut self.clip_mut().category, Some(c.clone()), tr_owned(&c));
                }
            });
            ui.label(RichText::new("●").color(BLUE));
            ui.checkbox(&mut self.clip_mut().show_builtin, tr("Included")).on_hover_text(tr("Pictures that come with the program"));
            ui.label(RichText::new("●").color(GREEN));
            ui.checkbox(&mut self.clip_mut().show_user, tr("Mine")).on_hover_text(tr("Pictures you downloaded or added"));
        });
        ui.horizontal(|ui| {
            ui.label(tr("Size when placed"));
            drag_len(ui, units, &mut self.clip_mut().size_mm, 1.0, Some((2.0, 2000.0)));
        });
        ui.separator();
        // Pictures that match the search and the filters.
        let q = self.clip_mut().filter.to_lowercase();
        let mut tiles: Vec<Tile> = vec![];
        if self.clip_mut().show_builtin {
            for (i, (cat, name, _)) in self.clip().builtin.iter().enumerate() {
                tiles.push(Tile { source: Source::Builtin(i), name: name.to_string(), category: cat.to_string() });
            }
        }
        if self.clip_mut().show_user {
            for c in &self.clip_mut().lib.items {
                tiles.push(Tile { source: Source::User(c.id.clone()), name: c.name.clone(), category: c.category.clone() });
            }
        }
        tiles.retain(|t| self.clip_mut().category.as_ref().map_or(true, |c| *c == t.category) && (q.is_empty() || t.name.to_lowercase().contains(&q) || tr_owned(&t.category).to_lowercase().contains(&q)));
        tiles.sort_by(|a, b| (tr_owned(&a.category).to_lowercase(), a.name.to_lowercase()).cmp(&(tr_owned(&b.category).to_lowercase(), b.name.to_lowercase())));
        let mut action: Option<Action> = None;
        let mut collapsed = std::mem::take(&mut self.clip_mut().collapsed);
        egui::ScrollArea::vertical().max_height(crate::window::fill_size(ui).y).auto_shrink([false, false]).show(ui, |ui| {
            if tiles.is_empty() {
                ui.label(RichText::new(tr("No pictures here. Use Online… to find some.")).color(theme::text_dim()));
            }
            let mut last_cat = String::new();
            let mut row: Vec<&Tile> = vec![];
            let flush = |ui: &mut egui::Ui, row: &mut Vec<&Tile>, cat: &str, app: &Self, action: &mut Option<Action>, collapsed: &mut std::collections::HashSet<String>| {
                if row.is_empty() {
                    return;
                }
                ui.add_space(4.0);
                // Title line with an arrow: click it to fold the category up or open it again.
                let open = !collapsed.contains(cat);
                let (rect, resp) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 22.0), egui::Sense::click());
                if resp.hovered() {
                    ui.painter().rect_filled(rect, 4.0, theme::panel_dark());
                }
                let arrow = egui::Rect::from_center_size(egui::pos2(rect.left() + 11.0, rect.center().y), egui::vec2(14.0, 14.0));
                crate::icons::paint(ui, arrow, if open { "chevron-down" } else { "chevron-right" }, theme::text());
                ui.painter().text(
                    egui::pos2(rect.left() + 24.0, rect.center().y),
                    egui::Align2::LEFT_CENTER,
                    format!("{}  ({})", tr_owned(cat), row.len()),
                    egui::FontId::proportional(13.5),
                    theme::text(),
                );
                if resp.on_hover_text(tr(if open { "Fold up" } else { "Unfold" })).clicked() {
                    if open {
                        collapsed.insert(cat.to_string());
                    } else {
                        collapsed.remove(cat);
                    }
                }
                if open {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                        for t in row.iter() {
                            app.draw_tile(ui, t, action);
                        }
                    });
                }
                row.clear();
            };
            for t in &tiles {
                if t.category != last_cat {
                    flush(ui, &mut row, &last_cat.clone(), self, &mut action, &mut collapsed);
                    last_cat = t.category.clone();
                }
                row.push(t);
            }
            flush(ui, &mut row, &last_cat.clone(), self, &mut action, &mut collapsed);
        });
        self.clip_mut().collapsed = collapsed;
        match action {
            Some(Action::Place(bytes, label)) => self.place_clip(&bytes, &label),
            Some(Action::Dialog(d)) => self.clip_mut().dialog = Some(d),
            None => {}
        }
    }

    fn draw_tile(&self, ui: &mut egui::Ui, t: &Tile, action: &mut Option<Action>) {
        let (rect, resp) = light_tile(ui, TILE);
        let (bytes, uri, dot): (Option<egui::load::Bytes>, String, Color32) = match &t.source {
            Source::Builtin(i) => (Some(egui::load::Bytes::Static(self.clip().builtin[*i].2)), format!("bytes://clip/b/{i}.svg"), BLUE),
            Source::User(id) => (self.clip().user_svgs.get(id).map(|b| egui::load::Bytes::Shared(b.clone())), format!("bytes://clip/u/{id}.svg"), GREEN),
        };
        if let Some(bytes) = bytes.clone() {
            egui::Image::from_bytes(uri, bytes).fit_to_exact_size(egui::vec2(TILE - 16.0, TILE - 16.0)).paint_at(ui, rect.shrink(8.0));
        }
        ui.painter().circle_filled(rect.min + egui::vec2(9.0, 9.0), 4.5, dot);
        let tip = match &t.source {
            Source::Builtin(_) => format!("{}\n{} · {}", t.name, tr_owned(&t.category), tr("comes with the program")),
            Source::User(id) => {
                let c = self.clip().lib.items.iter().find(|c| c.id == *id);
                format!("{}\n{} · {}{}", t.name, tr_owned(&t.category), tr("yours"), c.filter(|c| !c.license.is_empty()).map(|c| format!(" · {}", c.license)).unwrap_or_default())
            }
        };
        let resp = resp.on_hover_text(format!("{tip}\n{}", tr("Click to place on the work area")));
        if resp.clicked() {
            if let Some(b) = bytes {
                *action = Some(Action::Place(b.to_vec(), t.name.clone()));
            }
        }
        if let Source::User(id) = &t.source {
            resp.context_menu(|ui| {
                if ui.button(tr("Rename…")).clicked() {
                    *action = Some(Action::Dialog(ClipDialog::Rename { id: id.clone(), name: t.name.clone() }));
                    ui.close();
                }
                if ui.button(tr("Change category…")).clicked() {
                    *action = Some(Action::Dialog(ClipDialog::Move { id: id.clone(), choice: t.category.clone(), new_cat: String::new() }));
                    ui.close();
                }
                if ui.button(tr("Details…")).clicked() {
                    *action = Some(Action::Dialog(ClipDialog::Info { id: id.clone() }));
                    ui.close();
                }
                ui.separator();
                if delete_button(ui, &tr("Delete…")).clicked() {
                    *action = Some(Action::Dialog(ClipDialog::Delete { id: id.clone() }));
                    ui.close();
                }
            });
        } else {
            resp.context_menu(|ui| {
                if save_button(ui, &tr("Save a copy in my clipart")).clicked() {
                    if let Source::Builtin(i) = &t.source {
                        *action = Some(Action::Dialog(ClipDialog::Move { id: format!("builtin:{i}"), choice: DEFAULT_CATEGORY.to_string(), new_cat: String::new() }));
                    }
                    ui.close();
                }
            });
        }
    }

    /// Choose SVG files and add them to the user's clipart.
    fn add_clip_files(&mut self) {
        let Some(files) = rfd::FileDialog::new().add_filter("SVG", &["svg"]).pick_files() else { return };
        let cat = self.clip_mut().category.clone().filter(|c| self.clip_mut().user_categories().contains(c)).unwrap_or_else(|| DEFAULT_CATEGORY.to_string());
        let mut n = 0;
        for f in files {
            let name = f.file_stem().map(|s| s.to_string_lossy().replace(['-', '_'], " ")).unwrap_or_default();
            match std::fs::read_to_string(&f).map_err(|e| e.to_string()).and_then(|s| self.clip_mut().lib.add(&s, &name, &cat, "file", "", "")) {
                Ok(_) => n += 1,
                Err(e) => self.say(trf("Import failed: {}", &[&e])),
            }
        }
        self.clip_mut().reload();
        if n > 0 {
            self.say(trf("Added {} picture(s) to {}", &[&n, &tr_owned(&cat)]));
        }
    }

    /// Dialogs for pictures of the user's library: rename, move to another category, details, delete.
    fn clip_dialogs(&mut self, ctx: &egui::Context) {
        let Some(mut d) = self.clip_mut().dialog.take() else { return };
        let mut keep = true;
        let cats = self.clip_mut().user_categories();
        match &mut d {
            ClipDialog::Rename { id, name } => {
                let mut done = false;
                popup(ctx, "clip_rename", &tr("Rename"), [300.0, 120.0], &mut keep, |ui| {
                    ui.add(egui::TextEdit::singleline(name).desired_width(220.0));
                    done = ui.button(tr("OK")).clicked();
                });
                if done {
                    let _ = self.clip_mut().lib.rename(id, name);
                    keep = false;
                }
            }
            ClipDialog::Move { id, choice, new_cat } => {
                let mut done = false;
                let title = if id.starts_with("builtin:") { tr("Save a copy in my clipart") } else { tr("Change category") };
                popup(ctx, "clip_move", &title, [320.0, 180.0], &mut keep, |ui| {
                    egui::ComboBox::from_id_salt("move_cat").width(200.0).selected_text(tr_owned(choice)).show_ui(ui, |ui| {
                        for c in &cats {
                            ui.selectable_value(choice, c.clone(), tr_owned(c));
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label(tr("or a new category"));
                        ui.add(egui::TextEdit::singleline(new_cat).desired_width(120.0));
                    });
                    done = ui.button(tr("OK")).clicked();
                });
                if done {
                    let cat = if new_cat.trim().is_empty() { choice.clone() } else { new_cat.trim().to_string() };
                    if let Some(i) = id.strip_prefix("builtin:").and_then(|n| n.parse::<usize>().ok()) {
                        if let Some((_, name, svg)) = self.clip().builtin.get(i) {
                            let r = self.clip_mut().lib.add(&String::from_utf8_lossy(svg), name, &cat, "Material Design Icons", "Apache-2.0", "Pictogrammers");
                            self.say(match r {
                                Ok(_) => trf("Added {} picture(s) to {}", &[&1, &tr_owned(&cat)]),
                                Err(e) => trf("Import failed: {}", &[&e]),
                            });
                        }
                    } else {
                        let _ = self.clip_mut().lib.set_category(id, &cat);
                    }
                    self.clip_mut().reload();
                    keep = false;
                }
            }
            ClipDialog::Info { id } => {
                let c: Option<Clip> = self.clip_mut().lib.items.iter().find(|c| c.id == *id).cloned();
                popup(ctx, "clip_details", &tr("Details"), [320.0, 220.0], &mut keep, |ui| {
                    if let Some(c) = &c {
                        egui::Grid::new("clip_info").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
                            for (k, v) in [(tr("Name"), c.name.clone()), (tr("Category"), tr_owned(&c.category)), (tr("Licence"), c.license.clone()), (tr("Author"), c.author.clone())] {
                                ui.label(RichText::new(k).color(theme::text_dim()));
                                ui.label(v);
                                ui.end_row();
                            }
                            ui.label(RichText::new(tr("Source")).color(theme::text_dim()));
                            if c.source.starts_with("http") {
                                ui.hyperlink(&c.source);
                            } else {
                                ui.label(&c.source);
                            }
                            ui.end_row();
                        });
                    }
                });
            }
            ClipDialog::Delete { id } => {
                let mut yes = false;
                let mut no = false;
                popup(ctx, "clip_delete", &tr("Delete…"), [320.0, 160.0], &mut keep, |ui| {
                    ui.label(tr("Remove this picture from your clipart? The file on your disk is deleted."));
                    ui.horizontal(|ui| {
                        yes = delete_button(ui, &tr("Delete")).clicked();
                        no = ui.button(tr("Cancel")).clicked();
                    });
                });
                if yes {
                    let _ = self.clip_mut().lib.remove(id);
                    self.clip_mut().reload();
                }
                if yes || no {
                    keep = false;
                }
            }
        }
        if keep {
            self.clip_mut().dialog = Some(d);
        }
    }

    fn open_online_clipart(&mut self, ctx: &egui::Context) {
        if self.clip_mut().online.is_none() {
            self.clip_mut().online = Some(OnlineDlg::new(ctx, iconify::API));
        }
    }

    fn online_clipart_window(&mut self, ctx: &egui::Context) {
        let Some(mut o) = self.clip_mut().online.take() else { return };
        // Answers from the network.
        while let Ok(r) = o.net.rx.try_recv() {
            match r {
                Reply::Hits { query, result } => {
                    if query != o.sent_query {
                        continue;
                    }
                    o.busy = false;
                    match result {
                        Ok(h) => {
                            o.status = if h.is_empty() { tr("Nothing found.").to_string() } else { trf("{} pictures found", &[&h.len()]) };
                            o.hits = h;
                            o.thumbs.clear();
                            o.requested.clear();
                            o.sel = None;
                        }
                        Err(e) => o.status = trf("Search failed: {}", &[&e]),
                    }
                }
                Reply::Svg { key, result } => {
                    if let Some(url) = key.strip_prefix("url:") {
                        if let Some((u, name, cat)) = o.pending_url.take().filter(|p| p.0 == url) {
                            o.busy = false;
                            let saved = result.and_then(|b| base_core::clipart::normalize_svg(&String::from_utf8_lossy(&b)).and_then(|s| self.clip_mut().lib.add(&s, &name, &cat, &u, "", "")));
                            o.status = match saved {
                                Ok(_) => trf("Saved {} in {}", &[&name, &tr_owned(&cat)]),
                                Err(e) => trf("Download failed: {}", &[&e]),
                            };
                            self.clip_mut().reload();
                        }
                    } else if let Ok(b) = result {
                        o.thumbs.insert(key, Arc::from(b.into_boxed_slice()));
                    }
                }
            }
        }
        // Ask for the thumbnails of new results.
        for h in &o.hits {
            let key = h.key();
            if o.requested.insert(key.clone()) {
                o.net.send(Req::Fetch { key, url: h.svg_url(o.net.base()) });
            }
        }
        let mut open = true;
        let cats = self.clip_mut().user_categories();
        let mut search = false;
        let mut save_hit = false;
        let mut save_url = false;
        popup(ctx, "w_clipart_ui_8", &tr("Online clipart"), [340.0, 220.0], &mut open, |ui| {
            ui.label(RichText::new(tr("Search open icon collections (Iconify). Pictures are saved on your disk, in a category of your choice.")).color(theme::text_dim()));
            ui.horizontal(|ui| {
                let r = ui.add(egui::TextEdit::singleline(&mut o.query).hint_text(tr("e.g. cat, flower, gear")).desired_width(260.0));
                search = ui.add_enabled(!o.query.trim().is_empty() && !o.busy, egui::Button::new(tr("Search"))).clicked() || (r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) && !o.query.trim().is_empty());
                if o.busy {
                    ui.spinner();
                }
            });
            if !o.status.is_empty() {
                ui.label(RichText::new(&o.status).color(theme::text_dim()));
            }
            ui.separator();
            egui::ScrollArea::vertical().max_height(300.0).auto_shrink([false, false]).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
                    for (i, h) in o.hits.iter().enumerate() {
                        let (rect, resp) = light_tile(ui, TILE);
                        if let Some(b) = o.thumbs.get(&h.key()) {
                            egui::Image::from_bytes(format!("bytes://clip/o/{}.svg", h.key()), egui::load::Bytes::Shared(b.clone())).fit_to_exact_size(egui::vec2(TILE - 16.0, TILE - 16.0)).tint(Color32::BLACK).paint_at(ui, rect.shrink(8.0));
                        } else {
                            ui.put(egui::Rect::from_center_size(rect.center(), egui::vec2(16.0, 16.0)), egui::Spinner::new());
                        }
                        if !h.needs_no_credit() {
                            ui.painter().circle_filled(rect.right_top() + egui::vec2(-9.0, 9.0), 4.5, Color32::from_rgb(0xe0, 0x90, 0x20));
                        }
                        if o.sel == Some(i) {
                            ui.painter().rect_stroke(rect, 6.0, egui::Stroke::new(2.5, theme::accent()), egui::StrokeKind::Inside);
                        }
                        if resp.on_hover_text(format!("{} · {} · {}", h.name, h.set_name, h.license_title)).clicked() {
                            o.sel = Some(i);
                            o.name = h.name.replace(['-', '_'], " ");
                        }
                    }
                });
            });
            ui.separator();
            if let Some(h) = o.sel.and_then(|i| o.hits.get(i)).cloned() {
                egui::Grid::new("online_detail").num_columns(2).spacing([12.0, 5.0]).show(ui, |ui| {
                    ui.label(tr("Name"));
                    ui.add(egui::TextEdit::singleline(&mut o.name).desired_width(200.0));
                    ui.end_row();
                    ui.label(tr("Collection"));
                    ui.label(format!("{} ({})", h.set_name, h.prefix));
                    ui.end_row();
                    ui.label(tr("Licence"));
                    ui.horizontal(|ui| {
                        if h.license_url.is_empty() {
                            ui.label(&h.license_title);
                        } else {
                            ui.hyperlink_to(&h.license_title, &h.license_url);
                        }
                        if h.needs_no_credit() {
                            ui.label(RichText::new(tr("no credit needed")).color(GREEN));
                        } else {
                            ui.label(RichText::new(tr("credit the author")).color(Color32::from_rgb(0xe0, 0x90, 0x20)));
                        }
                    });
                    ui.end_row();
                    ui.label(tr("Author"));
                    ui.label(&h.author);
                    ui.end_row();
                    ui.label(tr("Category"));
                    ui.horizontal(|ui| {
                        egui::ComboBox::from_id_salt("online_cat").width(150.0).selected_text(tr_owned(&o.choice)).show_ui(ui, |ui| {
                            for c in &cats {
                                ui.selectable_value(&mut o.choice, c.clone(), tr_owned(c));
                            }
                        });
                        ui.add(egui::TextEdit::singleline(&mut o.new_cat).hint_text(tr("or a new category")).desired_width(120.0));
                    });
                    ui.end_row();
                });
                save_hit = save_button_enabled(ui, o.thumbs.contains_key(&h.key()) && !o.name.trim().is_empty(), &tr("Save to my clipart")).clicked();
            } else {
                ui.label(RichText::new(tr("Click a picture to see its licence and save it.")).color(theme::text_dim()));
            }
            ui.add_space(6.0);
            ui.separator();
            ui.label(RichText::new(tr("Or download a picture from an address")).strong());
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut o.url).hint_text("https://example.org/picture.svg").desired_width(330.0));
                save_url = ui.add_enabled(!o.url.trim().is_empty() && !o.busy, egui::Button::new(tr("Download"))).clicked();
            });
            ui.label(RichText::new(tr("Only save pictures you may use. Check the licence of the source.")).color(theme::text_dim()));
        });
        let new_cat = o.new_cat.trim().to_string();
        let target_cat = if new_cat.is_empty() { o.choice.clone() } else { new_cat };
        if search {
            o.busy = true;
            o.status = tr("Searching…").to_string();
            o.sent_query = o.query.trim().to_string();
            o.net.send(Req::Search { query: o.sent_query.clone() });
        }
        if save_hit {
            if let (Some(h), Some(bytes)) = (o.sel.and_then(|i| o.hits.get(i)).cloned(), o.sel.and_then(|i| o.hits.get(i)).and_then(|h| o.thumbs.get(&h.key())).cloned()) {
                let svg = String::from_utf8_lossy(&bytes).to_string();
                let r = self.clip_mut().lib.add(&svg, o.name.trim(), &target_cat, &h.page_url(), if h.license_spdx.is_empty() { &h.license_title } else { &h.license_spdx }, &h.author);
                o.status = match r {
                    Ok(_) => trf("Saved {} in {}", &[&o.name.trim(), &tr_owned(&target_cat)]),
                    Err(e) => trf("Download failed: {}", &[&e]),
                };
                self.clip_mut().reload();
                self.clip_mut().show_user = true;
            }
        }
        if save_url {
            let url = o.url.trim().to_string();
            let stem = url.rsplit('/').next().unwrap_or("picture").split(['?', '#']).next().unwrap_or("picture").trim_end_matches(".svg").replace(['-', '_'], " ");
            let name = if stem.trim().is_empty() { "picture".to_string() } else { stem };
            o.busy = true;
            o.status = tr("Downloading…").to_string();
            o.pending_url = Some((url.clone(), name, target_cat));
            o.net.send(Req::Fetch { key: format!("url:{url}"), url });
        }
        if open {
            self.clip_mut().online = Some(o);
        }
    }
}

impl<T: ClipHost> ClipGallery for T {}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::scene::Scene;
    use std::collections::HashSet;

    #[test]
    fn every_shipped_picture_places_cleanly() {
        assert!(BUILTIN_CLIPART.len() >= 300, "{} pictures", BUILTIN_CLIPART.len());
        let cats: HashSet<&str> = BUILTIN_CLIPART.iter().map(|c| c.0).collect();
        assert_eq!(cats.len(), 9, "{cats:?}");
        for (cat, name, svg) in BUILTIN_CLIPART {
            let mut d = Scene::default();
            let ids = base_core::clipart::import_clip(svg, &mut d, 2, 40.0, Pt::new(100.0, 100.0)).unwrap_or_else(|e| panic!("{cat}/{name}: {e}"));
            assert!(!ids.is_empty(), "{cat}/{name}");
        }
    }
}
