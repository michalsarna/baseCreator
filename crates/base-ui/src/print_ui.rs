//! File > Print…: the design on a paper printer. The page is a PDF made by `base_core::print`; it goes to the
//! system printer (`lp` on macOS and Linux, the Print verb of the PDF viewer on Windows) or is saved or opened.
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::{popup, save_button_enabled};
use base_core::print::{self, Paper, PrintOptions, PrintSource, Scope};
use egui::{Color32, RichText};
use std::process::Command;

pub struct PrintDlg {
    pub opts: PrintOptions,
    /// Printers the system lists (macOS and Linux); empty = only the default one.
    pub printers: Vec<String>,
    /// Empty = the default printer.
    pub printer: String,
    pub message: String,
}

/// Names of the printers (`lpstat -e`), where that exists.
fn list_printers() -> Vec<String> {
    if cfg!(windows) {
        return vec![];
    }
    let Ok(out) = Command::new("lpstat").arg("-e").output() else { return vec![] };
    String::from_utf8_lossy(&out.stdout).lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty() && printer_name_ok(l)).collect()
}

/// A printer name is passed to `lp -d` as one argument; keep it to plain characters.
pub fn printer_name_ok(s: &str) -> bool {
    !s.is_empty() && s.len() <= 100 && s.chars().all(|c| c.is_alphanumeric() || matches!(c, '_' | '-' | '.' | '@' | ':'))
}

/// Hand the PDF to the printer.
fn send_to_printer(path: &std::path::Path, printer: &str) -> Result<String, String> {
    if cfg!(windows) {
        let p = path.to_string_lossy().to_string();
        if p.contains('\'') {
            return Err("unsafe temporary path".into());
        }
        let out = Command::new("powershell").args(["-NoProfile", "-Command", &format!("Start-Process -FilePath '{p}' -Verb Print")]).output().map_err(|e| e.to_string())?;
        return if out.status.success() { Ok(String::new()) } else { Err(String::from_utf8_lossy(&out.stderr).trim().to_string()) };
    }
    let mut cmd = Command::new("lp");
    if !printer.is_empty() {
        if !printer_name_ok(printer) {
            return Err("unsafe printer name".into());
        }
        cmd.arg("-d").arg(printer);
    }
    let out = cmd.arg(path).output().map_err(|e| format!("lp: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

fn open_in_viewer(path: &std::path::Path) -> Result<(), String> {
    let r = if cfg!(target_os = "macos") {
        Command::new("open").arg(path).spawn()
    } else if cfg!(windows) {
        Command::new("cmd").args(["/C", "start", ""]).arg(path).spawn()
    } else {
        Command::new("xdg-open").arg(path).spawn()
    };
    r.map(|_| ()).map_err(|e| e.to_string())
}

impl PrintDlg {
    /// The window as it opens: the options of last time are not kept, the printers are asked now.
    pub fn open() -> PrintDlg {
        PrintDlg { opts: PrintOptions::default(), printers: list_printers(), printer: String::new(), message: String::new() }
    }
}

/// The window of File > Print…, while `dlg` holds it.
pub fn window(ctx: &egui::Context, dlg: &mut Option<PrintDlg>, src: &PrintSource) {
    let Some(mut d) = dlg.take() else { return };
    let mut open = true;
    let outside = print::outside_area(src, &d.opts);
    let nothing = print::region(src, &d.opts).is_err();
    let mut action = 0; // 1 print, 2 save, 3 open
    popup(ctx, "print_window", &tr("Print"), [380.0, 330.0], &mut open, |ui| {
        ui.label(RichText::new(tr("Prints the design on a paper printer, for checking placement or as a template.")).color(theme::text_dim()));
        ui.add_space(4.0);
        ui.radio_value(&mut d.opts.scope, Scope::WorkArea, tr("The whole work area, to scale"));
        ui.radio_value(&mut d.opts.scope, Scope::Objects, tr("Only the objects, as large as the page allows"));
        ui.checkbox(&mut d.opts.visible_only, tr("Only objects on visible layers"));
        ui.horizontal(|ui| {
            ui.label(tr("Paper"));
            ui.selectable_value(&mut d.opts.paper, Paper::A4, "A4");
            ui.selectable_value(&mut d.opts.paper, Paper::Letter, "Letter");
            ui.label(tr("Margin"));
            ui.add(egui::DragValue::new(&mut d.opts.margin_mm).range(0.0..=40.0).suffix(" mm"));
        });
        if !d.printers.is_empty() {
            ui.horizontal(|ui| {
                ui.label(tr("Printer"));
                let shown = if d.printer.is_empty() { tr("Default printer").to_string() } else { d.printer.clone() };
                egui::ComboBox::from_id_salt("print_printer").selected_text(shown).show_ui(ui, |ui| {
                    ui.selectable_value(&mut d.printer, String::new(), tr("Default printer"));
                    for p in d.printers.clone() {
                        ui.selectable_value(&mut d.printer, p.clone(), p);
                    }
                });
            });
        }
        if outside > 0 {
            let note = if d.opts.scope == Scope::WorkArea { trf("{} object(s) reach outside the work area; on the paper they are cut off at its edge.", &[&outside]) } else { trf("{} object(s) reach outside the work area; they are printed anyway.", &[&outside]) };
            ui.label(RichText::new(note).color(Color32::from_rgb(0xc0, 0x80, 0x20)));
        }
        if nothing {
            ui.label(RichText::new(tr("There is nothing to print.")).color(Color32::from_rgb(0xc0, 0x30, 0x30)));
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            if ui.add_enabled(!nothing, egui::Button::new(tr("Print"))).clicked() {
                action = 1;
            }
            if save_button_enabled(ui, !nothing, &tr("Save as PDF…")).clicked() {
                action = 2;
            }
            if ui.add_enabled(!nothing, egui::Button::new(tr("Open the PDF"))).on_hover_text(tr("Opens the page in the PDF viewer, which can print it with more options")).clicked() {
                action = 3;
            }
        });
        if !d.message.is_empty() {
            ui.label(&d.message);
        }
    });
    if action != 0 {
        d.message = match print::pdf(src, &d.opts) {
            Err(e) => trf("Could not make the page: {}", &[&e]),
            Ok(bytes) => match action {
                2 => match rfd::FileDialog::new().add_filter("PDF", &["pdf"]).set_file_name("design.pdf").save_file() {
                    Some(p) => match std::fs::write(&p, bytes) {
                        Ok(_) => trf("Saved {}", &[&p.display()]),
                        Err(e) => trf("Save failed: {}", &[&e]),
                    },
                    None => String::new(),
                },
                _ => {
                    // A private folder, so nobody else can put another file (or a link) in its place.
                    match base_core::paths::private_temp_file(&format!("{}-print", src.program.to_lowercase()), "design.pdf", &bytes) {
                        Err(e) => trf("Save failed: {}", &[&e]),
                        Ok(path) if action == 3 => match open_in_viewer(&path) {
                            Ok(()) => tr("Opened in the PDF viewer.").to_string(),
                            Err(e) => trf("Could not open the viewer: {}", &[&e]),
                        },
                        Ok(path) => {
                            let r = match send_to_printer(&path, &d.printer) {
                                Ok(info) => trf("Sent to the printer. {}", &[&info]),
                                Err(e) => trf("The printer did not take it: {}", &[&e]),
                            };
                            // The printing command has finished with the file.
                            if let Some(dir) = path.parent() {
                                let _ = std::fs::remove_dir_all(dir);
                            }
                            r
                        }
                    }
                }
            },
        };
    }
    if open {
        *dlg = Some(d);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::print::PrintLayer;

    #[test]
    fn printer_names_are_plain() {
        for ok in ["HP_LaserJet-1020", "Brother.local", "office@host", "Canon:1"] {
            assert!(printer_name_ok(ok), "{ok}");
        }
        for bad in ["", "a b", "-o raw", "x;rm", "$(id)", "`x`", "a/b", &"x".repeat(101)] {
            assert!(!printer_name_ok(bad), "{bad}");
        }
    }

    #[test]
    fn the_window_stays_open_when_there_is_nothing_to_print() {
        let ctx = egui::Context::default();
        let src = PrintSource { shapes: &[], layers: vec![PrintLayer { rgb: [0, 0, 0], filled: false, visible: true }], order: vec![0], work: (100.0, 100.0), program: "Test", device: "x" };
        let mut dlg = Some(PrintDlg { opts: PrintOptions { scope: Scope::Objects, ..Default::default() }, printers: vec![], printer: String::new(), message: String::new() });
        let mut out = ctx.run_ui(egui::RawInput::default(), |_| window(&ctx, &mut dlg, &src));
        out.textures_delta.clear();
        assert!(dlg.is_some(), "stays open");
    }
}
