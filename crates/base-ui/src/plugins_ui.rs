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
    let agent: ureq::Agent = ureq::Agent::config_builder().timeout_connect(Some(Duration::from_secs(8))).timeout_recv_response(Some(Duration::from_secs(20))).timeout_recv_body(Some(Duration::from_secs(60))).user_agent("baseCreator-plugins").build().into();
    let resp = agent.get(url).call().map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    resp.into_body().into_reader().take(max + 1).read_to_end(&mut buf).map_err(|e| e.to_string())?;
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
    /// An import that waits in its window (a form, a preview).
    pending: Option<Pending>,
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
        PluginsUi { app: app.to_string(), tab: Tab::Installed, net: NetState::default(), registry: Registry::load(base_core::plugins::plugins_dir(base)), open: false, gen_open: false, pending: None, gen_sel: 0, values: Default::default(), post: None, confirm_remove: None, message: String::new(), log: vec![] }
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

    /// Read a file with the importer of a plugin for its extension. `None` when no plugin reads this kind of file. An importer
    /// with a form or a preview opens a window instead of making the shapes at once (`import_window` draws it).
    pub fn import_file(&mut self, path: &Path, area: (f64, f64)) -> Option<ImportStart> {
        let ext = path.extension()?.to_string_lossy().to_lowercase();
        let (p, def) = self.registry.importer_for(&self.app, &ext)?;
        let (p, def) = (p.clone(), def.clone());
        // A file of a few hundred MB is not read into memory just to be refused by the script.
        let bytes = match base_core::plugins::read_limited(path, plugin_script::MAX_IMPORT_BYTES) {
            Ok(b) => b,
            Err(e) => return Some(ImportStart::Done(Err(e))),
        };
        let ctx = Ctx { app: self.app.clone(), area_w: area.0, area_h: area.1 };
        if !def.interactive() {
            return Some(ImportStart::Done(plugin_script::import(&p, &def, &bytes, &ctx)));
        }
        let extra = match plugin_script::form_for(&p, &def, &bytes, &ctx) {
            Ok(e) => e,
            Err(e) => return Some(ImportStart::Done(Err(e))),
        };
        let all: Vec<_> = def.params.iter().chain(extra.iter()).cloned().collect();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        let model = if def.view3d { Model3d::new(&bytes) } else { None };
        self.pending = Some(Pending { name, plugin: p, def, bytes: std::sync::Arc::new(bytes), extra, values: plugin_script::defaults_of(&all), ctx, version: 0, shown: u64::MAX, started: None, changed_at: 0.0, running: None, result: None, model });
        Some(ImportStart::Window)
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
    pub fn import_dialog(&mut self, area: (f64, f64)) -> Option<(PathBuf, ImportStart)> {
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

/// Draw the fields of a form and keep the values in `values`. Returns true when one was changed.
pub fn form_ui(ui: &mut egui::Ui, id: &str, registry: &Registry, params: &[base_core::plugins::ParamDef], values: &mut serde_json::Map<String, serde_json::Value>) -> bool {
    let mut changed = false;
    egui::Grid::new(id).num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
        for p in params {
            ui.label(pt(registry, if p.label.is_empty() { &p.id } else { &p.label }));
            match p.kind {
                ParamKind::Number => {
                    let mut v = values.get(&p.id).and_then(|v| v.as_f64()).unwrap_or(0.0);
                    let mut d = egui::DragValue::new(&mut v).speed(0.1).range(p.min.unwrap_or(f64::MIN)..=p.max.unwrap_or(f64::MAX));
                    if !p.unit.is_empty() {
                        d = d.suffix(format!(" {}", p.unit));
                    }
                    if ui.add(d).changed() {
                        values.insert(p.id.clone(), serde_json::json!(v));
                        changed = true;
                    }
                }
                ParamKind::Bool => {
                    let mut v = values.get(&p.id).and_then(|v| v.as_bool()).unwrap_or(false);
                    if ui.checkbox(&mut v, "").changed() {
                        values.insert(p.id.clone(), serde_json::json!(v));
                        changed = true;
                    }
                }
                ParamKind::Text => {
                    let mut v = values.get(&p.id).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    if ui.text_edit_singleline(&mut v).changed() {
                        values.insert(p.id.clone(), serde_json::json!(v));
                        changed = true;
                    }
                }
                ParamKind::Choice => {
                    let before = values.get(&p.id).and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let mut v = before.clone();
                    egui::ComboBox::from_id_salt(format!("{id}_{}", p.id)).selected_text(pt(registry, &v)).show_ui(ui, |ui| {
                        for o in &p.options {
                            ui.selectable_value(&mut v, o.clone(), pt(registry, o));
                        }
                    });
                    if v != before {
                        changed = true;
                    }
                    values.insert(p.id.clone(), serde_json::json!(v));
                }
            }
            ui.end_row();
        }
    });
    changed
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
        form_ui(ui, "gen_form", &st.registry, &def.params, &mut st.values);
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

/// What happened when a file was handed to an importer.
pub enum ImportStart {
    /// The importer made the shapes (or failed) at once.
    Done(Result<Shapes, String>),
    /// The importer has a form or a preview: its window is open (`import_window`).
    Window,
}

/// An import waiting in its window: the file, the importer, the form and the preview that is made in the background.
struct Pending {
    name: String,
    plugin: base_core::plugins::Plugin,
    def: base_core::plugins::ImporterDef,
    bytes: std::sync::Arc<Vec<u8>>,
    /// The fields the script made from the file.
    extra: Vec<base_core::plugins::ParamDef>,
    values: serde_json::Map<String, serde_json::Value>,
    ctx: Ctx,
    /// Counts the changes of the form; the preview shows the form of the version `shown`.
    version: u64,
    shown: u64,
    /// The version the running preview was started for.
    started: Option<u64>,
    /// When the form was last changed (the preview waits a moment for more).
    changed_at: f64,
    running: Option<Receiver<Result<Shapes, String>>>,
    result: Option<Result<Shapes, String>>,
    /// The model of the 3D view, for an importer with `view3d`.
    model: Option<Model3d>,
}

impl Pending {
    /// Import was pressed: the shapes that are shown, or, when the form changed after them, shapes made again.
    fn finish(self) -> Option<(String, Shapes)> {
        if self.shown == self.version {
            if let Some(Ok(s)) = self.result {
                return Some((self.name, s));
            }
        }
        plugin_script::import_with(&self.plugin, &self.def, &self.bytes, &self.values, &self.extra, &self.ctx).ok().map(|s| (self.name, s))
    }

    fn start(&mut self) {
        let (tx, rx) = channel();
        let (plugin, def, bytes, extra, values, ctx) = (self.plugin.clone(), self.def.clone(), self.bytes.clone(), self.extra.clone(), self.values.clone(), self.ctx.clone());
        std::thread::spawn(move || {
            let _ = tx.send(plugin_script::import_with(&plugin, &def, &bytes, &values, &extra, &ctx));
        });
        self.started = Some(self.version);
        self.running = Some(rx);
    }
}

/// The most triangles the 3D view draws (the biggest ones, when the model has more): it has no depth buffer, so they are sorted
/// by depth in every frame.
const MAX_DRAWN: usize = 80_000;

/// The model of a 3D view of an import: what is drawn, and how it is turned (`Mesh::rotated`: 0, 0 is the view from above).
struct Model3d {
    tris: Vec<[[f32; 3]; 3]>,
    normals: Vec<[f32; 3]>,
    center: [f32; 3],
    /// Half the diagonal of the box of the model: the scale does not change when the model is turned.
    radius: f32,
    yaw: f64,
    pitch: f64,
}

/// The views a button gives: the label, the turn about Z and the tilt.
const VIEWS: [(&str, f64, f64); 6] = [("Top", 0.0, 0.0), ("Bottom", 0.0, 180.0), ("Front", 0.0, 90.0), ("Back", 180.0, 90.0), ("Left", 90.0, 90.0), ("Right", -90.0, 90.0)];

/// An angle in degrees brought to -180…180.
fn wrap_deg(a: f64) -> f64 {
    let a = a.rem_euclid(360.0);
    if a > 180.0 {
        a - 360.0
    } else {
        a
    }
}

impl Model3d {
    fn new(bytes: &[u8]) -> Option<Model3d> {
        let mesh = base_core::mesh::Mesh::from_stl(bytes).ok()?;
        let b = mesh.bounds()?;
        let mut faces: Vec<(usize, [f64; 3], f64)> = (0..mesh.len()).filter_map(|i| mesh.normal_area(i).map(|(n, a)| (i, n, a))).collect();
        if faces.len() > MAX_DRAWN {
            faces.sort_by(|x, y| y.2.total_cmp(&x.2));
            faces.truncate(MAX_DRAWN);
        }
        let f3 = |v: [f64; 3]| [v[0] as f32, v[1] as f32, v[2] as f32];
        let tris = faces.iter().map(|(i, _, _)| mesh.tris[*i].map(f3)).collect();
        let normals = faces.iter().map(|(_, n, _)| f3(*n)).collect();
        let center = [((b[0] + b[3]) / 2.0) as f32, ((b[1] + b[4]) / 2.0) as f32, ((b[2] + b[5]) / 2.0) as f32];
        let radius = (((b[3] - b[0]).powi(2) + (b[4] - b[1]).powi(2) + (b[5] - b[2]).powi(2)).sqrt() / 2.0).max(1e-6) as f32;
        Some(Model3d { tris, normals, center, radius, yaw: 0.0, pitch: 0.0 })
    }

    fn is_view(&self, yaw: f64, pitch: f64) -> bool {
        (wrap_deg(self.yaw - yaw)).abs() < 0.5 && (wrap_deg(self.pitch - pitch)).abs() < 0.5
    }

    /// The model seen from above after the turn, as a mesh of flat triangles in `rect` (far ones first).
    fn paint(&self, painter: &egui::Painter, rect: egui::Rect) {
        let r = base_core::mesh::view_rot(self.yaw, self.pitch).map(|row| row.map(|v| v as f32));
        let turn = |p: [f32; 3]| [0, 1, 2].map(|i| r[i][0] * p[0] + r[i][1] * p[1] + r[i][2] * p[2]);
        let k = rect.width().min(rect.height()) * 0.45 / self.radius;
        let c = rect.center();
        let light = {
            let l = [-0.3f32, 0.4, 0.85];
            let n = (l[0] * l[0] + l[1] * l[1] + l[2] * l[2]).sqrt();
            l.map(|v| v / n)
        };
        let base = theme::accent();
        let mut order: Vec<(f32, usize, [egui::Pos2; 3])> = Vec::with_capacity(self.tris.len());
        for (i, t) in self.tris.iter().enumerate() {
            let q = t.map(|p| turn([p[0] - self.center[0], p[1] - self.center[1], p[2] - self.center[2]]));
            let depth = (q[0][2] + q[1][2] + q[2][2]) / 3.0;
            order.push((depth, i, q.map(|v| egui::pos2(c.x + v[0] * k, c.y - v[1] * k))));
        }
        order.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut mesh = egui::Mesh::default();
        mesh.reserve_triangles(order.len());
        mesh.reserve_vertices(order.len() * 3);
        for (_, i, pos) in order {
            let n = turn(self.normals[i]);
            // Both sides are lit the same way: a mesh with some triangles turned inside out is still readable.
            let lit = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).abs();
            let shade = 0.25 + 0.75 * lit;
            let color = egui::Color32::from_rgb((base.r() as f32 * shade) as u8, (base.g() as f32 * shade) as u8, (base.b() as f32 * shade) as u8);
            let at = mesh.vertices.len() as u32;
            for p in pos {
                mesh.vertices.push(egui::epaint::Vertex { pos: p, uv: egui::epaint::WHITE_UV, color });
            }
            mesh.indices.extend([at, at + 1, at + 2]);
        }
        painter.add(egui::Shape::mesh(mesh));
    }
}

/// The 3D view: the buttons of the views and the model, which turns when it is dragged. True when a new view was chosen (a button,
/// or the end of a drag), the moment to make the outlines again.
fn view3d_ui(ui: &mut egui::Ui, height: f32, m: &mut Model3d) -> bool {
    let mut chosen = false;
    ui.horizontal_wrapped(|ui| {
        for (label, yaw, pitch) in VIEWS {
            if ui.selectable_label(m.is_view(yaw, pitch), tr(label)).clicked() {
                (m.yaw, m.pitch) = (yaw, pitch);
                chosen = true;
            }
        }
    });
    let size = egui::vec2(ui.available_width(), height.max(60.0));
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::drag());
    if resp.dragged() {
        let d = resp.drag_delta();
        m.yaw = wrap_deg(m.yaw + d.x as f64 * 0.5);
        m.pitch = wrap_deg(m.pitch - d.y as f64 * 0.5);
    }
    if resp.drag_stopped() {
        chosen = true;
    }
    if resp.hovered() || resp.dragged() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::panel_dark());
    m.paint(&painter, rect);
    chosen
}

/// The box round some polylines.
fn box_of(polys: &[base_core::geom::Polyline]) -> Option<base_core::geom::Rect> {
    polys.iter().filter_map(|p| p.bounds()).reduce(|a, b| a.union(b))
}

/// Draw the shapes of a preview, fitted into `rect`.
fn draw_preview(ui: &egui::Ui, rect: egui::Rect, shapes: &Shapes) {
    let polys: Vec<base_core::geom::Polyline> = shapes.scene.shapes.iter().flat_map(|s| s.polys()).collect();
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 4.0, theme::panel_dark());
    let Some(b) = box_of(&polys) else { return };
    let (w, h) = (b.width().max(1e-9), b.height().max(1e-9));
    let k = ((rect.width() - 16.0) as f64 / w).min((rect.height() - 16.0) as f64 / h);
    let o = rect.center() - egui::vec2((w * k / 2.0) as f32, (h * k / 2.0) as f32);
    let to = |p: base_core::geom::Pt| egui::pos2(o.x + ((p.x - b.min.x) * k) as f32, o.y + ((p.y - b.min.y) * k) as f32);
    let stroke = egui::Stroke::new(1.0, theme::accent());
    // A very detailed outline is drawn with fewer points: the preview is only to look at.
    let step = (polys.iter().map(|p| p.pts.len()).sum::<usize>() / 20_000).max(1);
    for p in &polys {
        let pts: Vec<egui::Pos2> = p.pts.iter().step_by(step).map(|q| to(*q)).collect();
        if pts.len() >= 2 {
            painter.add(egui::Shape::line(if p.closed { pts.iter().copied().chain(pts.first().copied()).collect() } else { pts }, stroke));
        }
    }
}

/// The window of an import that has a form and a preview. Returns the name of the file and the shapes when Import was pressed.
pub fn import_window(ctx: &egui::Context, st: &mut PluginsUi) -> Option<(String, Shapes)> {
    let PluginsUi { registry, pending: slot, .. } = st;
    let registry: &Registry = registry;
    let pending = slot.as_mut()?;
    let now = ctx.input(|i| i.time);
    // The answer of the background run.
    if let Some(rx) = &pending.running {
        match rx.try_recv() {
            Ok(r) => {
                pending.shown = pending.started.unwrap_or(0);
                pending.result = Some(r);
                pending.running = None;
            }
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                pending.result = Some(Err("the preview stopped".into()));
                pending.running = None;
                pending.shown = pending.version;
            }
            Err(std::sync::mpsc::TryRecvError::Empty) => ctx.request_repaint_after(Duration::from_millis(100)),
        }
    }
    // A new run when the form changed and nothing has been changed for a moment.
    if pending.running.is_none() && pending.shown != pending.version {
        if now - pending.changed_at > 0.25 || pending.result.is_none() {
            pending.start();
        }
        ctx.request_repaint_after(Duration::from_millis(100));
    }
    let mut open = true;
    let (mut go, mut cancel) = (false, false);
    let title = trf("Import {}", &[&pending.name]);
    popup(ctx, "plugin_import_window", &title, [900.0, 640.0], &mut open, |ui| {
        if !pending.def.description.is_empty() {
            ui.label(RichText::new(pt(registry, &pending.def.description)).color(theme::text_dim()));
        }
        // The window can be resized: the form takes the height that is left above the buttons, the preview the rest of the width.
        let body_h = (ui.available_height() - 46.0).max(240.0);
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(330.0);
                let all: Vec<_> = pending.def.params.iter().chain(pending.extra.iter()).cloned().collect();
                egui::ScrollArea::vertical().id_salt("plugin_import_form").max_height(body_h).auto_shrink([false, false]).show(ui, |ui| {
                    if form_ui(ui, "plugin_import_grid", registry, &all, &mut pending.values) {
                        pending.version += 1;
                        pending.changed_at = now;
                    }
                });
            });
            ui.vertical(|ui| {
                let w = (ui.available_width() - 2.0).max(200.0);
                // Under the preview: "Working…", the size, the warnings. A fixed height, so that a long list of warnings does not make the
                // window grow (and the preview with it).
                let notes_h = 56.0;
                if let Some(m) = pending.model.as_mut() {
                    // The 3D view takes a part of the height; the outlines get what is left.
                    let view_h = ((body_h - notes_h - 70.0) * 0.5).max(80.0);
                    if view3d_ui(ui, view_h, m) {
                        pending.values.insert("view_yaw".into(), serde_json::json!(m.yaw));
                        pending.values.insert("view_pitch".into(), serde_json::json!(m.pitch));
                        pending.version += 1;
                        pending.changed_at = now;
                    }
                    ui.label(RichText::new(tr("Drag the model to turn it. The outlines below are what its view gives.")).color(theme::text_dim()));
                }
                let plot_h = (body_h - notes_h - ui.min_rect().height() - 2.0 * ui.spacing().item_spacing.y).max(80.0);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(w, plot_h), egui::Sense::hover());
                match &pending.result {
                    Some(Ok(s)) => draw_preview(ui, rect, s),
                    _ => {
                        ui.painter().rect_filled(rect, 4.0, theme::panel_dark());
                    }
                }
                egui::ScrollArea::vertical().id_salt("plugin_import_notes").max_height(notes_h).auto_shrink([false, false]).show(ui, |ui| {
                    if pending.running.is_some() || pending.shown != pending.version {
                        ui.label(RichText::new(tr("Working…")).color(theme::text_dim()));
                    }
                    match &pending.result {
                        Some(Ok(s)) => {
                            let polys: Vec<_> = s.scene.shapes.iter().flat_map(|x| x.polys()).collect();
                            if let Some(b) = box_of(&polys) {
                                ui.label(trf("{} shapes, {} × {} mm", &[&s.scene.shapes.len(), &format!("{:.1}", b.width()), &format!("{:.1}", b.height())]));
                            }
                            for w in &s.warnings {
                                ui.label(RichText::new(w).color(theme::accent()));
                            }
                        }
                        Some(Err(e)) => {
                            ui.label(RichText::new(trf("The script failed: {}", &[e])).color(theme::accent()));
                        }
                        None => {}
                    }
                });
            });
        });
        ui.separator();
        ui.horizontal(|ui| {
            let ready = matches!(&pending.result, Some(Ok(s)) if !s.scene.shapes.is_empty());
            go = ui.add_enabled(ready, egui::Button::new(RichText::new(tr("Import")).strong())).clicked();
            cancel = ui.button(tr("Cancel")).clicked();
        });
    });
    if go {
        return slot.take()?.finish();
    }
    if cancel || !open {
        *slot = None;
    }
    None
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
        let mut st = ui_with_examples();
        let dir = std::env::temp_dir().join(format!("bc-pui-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("a.csv");
        std::fs::write(&f, "0,0\n10,10\n").unwrap();
        let Some(ImportStart::Done(Ok(s))) = st.import_file(&f, (100.0, 100.0)) else { panic!("a plugin reads csv at once") };
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

    /// Against the real official list: `cargo test -p base-ui -- --ignored real_network` (needs the internet).
    #[test]
    #[ignore = "uses the internet"]
    fn real_network_the_official_list_and_a_plugin_install() {
        use base_core::plugins::Registry;
        let text = String::from_utf8(get(registry::INDEX_URL, registry::MAX_INDEX as u64).expect("the list")).unwrap();
        let index = registry::parse_index(&text).expect("a valid list");
        assert!(!index.plugins.is_empty());
        let dir = std::env::temp_dir().join(format!("bc-real-net-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut reg = Registry::load(dir.join("plugins"));
        for e in &index.plugins {
            let bytes = get(&e.url, registry::MAX_ZIP).expect("the zip");
            registry::install(&mut reg, e, &bytes).unwrap_or_else(|err| panic!("{}: {err}", e.id));
            assert_eq!(e.status(&reg), registry::Status::UpToDate);
        }
        assert!(reg.generators("millcreator").len() >= 3);
        std::fs::remove_dir_all(dir).ok();
    }

    /// A binary STL of two boxes: a plate 30 x 20 x 2 and a box 4 x 6 x 6 on it.
    fn two_boxes() -> Vec<u8> {
        let boxed = |x: f32, y: f32, z: f32, w: f32, d: f32, h: f32| {
            let v = |a: f32, b: f32, c: f32| [x + a * w, y + b * d, z + c * h];
            let quad = |a: [f32; 3], b: [f32; 3], c: [f32; 3], e: [f32; 3]| vec![[a, b, c], [a, c, e]];
            let mut t = vec![];
            t.extend(quad(v(0., 0., 0.), v(0., 1., 0.), v(1., 1., 0.), v(1., 0., 0.)));
            t.extend(quad(v(0., 0., 1.), v(1., 0., 1.), v(1., 1., 1.), v(0., 1., 1.)));
            t.extend(quad(v(0., 0., 0.), v(1., 0., 0.), v(1., 0., 1.), v(0., 0., 1.)));
            t.extend(quad(v(0., 1., 0.), v(0., 1., 1.), v(1., 1., 1.), v(1., 1., 0.)));
            t.extend(quad(v(0., 0., 0.), v(0., 0., 1.), v(0., 1., 1.), v(0., 1., 0.)));
            t.extend(quad(v(1., 0., 0.), v(1., 1., 0.), v(1., 1., 1.), v(1., 0., 1.)));
            t
        };
        let mut tris = boxed(0.0, 0.0, 0.0, 30.0, 20.0, 2.0);
        tris.extend(boxed(1.0, 4.0, 2.0, 4.0, 6.0, 6.0));
        let mut b = vec![0u8; 80];
        b.extend((tris.len() as u32).to_le_bytes());
        for t in &tris {
            b.extend([0u8; 12]);
            for p in t {
                for c in p {
                    b.extend(c.to_le_bytes());
                }
            }
            b.extend([0u8; 2]);
        }
        b
    }

    #[test]
    fn an_importer_with_a_form_opens_a_window_with_a_preview_made_in_the_background() {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base-core/tests/fixtures");
        let mut st = PluginsUi::new("pencreator", &base);
        assert!(st.registry.broken.is_empty(), "{:?}", st.registry.broken);
        let dir = std::env::temp_dir().join(format!("bc-stl-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("model.stl");
        std::fs::write(&f, two_boxes()).unwrap();
        assert!(matches!(st.import_file(&f, (300.0, 200.0)), Some(ImportStart::Window)), "a form: a window");
        let ctx = egui::Context::default();
        // The first runs start the preview; it arrives from another thread.
        let mut shapes_seen = 0;
        for i in 0..400 {
            let input = egui::RawInput { time: Some(i as f64 * 0.05), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                assert!(import_window(ui.ctx(), &mut st).is_none(), "nothing is imported until the button is pressed");
            });
            out.textures_delta.clear();
            if let Some(Pending { result: Some(Ok(s)), shown, version, .. }) = &st.pending {
                if shown == version {
                    shapes_seen = s.scene.shapes.len();
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        assert_eq!(shapes_seen, 1, "the shadow of the whole model is the plate");
        // A change of the form makes the preview again: the slice (not the default of this program) of the little box.
        {
            let p = st.pending.as_mut().unwrap();
            p.values.insert("mode".into(), serde_json::json!("Slice at a depth"));
            p.values.insert("depth".into(), serde_json::json!(5.0));
            p.version += 1;
            p.changed_at = -10.0;
        }
        for i in 0..400 {
            let input = egui::RawInput { time: Some(100.0 + i as f64 * 0.05), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                import_window(ui.ctx(), &mut st);
            });
            out.textures_delta.clear();
            if matches!(&st.pending, Some(p) if p.shown == p.version) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        let (name, s) = st.pending.take().unwrap().finish().expect("the shapes of the last form");
        assert_eq!(name, "model.stl");
        let b = s.scene.shapes[0].bounds().unwrap();
        assert!((b.width() - 4.0).abs() < 1e-3 && (b.height() - 6.0).abs() < 1e-3, "the slice through the little box only: {b:?}");
        std::fs::remove_dir_all(dir).ok();
    }

    /// An import window of the little STL model with the 3D view, with its first outlines made.
    fn stl_window(name: &str) -> (PluginsUi, egui::Context, std::path::PathBuf) {
        let base = Path::new(env!("CARGO_MANIFEST_DIR")).join("../base-core/tests/fixtures");
        let mut st = PluginsUi::new("pencreator", &base);
        let dir = std::env::temp_dir().join(format!("bc-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = dir.join("model.stl");
        std::fs::write(&f, two_boxes()).unwrap();
        assert!(matches!(st.import_file(&f, (300.0, 200.0)), Some(ImportStart::Window)));
        let ctx = egui::Context::default();
        let mut t = 0.0;
        wait_for_preview(&mut st, &ctx, &mut t);
        (st, ctx, dir)
    }

    /// Draw the window until the outlines of the last form have arrived. Returns the height of the window.
    fn wait_for_preview(st: &mut PluginsUi, ctx: &egui::Context, t: &mut f64) -> f32 {
        let mut height = 0.0;
        for _ in 0..600 {
            *t += 0.05;
            let input = egui::RawInput { time: Some(*t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                import_window(ui.ctx(), st);
            });
            out.textures_delta.clear();
            height = ctx.memory(|m| m.area_rect(egui::Id::new(Some("plugin_import_window")))).map(|r| r.height()).unwrap_or(0.0);
            if matches!(&st.pending, Some(p) if p.shown == p.version && p.result.is_some()) {
                break;
            }
            std::thread::sleep(Duration::from_millis(5));
        }
        height
    }

    #[test]
    fn the_3d_view_turns_the_model_and_the_outlines_follow_the_view() {
        let (mut st, ctx, dir) = stl_window("view3d");
        let mut t = 100.0;
        {
            let p = st.pending.as_mut().unwrap();
            let m = p.model.as_mut().expect("an importer with view3d has a model to show");
            assert!(m.is_view(0.0, 0.0), "it starts seen from above");
            // From above the shadow is the plate; from the front it is the plate seen edge on with the little box on it.
            let (_, yaw, pitch) = VIEWS.iter().copied().find(|v| v.0 == "Front").unwrap();
            (m.yaw, m.pitch) = (yaw, pitch);
            p.values.insert("view_yaw".into(), serde_json::json!(yaw));
            p.values.insert("view_pitch".into(), serde_json::json!(pitch));
            p.version += 1;
            p.changed_at = -10.0;
        }
        wait_for_preview(&mut st, &ctx, &mut t);
        let (_, s) = st.pending.take().unwrap().finish().expect("the outlines of the front view");
        let b = s.scene.shapes[0].bounds().unwrap();
        assert!((b.width() - 30.0).abs() < 1e-3 && (b.height() - 8.0).abs() < 1e-3, "30 wide, 2 + 6 high: {b:?}");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn the_import_window_keeps_its_size_while_the_view_is_turned_and_warnings_come() {
        let (mut st, ctx, dir) = stl_window("view3d-size");
        let mut t = 100.0;
        let first = wait_for_preview(&mut st, &ctx, &mut t);
        assert!(first > 300.0 && first <= 900.0, "{first}");
        // Many messages from the script, and a turned model: the height does not run away.
        {
            let p = st.pending.as_mut().unwrap();
            if let Some(Ok(s)) = p.result.as_mut() {
                s.warnings = (0..40).map(|i| format!("a long warning of the script number {i}, long enough to be wrapped in the narrow column")).collect();
            }
            let m = p.model.as_mut().unwrap();
            m.yaw = 33.0;
            m.pitch = 61.0;
        }
        let mut heights = vec![];
        for _ in 0..12 {
            t += 0.05;
            let input = egui::RawInput { time: Some(t), screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1400.0, 900.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |ui| {
                import_window(ui.ctx(), &mut st);
            });
            out.textures_delta.clear();
            heights.push(ctx.memory(|m| m.area_rect(egui::Id::new(Some("plugin_import_window")))).unwrap().height());
        }
        assert!(heights.windows(2).all(|w| (w[0] - w[1]).abs() < 0.5), "{heights:?}");
        assert!((heights[0] - first).abs() < 1.0, "{heights:?} after {first}");
        std::fs::remove_dir_all(dir).ok();
    }
}
