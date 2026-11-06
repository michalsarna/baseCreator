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

/// How many fonts the open list shows at once.
const FONTS_SEEN: usize = 10;

/// The font `by` places down (negative: up) the list from `current`, kept inside the list; the first one when `current` is
/// not in it (it was filtered out). `None` when there is nothing to move.
fn step_family(list: &[String], current: &str, by: i32) -> Option<String> {
    if by == 0 || list.is_empty() {
        return None;
    }
    let next = match list.iter().position(|f| f == current) {
        Some(i) => (i as i32 + by).clamp(0, list.len() as i32 - 1) as usize,
        None => 0,
    };
    (list[next] != current).then(|| list[next].clone())
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
        // Ten fonts are seen at once; the search field and the margins come on top.
        let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
        let list_h = row * FONTS_SEEN as f32;
        egui::ComboBox::from_id_salt("font_family").width(190.0).height(list_h + 3.0 * row).selected_text(t.family.clone()).show_ui(ui, |ui| {
            // The arrows go through the open list and change the font at once (taken before the search field sees them).
            let (down, up) = ui.input_mut(|i| (i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowDown), i.count_and_consume_key(egui::Modifiers::NONE, egui::Key::ArrowUp)));
            ui.add(egui::TextEdit::singleline(&mut h.ed_mut().font_filter).hint_text(tr("Search")));
            let q = h.ed().font_filter.to_lowercase();
            let shown: Vec<String> = base_core::text::families().into_iter().filter(|f| q.is_empty() || f.to_lowercase().contains(&q)).collect();
            let stepped = step_family(&shown, &t.family, down as i32 - up as i32);
            if let Some(f) = &stepped {
                t.family = f.clone();
                undo_point = true;
            }
            egui::ScrollArea::vertical().max_height(list_h).auto_shrink([false, true]).show(ui, |ui| {
                for fam in shown {
                    let r = ui.selectable_label(t.family == fam, &fam);
                    if stepped.as_ref() == Some(&fam) {
                        r.scroll_to_me(None);
                    }
                    if r.clicked() {
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

#[cfg(test)]
mod tests {
    use super::step_family;

    #[test]
    fn the_arrows_step_through_the_fonts_and_stop_at_the_ends() {
        let l: Vec<String> = ["Arial", "Courier", "Times"].iter().map(|s| s.to_string()).collect();
        assert_eq!(step_family(&l, "Arial", 1).as_deref(), Some("Courier"));
        assert_eq!(step_family(&l, "Courier", -1).as_deref(), Some("Arial"));
        assert_eq!(step_family(&l, "Times", 1), None, "the last stays");
        assert_eq!(step_family(&l, "Arial", -3), None, "the first stays");
        assert_eq!(step_family(&l, "Arial", 5).as_deref(), Some("Times"));
        assert_eq!(step_family(&l, "Hidden by the search", 1).as_deref(), Some("Arial"));
        assert_eq!(step_family(&l, "Arial", 0), None);
        assert_eq!(step_family(&[], "Arial", 1), None);
    }
}
