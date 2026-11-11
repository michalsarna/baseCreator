//! The no-go zones of a device in every program: the page of the device settings that lists them, and their red areas on the
//! work area (`base_core::zones`).
use crate::i18n::{tr, trf};
use crate::theme;
use crate::units_ui::drag_len;
use base_core::geom::Pt;
use base_core::units::Units;
use base_core::zones::{Zone, ZoneShape};
use egui::{Color32, RichText, Stroke};

/// The title of the page in the device settings.
pub const PAGE: &str = "No-go zones";

/// The page: a row for every zone (on or off, name, rectangle or circle, delete), its size, and the heights it stands between when
/// the machine has a Z axis. `heights` explains what the heights are counted from in this program. True when something changed.
pub fn page(ui: &mut egui::Ui, units: Units, zones: &mut Vec<Zone>, has_z: bool, heights: &str) -> bool {
    let before = zones.clone();
    ui.label(RichText::new(tr("Parts of the work area the tool must never enter (a clamp, a fixture). Moves through the air go round the zones by themselves; a cut that touches an enabled zone is reported before the job.")).color(theme::text_dim()));
    ui.add_space(4.0);
    let mut remove = None;
    for (i, z) in zones.iter_mut().enumerate() {
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut z.enabled, "");
            ui.add(egui::TextEdit::singleline(&mut z.name).hint_text(tr("Zone name")).desired_width(150.0));
            let rect = matches!(z.shape, ZoneShape::Rect { .. });
            let mut want_rect = rect;
            egui::ComboBox::from_id_salt(("zone_shape", i)).selected_text(tr(if rect { "Rectangle" } else { "Circle" })).show_ui(ui, |ui| {
                ui.selectable_value(&mut want_rect, true, tr("Rectangle"));
                ui.selectable_value(&mut want_rect, false, tr("Circle"));
            });
            if want_rect != rect {
                let b = z.shape.bounds();
                z.shape = if want_rect { ZoneShape::Rect { x: b.min.x, y: b.min.y, w: b.width(), h: b.height() } } else { ZoneShape::Circle { x: b.center().x, y: b.center().y, r: (b.width().max(b.height())) / 2.0 } };
            }
            if crate::window::delete_icon(ui, &tr("Delete this zone")).clicked() {
                remove = Some(i);
            }
        });
        if has_z {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(24.0);
                let mut limited = z.z_range.is_some();
                if ui.checkbox(&mut limited, tr("Only at these heights (Z)")).on_hover_text(format!("{}\n{}", tr("Like a box or a cylinder standing on the table: moves above or below the range are free."), heights)).changed() {
                    z.z_range = if limited { Some((0.0, 10.0)) } else { None };
                }
                if let Some((lo, hi)) = &mut z.z_range {
                    drag_len(ui, units, lo, 0.5, Some((-10_000.0, 10_000.0)));
                    ui.label("–");
                    drag_len(ui, units, hi, 0.5, Some((-10_000.0, 10_000.0)));
                }
            });
        }
        ui.horizontal_wrapped(|ui| {
            ui.add_space(24.0);
            match &mut z.shape {
                ZoneShape::Rect { x, y, w, h } => {
                    for (label, v) in [("X", x), ("Y", y), ("W", w), ("H", h)] {
                        ui.label(label);
                        drag_len(ui, units, v, 0.5, Some((-5000.0, 5000.0)));
                    }
                }
                ZoneShape::Circle { x, y, r } => {
                    for (label, v) in [("X", x), ("Y", y), ("R", r)] {
                        ui.label(label);
                        drag_len(ui, units, v, 0.5, Some((-5000.0, 5000.0)));
                    }
                }
            }
        });
    }
    if let Some(i) = remove {
        zones.remove(i);
    }
    if zones.len() < 100 && ui.button(tr("Add a zone")).clicked() {
        zones.push(Zone { name: trf("Zone {}", &[&(zones.len() + 1)]), enabled: true, shape: ZoneShape::Rect { x: 10.0, y: 10.0, w: 50.0, h: 30.0 }, z_range: None });
    }
    *zones != before
}

/// The zones on the work area: red where they are on, grey where off, with their names. `to` takes a point of the design to the
/// The side panel of the zones, shown while the device has any: the same rows as the page of the device settings, with a word that
/// they belong to the device. True when something changed (the program then saves the device).
pub fn panel(ui: &mut egui::Ui, units: Units, zones: &mut Vec<Zone>, has_z: bool, heights: &str) -> bool {
    ui.label(RichText::new(tr(PAGE)).strong());
    ui.label(RichText::new(tr("The zones belong to the device and are kept with it, in every project.")).color(theme::text_dim()));
    ui.add_space(4.0);
    egui::ScrollArea::vertical().id_salt("zones_panel").auto_shrink([false, false]).show(ui, |ui| page(ui, units, zones, has_z, heights)).inner
}

/// screen and `zoom` is the pixels of a millimetre.
pub fn paint(painter: &egui::Painter, to: &dyn Fn(Pt) -> egui::Pos2, zoom: f32, zones: &[Zone]) {
    for z in zones {
        let (fill, edge) = if z.enabled { (Color32::from_rgba_unmultiplied(0xc0, 0x30, 0x30, 70), Color32::from_rgb(0xc0, 0x30, 0x30)) } else { (Color32::from_rgba_unmultiplied(0x90, 0x90, 0x90, 30), Color32::from_gray(0x90)) };
        let b = z.shape.bounds();
        match z.shape {
            ZoneShape::Rect { .. } => {
                let r = egui::Rect::from_two_pos(to(b.min), to(b.max));
                painter.rect_filled(r, 0.0, fill);
                painter.rect_stroke(r, 0.0, Stroke::new(1.5, edge), egui::StrokeKind::Middle);
            }
            ZoneShape::Circle { x, y, r } => {
                painter.circle(to(Pt::new(x, y)), r.abs() as f32 * zoom, fill, Stroke::new(1.5, edge));
            }
        }
        let corner = egui::Rect::from_two_pos(to(b.min), to(b.max)).left_top();
        painter.text(corner + egui::vec2(4.0, 2.0), egui::Align2::LEFT_TOP, &z.name, egui::FontId::proportional(11.0), edge);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_page_and_the_zones_draw() {
        let ctx = egui::Context::default();
        let mut zones = vec![Zone { name: "Clamp".into(), enabled: true, shape: ZoneShape::Rect { x: 1.0, y: 2.0, w: 3.0, h: 4.0 }, z_range: Some((0.0, 15.0)) }, Zone { name: "Pin".into(), enabled: false, shape: ZoneShape::Circle { x: 5.0, y: 5.0, r: 2.0 }, z_range: None }];
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            egui::CentralPanel::default().show(ui, |ui| {
                assert!(!page(ui, Units::Mm, &mut zones, true, "Z 0 is the top of the material."));
                paint(ui.painter(), &|p| egui::pos2(p.x as f32, p.y as f32), 1.0, &zones);
            });
        });
        out.textures_delta.clear();
    }
}
