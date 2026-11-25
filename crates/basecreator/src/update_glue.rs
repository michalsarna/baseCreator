//! Looking for a newer release on GitHub: once when the program starts (can be switched off in the program options) and on
//! demand from Help > Check for updates…. The releases of BaseCreator are in the repository of baseCreator and have the numbers of its
//! tags. The check and its window are in base-ui; this says which program they are about.
use crate::{App, VERSION};
use base_ui::dialogs::UpdateConfig;
use base_ui::i18n::tr;
use base_ui::window::popup;
use eframe::egui;

pub static CONFIG: UpdateConfig = UpdateConfig { app_name: "BaseCreator", repo: "michalsarna/baseCreator", version: VERSION };

impl App {
    /// Settings → Program options…: what the program does by itself.
    pub(crate) fn program_options_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_program_options;
        popup(ctx, "program_options", &tr("Program options"), [460.0, 440.0], &mut open, |ui| {
            use base_ui::widgets::{options_grid, options_line};
            options_grid(ui, "program_options_updates", |ui| {
                ui.label(tr("Updates"));
                ui.checkbox(&mut self.check_updates, tr("Look for a newer version when the program starts"));
                ui.end_row();
            });
            options_line(ui);
            options_grid(ui, "program_options_autosave", |ui| base_ui::autosave_ui::options_rows(ui, &mut self.autosave_opts));
            options_line(ui);
            options_grid(ui, "program_options_grid", |ui| {
                self.web.options_ui(ui, false);
                ui.end_row();
            });
            base_ui::authorship_ui::options_section(ui);
        });
        self.show_program_options = open;
    }
}
