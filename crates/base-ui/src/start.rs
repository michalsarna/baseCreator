//! The start screen's header and the language / colour scheme row, and the window icon and logo of an application.
use crate::i18n::{tr, Lang};
use crate::theme::{self, Scheme};
use egui::RichText;

/// Decode a PNG into a texture (the logo of the title bar, the start screen and the About window).
pub fn load_logo(ctx: &egui::Context, png: &[u8], name: &str) -> Option<egui::TextureHandle> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.to_rgba8();
    let size = [img.width() as usize, img.height() as usize];
    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
    Some(ctx.load_texture(name, color, egui::TextureOptions::LINEAR))
}

/// Decode a PNG into the icon of the window.
pub fn window_icon(png: &[u8]) -> Option<egui::IconData> {
    let img = image::load_from_memory_with_format(png, image::ImageFormat::Png).ok()?.to_rgba8();
    Some(egui::IconData { width: img.width(), height: img.height(), rgba: img.into_raw() })
}

/// The logo, the name and the version at the top of the start screen.
pub fn start_header(ui: &mut egui::Ui, logo: Option<&egui::TextureHandle>, name: &str, version: &str) {
    ui.add_space((ui.available_height() * 0.06).max(12.0));
    if let Some(logo) = logo {
        ui.image((logo.id(), egui::vec2(96.0, 96.0)));
    }
    ui.label(RichText::new(name).size(28.0).strong());
    ui.label(RichText::new(version).color(theme::text_dim()));
}

/// The two selection boxes at the bottom of the start screen. Returns what the user changed.
pub fn start_prefs(ui: &mut egui::Ui, lang: Lang, scheme: Scheme) -> (Option<Lang>, Option<Scheme>) {
    let (mut l, mut s) = (lang, scheme);
    ui.horizontal(|ui| {
        // centre the two combo boxes
        ui.add_space(((ui.available_width() - 360.0) / 2.0).max(0.0));
        ui.label(tr("Language"));
        egui::ComboBox::from_id_salt("start_lang").selected_text(l.name()).show_ui(ui, |ui| {
            for x in Lang::ALL {
                ui.selectable_value(&mut l, x, x.name());
            }
        });
        ui.add_space(12.0);
        ui.label(tr("Colour scheme"));
        egui::ComboBox::from_id_salt("start_scheme").selected_text(tr(s.label())).show_ui(ui, |ui| {
            for x in Scheme::ALL {
                ui.selectable_value(&mut s, x, tr(x.label()));
            }
        });
    });
    ((l != lang).then_some(l), (s != scheme).then_some(s))
}
