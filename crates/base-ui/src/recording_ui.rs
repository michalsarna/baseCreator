//! The Recording tab of the side panel, the same in every program: a panel for each camera of the device, named after it, with
//! everything it records (photo, video, time-lapse, the video format, the way of the time-lapse, the folder) and the way its
//! view opens; under them the photos of a time-lapse with the head moved aside and the long lines sent in parts.
use crate::camera_rec::Rigs;
use crate::camera_view::{StreamMode, MODE_ICONS};
use crate::i18n::tr;
use crate::theme;
use base_core::camimg::{CameraSource, CameraView};
use egui::{RichText, Ui};

/// What changed on the tab, for the program to keep.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TabOut {
    /// The recording settings of a camera (they are in `CameraSource::rec` now).
    pub cameras: bool,
    /// The way the view of a camera opens was picked: the camera and the way.
    pub mode: Option<(usize, StreamMode)>,
    /// The photos aside or the long lines.
    pub machine: bool,
}

/// What the tab needs of the machine.
pub struct MachineCtx {
    /// The machine position now, for "Use the current position".
    pub mpos: Option<[f64; 3]>,
    /// The machine has a spindle that may keep running on the way aside.
    pub spindle: bool,
}

/// The tab. `rigs` must be in step with `cams` (`Rigs::sync`).
pub fn tab(
    ui: &mut Ui,
    rigs: &mut Rigs,
    cams: &mut [CameraSource],
    view: &CameraView,
    park: &mut base_core::photo_park::ParkSettings,
    split: &mut base_core::gcode_split::SplitSettings,
    m: &MachineCtx,
) -> TabOut {
    let mut out = TabOut::default();
    if cams.is_empty() {
        ui.label(RichText::new(tr("This device has no camera that is switched on. Add one in the device settings.")).color(theme::text_dim()));
    }
    for (i, cam) in cams.iter_mut().enumerate() {
        egui::CollapsingHeader::new(RichText::new(&cam.name).strong()).id_salt(("recording_camera", i)).default_open(true).show(ui, |ui| {
            if !cam.enabled {
                ui.label(RichText::new(tr("This camera is switched off in the device settings.")).color(theme::text_dim()));
                return;
            }
            ui.horizontal_wrapped(|ui| {
                ui.label(tr("Camera view"));
                let now = StreamMode::of(view, Some(cam));
                for (mode, icon) in MODE_ICONS {
                    if crate::widgets::toggle_icon(ui, icon, &mode.tip(), now == mode) {
                        out.mode = Some((i, mode));
                    }
                }
            });
            if let Some(rec) = rigs.rec(i) {
                if rec.controls(ui) {
                    cam.rec = Some(rec.settings.clone());
                    out.cameras = true;
                }
            }
        });
    }
    out.machine = crate::control_ui::park_and_split_ui(ui, park, split, m.mpos, m.spindle);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_tab_draws_a_panel_for_each_camera_and_the_machine_settings() {
        let mut cams = vec![CameraSource::url("Top camera", "http://127.0.0.1:1/a", 0), CameraSource::url("Side camera", "http://127.0.0.1:1/b", 0)];
        cams[1].enabled = false;
        let mut rigs = Rigs::default();
        rigs.sync(&cams, None);
        let (mut park, mut split) = Default::default();
        let ctx = egui::Context::default();
        for _ in 0..2 {
            let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
                let o = tab(ui, &mut rigs, &mut cams, &CameraView::default(), &mut park, &mut split, &MachineCtx { mpos: None, spindle: true });
                assert_eq!(o, TabOut::default(), "nothing changes by drawing");
            });
            out.textures_delta.clear();
        }
        // No camera: the tab still has the settings of the machine.
        let mut none: Vec<CameraSource> = vec![];
        rigs.sync(&none, None);
        let mut out = ctx.run_ui(egui::RawInput::default(), |ui| {
            tab(ui, &mut rigs, &mut none, &CameraView::default(), &mut park, &mut split, &MachineCtx { mpos: Some([1.0, 2.0, 0.0]), spindle: false });
        });
        out.textures_delta.clear();
    }
}
