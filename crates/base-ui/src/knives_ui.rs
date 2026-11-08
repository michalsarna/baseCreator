//! The window of the knife library, the rows of the values of a drag knife and the choice of a knife of the library, shared by
//! the programs that cut with a drag knife (a cutter, a holder in a CNC machine).
use crate::i18n::tr;
use crate::theme;
use crate::window::{bottom, delete_button_enabled, fill_size, height_above, popup};
use base_core::drag_knife::Knife;
use base_core::knife_lib::{Blade, KnifeLibrary};
use egui::RichText;

/// The name and the notes of a knife as shown: those of a ready-made knife translated.
pub fn shown(b: &Blade) -> (String, String) {
    match base_core::knife_lib::BUILTIN.iter().find(|x| x.0 == b.id) {
        Some(&(_, name, .., notes)) => (tr(name).to_string(), tr(notes).to_string()),
        None => (b.name.clone(), b.notes.clone()),
    }
}

/// The state of the window.
#[derive(Default)]
pub struct KnivesWindow {
    pub open: bool,
    selected: Option<String>,
    /// Something in the library changed: the caller keeps it (in the settings).
    pub changed: bool,
}

/// The rows (in a grid of two columns) of the values of a knife; true when one changed.
pub fn knife_rows(ui: &mut egui::Ui, k: &mut Knife) -> bool {
    let mut changed = false;
    ui.label(tr("Offset of the tip"));
    changed |= ui.add(egui::DragValue::new(&mut k.offset).range(0.0..=3.0).speed(0.01).suffix(" mm")).on_hover_text(tr("How far the tip of the blade is behind its axis: the maker of the blade gives it")).changed();
    ui.end_row();
    ui.label(tr("Turn round corners sharper than"));
    changed |= ui.add(egui::DragValue::new(&mut k.swivel_deg).range(1.0..=90.0).speed(0.5).suffix("°")).changed();
    ui.end_row();
    ui.label(tr("Cut on past the start"));
    changed |= ui.add(egui::DragValue::new(&mut k.overcut).range(0.0..=5.0).speed(0.05).suffix(" mm")).on_hover_text(tr("A closed outline is cut on this far past its start, so it closes")).changed();
    ui.end_row();
    changed
}

/// The row of the knife of the library (a list of the knives and the button of the library): picking one takes its values.
/// Returns true when the values changed; `open_library` is set when the library is to be opened.
pub fn library_row(ui: &mut egui::Ui, salt: &str, lib: &KnifeLibrary, k: &mut Knife, open_library: &mut bool) -> bool {
    let mut changed = false;
    ui.label(tr("Knife"));
    let now = lib.matching(k).map(|b| shown(b).0).unwrap_or_else(|| tr("Own values").to_string());
    egui::ComboBox::from_id_salt(("knife_of_library", salt)).selected_text(now).width(180.0).show_ui(ui, |ui| {
        for b in lib.all() {
            let on = b.knife == *k;
            let (name, notes) = shown(b);
            if ui.selectable_label(on, name).on_hover_text(notes).clicked() && !on {
                *k = b.knife;
                changed = true;
            }
        }
    });
    ui.end_row();
    // A row of its own: a narrow side panel has no room for it next to the list.
    ui.label("");
    if ui.button(tr("Knife library…")).clicked() {
        *open_library = true;
    }
    ui.end_row();
    changed
}

/// The window of the knife library. `lib` is changed in place; the caller keeps it when `win.changed` is set.
pub fn knives_window(ctx: &egui::Context, win: &mut KnivesWindow, lib: &mut KnifeLibrary) {
    if !win.open {
        return;
    }
    let mut open = true;
    popup(ctx, "knife_library", tr("Knife library"), [680.0, 420.0], &mut open, |ui| {
        if win.selected.as_deref().is_none_or(|id| lib.get(id).is_none()) {
            win.selected = lib.builtin.first().map(|b| b.id.clone());
        }
        let total = fill_size(ui).y;
        ui.horizontal_top(|ui| {
            ui.vertical(|ui| {
                ui.set_width(230.0);
                let list_h = height_above(ui, "knife_left", total);
                egui::ScrollArea::vertical().id_salt("knife_list").max_height(list_h).min_scrolled_height(list_h).auto_shrink([false, false]).show(ui, |ui| {
                    for (title, user) in [(tr("Ready-made knives"), false), (tr("My knives"), true)] {
                        ui.label(RichText::new(title).strong().color(theme::text_dim()));
                        let list: &[Blade] = if user { &lib.user } else { &lib.builtin };
                        if list.is_empty() {
                            ui.label(RichText::new(tr("None yet: copy a knife to change it")).color(theme::text_dim()));
                        }
                        for b in list {
                            let on = win.selected.as_deref() == Some(b.id.as_str());
                            if ui.add_sized([ui.available_width(), 22.0], egui::Button::selectable(on, shown(b).0)).clicked() {
                                win.selected = Some(b.id.clone());
                            }
                        }
                        ui.add_space(6.0);
                    }
                });
                bottom(ui, "knife_left", |ui| {
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(tr("Copy")).on_hover_text(tr("A copy among your own knives, to change")).clicked() {
                            let name = win.selected.as_deref().and_then(|id| lib.get(id)).map(|b| format!("{} 2", shown(b).0)).unwrap_or_else(|| tr("My knife").to_string());
                            let id = lib.add_copy(win.selected.as_deref(), &name);
                            win.selected = Some(id);
                            win.changed = true;
                        }
                        let own = win.selected.as_deref().is_some_and(|id| !lib.is_builtin(id));
                        if delete_button_enabled(ui, own, tr("Delete")).clicked() {
                            if let Some(id) = win.selected.take() {
                                lib.remove(&id);
                                win.changed = true;
                            }
                        }
                    });
                });
            });
            ui.separator();
            ui.vertical(|ui| {
                let Some(id) = win.selected.clone() else { return };
                let builtin = lib.is_builtin(&id);
                let blade = if builtin { lib.builtin.iter_mut().find(|b| b.id == id) } else { lib.user.iter_mut().find(|b| b.id == id) };
                let Some(b) = blade else { return };
                if builtin {
                    let (name, notes) = shown(b);
                    ui.heading(name);
                    ui.label(notes);
                    ui.add_space(4.0);
                    ui.label(RichText::new(tr("A ready-made knife cannot be changed: copy it to change it.")).color(theme::text_dim()));
                    egui::Grid::new("knife_values").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.disable();
                        knife_rows(ui, &mut b.knife.clone());
                    });
                    return;
                }
                ui.scope(|ui| {
                    egui::Grid::new("knife_values").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
                        ui.label(tr("Name"));
                        win.changed |= ui.add(egui::TextEdit::singleline(&mut b.name).desired_width(240.0)).changed();
                        ui.end_row();
                        win.changed |= knife_rows(ui, &mut b.knife);
                        ui.label(tr("Notes"));
                        win.changed |= ui.add(egui::TextEdit::multiline(&mut b.notes).desired_rows(3).desired_width(240.0)).changed();
                        ui.end_row();
                    });
                });
            });
        });
    });
    win.open &= open;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_and_the_row_of_the_library_draw_without_changing_anything() {
        let ctx = egui::Context::default();
        let mut win = KnivesWindow { open: true, ..Default::default() };
        let mut lib = KnifeLibrary::default();
        let mut k = lib.get("std-45").unwrap().knife;
        let mut asked = false;
        for _ in 0..3 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                knives_window(ui.ctx(), &mut win, &mut lib);
                egui::Grid::new("g").show(ui, |ui| {
                    library_row(ui, "t", &lib, &mut k, &mut asked);
                });
            });
            out.textures_delta.clear();
        }
        assert!(win.open && !win.changed && !asked);
        assert_eq!(win.selected.as_deref(), Some("std-30"));
        assert_eq!(k, lib.get("std-45").unwrap().knife);
    }
}
