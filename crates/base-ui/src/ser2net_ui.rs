//! "Share over network (ser2net)": set up ser2net on this (Linux) computer so that the machine on its USB port
//! can be reached remotely, for example from the same program on another computer ("Network (TCP)" connection).
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::{popup, save_button};
use base_core::ser2net::{config, shell_safe, version_from_output, Settings, Version};
use egui::{Color32, RichText};
use std::sync::mpsc::{channel, Receiver};

pub struct Ser2NetDlg {
    pub version: Version,
    pub serial_port: String,
    pub baud: u32,
    pub tcp_port: u16,
    pub local_only: bool,
    pub kick: bool,
    /// What `ser2net -v` printed, if the program exists.
    pub installed: Option<String>,
    /// Addresses of this computer, for the "connect to" hint.
    pub addresses: Vec<String>,
    pub confirm: bool,
    pub message: String,
    pub busy: Option<Receiver<Result<String, String>>>,
}

impl Ser2NetDlg {
    fn settings(&self) -> Settings {
        Settings { serial_port: self.serial_port.clone(), baud: self.baud, tcp_port: self.tcp_port, bind: self.local_only.then(|| "127.0.0.1".to_string()), kick_old_user: self.kick }
    }
    pub fn text(&self) -> String {
        config(self.version, &self.settings())
    }
}

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = std::process::Command::new(cmd).args(args).output().ok()?;
    let mut s = String::from_utf8_lossy(&out.stdout).to_string();
    s.push_str(&String::from_utf8_lossy(&out.stderr));
    Some(s.trim().to_string())
}

fn cleanup(tmp: &std::path::Path) {
    if let Some(dir) = tmp.parent() {
        let _ = std::fs::remove_dir_all(dir);
    }
}

/// Write the file as administrator (pkexec) and restart the service. Keeps the first backup of an existing file.
fn install(text: &str, version: Version) -> Result<String, String> {
    // The file is copied by an administrator: it must not be replaceable by anybody else meanwhile (a fixed name in /tmp
    // could be), so it is made new, in a folder only this user can enter.
    let tmp = base_core::paths::private_temp_file("creator-ser2net", "ser2net.conf", text.as_bytes())?;
    let tmp_s = tmp.to_string_lossy().to_string();
    if !shell_safe(&tmp_s) {
        cleanup(&tmp);
        return Err("unsafe temporary path".into());
    }
    let t = version.file();
    let script = format!("if [ -f {t} ]; then cp -n {t} {t}.creator.bak; fi; cp '{tmp_s}' {t} && (systemctl restart ser2net || systemctl restart ser2net.service)");
    let out = std::process::Command::new("pkexec").arg("sh").arg("-c").arg(script).output().map_err(|e| {
        cleanup(&tmp);
        format!("pkexec: {e}")
    })?;
    cleanup(&tmp);
    if out.status.success() {
        Ok(t.to_string())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
    }
}

impl Ser2NetDlg {
    /// The window as it opens, for the port and the speeds of the device in use.
    pub fn open(port: &str, baud: u32, tcp_port: u16, ports: &[String]) -> Ser2NetDlg {
        let installed = run("ser2net", &["-v"]);
        let version = installed.as_deref().and_then(version_from_output).unwrap_or(Version::V4);
        let addresses: Vec<String> = if cfg!(target_os = "linux") { run("hostname", &["-I"]).unwrap_or_default().split_whitespace().map(|s| s.to_string()).collect() } else { vec![] };
        Ser2NetDlg {
            version,
            serial_port: if port.is_empty() { ports.first().cloned().unwrap_or_else(|| "/dev/ttyUSB0".into()) } else { port.to_string() },
            baud,
            tcp_port,
            local_only: false,
            kick: true,
            installed,
            addresses,
            confirm: false,
            message: String::new(),
            busy: None,
        }
    }
}

/// The window, while `dlg` holds it. `ports` are the serial ports to choose from; *Refresh* asks for them again (true comes back).
pub fn window(ctx: &egui::Context, dlg: &mut Option<Ser2NetDlg>, ports: &[String]) -> bool {
    let Some(mut d) = dlg.take() else { return false };
    let mut refresh = false;
    // Result of a running install.
    if let Some(rx) = &d.busy {
        if let Ok(r) = rx.try_recv() {
            d.message = match r {
                Ok(file) => trf("Installed {} and restarted ser2net.", &[&file]),
                Err(e) => trf("Could not install the configuration: {}", &[&e]),
            };
            d.busy = None;
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
    }
    let mut open = true;
    let text = d.text();
    popup(ctx, "w_ser2net_ui_4", &tr("Share over network (ser2net)"), [340.0, 220.0], &mut open, |ui| {
        match &d.installed {
            Some(v) => ui.label(RichText::new(trf("ser2net found: {}", &[v])).color(Color32::from_rgb(0x2e, 0xa0, 0x4f))),
            None => ui.label(RichText::new(tr("ser2net was not found. Install it first, for example: sudo apt install ser2net")).color(Color32::from_rgb(0xc0, 0x30, 0x30))),
        };
        ui.label(RichText::new(tr("This shares the serial port of the machine on this computer over the network, so the program on another computer can connect with the connection type Network (TCP).")).color(theme::text_dim()));
        ui.add_space(4.0);
        egui::Grid::new("s2n").num_columns(2).spacing([12.0, 6.0]).show(ui, |ui| {
            ui.label(tr("Port"));
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("s2n_port").width(170.0).selected_text(d.serial_port.clone()).show_ui(ui, |ui| {
                    for p in ports {
                        ui.selectable_value(&mut d.serial_port, p.clone(), p);
                    }
                });
                ui.add(egui::TextEdit::singleline(&mut d.serial_port).desired_width(140.0));
                if ui.button(tr("Refresh")).clicked() {
                    refresh = true;
                }
            });
            ui.end_row();
            ui.label(tr("Baud rate"));
            ui.add(egui::DragValue::new(&mut d.baud).range(1200..=1_000_000));
            ui.end_row();
            ui.label(tr("TCP port"));
            ui.add(egui::DragValue::new(&mut d.tcp_port).range(1..=65535));
            ui.end_row();
            ui.label(tr("ser2net version"));
            ui.horizontal(|ui| {
                ui.selectable_value(&mut d.version, Version::V4, "4.x (YAML)");
                ui.selectable_value(&mut d.version, Version::V3, "3.x");
            });
            ui.end_row();
        });
        ui.checkbox(&mut d.local_only, tr("Listen only on this computer (use with an SSH tunnel)"));
        ui.checkbox(&mut d.kick, tr("A new client replaces the connected one"));
        ui.add_space(4.0);
        ui.label(RichText::new(trf("File: {}", &[&d.version.file()])).color(theme::text_dim()));
        egui::Frame::group(ui.style()).show(ui, |ui| {
            ui.set_width(460.0);
            ui.label(RichText::new(text.trim_end()).monospace().size(12.0));
        });
        let addr = d.addresses.first().cloned().unwrap_or_else(|| "<this computer>".to_string());
        ui.label(RichText::new(trf("Then connect from the other computer to {}:{}", &[&addr, &d.tcp_port])).strong());
        ui.label(RichText::new(tr("Warning: anyone on the network who reaches this port can drive the machine. Use a trusted network, a firewall, or the local-only option with an SSH tunnel.")).color(Color32::from_rgb(0xc0, 0x80, 0x20)));
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button(tr("Copy")).clicked() {
                ctx.copy_text(text.clone());
                d.message = tr("Copied.").to_string();
            }
            if save_button(ui, &tr("Save as…")).clicked() {
                if let Some(p) = rfd::FileDialog::new().set_file_name(d.version.file().rsplit('/').next().unwrap_or("ser2net.yaml")).save_file() {
                    d.message = match std::fs::write(&p, &text) {
                        Ok(_) => trf("Saved {}", &[&p.display()]),
                        Err(e) => trf("Save failed: {}", &[&e]),
                    };
                }
            }
            if cfg!(target_os = "linux") {
                if !d.confirm {
                    if ui.add_enabled(d.busy.is_none(), egui::Button::new(tr("Install and restart ser2net…"))).clicked() {
                        d.confirm = true;
                    }
                } else {
                    ui.label(RichText::new(trf("This replaces {} (a backup is kept) and restarts ser2net. Administrator rights are requested.", &[&d.version.file()])).color(Color32::from_rgb(0xc0, 0x30, 0x30)));
                    if ui.button(tr("Confirm")).clicked() {
                        d.confirm = false;
                        let (tx, rx) = channel();
                        let (t, v) = (text.clone(), d.version);
                        std::thread::spawn(move || {
                            let _ = tx.send(install(&t, v));
                        });
                        d.busy = Some(rx);
                        d.message = tr("Waiting for administrator approval…").to_string();
                    }
                    if ui.button(tr("Cancel")).clicked() {
                        d.confirm = false;
                    }
                }
            }
        });
        if !d.message.is_empty() {
            ui.label(&d.message);
        }
        if cfg!(target_os = "linux") {
            ui.label(RichText::new(trf("By hand: sudo cp <saved file> {} && sudo systemctl restart ser2net", &[&d.version.file()])).monospace().size(11.0).color(theme::text_dim()));
        }
    });
    if open {
        *dlg = Some(d);
    }
    refresh
}
