//! Help > Download backup… and Restore from backup…: the settings and the shared gallery in one zip (see `base_core::backup`).
use crate::section::Page;
use crate::{data_paths, i18n, theme, App, Lang, Scheme};
use base_ui::backup_ui;
use eframe::egui;
use std::collections::BTreeMap;
use std::path::PathBuf;

impl App {
    /// Ask where to put the backup and write it.
    pub(crate) fn download_backup(&mut self) {
        let _ = data_paths();
        if let Some(line) = backup_ui::download("BaseCreator", env!("CARGO_PKG_VERSION"), &self.settings_pairs(), self.clip.lib.dir()) {
            self.status = line;
        }
    }

    /// Choose a backup file, ask, and restore from it.
    pub(crate) fn restore_backup_dialog(&mut self, ctx: &egui::Context) {
        let Some(src) = backup_ui::pick_restore(true) else { return };
        let result = base_core::backup::restore(&src, "BaseCreator", self.clip.lib.dir());
        if let Ok(r) = &result {
            self.apply_settings(ctx, &r.settings);
            self.clip.lib.refresh();
            self.clip.reload();
        }
        self.status = backup_ui::restore_status(&src, &result);
    }

    /// Take the settings of a backup (the strings of `settings_pairs`) into the running program.
    pub(crate) fn apply_settings(&mut self, ctx: &egui::Context, m: &BTreeMap<String, String>) {
        let get = |k: &str| m.get(k).map(String::as_str);
        if let Some(l) = get("lang").and_then(Lang::from_code) {
            self.lang = l;
            i18n::set_lang(l);
        }
        if let Some(s) = get("scheme").and_then(Scheme::from_id) {
            self.scheme = s;
            theme::apply(ctx, s);
        }
        if let Some(v) = get("grid_main_on") {
            self.ed.show_grid = v == "1";
        }
        if let Some(g) = get("grid_prefs").and_then(|j| serde_json::from_str(j).ok()) {
            self.ed.grid_prefs = g;
        }
        if let Some(g) = get("grid_main_mm").and_then(|v| v.parse::<f64>().ok()).filter(|g| *g >= 0.1) {
            self.ed.grid = g;
        }
        if let Some(v) = get("snap") {
            self.ed.snap = v == "1";
        }
        if let Some(v) = get("snap_objects") {
            self.ed.snap_objects = v == "1";
        }
        if let Some(p) = get("page").and_then(|j| serde_json::from_str::<Page>(j).ok()) {
            self.new_page = p.sane();
        }
        if let Some(u) = get("units").and_then(|j| serde_json::from_str(j).ok()) {
            self.units = u;
        }
        if let Some(v) = get("check_updates") {
            self.check_updates = v == "1";
        }
        self.web.restore(&|k| get(k).map(String::from));
        self.tabs.restore(get("tab_order").unwrap_or(""), get("side_collapsed").is_some_and(|v| v == "1"));
        if let Some(r) = get("recent_files") {
            self.recent = r.lines().map(PathBuf::from).take(crate::MAX_RECENT).collect();
        }
    }
}
