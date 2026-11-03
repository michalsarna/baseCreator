//! Plugins in BaseCreator: the windows, the importers and the generators (the machinery is in baseCreator: `base_core::plugins`,
//! `base_ui::plugins_ui`). A plugin that changes G-code cannot be installed here, as this program has no G-code; one that another
//! program installed in the shared folder is shown as not for this program and can be removed.
use crate::{App, Screen};
use base_core::plugin_script::Shapes;
use base_ui::editor::EditorHost;
use base_ui::plugins_ui;
use std::path::PathBuf;

impl App {
    /// The windows of the plugins and the generators; what a generator makes is added to the drawing.
    pub(crate) fn plugin_windows(&mut self, ctx: &egui::Context) {
        if let Some(say) = plugins_ui::window(ctx, &mut self.plugins) {
            self.status = say;
            self.after_plugins_changed();
        }
        let area = self.work_size_mm();
        if let Some(s) = plugins_ui::generator_window(ctx, &mut self.plugins, area) {
            self.add_plugin_shapes(s, "the generator");
        }
        // An importer with a form and a preview waits in its own window.
        if let Some((name, s)) = plugins_ui::import_window(ctx, &mut self.plugins) {
            self.add_plugin_shapes(s, &name);
        }
    }

    /// Pictures that plugins bring join the gallery (once).
    pub(crate) fn after_plugins_changed(&mut self) {
        self.plugins.sync_clipart(&crate::data_paths().gallery);
        self.clip.lib.refresh();
    }

    /// Put shapes made by a plugin into the drawing: selected, and one undo takes them back.
    fn add_plugin_shapes(&mut self, s: Shapes, what: &str) {
        if self.screen == Screen::Start {
            self.new_design();
        }
        self.checkpoint();
        let n = s.scene.shapes.len();
        let last = crate::section::LAYER_COUNT - 1;
        let ids = self.scene.append(s.scene, last);
        self.ed.sel = ids.into_iter().collect();
        self.ed.node_sel.clear();
        self.touch_rev();
        self.ed.view.need_fit = true;
        self.status = format!("Added {n} shapes from {what}");
        if !s.warnings.is_empty() {
            self.status.push_str(&format!(" ({})", s.warnings.join("; ")));
        }
    }

    /// File > Import > Import with a plugin…
    pub(crate) fn import_with_plugin_dialog(&mut self) {
        let area = self.work_size_mm();
        match self.plugins.import_dialog(area) {
            Some((p, start)) => self.start_plugin_import(p, start),
            None => self.status = "No plugin reads files: install one in the Plugins window".into(),
        }
    }

    /// A file a plugin reads (by its extension): imported into the drawing. `false` when no plugin reads it.
    pub(crate) fn try_plugin_import(&mut self, path: &std::path::Path) -> bool {
        let area = self.work_size_mm();
        match self.plugins.import_file(path, area) {
            Some(start) => {
                self.start_plugin_import(path.to_path_buf(), start);
                true
            }
            None => false,
        }
    }

    /// The importer made the shapes at once, or opened its window (the shapes come from `plugin_windows` then).
    fn start_plugin_import(&mut self, path: PathBuf, start: plugins_ui::ImportStart) {
        match start {
            plugins_ui::ImportStart::Done(r) => self.finish_plugin_import(path, r),
            plugins_ui::ImportStart::Window => {
                // The window needs the editor behind it; the shapes are put in when Import is pressed.
                if self.screen == Screen::Start {
                    self.new_design();
                }
            }
        }
    }

    fn finish_plugin_import(&mut self, path: PathBuf, r: Result<Shapes, String>) {
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        match r {
            Ok(s) => self.add_plugin_shapes(s, &name),
            Err(e) => self.status = format!("Could not import {name}: {e}"),
        }
    }
}
