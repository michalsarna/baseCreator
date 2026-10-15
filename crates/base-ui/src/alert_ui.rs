//! The window that says the machine stopped: an alarm, an error in the middle of a job, a controller that does not
//! answer. Big enough to be seen from across the workshop, with STOP (red) and UNLOCK (orange).
use crate::i18n::tr;
use crate::machine_ctl::Machine;
use egui::{Color32, RichText};

const RED: Color32 = Color32::from_rgb(0xc0, 0x30, 0x30);
const ORANGE: Color32 = Color32::from_rgb(0xe8, 0x7a, 0x14);

/// Show the window while the machine has a problem. Returns true when a button was used.
pub fn window(ctx: &egui::Context, m: &mut Machine) -> bool {
    // An alarm that was cleared (or a link that went away) is no problem any more. The state report comes a little after the
    // alarm itself, so "not in alarm" only counts after the alarm state was seen.
    let seen_id = egui::Id::new("machine_alert_seen_alarm");
    let seen: bool = ctx.data(|d| d.get_temp(seen_id)).unwrap_or(false);
    if m.is_alarm() {
        ctx.data_mut(|d| d.insert_temp(seen_id, true));
    }
    if m.problem.as_ref().is_some_and(|p| p.alarm) && (!m.connected || (seen && !m.is_alarm())) {
        m.problem = None;
        ctx.data_mut(|d| d.insert_temp(seen_id, false));
    }
    let Some(p) = m.problem.clone() else { return false };
    let mut used = false;
    let mut close = false;
    let frame = egui::Frame::window(&ctx.global_style()).inner_margin(egui::Margin::same(22)).stroke(egui::Stroke::new(3.0, RED));
    egui::Window::new("machine_alert").title_bar(false).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).frame(frame).order(egui::Order::Foreground).show(ctx, |ui| {
        ui.set_width(460.0);
        ui.vertical_centered(|ui| {
            ui.label(RichText::new(&p.title).size(30.0).strong().color(RED));
        });
        ui.add_space(10.0);
        ui.label(RichText::new(&p.text).size(18.0));
        if let Some(l) = &p.line {
            ui.add_space(6.0);
            ui.label(RichText::new(l).monospace().size(15.0));
        }
        ui.add_space(8.0);
        ui.label(RichText::new(tr("Look why it stopped before you unlock the machine: the tool may be in the material.")).size(14.0).color(crate::theme::text_dim()));
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            let size = egui::vec2((ui.available_width() - ui.spacing().item_spacing.x) / 2.0, 56.0);
            let stop = egui::Button::new(RichText::new(tr("Stop").to_uppercase()).size(24.0).strong().color(Color32::WHITE)).fill(RED);
            if ui.add_sized(size, stop).on_hover_text(tr("Stop the job and the spindle")).clicked() {
                m.stop_job();
                if !p.alarm {
                    close = true;
                }
                used = true;
            }
            let unlock = egui::Button::new(RichText::new(tr("Unlock").to_uppercase()).size(24.0).strong().color(Color32::WHITE)).fill(ORANGE);
            if ui.add_sized(size, unlock).on_hover_text(tr("Clear an alarm ($X)")).clicked() {
                m.unlock();
                close = true;
                used = true;
            }
        });
        ui.add_space(6.0);
        ui.vertical_centered(|ui| {
            if ui.small_button(tr("Close")).clicked() {
                close = true;
            }
        });
    });
    if close {
        m.problem = None;
    }
    used
}

/// An alarm of the controller: the window stays until it is unlocked.
pub fn raise_alarm(m: &mut Machine, code: u32, explanation: &str) {
    let title = if code > 0 { format!("{} {code}", tr("Alarm").to_uppercase()) } else { tr("Alarm").to_uppercase() };
    m.problem = Some(crate::machine_ctl::Problem { alarm: true, title, text: explanation.to_string(), line: None });
}

/// The controller refused a line of the job (or the job could not go on): `text` says what and where.
pub fn raise_job_stopped(m: &mut Machine, text: &str) {
    m.problem = Some(crate::machine_ctl::Problem { alarm: false, title: tr("The job stopped").to_string(), text: text.to_string(), line: None });
}

/// A job is running and the controller has not answered for a while.
pub fn raise_no_answer(m: &mut Machine, text: &str) {
    m.problem = Some(crate::machine_ctl::Problem { alarm: false, title: tr("No answer from the machine").to_string(), text: text.to_string(), line: None });
}
