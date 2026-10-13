//! The description of a project: a side-panel tab with a plain text of several lines. The text is kept in the project
//! file (`base_core::project::Project::description`), so it is the same in every program that opens the project.
use crate::i18n::tr;

/// The translated title of the tab.
pub fn tab_label() -> &'static str {
    tr("Description")
}

/// The tab: one text field over the whole panel. Returns true when the text was changed this frame.
pub fn panel(ui: &mut egui::Ui, text: &mut String) -> bool {
    let h = ui.available_height().max(120.0);
    let r = egui::ScrollArea::vertical().id_salt("tab_description").auto_shrink([false, false]).show(ui, |ui| {
        ui.add_sized([ui.available_width(), h - 12.0], egui::TextEdit::multiline(text).hint_text(tr("Describe the project: what it is, the settings that worked, notes for next time.")).desired_width(f32::INFINITY))
    });
    r.inner.changed()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_panel_draws_and_leaves_the_text_alone_without_input() {
        let ctx = egui::Context::default();
        let mut text = "Birch plywood 3 mm".to_string();
        let mut changed = true;
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            changed = panel(ui, &mut text);
        });
        out.textures_delta.clear();
        assert!(!changed);
        assert_eq!(text, "Birch plywood 3 mm");
        assert!(!tab_label().is_empty());
    }
}
