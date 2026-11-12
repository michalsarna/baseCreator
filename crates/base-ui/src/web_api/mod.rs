//! The web server and API of every program (first made for LightCreator): the server itself ([`server`]), and here what a program
//! keeps of it: the settings (on or off, the port, the network, the access phrase), starting and stopping the server to match them,
//! the commands that come back, the rows of *Program options* and the word in the status line. Each program says what its status
//! is (the machine, the job, the project); a program without a machine says `"machine": false` and the page shows only the project.
pub mod server;

pub use server::{access_ok, Command, Jog, Server, Shared, MAX_ACCESS};

use crate::i18n::{tr, trf};
use base_core::camimg::{CameraKind, CameraSource};
use egui::{Color32, RichText};
use serde_json::{json, Value};
use std::time::{Duration, Instant};

/// The port each program listens on at first: one after the other, so that all of them can run on one computer.
pub const PORT_LIGHTCREATOR: u16 = 8765;
pub const PORT_MILLCREATOR: u16 = 8766;
pub const PORT_PENCREATOR: u16 = 8767;
pub const PORT_BASECREATOR: u16 = 8768;

/// The keys of the settings.
const KEY_ON: &str = "api_on";
const KEY_PORT: &str = "api_port";
const KEY_LAN: &str = "api_lan";
/// The key of the access phrase (kept as base64, left out of diagnostics).
pub const KEY_ACCESS: &str = "api_access";

/// The web server of a program: its settings and what is running now.
pub struct WebApi {
    /// The name of the program (the page, the question of the browser, the first access phrase).
    pub app: &'static str,
    pub on: bool,
    pub port: u16,
    pub lan: bool,
    pub access: String,
    /// The phrase being typed in *Program options*, kept only when saved.
    pub access_edit: String,
    pub access_typing: bool,
    pub server: Option<Server>,
    started_with: (u16, bool),
    pub error: String,
    last_publish: Option<Instant>,
}

impl WebApi {
    /// Off, on the default port of the program, with its name as the access phrase.
    pub fn new(app: &'static str, default_port: u16) -> WebApi {
        WebApi { app, on: false, port: default_port, lan: false, access: app.to_string(), access_edit: String::new(), access_typing: false, server: None, started_with: (0, false), error: String::new(), last_publish: None }
    }

    /// The settings that were kept (`saved` gives the value of a key).
    pub fn from_saved(app: &'static str, default_port: u16, saved: &dyn Fn(&str) -> Option<String>) -> WebApi {
        let mut w = WebApi::new(app, default_port);
        w.restore(saved);
        w
    }

    /// Take the settings that are there (from the settings file or a backup); the others stay.
    pub fn restore(&mut self, get: &dyn Fn(&str) -> Option<String>) {
        if let Some(v) = get(KEY_ON) {
            self.on = v == "1";
        }
        if let Some(v) = get(KEY_PORT).and_then(|v| v.parse::<u16>().ok()).filter(|p| *p >= 1024) {
            self.port = v;
        }
        if let Some(v) = get(KEY_LAN) {
            self.lan = v == "1";
        }
        if let Some(v) = get(KEY_ACCESS).and_then(|v| server::access_from_setting(&v)) {
            self.access = v;
            self.access_edit.clear();
        }
    }

    /// What is kept between runs.
    pub fn settings_pairs(&self) -> Vec<(&'static str, String)> {
        let flag = |b: bool| if b { "1" } else { "0" }.to_string();
        vec![(KEY_ON, flag(self.on)), (KEY_PORT, self.port.to_string()), (KEY_LAN, flag(self.lan)), (KEY_ACCESS, server::access_to_setting(&self.access))]
    }

    /// Start, restart or stop the server to match the settings. Returns the commands that came in, and the shared state when it is
    /// time to publish (four times a second): the program then fills it in (see [`WebApi::publish`]).
    pub fn tick(&mut self, ctx: &egui::Context) -> (Vec<Command>, Option<std::sync::Arc<Shared>>) {
        let want = (self.port, self.lan);
        if !self.on {
            if self.server.take().is_some() {
                self.error.clear();
            }
            return (vec![], None);
        }
        if self.server.is_none() || self.started_with != want {
            self.server = None;
            match Server::start(want.0, want.1, self.app) {
                Ok(s) => {
                    self.server = Some(s);
                    self.error.clear();
                }
                Err(e) => self.error = e,
            }
            self.started_with = want;
            self.last_publish = None;
        }
        let Some(server) = &self.server else { return (vec![], None) };
        let shared = server.shared.clone();
        // A changed access phrase counts from now on, without restarting the server.
        if let Ok(mut a) = shared.access.lock() {
            if *a != self.access {
                *a = self.access.clone();
            }
        }
        // The commands first, so that STOP does not wait for the next publication.
        let cmds: Vec<Command> = shared.commands.lock().map(|mut q| q.drain(..).collect()).unwrap_or_default();
        ctx.request_repaint_after(Duration::from_millis(250));
        if self.last_publish.is_some_and(|t| t.elapsed() < Duration::from_millis(250)) {
            return (cmds, None);
        }
        self.last_publish = Some(Instant::now());
        if let Ok(mut t) = shared.themes.lock() {
            if t.is_null() {
                *t = crate::theme::web_themes();
            }
        }
        (cmds, Some(shared))
    }

    /// Publish at the next tick, not a quarter of a second later (after a change the page should see).
    pub fn publish_soon(&mut self) {
        self.last_publish = None;
    }

    /// Put the status, the cameras and (when the revision changed) the project into the shared state. `svg` is made only when needed.
    pub fn publish(shared: &Shared, status: Value, cameras: Value, revision: u64, svg: &dyn Fn() -> String) {
        if let Ok(mut s) = shared.status.lock() {
            *s = status;
        }
        if let Ok(mut c) = shared.cameras.lock() {
            *c = cameras;
        }
        let stale = shared.svg.lock().map(|s| s.0 != revision || s.1.is_empty()).unwrap_or(false);
        if stale {
            let text = std::sync::Arc::new(svg());
            if let Ok(mut s) = shared.svg.lock() {
                *s = (revision, text);
            }
        }
    }

    /// Hand the newest camera picture to the server (only while it runs).
    pub fn publish_frame(&self, w: u32, h: u32, rgb: &[u8]) {
        if let Some(s) = &self.server {
            if let Ok(mut f) = s.shared.frame.lock() {
                *f = Some((w, h, std::sync::Arc::new(rgb.to_vec())));
            }
        }
    }

    /// Hand a picture of the camera view to the server (only while it runs), for `/api/camera.jpg`.
    pub fn publish_image(&self, img: &egui::ColorImage) {
        if self.server.is_none() {
            return;
        }
        let rgb: Vec<u8> = img.pixels.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
        self.publish_frame(img.size[0] as u32, img.size[1] as u32, &rgb);
    }

    /// The port the server listens on, while it runs.
    pub fn port_now(&self) -> Option<u16> {
        self.server.as_ref().map(|s| s.addr.port())
    }

    /// The word of the status line: on with the port, or an error; `None` while it is off.
    pub fn status_word(&self) -> Option<(String, Color32)> {
        match self.port_now() {
            Some(p) => Some((trf("Web server: on, port {}", &[&p]), Color32::from_rgb(0x2e, 0xa0, 0x4f))),
            None if self.on => Some((tr("Web server: error").to_string(), Color32::from_rgb(0xc0, 0x30, 0x30))),
            None => None,
        }
    }

    /// The rows of *Program options* (a grid of two columns): on or off, the port, the network, the access phrase, and what is
    /// running. `machine` says whether the program drives a machine (the warning about stopping and moving it).
    pub fn options_ui(&mut self, ui: &mut egui::Ui, machine: bool) {
        ui.label(tr("Web server and API"));
        ui.vertical(|ui| {
            let hint = if machine { tr("A web page with the state of the machine, the project and the camera, and an API for other programs. Every request needs the access phrase; there is no encryption.") } else { tr("A web page with the project, and an API for other programs. Every request needs the access phrase; there is no encryption.") };
            ui.checkbox(&mut self.on, tr("Switch on the web server and the API")).on_hover_text(hint);
            ui.add_enabled_ui(self.on, |ui| {
                ui.horizontal(|ui| {
                    ui.label(tr("Port"));
                    ui.add(egui::DragValue::new(&mut self.port).range(1024..=65535));
                });
                ui.checkbox(&mut self.lan, tr("Reachable from other computers on the network"));
            });
            ui.add_enabled_ui(self.on, |ui| {
                if self.access_edit.is_empty() && !self.access_typing {
                    self.access_edit = self.access.clone();
                }
                let ok = access_ok(&self.access_edit);
                ui.horizontal(|ui| {
                    ui.label(tr("Access phrase"));
                    let r = ui.add(egui::TextEdit::singleline(&mut self.access_edit).password(true).desired_width(140.0).char_limit(MAX_ACCESS));
                    if r.changed() {
                        self.access_typing = true;
                    }
                    let changed = self.access_edit != self.access;
                    if crate::window::save_button_enabled(ui, ok && changed, &tr("Save")).on_hover_text(tr("Use this access phrase from now on")).clicked() {
                        self.access = self.access_edit.clone();
                        self.access_typing = false;
                    }
                });
                if !ok {
                    ui.label(RichText::new(tr("The access phrase cannot be empty or have control characters.")).color(Color32::from_rgb(0xc0, 0x30, 0x30)));
                }
            });
            if self.on && self.lan {
                let warn = if machine { tr("Anyone on the network who knows the access phrase can see the machine and stop or move it. The phrase is sent without encryption: use only on a network you trust.") } else { tr("Anyone on the network who knows the access phrase can see the project. The phrase is sent without encryption: use only on a network you trust.") };
                ui.label(RichText::new(warn).color(Color32::from_rgb(0xc0, 0x80, 0x20)));
            }
            if let Some(p) = self.port_now() {
                let host = if self.lan { "<this computer>".to_string() } else { "localhost".to_string() };
                ui.label(RichText::new(trf("Listening on {}", &[&format!("http://{host}:{p}")])).color(Color32::from_rgb(0x2e, 0xa0, 0x4f)));
            } else if self.on && !self.error.is_empty() {
                ui.label(RichText::new(trf("The server could not start: {}", &[&self.error])).color(Color32::from_rgb(0xc0, 0x30, 0x30)));
            }
        });
    }
}

/// The cameras of a device as the page wants them: each with its address (when it has one), the snapshot of the program, and its turn.
pub fn cameras_json(cameras: &[CameraSource], active: usize) -> Value {
    json!({
        "active": active,
        "cameras": cameras.iter().enumerate().map(|(i, c)| json!({
            "index": i,
            "name": c.name,
            "enabled": c.enabled,
            "active": i == active,
            // The address of a camera that has one; a camera of this computer only has the snapshot.
            "url": match &c.kind { CameraKind::Url(u) if c.enabled && c.is_valid() => json!(u.trim()), _ => json!(null) },
            "snapshot": "/api/camera.jpg",
            // The turn set for this camera in the program, in degrees clockwise ...
            "rotation": (c.rotation % 4) as u32 * 90,
            // ... and how much of it is already in the snapshot (the program turns its picture; a camera's own address is raw).
            "rotated": match &c.kind { CameraKind::Url(u) if c.enabled && c.is_valid() && !u.trim().is_empty() => 0, _ => (c.rotation % 4) as u32 * 90 },
        })).collect::<Vec<_>>(),
    })
}

/// The device part of the status of a program that drives a GRBL machine (`machine_ctl::Machine`): its name and controller, the
/// connection, the state and the work position, and the job (lines sent of all).
pub fn machine_status(m: &crate::machine_ctl::Machine, device: &str, controller: &str, estimate_s: f64, elapsed_s: f64) -> (Value, Value) {
    let (done, total) = m.progress;
    let pos = |v: f64| if v.is_finite() { v } else { 0.0 };
    (
        json!({ "name": device, "controller": controller, "connected": m.connected, "state": m.state, "x": pos(m.pos[0]), "y": pos(m.pos[1]), "z": pos(m.pos[2]) }),
        json!({ "running": m.job_running(), "done": done, "total": total, "elapsed_s": elapsed_s, "estimate_s": estimate_s }),
    )
}

/// Carry out Home, Cancel jog and a jog step on a GRBL machine, with the rules of their buttons (a connection, no job running).
/// STOP and the choice of a camera are left to the program; returns false for them.
pub fn machine_command(m: &mut crate::machine_ctl::Machine, c: &Command, step: f64, feed: f64) -> bool {
    match c {
        Command::Home => {
            if m.connected && !m.job_running() {
                m.home();
            }
            true
        }
        Command::CancelJog => {
            if m.connected {
                m.cancel_jog();
            }
            true
        }
        Command::Jog(d) => {
            if m.connected && !m.job_running() && step > 0.0 && feed > 0.0 {
                // The arrows are those of the screen: up is +Y of the machine.
                let (dx, dy) = d.delta();
                m.jog(dx * step, -dy * step, 0.0, feed);
            }
            true
        }
        Command::Stop | Command::SelectCamera(_) => false,
    }
}

/// The drawing as an SVG for the page: the shapes on the work area in the colours of their layers.
pub fn drawing_svg<H: crate::editor::EditorHost>(h: &H) -> String {
    let (w, hh) = h.work_size();
    let name = |l: usize| h.layer_title(l);
    let color = |l: usize| -> [u8; 3] {
        let c = h.layer_color(l);
        [c.r(), c.g(), c.b()]
    };
    base_core::export::svg(h.scene(), &[], w, hh, &name, &color)
}

/// The status of a program without a machine: its name, version, theme and project.
pub fn drawing_status(app: &str, version: &str, theme: &str, project: &str, revision: u64, description: &str) -> Value {
    json!({
        "app": app,
        "version": version,
        "theme": theme,
        "machine": false,
        "project": { "name": project, "revision": revision, "description": description },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_settings_are_kept_and_read_back_and_the_ports_go_up() {
        assert!(PORT_LIGHTCREATOR < PORT_MILLCREATOR && PORT_MILLCREATOR < PORT_PENCREATOR && PORT_PENCREATOR < PORT_BASECREATOR);
        let mut w = WebApi::new("MillCreator", PORT_MILLCREATOR);
        assert!(!w.on && w.port == 8766 && w.access == "MillCreator");
        w.on = true;
        w.lan = true;
        w.port = 9000;
        w.access = "my phrase ü".into();
        let pairs = w.settings_pairs();
        let kept = pairs.iter().find(|(k, _)| *k == KEY_ACCESS).unwrap().1.clone();
        assert!(!kept.contains("phrase"), "not in plain text");
        let get = |k: &str| pairs.iter().find(|(a, _)| *a == k).map(|(_, v)| v.clone());
        let back = WebApi::from_saved("MillCreator", PORT_MILLCREATOR, &get);
        assert!(back.on && back.lan && back.port == 9000 && back.access == "my phrase ü");
        // A port below 1024 and a broken phrase are not taken.
        let bad = WebApi::from_saved("MillCreator", PORT_MILLCREATOR, &|k: &str| match k {
            "api_port" => Some("80".into()),
            "api_access" => Some("!!!".into()),
            _ => None,
        });
        assert!(bad.port == 8766 && bad.access == "MillCreator");
    }

    #[test]
    fn the_server_follows_the_settings_and_hands_over_the_commands() {
        let ctx = egui::Context::default();
        let mut w = WebApi::new("PenCreator", 0);
        assert!(w.tick(&ctx).1.is_none() && w.server.is_none(), "off");
        w.on = true;
        let (_, shared) = w.tick(&ctx);
        let shared = shared.expect("published at once");
        assert!(w.server.as_ref().unwrap().addr.ip().is_loopback());
        assert_eq!(*shared.access.lock().unwrap(), "PenCreator", "the name of the program at first");
        assert!(shared.themes.lock().unwrap().is_array());
        w.access = "other".into();
        shared.commands.lock().unwrap().push_back(Command::Stop);
        let (cmds, again) = w.tick(&ctx);
        assert_eq!(cmds, vec![Command::Stop]);
        assert!(again.is_none(), "not again within a quarter of a second");
        assert_eq!(*shared.access.lock().unwrap(), "other", "a new phrase at once");
        WebApi::publish(&shared, drawing_status("PenCreator", "v1", "dark", "a.cre", 3, ""), json!({"cameras": []}), 3, &|| "<svg/>".into());
        assert_eq!(shared.status.lock().unwrap()["machine"], false);
        assert_eq!(shared.svg.lock().unwrap().1.as_str(), "<svg/>");
        assert!(w.status_word().is_some_and(|(t, _)| t.contains("on")));
        w.lan = true;
        w.tick(&ctx);
        assert!(w.server.as_ref().unwrap().addr.ip().is_unspecified(), "started again on every interface");
        w.on = false;
        w.tick(&ctx);
        assert!(w.server.is_none() && w.status_word().is_none());
    }

    #[test]
    fn the_cameras_carry_their_turn_and_how_much_of_it_is_in_the_snapshot() {
        let mut own = CameraSource::url("Own", "http://a/x", 1);
        own.enabled = true;
        let mut local = CameraSource::url("Local", "", 3);
        local.kind = CameraKind::Local(0);
        let c = cameras_json(&[own, local], 0);
        assert_eq!((c["cameras"][0]["rotation"].as_u64(), c["cameras"][0]["rotated"].as_u64()), (Some(90), Some(0)), "the camera's own address is raw");
        assert_eq!((c["cameras"][1]["rotation"].as_u64(), c["cameras"][1]["rotated"].as_u64()), (Some(270), Some(270)), "the program turns its own picture");
        assert_eq!(c["cameras"][0]["url"], "http://a/x");
    }
}
