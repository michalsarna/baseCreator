//! The window that adds a device, the same in every program: one window with the basic settings, the connection with a read from
//! the controller, an optional camera, the things worth checking and the buttons Add device, More settings…, I have no device and
//! Cancel. The rows that belong to the machine (the laser, the spindle, the pen) are the program's, drawn in the same grid.
use crate::i18n::tr;
use crate::link::PortInfo;
use base_core::camimg::{CameraKind, CameraSource};
use base_core::machine::{Connection, LinkKind};
use egui::{Color32, RichText, Ui};

/// What a button of the window asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AddAction {
    /// Add the device with what is entered.
    Add,
    /// Open the whole device window with what is entered.
    More,
    /// Work without a machine: the stand-in device Dummy.
    Dummy,
    Cancel,
}

/// How the read from the controller went.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReadState {
    #[default]
    Idle,
    Reading,
    Done,
    Failed,
}

/// The window. `body` draws the settings (in a scrolled part) and says whether the device can be added (a name, no camera without an
/// address). The buttons are under it. `open` is cleared by the close button of the window.
pub fn window(ctx: &egui::Context, open: &mut bool, body: impl FnOnce(&mut Ui) -> bool) -> Option<AddAction> {
    let mut action = None;
    crate::window::popup(ctx, "first_setup", &tr("Add a device"), [520.0, 640.0], open, |ui| {
        let mut can_add = false;
        egui::ScrollArea::vertical().max_height((ui.ctx().content_rect().height() - 190.0).clamp(200.0, 600.0)).auto_shrink([false, true]).show(ui, |ui| {
            can_add = body(ui);
        });
        ui.add_space(8.0);
        action = buttons(ui, can_add);
    });
    action
}

/// The grid of the settings: two columns, the labels on the left.
pub fn grid(ui: &mut Ui, rows: impl FnOnce(&mut Ui)) {
    egui::Grid::new("add_dev").num_columns(2).min_col_width(140.0).spacing([12.0, 6.0]).show(ui, rows);
}

/// The rows of the connection, in the grid: its type, and the port (with Refresh) or the host and its TCP port. True when Refresh
/// was pressed.
pub fn connection_rows(ui: &mut Ui, c: &mut Connection, ports: &[PortInfo]) -> bool {
    let mut refresh = false;
    ui.label(tr("Connection type"));
    egui::ComboBox::from_id_salt("add_link").selected_text(tr(c.link.label())).show_ui(ui, |ui| {
        for k in LinkKind::ALL {
            ui.selectable_value(&mut c.link, k, tr(k.label()));
        }
    });
    ui.end_row();
    if c.link == LinkKind::Serial {
        ui.label(tr("Port"));
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("add_port").width(220.0).selected_text(if c.port.is_empty() { tr("No port") } else { c.port.as_str() }).show_ui(ui, |ui| {
                for p in ports {
                    ui.selectable_value(&mut c.port, p.name.clone(), p.label());
                }
            });
            if ui.button(tr("Refresh")).clicked() {
                refresh = true;
            }
        });
        ui.end_row();
    } else {
        ui.label(tr("Host"));
        ui.horizontal(|ui| {
            ui.add(egui::TextEdit::singleline(&mut c.host).hint_text("raspberrypi.local").desired_width(160.0));
            ui.label(tr("TCP port"));
            ui.add(egui::DragValue::new(&mut c.tcp_port).range(1..=65535));
        });
        ui.end_row();
    }
    refresh
}

/// The row of the read from the controller, in the grid: the button says how the last read went. True when it was pressed.
pub fn read_row(ui: &mut Ui, state: ReadState, enabled: bool) -> bool {
    ui.label("");
    let blue = Color32::from_rgb(0x2a, 0x6f, 0xd6);
    let (label, fill) = match state {
        ReadState::Reading => (tr("Reading…"), blue),
        ReadState::Done => (tr("Read OK"), blue),
        ReadState::Failed => (tr("Error reading"), Color32::from_rgb(0xc0, 0x30, 0x30)),
        ReadState::Idle => (tr("Read from device"), blue),
    };
    let button = egui::Button::new(RichText::new(label).color(Color32::WHITE).strong()).fill(fill);
    let on = enabled && !matches!(state, ReadState::Reading | ReadState::Done);
    let clicked = ui.add_enabled(on, button).on_hover_text(tr("If the machine is connected now, its work area and speed can be read from the controller.")).clicked();
    ui.end_row();
    clicked
}

/// The optional camera of a new device: nothing until "Add a camera"; then its name, where the picture comes from, the rotation and a
/// test. Its other settings are in the device window. `local` says whether this build can open a camera of the computer. The camera
/// to test, when Test was pressed.
pub fn camera_section(ui: &mut Ui, cameras: &mut Vec<CameraSource>, active: &mut usize, local: bool) -> Option<CameraSource> {
    let mut test = None;
    egui::CollapsingHeader::new(RichText::new(tr("Camera")).strong()).id_salt("add_cam").default_open(!cameras.is_empty()).show(ui, |ui| {
        let mut remove = false;
        if let Some(c) = cameras.first_mut() {
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut c.name).hint_text(tr("Camera name")).desired_width(180.0));
                if crate::window::delete_icon(ui, &tr("Delete this camera")).clicked() {
                    remove = true;
                }
            });
            ui.horizontal(|ui| {
                let is_local = matches!(c.kind, CameraKind::Local(_));
                let mut want_local = is_local;
                ui.radio_value(&mut want_local, false, tr("Address"));
                ui.radio_value(&mut want_local, true, tr("Camera of this computer"));
                if want_local != is_local {
                    c.kind = if want_local { CameraKind::Local(0) } else { CameraKind::Url(String::new()) };
                }
            });
            match &mut c.kind {
                CameraKind::Url(u) => crate::camera_feed::address_field(ui, u),
                CameraKind::Local(i) => {
                    ui.horizontal(|ui| {
                        ui.label(tr("Camera number"));
                        ui.add(egui::DragValue::new(i).range(0..=15));
                    });
                }
            }
            crate::camera_feed::stream_options_ui(ui, c);
            let testable = match &c.kind {
                CameraKind::Url(u) => base_core::camimg::valid_url(u),
                CameraKind::Local(_) => local,
            };
            ui.horizontal(|ui| {
                if ui.add_enabled(testable, egui::Button::new(tr("Test"))).on_hover_text(tr("Opens the picture of this camera as set here, without saving")).clicked() {
                    test = Some(c.clone());
                }
                ui.label(tr("Camera rotation"));
                for q in 0..4u8 {
                    ui.selectable_value(&mut c.rotation, q, format!("{}°", q as u32 * 90));
                }
            });
        } else if ui.button(tr("Add a camera")).clicked() {
            cameras.push(CameraSource::url(&crate::i18n::trf("Camera {}", &[&(cameras.len() + 1)]), "", 0));
            *active = cameras.len() - 1;
        }
        if remove {
            cameras.clear();
            *active = 0;
        }
    });
    test
}

/// The things worth checking before the device is used, and a line when the settings were read.
pub fn warnings(ui: &mut Ui, list: &[String], read: bool) {
    if read {
        ui.label(RichText::new(tr("The settings were read from the controller.")).color(Color32::from_rgb(0x2e, 0xa0, 0x4f)));
    }
    ui.add_space(6.0);
    ui.label(RichText::new(tr("Worth checking")).strong());
    for w in list {
        egui::Frame::new().fill(Color32::from_rgb(0xe8, 0x8a, 0x1f)).corner_radius(3).inner_margin(egui::Margin::symmetric(6, 3)).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new(format!("• {w}")).size(14.0).color(Color32::BLACK));
        });
    }
}

/// The buttons under the settings.
pub fn buttons(ui: &mut Ui, can_add: bool) -> Option<AddAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        let go = egui::Button::new(RichText::new(tr("Add device")).color(Color32::WHITE).strong()).fill(Color32::from_rgb(0x2e, 0xa0, 0x4f));
        if ui.add_enabled(can_add, go).clicked() {
            action = Some(AddAction::Add);
        }
        if ui.button(tr("More settings…")).on_hover_text(tr("Open the whole device window with what is entered here")).clicked() {
            action = Some(AddAction::More);
        }
        if ui.button(tr("I have no device")).on_hover_text(tr("Work on a project without a machine: a plain device called Dummy, with the default work area and speeds")).clicked() {
            action = Some(AddAction::Dummy);
        }
        if ui.button(tr("Cancel")).clicked() {
            action = Some(AddAction::Cancel);
        }
    });
    action
}

/// The common things worth checking: no port or host yet (for a machine that is connected), and the reminder that nothing was tried
/// on the machine (`last`, the program's own words for its kind of machine).
pub fn common_warnings(connection: &Connection, connects: bool, last: &str) -> Vec<String> {
    let mut w = vec![];
    let has_target = match connection.link {
        LinkKind::Serial => !connection.port.trim().is_empty(),
        LinkKind::Tcp => !connection.host.trim().is_empty(),
    };
    if connects && !has_target {
        w.push(tr("No port or host is set yet: choose it in the device settings before connecting.").to_string());
    }
    w.push(last.to_string());
    w
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_draws_the_shared_parts_and_a_camera_is_added_and_removed() {
        let ctx = egui::Context::default();
        let (mut cams, mut active) = (vec![], 0usize);
        let mut conn = Connection::default();
        let mut out = ctx.run_ui(Default::default(), |ui| {
            let mut open = true;
            let got = window(ui.ctx(), &mut open, |ui| {
                grid(ui, |ui| {
                    connection_rows(ui, &mut conn, &[]);
                    read_row(ui, ReadState::Failed, true);
                });
                camera_section(ui, &mut cams, &mut active, false);
                warnings(ui, &common_warnings(&conn, true, "try it"), false);
                true
            });
            assert!(got.is_none());
        });
        out.textures_delta.clear();
        assert!(cams.is_empty());
        cams.push(CameraSource::url("c", "", 0));
        let mut out = ctx.run_ui(Default::default(), |ui| {
            camera_section(ui, &mut cams, &mut active, false);
        });
        out.textures_delta.clear();
        assert_eq!(cams.len(), 1);
        assert_eq!(common_warnings(&conn, true, "x").len(), 2, "no port yet");
        assert_eq!(common_warnings(&conn, false, "x").len(), 1, "not connected at all");
    }
}
