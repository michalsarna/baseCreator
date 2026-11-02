//! The Layers tab: a layer is a group of shapes with a name and a colour, and can be hidden.
use crate::App;
use base_ui::editor::EditorHost;
use base_ui::i18n::{tr, trf};
use base_ui::theme;
use eframe::egui::{self, RichText};

impl App {
    /// How many shapes lie on each layer.
    pub(crate) fn shapes_per_layer(&self) -> Vec<usize> {
        let mut n = vec![0; self.layers.len()];
        for s in &self.scene.shapes {
            if let Some(c) = n.get_mut(s.layer) {
                *c += 1;
            }
        }
        n
    }

    /// Put the selected shapes on a layer.
    pub(crate) fn assign_selection_to_layer(&mut self, layer: usize) {
        let ids = self.scene.unlocked_ids(&self.ed.sel.iter().copied().collect::<Vec<_>>());
        if ids.is_empty() {
            return;
        }
        self.checkpoint();
        for id in ids {
            if let Some(s) = self.scene.shape_mut(id) {
                s.layer = layer;
            }
        }
        self.touch();
    }

    pub(crate) fn layers_panel(&mut self, ui: &mut egui::Ui) {
        let counts = self.shapes_per_layer();
        let active = self.ed.active_layer.min(self.layers.len().saturating_sub(1));
        let mut pick = None;
        let mut changed = false;
        let show_all = self.show_all_layers;
        base_ui::widgets::list_with_splitter(ui, "layer_list", 190.0, |ui| {
            for (i, l) in self.layers.iter_mut().enumerate() {
                // Of 64 layers show the ones with shapes and the one being edited (the colour bar has all of them).
                if !(show_all || counts[i] > 0 || i == active) {
                    continue;
                }
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    ui.painter().rect_filled(rect, 3.0, l.color32());
                    if base_ui::widgets::toggle_icon(ui, "eye", &tr("Shown in the work area"), l.visible) {
                        l.visible = !l.visible;
                        changed = true;
                    }
                    if ui.add_sized([ui.available_width() - 44.0, 22.0], egui::Button::selectable(i == active, l.name.as_str())).clicked() {
                        pick = Some(i);
                    }
                    ui.label(RichText::new(counts[i].to_string()).color(theme::text_dim())).on_hover_text(tr("Shapes on this layer"));
                });
            }
        });
        ui.checkbox(&mut self.show_all_layers, trf("Show all {} layers", &[&self.layers.len()]));
        if let Some(i) = pick {
            self.ed.active_layer = i;
        }
        let li = self.ed.active_layer.min(self.layers.len().saturating_sub(1));
        let mut assign = false;
        ui.add_space(4.0);
        let l = &mut self.layers[li];
        egui::Grid::new("layer_grid").num_columns(2).spacing([10.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Name"));
            changed |= ui.text_edit_singleline(&mut l.name).changed();
            ui.end_row();
            ui.label(tr("Colour"));
            changed |= ui.color_edit_button_srgb(&mut l.color).changed();
            ui.end_row();
        });
        ui.add_space(6.0);
        if ui.button(tr("Put the selected shapes on this layer")).clicked() {
            assign = true;
        }
        if assign {
            self.assign_selection_to_layer(li);
        }
        if changed {
            self.touch();
        }
    }
}
