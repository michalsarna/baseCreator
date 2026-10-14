//! Windows around a job: the pause where the person changes the tool or the pen, and "start from this line".
use crate::i18n::{tr, trf};
use crate::machine_ctl::{Machine, TlsMode};
use crate::theme;
use crate::window::{popup, DELETE_RED};
use base_core::control::{Axis, Control};
use base_core::resume;
use egui::{Color32, RichText};

/// The window of a pause (M0, M6) of the job: what to do, a probe to set Z again, and the way on. Returns a line for the
/// status bar. `probe` is the settings and the diameter of the tool when the machine has a probe to offer.
pub fn barrier_window(
    ctx: &egui::Context,
    m: &mut Machine,
    probe: Option<(&Control, f64)>,
) -> Option<String> {
    let b = m.barrier.clone()?;
    let mut say = None;
    let mut open = true;
    let (mut go, mut stop) = (false, false);
    popup(
        ctx,
        "barrier_window",
        &tr("The job is paused"),
        [460.0, 240.0],
        &mut open,
        |ui| {
            ui.label(RichText::new(b.message()).size(16.0).strong());
            ui.add_space(4.0);
            ui.label(RichText::new(tr("Change the tool or the pen now. The machine waits; nothing moves until you go on.")).color(theme::text_dim()));
            if let Some((c, d)) = probe {
                ui.add_space(6.0);
                ui.horizontal_wrapped(|ui| {
                if ui.button(tr("Probe Z")).on_hover_text(tr("Set Z again with the probe: the new tool touches the plate and Z becomes the thickness of the plate")).clicked() {
                    if let Err(e) = m.probe(c, Axis::Z, -1, d) {
                        say = Some(e);
                    }
                }
                if let Some((p, touched)) = m.probe {
                    let t = if touched { tr("touched") } else { tr("no contact") };
                    ui.label(RichText::new(trf("Last probe: Z {} ({})", &[&format!("{:.3}", p[2]), &t])).color(theme::text_dim()));
                }
            });
                if c.tls.set && c.tls.ref_wz.is_some() {
                    ui.horizontal_wrapped(|ui| {
                        if ui.add_enabled(!m.tls_running(), egui::Button::new(tr("Measure the tool (sensor)"))).on_hover_text(tr("The machine goes to the tool length sensor; the new tool gets the Z of the reference tool")).clicked() {
                            if let Err(e) = m.tls_start(&c.tls, TlsMode::Tool) {
                                say = Some(e);
                            }
                        }
                        if m.tls_running() {
                            ui.label(RichText::new(tr("Measuring…")).color(theme::text_dim()));
                        }
                    });
                }
            }
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                go = ui
                    .add(
                        egui::Button::new(
                            RichText::new(tr("Continue the job"))
                                .color(Color32::WHITE)
                                .strong(),
                        )
                        .fill(crate::window::SAVE_GREEN),
                    )
                    .clicked();
                stop = ui
                    .add(
                        egui::Button::new(RichText::new(tr("Stop the job")).color(Color32::WHITE))
                            .fill(DELETE_RED),
                    )
                    .clicked();
            });
        },
    );
    if go {
        m.continue_job();
    } else if stop {
        m.stop_job();
    }
    say
}

/// "Start from line…": what the window remembers.
pub struct StartFrom {
    pub open: bool,
    /// The line to start at, counted from 1 as in an editor.
    pub line: usize,
}

impl Default for StartFrom {
    fn default() -> StartFrom {
        StartFrom {
            open: false,
            line: 1,
        }
    }
}

/// The window. `lines` is the whole program, `safe_z` and `spin_up` come from the device. Returns the line to start at
/// (0-based) when the person pressed the button.
pub fn start_from_window(
    ctx: &egui::Context,
    st: &mut StartFrom,
    lines: &[String],
    safe_z: f64,
    spin_up: f64,
) -> Option<usize> {
    if !st.open {
        return None;
    }
    let mut out = None;
    let total = lines.len().max(1);
    st.line = st.line.clamp(1, total);
    let mut open = st.open;
    let mut line = st.line;
    popup(
        ctx,
        "start_from_window",
        &tr("Start from line"),
        [560.0, 440.0],
        &mut open,
        |ui| {
            ui.label(RichText::new(tr("The machine is brought to the state the program has at that line: units, spindle, speed, position. Be sure the tool and the work are where they were.")).color(theme::text_dim()));
            ui.horizontal(|ui| {
                ui.label(tr("Line"));
                ui.add(egui::DragValue::new(&mut line).range(1..=total).speed(1.0));
                ui.label(RichText::new(trf("of {}", &[&total])).color(theme::text_dim()));
            });
            let at = line - 1;
            egui::Frame::new()
                .fill(theme::console_bg())
                .stroke(egui::Stroke::new(1.0, theme::border()))
                .inner_margin(4.0)
                .show(ui, |ui| {
                    for i in at.saturating_sub(3)..(at + 4).min(lines.len()) {
                        let text = RichText::new(format!("{:>5}  {}", i + 1, lines[i]))
                            .monospace()
                            .size(11.0);
                        ui.label(if i == at {
                            text.strong().color(theme::accent())
                        } else {
                            text.color(theme::text_dim())
                        });
                    }
                });
            let plan = resume::plan(lines, at, safe_z, spin_up);
            match &plan {
                Ok(p) => {
                    let m = resume::modal_at(lines, at);
                    if at > 0 {
                        let spindle = match m.spindle {
                            Some(d) => trf("spindle on (M{}) at {}", &[&d, &m.rpm]),
                            None => tr("spindle off").to_string(),
                        };
                        let num = |v: f64| {
                            if v.is_finite() {
                                format!("{v}")
                            } else {
                                "–".to_string()
                            }
                        };
                        ui.label(trf(
                            "At this line: X {} Y {} Z {}, {}, feed {}",
                            &[
                                &num(m.pos[0]),
                                &num(m.pos[1]),
                                &num(m.pos[2]),
                                &spindle,
                                &m.feed,
                            ],
                        ));
                        ui.label(
                            RichText::new(trf(
                                "{} lines will be sent first to set this up.",
                                &[&(p.len() - (lines.len() - at))],
                            ))
                            .color(theme::text_dim()),
                        );
                    }
                }
                Err(e) => {
                    ui.label(
                        RichText::new(trf("Cannot start here: {}", &[&tr(e)])).color(DELETE_RED),
                    );
                }
            }
            ui.add_space(8.0);
            if ui
                .add_enabled(
                    plan.is_ok(),
                    egui::Button::new(
                        RichText::new(tr("Start from here"))
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(crate::window::SAVE_GREEN),
                )
                .clicked()
            {
                out = Some(at);
            }
        },
    );
    st.line = line;
    st.open = open && out.is_none();
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_windows_draw() {
        let ctx = egui::Context::default();
        let mut m = Machine::new(&ctx);
        m.barrier = Some(crate::machine_ctl::Barrier {
            text: "M0 (Change the bit to: V-bit)".into(),
            line: 3,
        });
        let c = Control::default();
        let lines: Vec<String> = "G21 G90\nG0 X1 Y1\nS1000 M3\nF200\nG1 Z-1\nG1 X5\nM5"
            .lines()
            .map(String::from)
            .collect();
        let mut st = StartFrom {
            open: true,
            line: 6,
        };
        let mut sent = None;
        let mut out = ctx.run_ui(egui::RawInput::default(), |_ui| {
            barrier_window(&ctx, &mut m, Some((&c, 3.0)));
            sent = start_from_window(&ctx, &mut st, &lines, 5.0, 1.0);
        });
        out.textures_delta.clear();
        assert!(sent.is_none() && st.open);
        // A line past the end is brought back, and a closed window draws nothing.
        st.line = 999;
        st.open = false;
        assert!(start_from_window(&ctx, &mut st, &lines, 5.0, 1.0).is_none());
    }
}
