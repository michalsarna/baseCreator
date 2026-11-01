//! The row of layer colours under the editor, the same in every program: one swatch for each layer, in one row that
//! scrolls sideways (arrows at both ends, or the wheel). A click puts the selected shapes on that layer (or, with nothing
//! selected, makes it the layer new shapes go to); a double click opens the options of the layer. With *Show layer names* on
//! (the view options) a layer that has shapes shows its name beside the colour.
use crate::editor::EditorHost;
use crate::i18n::trf;
use crate::{icons, theme};
use egui::{Color32, Sense};

const SWATCH: f32 = 22.0;
const GAP: f32 = 7.0;
/// The width of the ring round the active swatch.
const RING: f32 = 2.0;
/// Room before the first and after the last swatch, so the outline of the active one is not cut off at the ends of the row.
const PAD: f32 = 6.0;
const ARROW: f32 = 24.0;
/// The space between a swatch and its name, and the most characters of a name.
const NAME_GAP: f32 = 4.0;
const NAME_CHARS: usize = 14;

/// The name cut to fit the bar (the whole one is in the tip).
fn shorten(name: &str) -> String {
    if name.chars().count() <= NAME_CHARS {
        name.to_string()
    } else {
        format!("{}…", name.chars().take(NAME_CHARS - 1).collect::<String>())
    }
}

pub trait LayerBarHost: EditorHost {
    /// The name of a layer, for the tip.
    fn layer_label(&self, layer: usize) -> String;
    /// How many layers there are.
    fn layer_total(&self) -> usize {
        base_core::layers::LAYER_COUNT
    }
    /// A double click: show the options of the layer.
    fn open_layer_options(&mut self, layer: usize);
    /// A click: put the selected shapes on the layer, and make it the layer new shapes go to. Locked shapes stay.
    fn click_layer(&mut self, layer: usize) {
        self.ed_mut().active_layer = layer;
        if self.ed().sel.is_empty() {
            return;
        }
        self.checkpoint();
        for id in self.ed().sel.clone() {
            if let Some(s) = self.scene_mut().shape_mut(id) {
                if !s.locked {
                    s.layer = layer;
                }
            }
        }
        self.touch();
    }
}

/// Black or white, whichever reads better on `c`.
fn ink_on(c: Color32) -> Color32 {
    let lum = 0.299 * c.r() as f32 + 0.587 * c.g() as f32 + 0.114 * c.b() as f32;
    if lum > 140.0 {
        Color32::BLACK
    } else {
        Color32::WHITE
    }
}

fn arrow(ui: &mut egui::Ui, icon: &str, enabled: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(ARROW, SWATCH + 4.0), Sense::click());
    if resp.hovered() && enabled {
        ui.painter().rect_filled(rect, 4.0, theme::panel_dark());
    }
    let ink = if enabled { theme::text() } else { theme::text_dim().gamma_multiply(0.5) };
    icons::paint(ui, rect.shrink2(egui::vec2(4.0, 5.0)), icon, ink);
    enabled && resp.clicked()
}

pub fn layer_bar<H: LayerBarHost>(h: &mut H, ui: &mut egui::Ui) {
    let total = h.layer_total();
    let used: Vec<bool> = {
        let mut u = vec![false; total];
        for s in &h.scene().shapes {
            if s.layer < total {
                u[s.layer] = true;
            }
        }
        u
    };
    let offset_id = egui::Id::unique("layer_bar_offset");
    let mut offset: f32 = ui.data(|d| d.get_temp(offset_id)).unwrap_or(0.0);
    // The names of the layers that have shapes, shortened, when the person wants them; and how wide each cell is.
    let show_names = h.ed().grid_prefs.layer_names;
    let names: Vec<Option<(std::sync::Arc<egui::Galley>, String)>> = (0..total)
        .map(|i| {
            if !(show_names && used[i]) {
                return None;
            }
            let full = h.layer_label(i);
            let text = shorten(&full);
            let galley = ui.painter().layout_no_wrap(text, egui::FontId::proportional(12.0), theme::text());
            Some((galley, full))
        })
        .collect();
    let cell_w: Vec<f32> = names.iter().map(|n| SWATCH + n.as_ref().map_or(0.0, |(g, _)| NAME_GAP + g.size().x + 2.0)).collect();
    // Room, the gap, the cells with a gap after each, room.
    let content_w = cell_w.iter().map(|w| w + GAP).sum::<f32>() + 2.0 * PAD + GAP;
    let mut clicked = None;
    let mut double = None;
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = GAP;
        let avail = (ui.available_width() - 2.0 * (ARROW + GAP)).max(60.0);
        let max_offset = (content_w - avail).max(0.0);
        let step = (avail * 0.6).max(60.0);
        if arrow(ui, "chevron-left", offset > 0.5) {
            offset = (offset - step).max(0.0);
        }
        let area = egui::ScrollArea::horizontal().id_salt("layer_bar").scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysHidden).max_width(avail).horizontal_scroll_offset(offset).auto_shrink([false, true]);
        let out = area.show(ui, |ui| {
            ui.horizontal_centered(|ui| {
                ui.spacing_mut().item_spacing.x = GAP;
                ui.add_space(PAD);
                for i in 0..total {
                    let fill = {
                        let c = h.layer_color(i);
                        if h.layer_visible(i) {
                            c
                        } else {
                            c.gamma_multiply(0.35)
                        }
                    };
                    // The cell is taller than the swatch: the outline of the active one is drawn outside it and would be cut
                    // off at the edge of the scrolling row.
                    let (cell_all, resp) = ui.allocate_exact_size(egui::vec2(cell_w[i], SWATCH + 10.0), Sense::click());
                    let cell = egui::Rect::from_min_size(cell_all.min, egui::vec2(SWATCH, cell_all.height()));
                    // The swatch is a little smaller than its cell and the ring of the active one is drawn inside the cell: the scrolling
                    // row clips whatever goes beyond it, which cut the ring off at the sides.
                    let ring = egui::Rect::from_center_size(cell.center(), egui::vec2(SWATCH, SWATCH));
                    let r = ring.shrink(RING + 1.0);
                    ui.painter().rect_filled(r, 3.0, fill);
                    if used[i] {
                        // A dot: there are shapes on this layer.
                        ui.painter().circle_filled(r.center(), 2.5, ink_on(fill));
                    }
                    if h.ed().active_layer == i {
                        ui.painter().rect_stroke(ring, 4.0, egui::Stroke::new(RING, theme::accent()), egui::StrokeKind::Inside);
                    }
                    if let Some((galley, _)) = &names[i] {
                        let at = egui::pos2(cell.right() + NAME_GAP, cell_all.center().y - galley.size().y / 2.0);
                        let ink = if h.layer_visible(i) { theme::text() } else { theme::text_dim() };
                        ui.painter().galley(at, galley.clone(), ink);
                    }
                    let name = h.layer_label(i);
                    let tip = trf("{} — click to assign selection / set active; double-click for options", &[&format!("{}: {}", i + 1, name)]);
                    let resp = resp.on_hover_text(tip);
                    if resp.double_clicked() {
                        double = Some(i);
                    } else if resp.clicked() {
                        clicked = Some(i);
                    }
                }
                ui.add_space(PAD);
            });
        });
        offset = out.state.clamped_offset().x;
        // The wheel moves the row sideways too.
        if ui.rect_contains_pointer(out.inner_rect) {
            let d = ui.input(|i| i.smooth_scroll_delta);
            if d.y != 0.0 || d.x != 0.0 {
                offset = (offset - (if d.x != 0.0 { d.x } else { d.y })).clamp(0.0, max_offset);
            }
        }
        if arrow(ui, "chevron-right", offset < max_offset - 0.5) {
            offset = (offset + step).min(max_offset);
        }
    });
    // Keep the active layer in view when it changes from elsewhere (a click on a shape, the list of layers).
    let active = h.ed().active_layer;
    let seen_id = egui::Id::unique("layer_bar_active");
    let was: Option<usize> = ui.data(|d| d.get_temp(seen_id));
    if was != Some(active) {
        ui.data_mut(|d| d.insert_temp(seen_id, active));
        let x = PAD + GAP + cell_w.iter().take(active).map(|w| w + GAP).sum::<f32>();
        let view = (ui.max_rect().width() - 2.0 * (ARROW + GAP)).max(60.0);
        if x < offset || x + SWATCH > offset + view {
            offset = (x - view / 2.0).clamp(0.0, (content_w - view).max(0.0));
        }
    }
    ui.data_mut(|d| d.insert_temp(offset_id, offset));
    if let Some(i) = double {
        h.ed_mut().active_layer = i;
        h.open_layer_options(i);
    } else if let Some(i) = clicked {
        h.click_layer(i);
    }
}
