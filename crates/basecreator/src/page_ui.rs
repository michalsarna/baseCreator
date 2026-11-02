//! File > Page setup…: the page takes the place of the work area of a machine.
use crate::section::{Page, PAGE_MAX, PAGE_MIN, SHEETS};
use crate::App;
use base_core::units::Units;
use base_ui::i18n::tr;
use base_ui::units_ui::drag_len;
use base_ui::window::popup;
use eframe::egui;

impl App {
    pub(crate) fn page_window(&mut self, ctx: &egui::Context) {
        let mut open = self.show_page;
        let before = self.page;
        let title = tr("Page setup…");
        let mut page = self.page;
        let mut units = self.units;
        popup(ctx, "page_setup", title.trim_end_matches('…'), [360.0, 190.0], &mut open, |ui| {
            egui::Grid::new("page_grid").num_columns(2).spacing([12.0, 8.0]).show(ui, |ui| {
                ui.label(tr("Size"));
                egui::ComboBox::from_id_salt("page_sheet").selected_text(tr("A usual sheet size…")).show_ui(ui, |ui| {
                    for (name, w, h) in SHEETS {
                        if ui.selectable_label(false, name).clicked() {
                            page = Page { w, h }.turned(page.landscape());
                        }
                    }
                });
                ui.end_row();
                ui.label(tr("Width"));
                drag_len(ui, units, &mut page.w, 1.0, Some((PAGE_MIN, PAGE_MAX)));
                ui.end_row();
                ui.label(tr("Height"));
                drag_len(ui, units, &mut page.h, 1.0, Some((PAGE_MIN, PAGE_MAX)));
                ui.end_row();
                ui.label("");
                ui.horizontal(|ui| {
                    if ui.add(egui::Button::selectable(page.landscape(), tr("Landscape"))).clicked() {
                        page = page.turned(true);
                    }
                    if ui.add(egui::Button::selectable(!page.landscape(), tr("Portrait"))).clicked() {
                        page = page.turned(false);
                    }
                });
                ui.end_row();
                ui.label(tr("Units"));
                egui::ComboBox::from_id_salt("page_units").selected_text(tr(units.label())).show_ui(ui, |ui| {
                    for u in Units::ALL {
                        ui.selectable_value(&mut units, u, tr(u.label()));
                    }
                });
                ui.end_row();
            });
        });
        self.show_page = open;
        self.units = units;
        if page != before {
            self.set_page(page);
        }
    }

    /// A new size of the page of the shown drawing; the next new drawing gets it too.
    pub(crate) fn set_page(&mut self, page: Page) {
        self.page = page.sane();
        self.new_page = self.page;
        self.touch_rev();
        self.ed.view.need_fit = true;
    }
}
