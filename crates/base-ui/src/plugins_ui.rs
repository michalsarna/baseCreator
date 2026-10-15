//! The plugins window and the generator window, the same in every program. The program owns a [`PluginsUi`], shows the
//! two windows each frame, and asks it to run importers and postprocessors (`base_core::plugins`).
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::{delete_button, import_button, popup};
use base_core::plugin_script::{self, Ctx, Shapes};
use base_core::plugins::{ParamKind, Registry};
use base_core::registry::{self, Entry, Index, Status};
use egui::RichText;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// Menu labels (English; the menu translates them).
pub const MENU_PLUGINS: &str = "Plugins…";
pub const MENU_GENERATE: &str = "Generate shape…";
pub const MENU_IMPORT: &str = "Import with a plugin…";

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Installed,
    Official,
}

enum Net {
    Index(Result<Index, String>),
    Zip(String, Result<Vec<u8>, String>),
}

#[derive(Default)]
struct NetState {
    rx: Option<Receiver<Net>>,
    index: Option<Index>,
    error: String,
    loading: bool,
    /// The id of the plugin being downloaded.
    busy: Option<String>,
    auto_done: bool,
    /// The list was asked for by the person (not by the check at the start).
    manual: bool,
}

fn get(url: &str, max: u64) -> Result<Vec<u8>, String> {
    let agent = ureq::AgentBuilder::new().timeout_connect(Duration::from_secs(8)).timeout_read(Duration::from_secs(20)).user_agent("baseCreator-plugins").build();
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_reader().take(max + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
    if buf.len() as u64 > max {
        return Err("the answer is too large".into());
    }
    Ok(buf)
}

pub struct PluginsUi {
    app: String,
    tab: Tab,
    net: NetState,
    pub registry: Registry,
    pub open: bool,
    pub gen_open: bool,
    gen_sel: usize,
    values: serde_json::Map<String, serde_json::Value>,
    /// The postprocessor chosen for the next export: `plugin-id/postprocessor-id`.
    pub post: Option<String>,
    confirm_remove: Option<String>,
    message: String,
    log: Vec<String>,
}

impl PluginsUi {
    /// `app` is `lightcreator`, `millcreator` or `pencreator`; `base` is the shared data folder.
    pub fn new(app: &str, base: &Path) -> PluginsUi {
        PluginsUi { app: app.to_string(), tab: Tab::Installed, net: NetState::default(), registry: Registry::load(base_core::plugins::plugins_dir(base)), open: false, gen_open: false, gen_sel: 0, values: Default::default(), post: None, confirm_remove: None, message: String::new(), log: vec![] }
    }

    pub fn reload(&mut self) {
        self.registry = Registry::load(self.registry.dir().to_path_buf());
        self.gen_sel = 0;
        self.values.clear();
        if let Some(p) = &self.post {
            if !self.registry.postprocessors(&self.app).iter().any(|(pl, d)| format!("{}/{}", pl.manifest.id, d.id) == *p) {
                self.post = None;
            }
        }
    }

    /// Read a file with the importer of a plugin for its extension. `None` when no plugin reads this kind of file.
    pub fn import_file(&self, path: &Path, area: (f64, f64)) -> Option<Result<Shapes, String>> {
        let ext = path.extension()?.to_string_lossy().to_lowercase();
        let (p, def) = self.registry.importer_for(&self.app, &ext)?;
        // A file of a few hundred MB is not read into memory just to be refused by the script.
        let bytes = match base_core::plugins::read_limited(path, plugin_script::MAX_IMPORT_BYTES) {
            Ok(b) => b,
            Err(e) => return Some(Err(e)),
        };
        Some(plugin_script::import(p, def, &bytes, &Ctx { app: self.app.clone(), area_w: area.0, area_h: area.1 }))
    }

    /// The extensions the importers of the plugins read, for a file dialog.
    pub fn import_extensions(&self) -> Vec<String> {
        self.registry.importers(&self.app).iter().flat_map(|(_, i)| i.extensions.iter().cloned()).collect()
    }

    /// Does a postprocessor stand between the program and the file? Returns the lines it makes (or the lines as they are).
    pub fn postprocess(&self, lines: Vec<String>, area: (f64, f64)) -> Result<(Vec<String>, Vec<String>), String> {
        let Some(sel) = &self.post else { return Ok((lines, vec![])) };
        let Some((p, d)) = self.registry.postprocessors(&self.app).into_iter().find(|(pl, d)| format!("{}/{}", pl.manifest.id, d.id) == *sel) else { return Ok((lines, vec![])) };
        plugin_script::postprocess(p, d, &lines, &Ctx { app: self.app.clone(), area_w: area.0, area_h: area.1 })
    }

    /// Ask the official list (in the background; the answer is read by `poll`).
    fn fetch_index(&mut self, ctx: &egui::Context, manual: bool) {
        if self.net.loading {
            return;
        }
        let (tx, rx) = channel();
        self.net.rx = Some(rx);
        self.net.loading = true;
        self.net.manual = manual;
        self.net.error.clear();
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = get(registry::INDEX_URL, registry::MAX_INDEX as u64).and_then(|b| String::from_utf8(b).map_err(|_| "the list is not text".to_string())).and_then(|t| registry::parse_index(&t));
            let _ = tx.send(Net::Index(r));
            ctx.request_repaint();
        });
    }

    fn fetch_zip(&mut self, ctx: &egui::Context, e: &Entry) {
        if self.net.busy.is_some() {
            return;
        }
        let (tx, rx) = channel();
        self.net.rx = Some(rx);
        self.net.busy = Some(e.id.clone());
        let (id, url) = (e.id.clone(), e.url.clone());
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let r = if registry::url_allowed(&url) { get(&url, registry::MAX_ZIP) } else { Err("this address is not official".into()) };
            let _ = tx.send(Net::Zip(id, r));
            ctx.request_repaint();
        });
    }

    /// Read what the background threads found out; returns a line for the status bar. Also starts the check at the start of
    /// the program when the person switched it on.
    pub fn poll(&mut self, ctx: &egui::Context) -> Option<String> {
        if !self.net.auto_done {
            self.net.auto_done = true;
            if self.registry.auto_check() {
                self.fetch_index(ctx, false);
            }
        }
        let mut said = None;
        let mut msgs = vec![];
        if let Some(rx) = &self.net.rx {
            while let Ok(m) = rx.try_recv() {
                msgs.push(m);
            }
        }
        for m in msgs {
            match m {
                Net::Index(r) => {
                    self.net.loading = false;
                    match r {
                        Ok(idx) => {
                            if !self.net.manual {
                                let n = idx.updates(&self.registry).len();
                                if n > 0 {
                                    said = Some(trf("Plugin updates available: {}", &[&n]));
                                }
                            }
                            self.net.index = Some(idx);
                        }
                        Err(e) => self.net.error = trf("Could not get the list: {}", &[&e]),
                    }
                }
                Net::Zip(id, r) => {
                    self.net.busy = None;
                    let entry = self.net.index.as_ref().and_then(|i| i.plugins.iter().find(|e| e.id == id)).cloned();
                    let was = self.registry.find(&id).map(|p| p.manifest.version.clone());
                    let done = match (r, entry) {
                        (Ok(bytes), Some(e)) => registry::install(&mut self.registry, &e, &bytes),
                        (Err(e), _) => Err(e),
                        (_, None) => Err("the plugin is not in the list any more".into()),
                    };
                    match done {
                        Ok(id) => {
                            self.reload();
                            said = Some(if was.is_some() { trf("Plugin updated: {}", &[&id]) } else { trf("Plugin installed: {}", &[&id]) });
                        }
                        Err(e) => said = Some(trf("Download failed: {}", &[&e])),
                    }
                    if let Some(s) = &said {
                        self.message = s.clone();
                    }
                }
            }
        }
        said
    }

    /// The name of the generator chosen in the generator window.
    pub fn last_generator_name(&self) -> String {
        let gens = self.registry.generators(&self.app);
        gens.get(self.gen_sel.min(gens.len().saturating_sub(1))).map(|(_, g)| pt(&self.registry, &g.name)).unwrap_or_default()
    }

    /// Add the clipart of the plugins to the shared gallery (once; safe to call again).
    pub fn sync_clipart(&self, gallery: &Path) -> usize {
        self.registry.sync_clipart(&self.app, gallery)
    }

    /// The text of a G-code file after the chosen postprocessor (the lines as they are when none is chosen). The second
    /// value is what the script said. The text ends with a newline.
    pub fn export_text(&self, text: &str, area: (f64, f64)) -> Result<(String, Vec<String>), String> {
        if self.post.is_none() {
            return Ok((text.to_string(), vec![]));
        }
        let (lines, said) = self.postprocess(text.lines().map(|l| l.to_string()).collect(), area)?;
        let mut out = lines.join("\n");
        out.push('\n');
        Ok((out, said))
    }

    /// The name of the chosen postprocessor, for a line of the status bar.
    pub fn post_name(&self) -> Option<String> {
        let sel = self.post.as_ref()?;
        self.registry.postprocessors(&self.app).into_iter().find(|(p, d)| format!("{}/{}", p.manifest.id, d.id) == *sel).map(|(_, d)| pt(&self.registry, &d.name))
    }

    /// Ask for a file one of the importers reads, and read it. `None` when the person cancels or no plugin imports.
    pub fn import_dialog(&self, area: (f64, f64)) -> Option<(PathBuf, Result<Shapes, String>)> {
        let exts = self.import_extensions();
        if exts.is_empty() {
            return None;
        }
        let refs: Vec<&str> = exts.iter().map(|e| e.as_str()).collect();
        let p = rfd::FileDialog::new().add_filter("Plugin", &refs).pick_file()?;
        let r = self.import_file(&p, area)?;
        Some((p, r))
    }

    /// A drop-down to choose the postprocessor of the next export.
    pub fn post_combo(&mut self, ui: &mut egui::Ui) {
        let posts: Vec<(String, String)> = self.registry.postprocessors(&self.app).iter().map(|(p, d)| (format!("{}/{}", p.manifest.id, d.id), d.name.clone())).collect();
        if posts.is_empty() {
            return;
        }
        let current = self.post.as_ref().and_then(|s| posts.iter().find(|(id, _)| id == s)).map(|(_, n)| n.clone()).unwrap_or_else(|| tr("None").to_string());
        ui.horizontal(|ui| {
            ui.label(tr("Postprocessor"));
            egui::ComboBox::from_id_salt("post_combo").selected_text(current).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.post, None, tr("None"));
                for (id, name) in &posts {
                    ui.selectable_value(&mut self.post, Some(id.clone()), name);
                }
            });
        });
    }
}

/// A text of a plugin (name, description, label) in the language of the program, when a plugin translates it.
fn pt(reg: &Registry, s: &str) -> String {
    reg.translate(crate::i18n::lang().code(), s).unwrap_or(s).to_string()
}

/// How tall a list in the window may be: a part of the screen. (A list that asks for "all the height there is" makes a window
/// grow by the spacing under it on every frame, up to the height of the screen.)
fn list_height(ui: &egui::Ui) -> f32 {
    (ui.ctx().content_rect().height() * 0.55).max(200.0)
}

fn open_folder(dir: &Path) {
    let _ = std::fs::create_dir_all(dir);
    let cmd = if cfg!(target_os = "macos") {
        "open"
    } else if cfg!(windows) {
        "explorer"
    } else {
        "xdg-open"
    };
    let _ = std::process::Command::new(cmd).arg(dir).spawn();
}

fn provides(m: &base_core::plugins::Manifest) -> Vec<String> {
    let mut v = vec![];
    for (name, n) in [("Presets", m.presets.len()), ("Clipart", m.clipart.len()), ("Translations", m.texts.len()), ("Data files", m.data.len()), ("Importers", m.importers.len()), ("Generators", m.generators.len()), ("Postprocessors", m.postprocessors.len())] {
        if n > 0 {
            v.push(format!("{}: {n}", tr(name)));
        }
    }
    v
}

/// The window with the installed plugins. Returns a line for the status bar when something was installed or removed.
pub fn window(ctx: &egui::Context, st: &mut PluginsUi) -> Option<String> {
    let mut say = st.poll(ctx);
    if !st.open {
        return say;
    }
    let mut open = st.open;
    let mut reload = false;
    let mut install: Option<PathBuf> = None;
    let mut remove: Option<String> = None;
    let mut toggle: Option<(String, bool)> = None;
    let dir = st.registry.dir().to_path_buf();
    let mut download: Option<Entry> = None;
    let mut check = false;
    let mut auto = st.registry.auto_check();
    popup(ctx, "plugins_window", &tr("Plugins"), [680.0, 600.0], &mut open, |ui| {
        ui.label(RichText::new(tr("Plugins run scripts that can create shapes and change G-code. Install only plugins you trust.")).color(theme::text_dim()));
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.selectable_value(&mut st.tab, Tab::Installed, tr("Installed"));
            ui.selectable_value(&mut st.tab, Tab::Official, tr("Official plugins"));
        });
        ui.separator();
        if st.tab == Tab::Official {
            official_tab(ui, st, &mut download, &mut check, &mut auto);
            return;
        }
        ui.horizontal(|ui| {
            if import_button(ui, &tr("Install plugin from zip…")).clicked() {
                install = rfd::FileDialog::new().add_filter("Zip", &["zip"]).pick_file();
            }
            if ui.button(tr("Open plugins folder")).clicked() {
                open_folder(&dir);
            }
            if ui.button(tr("Reload plugins")).clicked() {
                reload = true;
            }
        });
        st.post_combo(ui);
        if !st.message.is_empty() {
            ui.label(RichText::new(&st.message).color(theme::accent()));
        }
        ui.separator();
        egui::ScrollArea::vertical().max_height(list_height(ui)).auto_shrink([false, true]).show(ui, |ui| {
            if st.registry.plugins.is_empty() && st.registry.broken.is_empty() {
                ui.label(RichText::new(tr("No plugins yet. Put a plugin folder into the plugins folder or install a zip.")).color(theme::text_dim()));
            }
            for p in &st.registry.plugins {
                let m = &p.manifest;
                let for_here = m.is_for(&st.app);
                ui.horizontal(|ui| {
                    let mut on = p.enabled;
                    if ui.checkbox(&mut on, "").changed() {
                        toggle = Some((m.id.clone(), on));
                    }
                    ui.label(RichText::new(pt(&st.registry, &m.name)).strong());
                    if !m.version.is_empty() {
                        ui.label(RichText::new(trf("Version {}", &[&m.version])).color(theme::text_dim()));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if delete_button(ui, &tr("Remove")).clicked() {
                            st.confirm_remove = Some(m.id.clone());
                        }
                    });
                });
                if !m.description.is_empty() {
                    ui.label(RichText::new(pt(&st.registry, &m.description)).color(theme::text_dim()));
                }
                let mut meta = provides(m);
                if !m.author.is_empty() {
                    meta.push(m.author.clone());
                }
                if !m.license.is_empty() {
                    meta.push(m.license.clone());
                }
                if !for_here {
                    meta.push(tr("Not for this program").to_string());
                }
                ui.label(RichText::new(meta.join("  ·  ")).small().color(theme::text_dim()));
                if st.confirm_remove.as_deref() == Some(m.id.as_str()) {
                    ui.horizontal(|ui| {
                        ui.label(trf("Remove plugin {}? Its folder is deleted.", &[&m.name]));
                        if delete_button(ui, &tr("Remove")).clicked() {
                            remove = Some(m.id.clone());
                        }
                        if ui.button(tr("Cancel")).clicked() {
                            st.confirm_remove = None;
                        }
                    });
                }
                ui.separator();
            }
            if !st.registry.broken.is_empty() {
                ui.label(RichText::new(tr("Plugins that could not be read")).strong());
                for b in &st.registry.broken {
                    ui.label(RichText::new(format!("{}: {}", b.folder, b.reason)).color(egui::Color32::from_rgb(0xc0, 0x30, 0x30)));
                }
            }
        });
    });
    st.open = open;
    if auto != st.registry.auto_check() {
        let _ = st.registry.set_auto_check(auto);
    }
    if check {
        st.fetch_index(ctx, true);
    }
    if let Some(e) = download {
        st.fetch_zip(ctx, &e);
    }
    if let Some((id, on)) = toggle {
        if let Err(e) = st.registry.set_enabled(&id, on) {
            st.message = e;
        }
    }
    if let Some(zip) = install {
        match st.registry.install_zip(&zip) {
            Ok(id) => {
                st.message = trf("Plugin installed: {}", &[&id]);
                say = Some(st.message.clone());
                st.reload();
            }
            Err(e) => {
                st.message = trf("Plugin install failed: {}", &[&e]);
                say = Some(st.message.clone());
            }
        }
    }
    if let Some(id) = remove {
        st.confirm_remove = None;
        match st.registry.remove(&id) {
            Ok(()) => {
                st.message = trf("Plugin removed: {}", &[&id]);
                say = Some(st.message.clone());
                st.reload();
            }
            Err(e) => st.message = e,
        }
    }
    if reload {
        st.reload();
    }
    say
}

fn official_tab(ui: &mut egui::Ui, st: &mut PluginsUi, download: &mut Option<Entry>, check: &mut bool, auto: &mut bool) {
    ui.horizontal(|ui| {
        if import_button(ui, &tr("Check for updates")).clicked() {
            *check = true;
        }
        if ui.button(tr("Plugins repository")).clicked() {
            ui.ctx().open_url(egui::OpenUrl::new_tab(registry::REPO_URL));
        }
        ui.checkbox(auto, tr("Check for plugin updates at start"));
    });
    if st.net.loading {
        ui.label(RichText::new(tr("Checking the list…")).color(theme::text_dim()));
    }
    if !st.net.error.is_empty() {
        ui.label(RichText::new(&st.net.error).color(egui::Color32::from_rgb(0xc0, 0x30, 0x30)));
    }
    if !st.message.is_empty() {
        ui.label(RichText::new(&st.message).color(theme::accent()));
    }
    let Some(index) = &st.net.index else {
        ui.add_space(8.0);
        ui.label(RichText::new(tr("The list is not loaded yet. Press Check for updates.")).color(theme::text_dim()));
        return;
    };
    let updates = index.updates(&st.registry).len();
    ui.label(RichText::new(trf("{} plugins in the list, {} can be updated", &[&index.plugins.len(), &updates])).small().color(theme::text_dim()));
    ui.separator();
    egui::ScrollArea::vertical().max_height(list_height(ui)).auto_shrink([false, true]).show(ui, |ui| {
        let mut entries: Vec<&Entry> = index.plugins.iter().collect();
        entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
        for e in entries {
            let status = e.status(&st.registry);
            ui.horizontal(|ui| {
                ui.label(RichText::new(&e.name).strong());
                ui.label(RichText::new(format!("v{}", e.version)).color(theme::text_dim()));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let busy = st.net.busy.as_deref() == Some(e.id.as_str());
                    if busy {
                        ui.label(trf("Downloading {}…", &[&e.name]));
                        return;
                    }
                    match &status {
                        Status::NotInstalled => {
                            if import_button(ui, &tr("Install")).clicked() {
                                *download = Some(e.clone());
                            }
                        }
                        Status::Update { installed } => {
                            if import_button(ui, &tr("Update")).on_hover_text(&e.changelog).clicked() {
                                *download = Some(e.clone());
                            }
                            ui.label(RichText::new(trf("Update available: {} → {}", &[installed, &e.version])).color(theme::accent()));
                        }
                        Status::UpToDate | Status::Newer => {
                            ui.label(RichText::new(tr("Up to date")).color(theme::text_dim()));
                        }
                        Status::NeedsNewerProgram { min } => {
                            ui.label(RichText::new(trf("Needs program version {} or newer", &[min])).color(egui::Color32::from_rgb(0xc0, 0x30, 0x30)));
                        }
                    }
                });
            });
            if !e.description.is_empty() {
                ui.label(RichText::new(&e.description).color(theme::text_dim()));
            }
            let mut meta: Vec<String> = vec![];
            if !e.author.is_empty() {
                meta.push(e.author.clone());
            }
            if !e.license.is_empty() {
                meta.push(e.license.clone());
            }
            if !e.programs.is_empty() {
                meta.push(trf("For: {}", &[&e.programs.join(", ")]));
            }
            if !e.provides.is_empty() {
                meta.push(e.provides.join(", "));
            }
            if e.size > 0 {
                meta.push(format!("{:.0} kB", e.size as f64 / 1000.0));
            }
            ui.label(RichText::new(meta.join("  ·  ")).small().color(theme::text_dim()));
            ui.separator();
        }
    });
}

/// The window of the generators: choose one, fill in its form, Create. Returns the shapes when the script made them.
pub fn generator_window(ctx: &egui::Context, st: &mut PluginsUi, area: (f64, f64)) -> Option<Shapes> {
    if !st.gen_open {
        return None;
    }
    let gens: Vec<(String, String, String)> = st.registry.generators(&st.app).iter().map(|(p, g)| (p.manifest.id.clone(), g.id.clone(), g.name.clone())).collect();
    let mut out = None;
    let mut open = st.gen_open;
    let mut create = false;
    let mut changed_sel = false;
    popup(ctx, "generator_window", &tr("Generate shape…"), [460.0, 520.0], &mut open, |ui| {
        if gens.is_empty() {
            ui.label(RichText::new(tr("No generators installed. Plugins can add them (Plugins window).")).color(theme::text_dim()));
            return;
        }
        st.gen_sel = st.gen_sel.min(gens.len() - 1);
        ui.horizontal(|ui| {
            ui.label(tr("Generator"));
            egui::ComboBox::from_id_salt("gen_combo").selected_text(pt(&st.registry, &gens[st.gen_sel].2)).show_ui(ui, |ui| {
                for (i, g) in gens.iter().enumerate() {
                    if ui.selectable_value(&mut st.gen_sel, i, pt(&st.registry, &g.2)).changed() {
                        changed_sel = true;
                    }
                }
            });
        });
        let (pid, gid, _) = gens[st.gen_sel].clone();
        let Some(plugin) = st.registry.find(&pid) else { return };
        let Some(def) = plugin.manifest.generators.iter().find(|g| g.id == gid) else { return };
        if st.values.is_empty() || changed_sel {
            st.values = plugin_script::defaults(def);
        }
        if !def.description.is_empty() {
            ui.label(RichText::new(pt(&st.registry, &def.description)).color(theme::text_dim()));
        }
        ui.add_space(6.0);
        egui::Grid::new("gen_form").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            for p in &def.params {
                ui.label(pt(&st.registry, if p.label.is_empty() { &p.id } else { &p.label }));
                match p.kind {
                    ParamKind::Number => {
                        let mut v = st.values.get(&p.id).and_then(|v| v.as_f64()).unwrap_or(0.0);
                        let mut d = egui::DragValue::new(&mut v).speed(0.1).range(p.min.unwrap_or(f64::MIN)..=p.max.unwrap_or(f64::MAX));
                        if !p.unit.is_empty() {
                            d = d.suffix(format!(" {}", p.unit));
                        }
                        if ui.add(d).changed() {
                            st.values.insert(p.id.clone(), serde_json::json!(v));
                        }
                    }
                    ParamKind::Bool => {
                        let mut v = st.values.get(&p.id).and_then(|v| v.as_bool()).unwrap_or(false);
                        if ui.checkbox(&mut v, "").changed() {
                            st.values.insert(p.id.clone(), serde_json::json!(v));
                        }
                    }
                    ParamKind::Text => {
                        let mut v = st.values.get(&p.id).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        if ui.text_edit_singleline(&mut v).changed() {
                            st.values.insert(p.id.clone(), serde_json::json!(v));
                        }
                    }
                    ParamKind::Choice => {
                        let mut v = st.values.get(&p.id).and_then(|v| v.as_str()).unwrap_or("").to_string();
                        egui::ComboBox::from_id_salt(format!("gen_{}", p.id)).selected_text(&v).show_ui(ui, |ui| {
                            for o in &p.options {
                                ui.selectable_value(&mut v, o.clone(), o);
                            }
                        });
                        st.values.insert(p.id.clone(), serde_json::json!(v));
                    }
                }
                ui.end_row();
            }
        });
        ui.add_space(8.0);
        if import_button(ui, &tr("Create")).clicked() {
            create = true;
        }
        if !st.message.is_empty() {
            ui.label(RichText::new(&st.message).color(theme::accent()));
        }
        if !st.log.is_empty() {
            ui.label(RichText::new(tr("Messages of the script:")).strong());
            for l in &st.log {
                ui.label(l);
            }
        }
    });
    st.gen_open = open;
    if create && !gens.is_empty() {
        let (pid, gid, _) = gens[st.gen_sel].clone();
        if let Some(plugin) = st.registry.find(&pid) {
            if let Some(def) = plugin.manifest.generators.iter().find(|g| g.id == gid) {
                match plugin_script::generate(plugin, def, &st.values, &Ctx { app: st.app.clone(), area_w: area.0, area_h: area.1 }) {
                    Ok(s) => {
                        st.message = trf("Generated {} shapes", &[&s.scene.shapes.len()]);
                        st.log = s.warnings.clone();
                        out = Some(s);
                    }
                    Err(e) => {
                        st.message = trf("The script failed: {}", &[&e]);
                        st.log.clear();
                    }
                }
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ui_with_examples() -> PluginsUi {
        // The folder `examples` holds `plugins`, like the shared data folder does.
        PluginsUi::new("millcreator", &Path::new(env!("CARGO_MANIFEST_DIR")).join("../../examples"))
    }

    #[test]
    fn the_windows_draw_and_a_generator_makes_shapes() {
        let mut st = ui_with_examples();
        assert_eq!(st.registry.plugins.len(), 4, "{:?}", st.registry.broken);
        let ctx = egui::Context::default();
        st.open = true;
        st.gen_open = true;
        let mut made = None;
        for _ in 0..3 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                window(ui.ctx(), &mut st);
                made = made.take().or(generator_window(ui.ctx(), &mut st, (300.0, 200.0)));
            });
            out.textures_delta.clear();
        }
        assert!(made.is_none(), "nothing is made until Create is pressed");
    }

    #[test]
    fn the_official_tab_draws_a_list_with_its_states() {
        use base_core::registry::{Entry, Index};
        let mut st = ui_with_examples();
        let mk = |id: &str, version: &str, min: &str| Entry {
            id: id.into(),
            name: id.to_uppercase(),
            description: "d".into(),
            author: "a".into(),
            license: "MIT".into(),
            programs: vec!["millcreator".into()],
            provides: vec!["generators".into()],
            version: version.into(),
            min_base: min.into(),
            url: format!("{}zips/{id}.zip", registry::TRUSTED_PREFIX),
            sha256: "a".repeat(64),
            size: 5000,
            homepage: String::new(),
            changelog: "more".into(),
        };
        // `gcode-tools` is installed in 1.0.0: a newer one is an update; the other two are new or too new for the program.
        st.net.index = Some(Index { format: 1, updated: String::new(), plugins: vec![mk("gcode-tools", "1.1.0", ""), mk("fresh", "1.0.0", ""), mk("future", "1.0.0", "99.0.0")] });
        assert_eq!(st.net.index.as_ref().unwrap().updates(&st.registry).len(), 1);
        st.open = true;
        st.tab = Tab::Official;
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(Default::default(), |ui| {
                window(ui.ctx(), &mut st);
            });
            out.textures_delta.clear();
        }
    }

    /// The window keeps its size: a list that takes "all the height there is" made it grow to the height of the screen,
    /// and grow back after the person made it smaller.
    #[test]
    fn the_plugins_window_does_not_stretch_to_the_whole_height() {
        for official in [false, true] {
            let mut st = ui_with_examples();
            st.open = true;
            if official {
                st.tab = Tab::Official;
            }
            let ctx = egui::Context::default();
            let mut heights = vec![];
            for _ in 0..12 {
                let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1280.0, 900.0))), ..Default::default() };
                let mut out = ctx.run_ui(input, |ui| {
                    window(ui.ctx(), &mut st);
                });
                out.textures_delta.clear();
                if let Some(r) = ctx.memory(|m| m.area_rect(egui::Id::new(Some("plugins_window")))) {
                    heights.push(r.height());
                }
            }
            let last = *heights.last().expect("the window is shown");
            assert!(last < 700.0, "official={official}: {heights:?}");
            assert!((heights[4] - last).abs() < 1.0, "the size is stable: {heights:?}");
        }
    }

    #[test]
    fn files_are_read_by_the_plugin_of_their_extension() {
        let st = ui_with_examples();
        let dir = std::env::temp_dir().join(format!("bc-pui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.csv");
        std::fs::write(&f, "0,0\n10,10\n").unwrap();
        let s = st.import_file(&f, (100.0, 100.0)).expect("a plugin reads csv").unwrap();
        assert_eq!(s.scene.shapes.len(), 1);
        assert!(st.import_file(&dir.join("a.unknown"), (100.0, 100.0)).is_none());
        assert!(st.import_extensions().contains(&"csv".to_string()));
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_chosen_postprocessor_rewrites_the_program() {
        let mut st = ui_with_examples();
        let lines = vec!["G21".to_string(), "M5".to_string()];
        assert_eq!(st.postprocess(lines.clone(), (1.0, 1.0)).unwrap().0, lines, "none chosen: unchanged");
        st.post = Some("gcode-tools/line-numbers".into());
        assert_eq!(st.postprocess(lines, (1.0, 1.0)).unwrap().0, vec!["N10 G21".to_string(), "N20 M5".to_string()]);
    }
}
