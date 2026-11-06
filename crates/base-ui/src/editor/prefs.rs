//! The window with the options of the grids and of the colours of the work area.
use super::{EditorHost, GridPrefs};
use crate::i18n::tr;
use crate::units_ui::drag_len;
use crate::window::popup;
use crate::theme;
use egui::{Color32, RichText};

fn c32(c: [u8; 4]) -> Color32 {
    Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

/// The window with the options of the grids and of the colours of the work area.
/// The part of the View options about the console: what each kind of line looks like, and the size of the letters.
fn console_look(ui: &mut egui::Ui, look: &mut crate::console_ui::ConsoleLook) {
    egui::Grid::new("console_look").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Font size"));
        ui.add(egui::Slider::new(&mut look.font_size, crate::console_ui::ConsoleLook::FONT_SIZES).step_by(1.0));
        ui.end_row();
        for (label, colour) in [(tr("Your commands"), &mut look.user), (tr("Commands of the program"), &mut look.program), (tr("Replies of the machine"), &mut look.reply)] {
            ui.label(label);
            ui.color_edit_button_srgb(colour);
            ui.end_row();
        }
        let message = look.message_colour().to_srgba_unmultiplied();
        let background = look.background_colour().to_srgba_unmultiplied();
        for (label, colour, now) in [(tr("Messages of the program"), &mut look.message, message), (tr("Background"), &mut look.background, background)] {
            ui.label(label);
            ui.horizontal(|ui| {
                let mut auto = colour.is_none();
                if ui.checkbox(&mut auto, tr("Automatic")).changed() {
                    *colour = if auto { None } else { Some([now[0], now[1], now[2]]) };
                }
                if let Some(c) = colour {
                    ui.color_edit_button_srgb(c);
                }
            });
            ui.end_row();
        }
    });
}

pub fn view_options_window<H: EditorHost>(h: &mut H, ctx: &egui::Context, show: &mut bool) {
    if !*show {
        return;
    }
    h.ed_mut().clamp_grid();
    let units = h.units();
    let mut open = true;
    let has_machine = h.has_machine();
    popup(ctx, "w_prefs_2", &tr("View options"), [340.0, 220.0], &mut open, |ui| {
        ui.label(RichText::new(tr("Main grid")).strong());
        egui::Grid::new("grid_main").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Show"));
            ui.checkbox(&mut h.ed_mut().show_grid, "");
            ui.end_row();
            ui.label(tr("Distance between lines"));
            drag_len(ui, units, &mut h.ed_mut().grid, 0.5, Some((0.1, 1000.0)));
            ui.end_row();
            ui.label(tr("Colour"));
            let mut col = c32(h.ed_mut().grid_prefs.main_color);
            if egui::color_picker::color_edit_button_srgba(ui, &mut col, egui::color_picker::Alpha::OnlyBlend).changed() {
                h.ed_mut().grid_prefs.main_color = col.to_srgba_unmultiplied();
            }
            ui.end_row();
        });
        ui.add_space(8.0);
        ui.label(RichText::new(tr("Secondary grid")).strong());
        let max = (h.ed_mut().grid * 0.9).max(0.05);
        egui::Grid::new("grid_minor").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Show"));
            ui.checkbox(&mut h.ed_mut().grid_prefs.minor_on, "");
            ui.end_row();
            ui.label(tr("Distance between lines"));
            drag_len(ui, units, &mut h.ed_mut().grid_prefs.minor_mm, 0.1, Some((0.05, max)));
            ui.end_row();
            ui.label(tr("Colour"));
            let mut col = c32(h.ed_mut().grid_prefs.minor_color);
            if egui::color_picker::color_edit_button_srgba(ui, &mut col, egui::color_picker::Alpha::OnlyBlend).changed() {
                h.ed_mut().grid_prefs.minor_color = col.to_srgba_unmultiplied();
            }
            ui.end_row();
        });
        ui.label(RichText::new(tr("The secondary grid must be finer than the main grid.")).color(theme::text_dim()));
        ui.label(RichText::new(format!("{} {}", tr("Secondary grid"), tr("It is seen when zoomed in more than 200 %."))).color(theme::text_dim()));
        ui.add_space(8.0);
        ui.label(RichText::new(tr("Work area")).strong());
        egui::Grid::new("bed_bg").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Background colour"));
            ui.horizontal(|ui| {
                let mut auto = h.ed_mut().grid_prefs.bed_color.is_none();
                let current = h.bed_fill().to_srgba_unmultiplied();
                if ui.checkbox(&mut auto, tr("Automatic")).changed() {
                    h.ed_mut().grid_prefs.bed_color = if auto { None } else { Some(current) };
                }
                if let Some(c) = h.ed_mut().grid_prefs.bed_color {
                    let mut col = c32(c);
                    if egui::color_picker::color_edit_button_srgba(ui, &mut col, egui::color_picker::Alpha::Opaque).changed() {
                        h.ed_mut().grid_prefs.bed_color = Some(col.to_srgba_unmultiplied());
                    }
                }
            });
            ui.end_row();
            ui.label(tr("Workspace colour")).on_hover_text(tr("The space around the work area"));
            ui.horizontal(|ui| {
                let mut auto = h.ed_mut().grid_prefs.workspace_color.is_none();
                let current = h.workspace_fill().to_srgba_unmultiplied();
                if ui.checkbox(&mut auto, tr("Automatic")).changed() {
                    h.ed_mut().grid_prefs.workspace_color = if auto { None } else { Some(current) };
                }
                if let Some(c) = h.ed_mut().grid_prefs.workspace_color {
                    let mut col = c32(c);
                    if egui::color_picker::color_edit_button_srgba(ui, &mut col, egui::color_picker::Alpha::Opaque).changed() {
                        h.ed_mut().grid_prefs.workspace_color = Some(col.to_srgba_unmultiplied());
                    }
                }
            });
            ui.end_row();
            ui.label(tr("Line thickness"));
            ui.add(egui::Slider::new(&mut h.ed_mut().grid_prefs.line_width, 0.5..=3.0).step_by(0.1));
            ui.end_row();
            ui.label(tr("Layers"));
            ui.checkbox(&mut h.ed_mut().grid_prefs.layer_names, tr("Show layer names"));
            ui.end_row();
            if !has_machine {
                return;
            }
            ui.label("G-code");
            ui.checkbox(&mut h.ed_mut().grid_prefs.gcode_preview_after_save, tr("Preview G-code after saving"));
            ui.end_row();
            ui.label("");
            ui.checkbox(&mut h.ed_mut().grid_prefs.variable_names, tr("Show variable names")).on_hover_text(tr("Beside the label of a field that has a variable for the own G-code its name is shown; a click copies it"));
            ui.end_row();
        });
        if ui.button(tr("Reset grid")).clicked() {
            let prefs = &h.ed().grid_prefs;
            let kept = GridPrefs { layer_names: prefs.layer_names, gcode_preview_after_save: prefs.gcode_preview_after_save, variable_names: prefs.variable_names, console: prefs.console, ..Default::default() };
            h.ed_mut().grid_prefs = kept;
            h.ed_mut().grid = 10.0;
            h.ed_mut().show_grid = true;
        }
        if has_machine {
            ui.add_space(8.0);
            ui.label(RichText::new(tr("G-code console")).strong());
            console_look(ui, &mut h.ed_mut().grid_prefs.console);
        }
        h.view_options_extra(ui);
    });
    *show = open;
}
