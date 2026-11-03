//! The text tool of every program: a click puts a text on the page, and the Properties tab edits it.
use super::{EditorHost, EditorShapeOps};
use crate::i18n::tr;
use crate::units_ui::drag_len;
use crate::widgets::section_title;
use base_core::geom::{Pt, Xf};
use base_core::scene::Kind;
use base_core::text::TextData;
use egui::RichText;

/// Put a text on the active layer, with its top-left corner at `at`, select it and open the field to type in. It takes the
/// settings of the text changed last.
pub fn place<H: EditorHost>(h: &mut H, at: Pt) {
    h.checkpoint();
    let layer = h.ed().active_layer;
    let text = h.ed().text_default.clone();
    let id = h.scene_mut().add(layer, Kind::Text(text), Xf::translate(at.x, at.y));
    h.ed_mut().sel = vec![id];
    h.ed_mut().focus_text = true;
    h.touch();
    h.show_properties_tab();
}

/// The section of the Properties tab for one selected text. Draws nothing for anything else.
pub fn section<H: EditorHost>(h: &mut H, ui: &mut egui::Ui) {
    let [id] = h.ed().sel[..] else { return };
    let Some(Kind::Text(orig)) = h.scene().shape(id).map(|s| s.kind.clone()) else { return };
    let units = h.units();
    let mut t: TextData = orig.clone();
    ui.separator();
    section_title(ui, "type", &tr("Text"));
    let r = ui.add(egui::TextEdit::multiline(&mut t.text).desired_rows(3).desired_width(f32::INFINITY));
    if h.ed().focus_text {
        r.request_focus();
        // The section is at the end of the tab: the field may be out of sight.
        r.scroll_to_me(None);
        h.ed_mut().focus_text = false;
    }
    let mut undo_point = r.gained_focus();
    egui::Grid::new("text_props").num_columns(2).spacing([8.0, 6.0]).show(ui, |ui| {
        ui.label(tr("Font"));
        egui::ComboBox::from_id_salt("font_family").width(190.0).selected_text(t.family.clone()).show_ui(ui, |ui| {
            ui.add(egui::TextEdit::singleline(&mut h.ed_mut().font_filter).hint_text(tr("Search")));
            let q = h.ed().font_filter.to_lowercase();
            egui::ScrollArea::vertical().max_height(260.0).show(ui, |ui| {
                for fam in base_core::text::families() {
                    if (q.is_empty() || fam.to_lowercase().contains(&q)) && ui.selectable_label(t.family == fam, &fam).clicked() {
                        t.family = fam;
                        undo_point = true;
                    }
                }
            });
        });
        ui.end_row();
        ui.label(tr("Size"));
        drag_len(ui, units, &mut t.size, 0.2, Some((0.5, 1000.0)));
        ui.end_row();
        ui.label(tr("Style"));
        ui.horizontal(|ui| {
            ui.checkbox(&mut t.bold, tr("Bold"));
            ui.checkbox(&mut t.italic, tr("Italic"));
        });
        ui.end_row();
        ui.label(tr("Letter spacing"));
        ui.add(egui::DragValue::new(&mut t.spacing).speed(0.005).range(-0.5..=3.0));
        ui.end_row();
        ui.label(tr("Line spacing"));
        ui.add(egui::DragValue::new(&mut t.line_spacing).speed(0.01).range(0.5..=5.0));
        ui.end_row();
        ui.label(tr("Align"));
        ui.horizontal(|ui| {
            for (i, n) in [tr("Left"), tr("Centre"), tr("Right")].into_iter().enumerate() {
                ui.selectable_value(&mut t.align, i as u8, RichText::new(n));
            }
        });
        ui.end_row();
    });
    if ui.button(tr("Convert to curves")).on_hover_text(tr("Turn the text into editable Bézier shapes")).clicked() {
        h.to_curves();
        h.touch();
        return;
    }
    if t != orig {
        // One undo step per burst of editing: the first change after the field got the focus, or after a pause.
        let now = ui.input(|i| i.time);
        if undo_point || now - h.ed().text_changed_at > 0.8 {
            h.checkpoint();
        }
        h.ed_mut().text_changed_at = now;
        h.ed_mut().text_default = TextData { text: h.ed().text_default.text.clone(), ..t.clone() };
        if let Some(s) = h.scene_mut().unlocked_mut(id) {
            s.kind = Kind::Text(t);
        }
        h.touch();
    }
}
