//! The window of the settings of the controller (`$$`): read from the machine, searched, edited one at a time, backed up
//! to a text file and restored from it. GRBL only. The same in every program that drives a GRBL machine.
use crate::i18n::{tr, trf};
use crate::machine_ctl::{Machine, RESET_WORD};
use crate::theme;
use crate::window::{import_button, popup, save_button_enabled, DELETE_RED};
use base_core::control::{Change, FwCopies};
use base_core::grbl_settings as gs;
use egui::{Color32, RichText};

const ORANGE: Color32 = Color32::from_rgb(0xe8, 0x8a, 0x1f);

/// What the person asked for in the list of copies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyAct {
    /// Make a copy of the settings that were read, now.
    Make,
    /// Write the settings of this copy back (after a question).
    Restore(usize),
    Delete(usize),
}

/// `2026-10-14 15:04` (UTC) of a Unix time.
pub fn stamp(secs: u64) -> String {
    let d = base_core::backup::ymd(secs);
    format!("{}-{}-{} {:02}:{:02}", &d[..4], &d[4..6], &d[6..], secs / 3600 % 24, secs / 60 % 60)
}

/// The copies of the settings kept with the device, and what changed since the newest one. `can_make` is false until the
/// settings have been read.
pub fn copies_ui(ui: &mut egui::Ui, copies: &FwCopies, can_make: bool, changes: &[Change]) -> Option<CopyAct> {
    let mut act = None;
    if !changes.is_empty() {
        egui::Frame::new().fill(Color32::from_rgba_unmultiplied(0xe8, 0x8a, 0x1f, 40)).stroke(egui::Stroke::new(1.0, ORANGE)).inner_margin(6.0).corner_radius(4.0).show(ui, |ui| {
            ui.label(RichText::new(trf("Settings that differ from the copy saved with the device: {}. A new copy was saved.", &[&changes.len()])).strong());
            for c in changes.iter().take(8) {
                ui.label(RichText::new(format!("${}: {} → {}", c.id, c.old.as_deref().unwrap_or("–"), c.new.as_deref().unwrap_or("–"))).monospace());
            }
            if changes.len() > 8 {
                ui.label(trf("… and {} more", &[&(changes.len() - 8)]));
            }
        });
        ui.add_space(4.0);
    }
    egui::CollapsingHeader::new(RichText::new(trf("Copies of the settings ({})", &[&copies.0.len()])).strong()).id_salt("fw_copies").default_open(false).show(ui, |ui| {
        ui.label(RichText::new(tr("A copy of the settings of the controller is kept with the device. One is made when the machine is connected for the first time and again whenever the settings differ from the newest copy; the copies are part of the backup of the program.")).small().color(theme::text_dim()));
        if ui.add_enabled(can_make, egui::Button::new(tr("Make a copy now"))).clicked() {
            act = Some(CopyAct::Make);
        }
        for (i, c) in copies.0.iter().enumerate().rev() {
            ui.horizontal(|ui| {
                let why = match c.why.as_str() {
                    "own" => tr("after a change here"),
                    "manual" => tr("by hand"),
                    _ => tr("at connecting"),
                };
                ui.label(format!("{}  {}  ·  {}  ·  {}", stamp(c.secs), c.firmware, why, trf("{} settings", &[&c.values.len()])));
                if ui.small_button(tr("Restore…")).on_hover_text(tr("Compare with the controller and write the differences back")).clicked() {
                    act = Some(CopyAct::Restore(i));
                }
                if ui.small_button(tr("Delete")).clicked() {
                    act = Some(CopyAct::Delete(i));
                }
            });
        }
    });
    act
}

/// Do what the list of copies asked. Returns a line for the status bar.
pub fn run_copy_act(m: &mut Machine, copies: &mut FwCopies, act: CopyAct) -> Option<String> {
    match act {
        CopyAct::Make => {
            let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let firmware = if m.fw.info.is_known() { m.fw.info.label() } else { String::new() };
            copies.make(secs, &firmware, &m.fw.values).then(|| trf("A copy of the {} settings was saved with the device.", &[&m.fw.values.len()]))
        }
        CopyAct::Restore(i) => {
            if let Some(c) = copies.0.get(i) {
                m.fw.import = Some(c.values.iter().map(|(k, v)| (*k, v.clone())).collect());
            }
            None
        }
        CopyAct::Delete(i) => {
            copies.remove(i);
            None
        }
    }
}

/// The window. `open` is cleared by its close button. Returns a line for the status bar. `copies` are the copies of the
/// settings kept with the device.
pub fn window(ctx: &egui::Context, m: &mut Machine, copies: &mut FwCopies, open: &mut bool) -> Option<String> {
    let mut say = None;
    let was_open = *open;
    popup(
        ctx,
        "firmware_window",
        &tr("Firmware settings ($$)"),
        [640.0, 560.0],
        open,
        |ui| {
            if let Some(a) = copies_ui(ui, copies, !m.fw.values.is_empty(), &m.fw.changes) {
                say = run_copy_act(m, copies, a);
            }
            if let Some(s) = page(ui, m) {
                say = Some(s);
            }
        },
    );
    if was_open || m.fw.import.is_some() || m.fw.reset_open || m.fw.write_ask.is_some() {
        if let Some(s) = dialogs(ctx, m) {
            say = Some(s);
        }
    }
    if m.fw.reread_at.is_some() || m.fw.reading {
        ctx.request_repaint_after(std::time::Duration::from_millis(250));
    }
    say
}

/// The page itself: the buttons and the table.
pub fn page(ui: &mut egui::Ui, m: &mut Machine) -> Option<String> {
    let mut say = None;
    egui::Frame::new().fill(Color32::from_rgba_unmultiplied(0xe8, 0x8a, 0x1f, 40)).stroke(egui::Stroke::new(1.0, ORANGE)).inner_margin(8.0).corner_radius(4.0).show(ui, |ui| {
        ui.label(RichText::new(tr("These settings live in the controller, not in the device. Change one at a time and write down the previous value: a wrong value can make the machine move the wrong distance. The first change of a session saves all settings to a file.")).strong());
    });
    ui.add_space(4.0);
    if !m.connected {
        ui.label(
            RichText::new(tr("Connect to the machine first (Device tab)")).color(theme::text_dim()),
        );
        return None;
    }
    ui.horizontal_wrapped(|ui| {
        let read = if m.fw.reading {
            tr("Reading…")
        } else {
            tr("Read the settings ($$)")
        };
        if ui
            .add_enabled(!m.fw.reading, egui::Button::new(read))
            .clicked()
        {
            m.read_all();
        }
        if save_button_enabled(ui, !m.fw.values.is_empty(), &tr("Back up to a file…")).clicked() {
            say = export(m);
        }
        if import_button(ui, &tr("Restore from a file…")).clicked() {
            say = import_pick(m);
        }
        let red = egui::Button::new(
            RichText::new(tr("Factory settings ($RST=$)…")).color(Color32::WHITE),
        )
        .fill(DELETE_RED);
        if ui.add(red).clicked() {
            m.fw.reset_open = true;
            m.fw.reset_text.clear();
        }
    });
    if m.fw.info.is_known() {
        ui.label(
            RichText::new(m.fw.info.raw.join("  "))
                .small()
                .color(theme::text_dim()),
        );
    }
    for r in &m.fw.report {
        ui.label(RichText::new(r).color(ORANGE));
    }
    if m.fw.values.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(tr("Nothing read yet.")).color(theme::text_dim()));
        return say;
    }
    ui.horizontal(|ui| {
        ui.label(tr("Search"));
        ui.add(egui::TextEdit::singleline(&mut m.fw.search).desired_width(160.0));
        ui.checkbox(
            &mut m.fw.only_changed,
            tr("Only those that differ from the typical value"),
        );
    });
    ui.add_space(4.0);
    let needle = m.fw.search.trim().to_lowercase();
    let only = m.fw.only_changed;
    let mut ask = None;
    egui::ScrollArea::vertical()
        .max_height(crate::window::fill_size(ui).y)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            egui::Grid::new("fw_table")
                .num_columns(5)
                .striped(true)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    for h in ["Setting", "Value", "Unit", "Typical", ""] {
                        ui.label(RichText::new(tr(h)).strong());
                    }
                    ui.end_row();
                    let values = m.fw.values.clone();
                    for (id, value) in values {
                        let meta = gs::info(id);
                        let name = meta.map(|s| tr(s.name)).unwrap_or("?");
                        if !needle.is_empty()
                            && !(format!("${id}").contains(&needle)
                                || name.to_lowercase().contains(&needle)
                                || meta
                                    .is_some_and(|s| tr(s.desc).to_lowercase().contains(&needle)))
                        {
                            continue;
                        }
                        if only && meta.is_some_and(|s| gs::same_value(s.typical, &value)) {
                            continue;
                        }
                        let critical = meta.is_some_and(|s| s.critical);
                        let label = RichText::new(format!("${id}  {name}"));
                        let r = ui.label(if critical {
                            label.color(ORANGE).strong()
                        } else {
                            label
                        });
                        if let Some(s) = meta {
                            r.on_hover_text(tr(s.desc));
                        }
                        let text = m.fw.typed.entry(id).or_insert_with(|| value.clone());
                        ui.add(egui::TextEdit::singleline(text).desired_width(90.0));
                        ui.label(
                            RichText::new(meta.map(|s| s.unit).unwrap_or(""))
                                .color(theme::text_dim()),
                        );
                        ui.label(
                            RichText::new(meta.map(|s| s.typical).unwrap_or(""))
                                .color(theme::text_dim()),
                        );
                        if !gs::same_value(text, &value) {
                            match gs::validate(id, text) {
                                Ok(v) => {
                                    if ui.button(tr("Write")).clicked() {
                                        ask = Some((id, v));
                                    }
                                }
                                Err(e) => {
                                    ui.label(RichText::new(tr(e)).color(DELETE_RED));
                                }
                            }
                        } else {
                            ui.label("");
                        }
                        ui.end_row();
                    }
                });
        });
    if ask.is_some() {
        m.fw.write_ask = ask;
    }
    say
}

fn export(m: &mut Machine) -> Option<String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let date = base_core::backup::ymd(now);
    let p = rfd::FileDialog::new()
        .add_filter("GRBL settings", &["txt"])
        .set_file_name(format!("grbl-settings-{date}.txt"))
        .save_file()?;
    let text = gs::backup_text(&m.fw.values, &m.fw.info.raw, &date);
    Some(match std::fs::write(&p, text) {
        Ok(_) => trf("Saved the settings to {}", &[&p.display()]),
        Err(e) => trf("Export failed: {}", &[&e]),
    })
}

fn import_pick(m: &mut Machine) -> Option<String> {
    let p = rfd::FileDialog::new()
        .add_filter("GRBL settings", &["txt"])
        .pick_file()?;
    let text = match std::fs::metadata(&p) {
        Ok(meta) if meta.len() > 40_000 => Err("the file is too large".to_string()),
        _ => std::fs::read_to_string(&p).map_err(|e| e.to_string()),
    };
    match text.and_then(|t| gs::parse_backup(&t)) {
        Ok(file) => {
            m.fw.import = Some(file);
            None
        }
        Err(e) => Some(trf("Cannot use the file: {}", &[&e])),
    }
}

/// The questions of the page: write one setting, write a file of them, restore the factory settings.
pub(crate) fn dialogs(ctx: &egui::Context, m: &mut Machine) -> Option<String> {
    let mut say = None;
    if let Some((id, v)) = m.fw.write_ask.clone() {
        let old = m.fw.values.get(&id).cloned().unwrap_or_default();
        let mut open = true;
        let (mut yes, mut no) = (false, false);
        popup(
            ctx,
            "fw_write",
            &tr("Write a setting"),
            [440.0, 150.0],
            &mut open,
            |ui| {
                ui.label(trf("Write ${} = {} to the controller? It is now {}. Change one setting at a time and note the old value.", &[&id, &v, &old]));
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    yes = ui
                        .add(egui::Button::new(RichText::new(tr("Yes")).strong()))
                        .clicked();
                    no = ui.button(tr("Cancel")).clicked();
                });
            },
        );
        if yes {
            say = m.write_setting(id, &v);
            m.fw.write_ask = None;
        } else if no || !open {
            m.fw.write_ask = None;
        }
    }
    if let Some(file) = m.fw.import.clone() {
        let mut open = true;
        let (mut write, mut cancel) = (false, false);
        let current = m.fw.values.clone();
        popup(
            ctx,
            "fw_import",
            &tr("Restore the settings from a file"),
            [520.0, 420.0],
            &mut open,
            |ui| {
                ui.label(tr("These settings of the file differ from the controller. Nothing is written until you press Write all."));
                egui::ScrollArea::vertical()
                    .max_height(260.0)
                    .show(ui, |ui| {
                        egui::Grid::new("fw_imp")
                            .num_columns(3)
                            .striped(true)
                            .show(ui, |ui| {
                                ui.label(RichText::new(tr("Setting")).strong());
                                ui.label(RichText::new(tr("Controller")).strong());
                                ui.label(RichText::new(tr("File")).strong());
                                ui.end_row();
                                let mut shown = 0;
                                for (id, v) in &file {
                                    let now = current.get(id);
                                    if now.is_some_and(|n| gs::same_value(n, v)) {
                                        continue;
                                    }
                                    shown += 1;
                                    ui.label(format!(
                                        "${id}  {}",
                                        gs::info(*id).map(|s| tr(s.name)).unwrap_or("?")
                                    ));
                                    ui.label(now.cloned().unwrap_or_else(|| "–".into()));
                                    ui.label(v);
                                    ui.end_row();
                                }
                                if shown == 0 {
                                    ui.label(tr("The controller already has all of them."));
                                    ui.end_row();
                                }
                            });
                    });
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    let b = egui::Button::new(
                        RichText::new(tr("Write all"))
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(DELETE_RED);
                    write = ui.add(b).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
            },
        );
        if write {
            let n = file.len();
            say = m.write_settings(file).or_else(|| {
                Some(trf(
                    "Writing {} settings; they are read back afterwards",
                    &[&n],
                ))
            });
            m.fw.import = None;
        } else if cancel || !open {
            m.fw.import = None;
        }
    }
    if m.fw.reset_open {
        let mut open = true;
        let (mut go, mut cancel) = (false, false);
        let mut text = std::mem::take(&mut m.fw.reset_text);
        popup(
            ctx,
            "fw_reset",
            &tr("Restore the factory settings"),
            [460.0, 220.0],
            &mut open,
            |ui| {
                ui.label(RichText::new(tr("This writes $RST=$: every setting of the controller goes back to the firmware's defaults. Step counts, travel, speeds and homing will all change.")).color(DELETE_RED).strong());
                ui.label(trf(
                    "Back the settings up first. To go on, type {}:",
                    &[&RESET_WORD],
                ));
                ui.add(egui::TextEdit::singleline(&mut text).desired_width(160.0));
                let ready = text.trim() == RESET_WORD;
                ui.horizontal(|ui| {
                    let b = egui::Button::new(
                        RichText::new(tr("Restore the factory settings"))
                            .color(Color32::WHITE)
                            .strong(),
                    )
                    .fill(DELETE_RED);
                    go = ui.add_enabled(ready, b).clicked();
                    cancel = ui.button(tr("Cancel")).clicked();
                });
            },
        );
        m.fw.reset_text = text;
        if go {
            say = m
                .reset_settings()
                .or_else(|| Some(tr("Factory settings written ($RST=$)").to_string()));
            m.fw.reset_open = false;
        } else if cancel || !open {
            m.fw.reset_open = false;
        }
    }
    say
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_window_draws_with_and_without_settings_and_the_filters_work() {
        let ctx = egui::Context::default();
        let mut m = Machine::new(&ctx);
        let mut open = true;
        let run = |m: &mut Machine, open: &mut bool| {
            let mut out = ctx.run_ui(egui::RawInput::default(), |_ui| {
                window(&ctx, m, &mut FwCopies::default(), open);
            });
            out.textures_delta.clear();
        };
        run(&mut m, &mut open);
        m.connected = true;
        run(&mut m, &mut open);
        m.fw.values = [(30, "24000.000"), (110, "3000.000"), (130, "300.000")]
            .into_iter()
            .map(|(k, v)| (k, v.to_string()))
            .collect();
        m.fw.search = "travel".into();
        run(&mut m, &mut open);
        m.fw.only_changed = true;
        m.fw.search.clear();
        run(&mut m, &mut open);
        m.fw.write_ask = Some((110, "5000".into()));
        m.fw.import = Some(vec![(110, "5000".into())]);
        m.fw.reset_open = true;
        run(&mut m, &mut open);
        assert!(open);
    }

    #[test]
    fn the_window_keeps_its_height_while_it_is_drawn_again_and_again() {
        let ctx = egui::Context::default();
        let mut m = Machine::new(&ctx);
        m.connected = true;
        m.fw.values = (0..40).map(|i| (100 + i, "1.000".to_string())).collect();
        let mut open = true;
        let mut heights = vec![];
        for _ in 0..12 {
            let input = egui::RawInput { screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(1600.0, 1000.0))), ..Default::default() };
            let mut out = ctx.run_ui(input, |_ui| {
                window(&ctx, &mut m, &mut FwCopies::default(), &mut open);
            });
            out.textures_delta.clear();
            if let Some(r) = ctx.memory(|mem| mem.area_rect(egui::Id::unique(Some("firmware_window")))) {
                heights.push(r.height());
            }
        }
        assert!((heights[11] - heights[3]).abs() < 1.0 && heights[11] < 700.0, "{heights:?}");
    }
}
