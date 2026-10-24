//! The start screen's offer to recover a project that was not saved (see `base_core::autosave`).
use crate::i18n::{tr, trf};
use crate::{icons, theme};
use base_core::autosave::Found;
use egui::{Color32, RichText};

/// What the person chose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Recovery {
    Recover,
    Discard,
}

/// A big button "Recover", with a line about what was found and a small way to throw it away. `width` is the width of the
/// column of the start screen.
pub fn banner(ui: &mut egui::Ui, found: &Found, width: f32) -> Option<Recovery> {
    let mut chosen = None;
    let frame = egui::Frame::group(ui.style()).inner_margin(egui::Margin::same(12)).stroke(egui::Stroke::new(1.5, theme::accent()));
    frame.show(ui, |ui| {
        ui.set_width(width);
        ui.label(RichText::new(tr("A project that was not saved was found")).strong());
        ui.label(RichText::new(trf("{} shapes, kept automatically", &[&found.shapes])).color(theme::text_dim()));
        ui.add_space(6.0);
        let b = egui::Button::image_and_text(icons::slot(Some("history"), Color32::WHITE), RichText::new(tr("Recover")).color(Color32::WHITE).size(17.0).strong()).fill(theme::accent());
        if ui.add_sized([ui.available_width(), 40.0], b).clicked() {
            chosen = Some(Recovery::Recover);
        }
        ui.add_space(2.0);
        if ui.small_button(tr("Discard it")).clicked() {
            chosen = Some(Recovery::Discard);
        }
    });
    chosen
}

/// The same for all the recovery files that were found (one for each tab that was open): the number of shapes is the sum,
/// "Recover" opens every one of them in a tab of its own, and "Discard it" throws all of them away.
pub fn banner_all(ui: &mut egui::Ui, found: &[Found], width: f32) -> Option<Recovery> {
    let first = found.first()?;
    let all = Found { path: first.path.clone(), shapes: found.iter().map(|f| f.shapes).sum(), slot: first.slot };
    banner(ui, &all, width)
}
