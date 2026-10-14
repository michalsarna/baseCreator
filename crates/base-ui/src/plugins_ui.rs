//! The plugins window and the generator window, the same in every program. The program owns a [`PluginsUi`], shows the
//! two windows each frame, and asks it to run importers and postprocessors (`base_core::plugins`).
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::{delete_button, import_button, popup};
use base_core::plugin_script::{self, Ctx, Shapes};
use base_core::plugins::{ParamKind, Registry};
use egui::RichText;
use std::path::{Path, PathBuf};

/// Menu labels (English; the menu translates them).
pub const MENU_PLUGINS: &str = "Plugins…";
pub const MENU_GENERATE: &str = "Generate shape…";
pub const MENU_IMPORT: &str = "Import with a plugin…";

pub struct PluginsUi {
    app: String,
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
        PluginsUi { app: app.to_string(), registry: Registry::load(base_core::plugins::plugins_dir(base)), open: false, gen_open: false, gen_sel: 0, values: Default::default(), post: None, confirm_remove: None, message: String::new(), log: vec![] }
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
        let bytes = match std::fs::read(path) {
            Ok(b) => b,
            Err(e) => return Some(Err(e.to_string())),
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
    if !st.open {
        return None;
    }
    let mut say = None;
    let mut open = st.open;
    let mut reload = false;
    let mut install: Option<PathBuf> = None;
    let mut remove: Option<String> = None;
    let mut toggle: Option<(String, bool)> = None;
    let dir = st.registry.dir().to_path_buf();
    popup(ctx, "plugins_window", &tr("Plugins"), [640.0, 560.0], &mut open, |ui| {
        ui.label(RichText::new(tr("Plugins run scripts that can create shapes and change G-code. Install only plugins you trust.")).color(theme::text_dim()));
        ui.add_space(4.0);
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
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
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
