//! The USB keyboards and pads in the running program: reading them, what their presses do to the machine that is open, and the
//! windows of the settings. Only the window in front takes the presses; when it loses the focus, when the pad goes away or when the
//! machine changes, everything held is let go, so a move or a beam never goes on without a finger on it.
use crate::control_ui::ControlTarget;
use crate::i18n::{tr, trf};
use crate::theme;
use crate::window::popup;
use base_core::control_devices::{Action, ControlDevice, ControlDevices, Engine, Fired, Input, Kind, BACKUP_KEY, FILE, KEYBOARD_ID};
use egui::{Event, RichText};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime};

/// A stick counts as pressed from here, and as let go under `AXIS_OFF`, so that it does not flicker in between.
const AXIS_ON: f32 = 0.6;
const AXIS_OFF: f32 = 0.4;

/// A pad that the computer knows now.
#[derive(Clone, Debug, PartialEq)]
pub struct PadInfo {
    pub id: String,
    pub uuid: String,
    pub name: String,
}

/// The state of the devices in one program.
#[derive(Default)]
pub struct ControlDevicesUi {
    pub lib: ControlDevices,
    path: Option<PathBuf>,
    seen_file: Option<SystemTime>,
    next_look: Option<Instant>,
    gilrs: Option<gilrs::Gilrs>,
    pad_error: Option<String>,
    pads: Vec<PadInfo>,
    axes: BTreeMap<(String, String), i8>,
    engine: Engine,
    /// The device that the presses go through now.
    active: Option<String>,
    /// What was read and not handled yet: the id of a device, the input, pressed or let go. Also how the tests press.
    pending: Vec<(String, Input, bool)>,
    moves: BTreeSet<Action>,
    moving: Option<[i8; 3]>,
    last_segment: Option<Instant>,
    switched_on: BTreeSet<Action>,
    /// The windows.
    pub open: bool,
    pub choice_open: bool,
    selected: Option<String>,
    capture: Option<Action>,
    last_input: Option<String>,
}

/// How a move goes.
#[derive(Clone, Copy, Debug)]
pub struct JogCfg {
    pub step_xy: f64,
    pub step_z: f64,
    pub feed_xy: f64,
    pub feed_z: f64,
    /// The controller can stop a move at once: it moves while held. Otherwise each press is one step.
    pub hold: bool,
    pub z_axis: bool,
}

impl ControlDevicesUi {
    /// With the definitions of the shared data folder.
    pub fn from_file() -> ControlDevicesUi {
        let mut st = ControlDevicesUi { path: base_core::paths::base_dir().map(|d| d.join(FILE)), ..Default::default() };
        st.read_file();
        st
    }

    fn read_file(&mut self) {
        let Some(p) = &self.path else { return };
        self.seen_file = std::fs::metadata(p).and_then(|m| m.modified()).ok();
        if let Some(l) = std::fs::read_to_string(p).ok().and_then(|t| ControlDevices::parse(&t)) {
            self.lib = l;
        }
    }

    /// Another program may have changed the definitions: they are common.
    fn look_at_the_file(&mut self) {
        let now = Instant::now();
        if self.next_look.is_some_and(|t| now < t) {
            return;
        }
        self.next_look = Some(now + Duration::from_secs(1));
        let Some(p) = &self.path else { return };
        if std::fs::metadata(p).and_then(|m| m.modified()).ok() != self.seen_file {
            self.read_file();
        }
    }

    fn store(&mut self) {
        let Some(p) = &self.path else { return };
        // Through a second file: another program may be reading this one.
        let tmp = p.with_extension(format!("tmp{}", std::process::id()));
        if std::fs::write(&tmp, self.lib.to_text()).is_ok() && std::fs::rename(&tmp, p).is_ok() {
            self.seen_file = std::fs::metadata(p).and_then(|m| m.modified()).ok();
        }
    }

    /// The entry of the definitions in the settings of a backup.
    pub fn backup_pair(&self) -> (&'static str, String) {
        (BACKUP_KEY, self.lib.to_text())
    }

    /// Take the definitions of a backup. `false` changes nothing: the text was not such a list.
    pub fn restore(&mut self, text: &str) -> bool {
        let Some(l) = ControlDevices::parse(text) else { return false };
        self.lib = l;
        self.store();
        true
    }

    /// Press or let go, as if it came from the device (the tests have no pad).
    pub fn feed(&mut self, device: &str, input: Input, pressed: bool) {
        self.pending.push((device.to_string(), input, pressed));
    }

    fn read_pads(&mut self) {
        if self.gilrs.is_none() && self.pad_error.is_none() {
            match gilrs::Gilrs::new() {
                Ok(g) => self.gilrs = Some(g),
                Err(e) => self.pad_error = Some(e.to_string()),
            }
        }
        let Some(g) = self.gilrs.as_mut() else { return };
        let hex = |u: [u8; 16]| u.iter().map(|b| format!("{b:02x}")).collect::<String>();
        let mut pads_changed = self.pads.is_empty();
        while let Some(ev) = g.next_event() {
            let pad = g.gamepad(ev.id);
            let id = format!("pad-{}", hex(pad.uuid()));
            use gilrs::EventType as T;
            match ev.event {
                T::ButtonPressed(b, code) | T::ButtonReleased(b, code) => {
                    // A pad that the library does not know has buttons without a name: their number tells them apart.
                    let name = if b == gilrs::Button::Unknown { format!("Button {code}") } else { format!("{b:?}") };
                    self.pending.push((id, Input::Button(name), matches!(ev.event, T::ButtonPressed(..))));
                }
                T::AxisChanged(a, v, code) => {
                    let name = if a == gilrs::Axis::Unknown { format!("Axis {code}") } else { format!("{a:?}") };
                    let was = self.axes.get(&(id.clone(), name.clone())).copied().unwrap_or(0);
                    let now = if v >= AXIS_ON { 1 } else if v <= -AXIS_ON { -1 } else if v.abs() < AXIS_OFF { 0 } else { was };
                    if now != was {
                        if was != 0 {
                            self.pending.push((id.clone(), Input::Axis(name.clone(), was), false));
                        }
                        if now != 0 {
                            self.pending.push((id.clone(), Input::Axis(name.clone(), now), true));
                        }
                        self.axes.insert((id, name), now);
                    }
                }
                T::Connected => pads_changed = true,
                T::Disconnected | T::Dropped => {
                    pads_changed = true;
                    // Whatever it held is let go.
                    self.axes.retain(|(d, _), _| *d != id);
                    self.pending.push((id, Input::Button(String::new()), false));
                }
                _ => {}
            }
        }
        if pads_changed {
            self.pads = g.gamepads().filter(|(_, p)| p.is_connected()).map(|(_, p)| PadInfo { id: format!("pad-{}", hex(p.uuid())), uuid: hex(p.uuid()), name: p.name().to_string() }).collect();
        }
    }

    /// Read everything, once a frame and before the rest of the window: a key that does something here is not also a key of the editor.
    /// `own` is the device of the machine that is open; `listening` says there is a machine to drive.
    pub fn poll(&mut self, ctx: &egui::Context, own: Option<&str>, listening: bool) -> Vec<Fired> {
        self.look_at_the_file();
        let mut fired = vec![];
        let active = self.lib.for_machine(own).map(|d| d.id.clone());
        if active != self.active {
            fired.extend(self.engine.release_all());
            self.active = active.clone();
        }
        let focused = ctx.input(|i| i.focused);
        // In the window of the settings a key is being chosen, not used.
        let live = listening && focused && !self.open && active.is_some();
        if !live {
            fired.extend(self.engine.release_all());
        }
        let wants_pads = self.open || (listening && self.lib.has_pads());
        if wants_pads && !cfg!(test) {
            self.read_pads();
        }
        // The keys.
        let keyboard_live = live && active.as_deref() == Some(KEYBOARD_ID);
        let capturing_keys = self.capture.is_some() && self.selected.as_deref() == Some(KEYBOARD_ID);
        if keyboard_live || capturing_keys || self.open {
            let typing = ctx.egui_wants_keyboard_input();
            let dev = active.as_deref().and_then(|id| self.lib.get(id)).cloned();
            let engine = &self.engine;
            let mut keys = vec![];
            ctx.input_mut(|i| {
                i.events.retain(|e| {
                    let Event::Key { key, pressed, repeat, modifiers, .. } = e else { return true };
                    let input = Input::Key(format!("{key:?}"));
                    if capturing_keys {
                        if *pressed && !*repeat {
                            keys.push((input, true));
                        }
                        return false;
                    }
                    if !keyboard_live {
                        return true;
                    }
                    let held = engine.is_down(&input);
                    if !*pressed {
                        keys.push((input, false));
                        return !held;
                    }
                    if typing || modifiers.ctrl || modifiers.command || modifiers.alt {
                        return true;
                    }
                    let mine = held || dev.as_ref().is_some_and(|d| d.action_of(&input).is_some());
                    if mine && !*repeat {
                        keys.push((input, true));
                    }
                    !mine
                });
            });
            self.pending.extend(keys.into_iter().map(|(i, p)| (KEYBOARD_ID.to_string(), i, p)));
        }
        // Everything that was read goes to the settings or to the machine.
        for (device, input, pressed) in std::mem::take(&mut self.pending) {
            if self.open && pressed {
                self.last_input = Some(input.label());
            }
            if let Some(a) = self.capture {
                if pressed && self.selected.as_deref() == Some(device.as_str()) {
                    if let Some(d) = self.lib.devices.iter_mut().find(|d| d.id == device) {
                        d.assign(a, input.clone());
                    }
                    self.capture = None;
                    self.store();
                }
                continue;
            }
            if active.as_deref() != Some(device.as_str()) {
                continue;
            }
            // The pad went away.
            if input == Input::Button(String::new()) {
                fired.extend(self.engine.release_all());
                continue;
            }
            if !live && pressed {
                continue;
            }
            if let Some(d) = self.lib.get(&device).cloned() {
                fired.extend(self.engine.event(&d, &input, pressed));
            }
        }
        // A pad says nothing by itself: the window has to look.
        if self.open || (listening && focused && active.is_some()) {
            ctx.request_repaint_after(Duration::from_millis(50));
        }
        fired
    }

    /// Do the moves and take away what must wait for the end of a job; the rest is for the program, which knows its machine.
    pub fn apply_moves(&mut self, ctx: &egui::Context, fired: Vec<Fired>, m: &mut dyn ControlTarget, c: &JogCfg) -> Vec<Fired> {
        let busy = m.busy();
        let mut rest = vec![];
        for f in fired {
            match f {
                Fired::Press(a) if a.direction().is_some_and(|d| d[2] == 0 || c.z_axis) => {
                    if !busy && m.connected() {
                        self.moves.insert(a);
                        if !c.hold {
                            one_step(m, a.direction().unwrap_or_default(), c);
                        }
                    }
                }
                Fired::Release(a) if a.direction().is_some() => {
                    self.moves.remove(&a);
                }
                Fired::Press(a) if a.direction().is_some() => {}
                Fired::Press(a) if busy && !a.during_a_job() => {}
                other => rest.push(other),
            }
        }
        if busy || !m.connected() {
            self.moves.clear();
            self.switched_on.clear();
        }
        if !c.hold {
            self.moves.clear();
            return rest;
        }
        let mut dir = [0i8; 3];
        for a in &self.moves {
            let d = a.direction().unwrap_or_default();
            for k in 0..3 {
                dir[k] += d[k];
            }
        }
        // Z alone: a jog has one speed, and the axes have their own.
        if dir[2] != 0 {
            dir = [0, 0, dir[2]];
        }
        if dir != [0, 0, 0] {
            let due = self.last_segment.map_or(true, |t| t.elapsed() >= Duration::from_millis(120));
            if due || self.moving != Some(dir) {
                // As on the Control tab: short moves that overlap a little, so the tool never stops while it is held.
                let secs = 0.3;
                let (fx, fz) = (c.feed_xy / 60.0 * secs, c.feed_z / 60.0 * secs);
                m.jog(dir[0] as f64 * fx, dir[1] as f64 * fx, dir[2] as f64 * fz, if dir[2] != 0 { c.feed_z } else { c.feed_xy });
                self.last_segment = Some(Instant::now());
            }
            self.moving = Some(dir);
            ctx.request_repaint_after(Duration::from_millis(60));
        } else if self.moving.take().is_some() {
            m.cancel_jog();
            self.last_segment = None;
        }
        rest
    }

    /// Switch something on or off: the answer says which it is now.
    pub fn toggle(&mut self, a: Action) -> bool {
        if self.switched_on.remove(&a) {
            false
        } else {
            self.switched_on.insert(a);
            true
        }
    }

    pub fn is_on(&self, a: Action) -> bool {
        self.switched_on.contains(&a)
    }

    pub fn all_off(&mut self) {
        self.switched_on.clear();
    }

    /// The window of the devices and their keys, for the `actions` that this program has. `own` is the device of the open machine,
    /// when one is open; the answer says it was changed.
    pub fn window(&mut self, ctx: &egui::Context, actions: &[Action], own: &mut Option<String>, has_machine: bool) -> bool {
        let mut changed = false;
        let mut open = self.open;
        popup(ctx, "control_devices", &tr("Control devices"), [760.0, 560.0], &mut open, |ui| {
            if has_machine {
                ui.horizontal(|ui| {
                    ui.label(tr("Control device of this machine"));
                    changed = self.machine_choice(ui, own);
                });
                ui.separator();
            }
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(210.0);
                    self.device_list(ui);
                });
                ui.add_space(10.0);
                ui.vertical(|ui| self.keys_of_device(ui, actions));
            });
            ui.separator();
            ui.label(RichText::new(tr("Only the window in front takes the presses. A move lasts while the key is held and stops when it is let go or when another window comes to the front.")).color(theme::text_dim()));
            ui.label(RichText::new(tr("All keyboards are one device, and it does nothing while you type in a field. Two pads of the same model count as one device.")).color(theme::text_dim()));
        });
        self.open = open;
        if !self.open {
            self.capture = None;
        }
        changed
    }

    /// The small window of the Machine menu.
    pub fn choice_window(&mut self, ctx: &egui::Context, own: &mut Option<String>) -> bool {
        let mut changed = false;
        let mut open = self.choice_open;
        let mut settings = false;
        popup(ctx, "control_device_of_machine", &tr("Control device of this machine"), [420.0, 130.0], &mut open, |ui| {
            changed = self.machine_choice(ui, own);
            ui.add_space(6.0);
            settings = ui.button(tr("Control device settings…")).clicked();
        });
        self.choice_open = open && !settings;
        if settings {
            self.open = true;
        }
        changed
    }

    fn machine_choice(&mut self, ui: &mut egui::Ui, own: &mut Option<String>) -> bool {
        let default_text = match self.lib.default.as_deref().and_then(|id| self.lib.get(id)) {
            Some(d) => trf("The default one: {}", &[&d.name]),
            None => tr("No default device").to_string(),
        };
        let shown = own.as_deref().and_then(|id| self.lib.get(id)).map_or(default_text.clone(), |d| d.name.clone());
        let mut changed = false;
        egui::ComboBox::from_id_salt("control_device_choice").selected_text(shown).width(280.0).show_ui(ui, |ui| {
            if ui.selectable_label(own.is_none(), default_text).clicked() && own.is_some() {
                *own = None;
                changed = true;
            }
            for d in &self.lib.devices {
                if ui.selectable_label(own.as_deref() == Some(d.id.as_str()), &d.name).clicked() && own.as_deref() != Some(d.id.as_str()) {
                    *own = Some(d.id.clone());
                    changed = true;
                }
            }
        });
        changed
    }

    fn device_list(&mut self, ui: &mut egui::Ui) {
        if self.selected.as_deref().is_none_or(|id| self.lib.get(id).is_none()) {
            self.selected = self.lib.devices.first().map(|d| d.id.clone());
        }
        let mut changed = false;
        for d in &self.lib.devices {
            let connected = match &d.kind {
                Kind::Keyboard => true,
                Kind::Pad { .. } => self.pads.iter().any(|p| p.id == d.id),
            };
            let on = self.selected.as_deref() == Some(d.id.as_str());
            let text = if connected { RichText::new(&d.name) } else { RichText::new(trf("{} (not connected)", &[&d.name])).color(theme::text_dim()) };
            if ui.selectable_label(on, text).clicked() {
                self.selected = Some(d.id.clone());
                self.capture = None;
            }
        }
        ui.add_space(8.0);
        if self.lib.get(KEYBOARD_ID).is_none() && ui.button(tr("Add the keyboard")).clicked() {
            self.lib.add(ControlDevice::keyboard(tr("Keyboard")));
            self.selected = Some(KEYBOARD_ID.to_string());
            changed = true;
        }
        let new_pads: Vec<PadInfo> = self.pads.iter().filter(|p| self.lib.get(&p.id).is_none()).cloned().collect();
        for p in &new_pads {
            if ui.button(trf("Add {}", &[&p.name])).clicked() {
                self.lib.add(ControlDevice::pad(&p.uuid, &p.name));
                self.selected = Some(p.id.clone());
                changed = true;
            }
        }
        if new_pads.is_empty() {
            let text = self.pad_error.clone().unwrap_or_else(|| tr("No new pad found. Connect one and wait a moment.").to_string());
            ui.label(RichText::new(text).color(theme::text_dim()));
        }
        if changed {
            self.store();
        }
    }

    fn keys_of_device(&mut self, ui: &mut egui::Ui, actions: &[Action]) {
        let Some(id) = self.selected.clone() else { return };
        let Some(i) = self.lib.devices.iter().position(|d| d.id == id) else { return };
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(tr("Name"));
            changed |= ui.text_edit_singleline(&mut self.lib.devices[i].name).changed();
        });
        let mut is_default = self.lib.default.as_deref() == Some(id.as_str());
        if ui.checkbox(&mut is_default, tr("Default for all machines and programs")).changed() {
            self.lib.default = is_default.then(|| id.clone());
            changed = true;
        }
        ui.horizontal(|ui| {
            if ui.button(tr("Back to the first keys")).clicked() {
                self.lib.devices[i].reset_keys();
                self.capture = None;
                changed = true;
            }
            if ui.button(tr("Remove")).clicked() {
                self.lib.remove(&id);
                self.capture = None;
                self.selected = None;
                changed = true;
            }
            if let Some(l) = &self.last_input {
                ui.label(RichText::new(trf("Last pressed: {}", &[l])).color(theme::text_dim()));
            }
        });
        ui.add_space(4.0);
        if let Some(d) = self.lib.devices.get(i).filter(|d| d.id == id).cloned() {
            egui::ScrollArea::vertical().id_salt("control_device_keys").auto_shrink([false, false]).max_height(330.0).show(ui, |ui| {
                egui::Grid::new("control_device_keys_grid").num_columns(3).striped(true).show(ui, |ui| {
                    for a in actions {
                        ui.label(tr(a.label()));
                        ui.horizontal(|ui| {
                            for input in d.inputs(*a) {
                                if ui.small_button(format!("{} ×", input.label())).clicked() {
                                    self.lib.devices[i].unassign(*a, input);
                                    changed = true;
                                }
                            }
                        });
                        let capturing = self.capture == Some(*a);
                        let text = if capturing { tr("Press a key or a button…") } else { tr("Assign") };
                        if ui.selectable_label(capturing, text).clicked() {
                            self.capture = if capturing { None } else { Some(*a) };
                        }
                        ui.end_row();
                    }
                });
            });
        }
        if changed {
            self.store();
        }
    }
}

fn one_step(m: &mut dyn ControlTarget, d: [i8; 3], c: &JogCfg) {
    m.jog(d[0] as f64 * c.step_xy, d[1] as f64 * c.step_xy, d[2] as f64 * c.step_z, if d[2] != 0 { c.feed_z } else { c.feed_xy });
}

/// The label of the shared texts, for the check that each one has a translation.
pub fn action_labels() -> Vec<&'static str> {
    Action::ALL.iter().map(|a| a.label()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::machine_ctl::Machine;

    /// A machine that remembers what it was asked.
    struct Probe {
        m: Machine,
        busy: bool,
        jogs: Vec<[f64; 4]>,
        cancels: usize,
    }
    fn probe(ctx: &egui::Context) -> Probe {
        let mut m = Machine::new(ctx);
        m.connected = true;
        Probe { m, busy: false, jogs: vec![], cancels: 0 }
    }
    // The rest of what the tab needs does not matter here.
    impl ControlTarget for Probe {
        fn connected(&self) -> bool { self.m.connected }
        fn state(&self) -> String { String::new() }
        fn fw_label(&self) -> Option<String> { None }
        fn alarm_code(&self) -> Option<u32> { None }
        fn is_alarm(&self) -> bool { false }
        fn wpos(&self) -> [f64; 3] { [0.0; 3] }
        fn mpos(&self) -> [f64; 3] { [0.0; 3] }
        fn pins(&self) -> String { String::new() }
        fn wcs(&self) -> u8 { 1 }
        fn busy(&self) -> bool { self.busy }
        fn keyboard_jog(&self) -> bool { false }
        fn set_keyboard_jog(&mut self, _on: bool) {}
        fn set_position(&mut self, _v: &[(base_core::control::Axis, f64)]) {}
        fn jog(&mut self, dx: f64, dy: f64, dz: f64, feed: f64) { self.jogs.push([dx, dy, dz, feed]); }
        fn cancel_jog(&mut self) { self.cancels += 1; }
        fn unlock(&mut self) {}
        fn select_wcs(&mut self, _n: u8) {}
        fn lines(&mut self, _l: Vec<String>) {}
        fn line(&mut self, _l: String) {}
        fn run_macro(&mut self, _c: &str, _s: f64, _b: Option<([f64; 3], [f64; 3])>) -> Result<usize, String> { Ok(0) }
        fn probe_closed(&self) -> bool { false }
        fn last_probe(&self) -> Option<([f64; 3], bool)> { None }
        fn probe(&mut self, _c: &base_core::control::Control, _a: base_core::control::Axis, _d: i8, _t: f64) -> Result<(), String> { Ok(()) }
        fn spindle_s(&self, rpm: f64, _max: f64) -> f64 { rpm }
    }

    const HOLD: JogCfg = JogCfg { step_xy: 2.0, step_z: 0.5, feed_xy: 600.0, feed_z: 300.0, hold: true, z_axis: true };

    fn with_pad() -> ControlDevicesUi {
        let mut st = ControlDevicesUi::default();
        st.lib.add(ControlDevice::pad("abc", "Pad"));
        st
    }

    fn run(ctx: &egui::Context, raw: egui::RawInput, f: impl FnMut(&mut egui::Ui)) {
        let mut out = ctx.run_ui(raw, f);
        out.textures_delta.clear();
    }

    fn frame(ctx: &egui::Context, st: &mut ControlDevicesUi, focused: bool, listening: bool) -> Vec<Fired> {
        let mut out = vec![];
        let raw = egui::RawInput { focused, ..Default::default() };
        run(ctx, raw, |ui| out = st.poll(ui.ctx(), None, listening));
        out
    }

    fn left() -> Input {
        Input::Button("DPadLeft".into())
    }

    #[test]
    fn a_held_button_moves_until_it_is_let_go() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        let mut p = probe(&ctx);
        st.feed("pad-abc", left(), true);
        let f = frame(&ctx, &mut st, true, true);
        assert_eq!(f, vec![Fired::Press(Action::MoveLeft)]);
        assert!(st.apply_moves(&ctx, f, &mut p, &HOLD).is_empty());
        assert_eq!(p.jogs.len(), 1);
        assert!(p.jogs[0][0] < 0.0 && p.jogs[0][1] == 0.0, "{:?}", p.jogs);
        st.feed("pad-abc", left(), false);
        let f = frame(&ctx, &mut st, true, true);
        st.apply_moves(&ctx, f, &mut p, &HOLD);
        assert_eq!(p.cancels, 1);
    }

    #[test]
    fn when_the_window_loses_the_focus_the_move_stops_and_a_press_does_nothing() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        let mut p = probe(&ctx);
        st.feed("pad-abc", left(), true);
        let f = frame(&ctx, &mut st, true, true);
        st.apply_moves(&ctx, f, &mut p, &HOLD);
        let f = frame(&ctx, &mut st, false, true);
        assert_eq!(f, vec![Fired::Release(Action::MoveLeft)]);
        st.apply_moves(&ctx, f, &mut p, &HOLD);
        assert_eq!(p.cancels, 1);
        // In the background a press goes nowhere.
        st.feed("pad-abc", left(), true);
        assert!(frame(&ctx, &mut st, false, true).is_empty());
        // Nor does it wake up with the window, because it was never taken.
        assert!(frame(&ctx, &mut st, true, true).is_empty());
    }

    #[test]
    fn a_pad_that_goes_away_lets_go_of_what_it_held() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        st.feed("pad-abc", left(), true);
        frame(&ctx, &mut st, true, true);
        st.feed("pad-abc", Input::Button(String::new()), false);
        assert_eq!(frame(&ctx, &mut st, true, true), vec![Fired::Release(Action::MoveLeft)]);
    }

    #[test]
    fn only_the_device_of_the_machine_is_listened_to_and_changing_it_lets_go() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        st.lib.add(ControlDevice::pad("xyz", "Other"));
        st.feed("pad-xyz", left(), true);
        assert!(frame(&ctx, &mut st, true, true).is_empty(), "the default one is the first");
        st.feed("pad-abc", left(), true);
        assert_eq!(frame(&ctx, &mut st, true, true), vec![Fired::Press(Action::MoveLeft)]);
        let mut out = vec![];
        run(&ctx, egui::RawInput::default(), |ui| out = st.poll(ui.ctx(), Some("pad-xyz"), true));
        assert_eq!(out, vec![Fired::Release(Action::MoveLeft)]);
    }

    #[test]
    fn a_job_blocks_moves_but_not_stop() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        let mut p = probe(&ctx);
        p.busy = true;
        let rest = st.apply_moves(&ctx, vec![Fired::Press(Action::MoveLeft), Fired::Press(Action::Spindle), Fired::Press(Action::Stop), Fired::Press(Action::Pause), Fired::Release(Action::Laser)], &mut p, &HOLD);
        assert!(p.jogs.is_empty());
        assert_eq!(rest, vec![Fired::Press(Action::Stop), Fired::Press(Action::Pause), Fired::Release(Action::Laser)], "a beam can always be let go");
    }

    #[test]
    fn without_hold_each_press_is_one_step_and_z_needs_a_z_axis() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        let mut p = probe(&ctx);
        let c = JogCfg { hold: false, z_axis: false, ..HOLD };
        st.apply_moves(&ctx, vec![Fired::Press(Action::MoveBack), Fired::Press(Action::MoveUp)], &mut p, &c);
        assert_eq!(p.jogs, vec![[0.0, 2.0, 0.0, 600.0]]);
        assert_eq!(p.cancels, 0);
    }

    #[test]
    fn a_key_of_the_device_does_not_reach_the_editor_and_one_that_is_not_does() {
        let ctx = egui::Context::default();
        let mut st = ControlDevicesUi::default();
        st.lib.add(ControlDevice::keyboard("Keyboard"));
        let key = |k| Event::Key { key: k, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE };
        let mut out = vec![];
        let mut left = (false, false);
        let raw = egui::RawInput { events: vec![key(egui::Key::ArrowLeft), key(egui::Key::A)], ..Default::default() };
        run(&ctx, raw, |ui| {
            out = st.poll(ui.ctx(), None, true);
            left = ui.input(|i| (i.events.iter().any(|e| matches!(e, Event::Key { key: egui::Key::ArrowLeft, .. })), i.events.iter().any(|e| matches!(e, Event::Key { key: egui::Key::A, .. }))));
        });
        assert_eq!(out, vec![Fired::Press(Action::MoveLeft)]);
        assert_eq!(left, (false, true));
    }

    #[test]
    fn with_no_machine_a_key_stays_a_key_of_the_editor() {
        let ctx = egui::Context::default();
        let mut st = ControlDevicesUi::default();
        st.lib.add(ControlDevice::keyboard("Keyboard"));
        let raw = egui::RawInput { events: vec![Event::Key { key: egui::Key::ArrowLeft, physical_key: None, pressed: true, repeat: false, modifiers: egui::Modifiers::NONE }], ..Default::default() };
        let mut kept = false;
        run(&ctx, raw, |ui| {
            assert!(st.poll(ui.ctx(), None, false).is_empty());
            kept = ui.input(|i| !i.events.is_empty());
        });
        assert!(kept);
    }

    #[test]
    fn in_the_settings_a_press_is_given_to_the_action_and_the_machine_stays_still() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        st.open = true;
        st.selected = Some("pad-abc".into());
        st.capture = Some(Action::Spindle);
        st.feed("pad-abc", Input::Button("South".into()), true);
        assert!(frame(&ctx, &mut st, true, true).is_empty());
        assert_eq!(st.lib.get("pad-abc").unwrap().action_of(&Input::Button("South".into())), Some(Action::Spindle));
        assert!(st.capture.is_none());
        // With the window open a press only shows its name.
        st.feed("pad-abc", left(), true);
        assert!(frame(&ctx, &mut st, true, true).is_empty());
        assert_eq!(st.last_input.as_deref(), Some("DPadLeft"));
    }

    #[test]
    fn a_backup_brings_the_definitions_back_and_a_wrong_text_changes_nothing() {
        let st = with_pad();
        let (key, text) = st.backup_pair();
        assert_eq!(key, BACKUP_KEY);
        let mut other = ControlDevicesUi::default();
        assert!(other.restore(&text));
        assert_eq!(other.lib, st.lib);
        assert!(!other.restore("no"));
        assert_eq!(other.lib, st.lib);
    }

    #[test]
    fn two_programs_share_the_file() {
        let dir = std::env::temp_dir().join(format!("control-devices-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(FILE);
        let mut a = ControlDevicesUi { path: Some(path.clone()), ..Default::default() };
        let mut b = ControlDevicesUi { path: Some(path.clone()), ..Default::default() };
        b.read_file();
        a.lib.add(ControlDevice::keyboard("Keyboard"));
        a.store();
        // Later than the second of the file that `b` saw last.
        b.seen_file = None;
        b.next_look = None;
        b.look_at_the_file();
        assert_eq!(b.lib, a.lib);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_windows_draw_for_every_program() {
        let ctx = egui::Context::default();
        let mut st = with_pad();
        st.lib.add(ControlDevice::keyboard("Keyboard"));
        st.open = true;
        st.choice_open = true;
        let mut own = None;
        run(&ctx, egui::RawInput::default(), |ui| {
            st.window(ui.ctx(), &Action::ALL, &mut own, true);
            st.choice_window(ui.ctx(), &mut own);
        });
        assert!(st.open && st.choice_open);
    }

    #[test]
    fn every_action_has_a_shared_text() {
        for l in action_labels() {
            // "Stop" is the same word in Polish. Chinese has its own.
            assert!(crate::i18n::tr_in(crate::i18n::Lang::Pl, l) != l || crate::i18n::tr_in(crate::i18n::Lang::Zh, l) != l, "{l}");
        }
    }
}
