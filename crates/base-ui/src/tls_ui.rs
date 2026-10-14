//! The wizard of the tool length sensor: where the sensor is, the first (reference) tool, and the measuring of every
//! tool after it, so that a new bit gets the same Z as the one before.
use crate::i18n::{tr, trf};
use crate::machine_ctl::{Machine, TlsMode};
use crate::theme;
use crate::window::popup;
use base_core::control::Tls;
use egui::{Color32, RichText};

const ORANGE: Color32 = Color32::from_rgb(0xe8, 0x8a, 0x1f);
const GREEN: Color32 = Color32::from_rgb(0x2e, 0xa0, 0x4f);

#[derive(Default)]
pub struct TlsUi {
    pub open: bool,
}

/// The line for the status bar after a measurement.
pub fn describe(delta: f64) -> String {
    if delta.abs() < 0.0005 {
        tr("The tool is as long as the reference tool.").to_string()
    } else if delta > 0.0 {
        trf("The tool is {} mm longer than the reference tool.", &[&format!("{:.3}", delta)])
    } else {
        trf("The tool is {} mm shorter than the reference tool.", &[&format!("{:.3}", -delta)])
    }
}

pub fn window(ctx: &egui::Context, st: &mut TlsUi, m: &mut Machine, tls: &mut Tls) -> Option<String> {
    if !st.open {
        return None;
    }
    let mut say = None;
    let mut open = st.open;
    popup(ctx, "tls_window", &tr("Tool length sensor"), [600.0, 620.0], &mut open, |ui| {
        ui.label(RichText::new(tr("A tool length sensor is a switch at a fixed place on the machine. The tip of every tool touches it, so a new bit can get exactly the same Z as the one before without touching the work again. Its position is in machine coordinates: home the machine first, and never move the sensor.")).color(theme::text_dim()));
        ui.add_space(6.0);
        if !m.connected {
            ui.label(RichText::new(tr("Connect to the machine first (Device tab)")).color(theme::text_dim()));
            return;
        }
        // ---- 1. the place ----
        ui.label(RichText::new(tr("1. Where the sensor is")).strong());
        if tls.set {
            ui.label(RichText::new(trf("Machine X {}  Y {}, approach from Z {}", &[&tls.x, &tls.y, &tls.start_z])).monospace().color(GREEN));
        } else {
            ui.label(RichText::new(tr("Not set yet.")).color(ORANGE));
        }
        ui.label(tr("Jog the tool over the middle of the sensor, a few millimetres above it, and press the button."));
        let here = m.mpos.iter().all(|v| v.is_finite());
        if ui.add_enabled(here && !m.job_running(), egui::Button::new(tr("The sensor is here"))).clicked() {
            tls.x = (m.mpos[0] * 1000.0).round() / 1000.0;
            tls.y = (m.mpos[1] * 1000.0).round() / 1000.0;
            tls.start_z = (m.mpos[2] * 1000.0).round() / 1000.0;
            tls.set = true;
            tls.ref_wz = None;
        }
        egui::Grid::new("tls_settings").num_columns(2).spacing([12.0, 4.0]).show(ui, |ui| {
            let mm = |ui: &mut egui::Ui, label: &str, v: &mut f64, lo: f64, hi: f64| {
                ui.label(label);
                ui.add(egui::DragValue::new(v).range(lo..=hi).speed(0.5).max_decimals(3).suffix(" mm"));
                ui.end_row();
            };
            mm(ui, &tr("Travel height (machine Z)"), &mut tls.clear_z, -5000.0, 5000.0);
            mm(ui, &tr("Fail distance"), &mut tls.fail_distance, 1.0, 1000.0);
            mm(ui, &tr("Retract between the touches"), &mut tls.retract, 0.1, 100.0);
            ui.label(tr("Quick touch"));
            ui.add(egui::DragValue::new(&mut tls.seek_feed).range(1.0..=10_000.0).speed(5.0).suffix(" mm/min"));
            ui.end_row();
            ui.label(tr("Slow touch"));
            ui.add(egui::DragValue::new(&mut tls.slow_feed).range(1.0..=10_000.0).speed(1.0).suffix(" mm/min"));
            ui.end_row();
        });
        if let Err(e) = tls.check() {
            ui.label(RichText::new(tr(e)).color(ORANGE));
        }
        ui.add_space(8.0);
        // ---- 2. the reference ----
        ui.label(RichText::new(tr("2. The first tool, the reference")).strong());
        ui.label(tr("Put in the first bit and set Z on the work with it (the Control tab: probe Z, or touch the surface and press Z0). Then measure it on the sensor."));
        ui.horizontal(|ui| {
            if ui.add_enabled(tls.set && !m.tls_running(), egui::Button::new(tr("Measure the reference tool"))).on_hover_text(tr("The machine goes to the sensor and the tool touches it twice")).clicked() {
                if let Err(e) = m.tls_start(tls, TlsMode::Reference) {
                    say = Some(e);
                }
            }
            match tls.ref_wz {
                Some(z) => {
                    ui.label(RichText::new(trf("Reference: Z {} at the sensor", &[&format!("{:.3}", z)])).color(GREEN));
                    if ui.small_button(tr("Forget")).clicked() {
                        tls.ref_wz = None;
                    }
                }
                None => {
                    ui.label(RichText::new(tr("Not measured yet.")).color(ORANGE));
                }
            }
        });
        ui.add_space(8.0);
        // ---- 3. the other tools ----
        ui.label(RichText::new(tr("3. Every other tool")).strong());
        ui.label(tr("When a job pauses for a change of the bit, the window offers Measure the tool. It can also be done here, at any time."));
        ui.horizontal(|ui| {
            if ui.add_enabled(tls.set && tls.ref_wz.is_some() && !m.tls_running() && !m.job_running(), egui::Button::new(tr("Measure this tool and set Z"))).on_hover_text(tr("After the touch Z is set to the Z of the reference tool at the sensor")).clicked() {
                if let Err(e) = m.tls_start(tls, TlsMode::Tool) {
                    say = Some(e);
                }
            }
            if ui.add_enabled(tls.set && !m.tls_running() && !m.job_running(), egui::Button::new(tr("Only measure"))).on_hover_text(tr("Nothing is changed; the difference from the reference is shown")).clicked() {
                if let Err(e) = m.tls_start(tls, TlsMode::Test) {
                    say = Some(e);
                }
            }
        });
        if m.tls_running() {
            ui.label(RichText::new(tr("Measuring…")).color(ORANGE));
        }
        ui.add_space(8.0);
        ui.label(RichText::new(tr("The first time, keep a hand near the stop: the tool goes down towards the sensor.")).small().color(theme::text_dim()));
    });
    st.open = open;
    say
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_draws_and_the_difference_is_described() {
        let ctx = egui::Context::default();
        let mut m = Machine::new(&ctx);
        let mut st = TlsUi { open: true };
        let mut tls = Tls::default();
        for connected in [false, true] {
            m.connected = connected;
            m.mpos = [-10.0, -20.0, -30.0];
            tls.ref_wz = connected.then_some(-31.3);
            let mut out = ctx.run_ui(egui::RawInput::default(), |_ui| {
                window(&ctx, &mut st, &mut m, &mut tls);
            });
            out.textures_delta.clear();
        }
        assert!(describe(3.1).contains("3.100") && describe(-0.5).contains("0.500") && describe(0.0).len() > 5);
    }
}
