//! The USB keyboards and pads that drive a machine, the same for every program with a machine: which devices there are, which is
//! the default one, and which key or button does what. The definitions live in one file of the shared data folder; the machine only
//! keeps the id of its device. Nothing here reads a device or touches a machine: what was pressed comes in, what to do goes out.
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// The file in the shared data folder.
pub const FILE: &str = "control_devices.json";
/// The key of the definitions in the settings of a backup.
pub const BACKUP_KEY: &str = "control_devices";
/// The id of the keyboard: all the keyboards of the computer are one device, the window does not know which one was pressed.
pub const KEYBOARD_ID: &str = "keyboard";
pub const MAX_DEVICES: usize = 32;
const MAX_NAME: usize = 80;
/// Inputs of one action.
const MAX_INPUTS: usize = 8;

/// What a key or a button can do. A program offers the part that its machine has; a device keeps the keys of the others.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Action {
    MoveLeft,
    MoveRight,
    MoveForward,
    MoveBack,
    MoveUp,
    MoveDown,
    Stop,
    Pause,
    Resume,
    Home,
    Unlock,
    StartJob,
    Frame,
    Spindle,
    Air,
    Water,
    Laser,
    PenUp,
    PenDown,
}

impl Action {
    pub const ALL: [Action; 19] = [
        Action::MoveLeft, Action::MoveRight, Action::MoveForward, Action::MoveBack, Action::MoveUp, Action::MoveDown, Action::Stop, Action::Pause, Action::Resume, Action::Home,
        Action::Unlock, Action::StartJob, Action::Frame, Action::Spindle, Action::Air, Action::Water, Action::Laser, Action::PenUp, Action::PenDown,
    ];
    /// What every machine has.
    pub const COMMON: [Action; 11] = [Action::MoveLeft, Action::MoveRight, Action::MoveForward, Action::MoveBack, Action::Stop, Action::Pause, Action::Resume, Action::Home, Action::Unlock, Action::StartJob, Action::Frame];

    /// The name in the file: it never changes.
    pub fn id(self) -> &'static str {
        match self {
            Action::MoveLeft => "move-left",
            Action::MoveRight => "move-right",
            Action::MoveForward => "move-forward",
            Action::MoveBack => "move-back",
            Action::MoveUp => "move-up",
            Action::MoveDown => "move-down",
            Action::Stop => "stop",
            Action::Pause => "pause",
            Action::Resume => "resume",
            Action::Home => "home",
            Action::Unlock => "unlock",
            Action::StartJob => "start-job",
            Action::Frame => "frame",
            Action::Spindle => "spindle",
            Action::Air => "air",
            Action::Water => "water",
            Action::Laser => "laser",
            Action::PenUp => "pen-up",
            Action::PenDown => "pen-down",
        }
    }

    /// The name on the screen, in English (the translation is the program's).
    pub fn label(self) -> &'static str {
        match self {
            Action::MoveLeft => "Move left (X-)",
            Action::MoveRight => "Move right (X+)",
            Action::MoveForward => "Move forward (Y-)",
            Action::MoveBack => "Move back (Y+)",
            Action::MoveUp => "Move up (Z+)",
            Action::MoveDown => "Move down (Z-)",
            Action::Stop => "Stop",
            Action::Pause => "Pause",
            Action::Resume => "Resume",
            Action::Home => "Home",
            Action::Unlock => "Unlock",
            Action::StartJob => "Start job",
            Action::Frame => "Frame",
            Action::Spindle => "Spindle on / off",
            Action::Air => "Air on / off",
            Action::Water => "Water on / off",
            Action::Laser => "Low-power beam while held",
            Action::PenUp => "Pen up",
            Action::PenDown => "Pen down",
        }
    }

    /// The direction of a move, with Y up towards the back (as on the pad of the Control tab).
    pub fn direction(self) -> Option<[i8; 3]> {
        match self {
            Action::MoveLeft => Some([-1, 0, 0]),
            Action::MoveRight => Some([1, 0, 0]),
            Action::MoveForward => Some([0, -1, 0]),
            Action::MoveBack => Some([0, 1, 0]),
            Action::MoveUp => Some([0, 0, 1]),
            Action::MoveDown => Some([0, 0, -1]),
            _ => None,
        }
    }

    /// May it be used while a job runs? Everything else waits until the machine is free.
    pub fn during_a_job(self) -> bool {
        matches!(self, Action::Stop | Action::Pause | Action::Resume)
    }
}

/// A key of the keyboard, a button of a pad, or one side of an axis of a pad (a stick counts as pressed when pushed far enough).
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Input {
    Key(String),
    Button(String),
    Axis(String, i8),
}

impl Input {
    /// As shown on the screen.
    pub fn label(&self) -> String {
        match self {
            Input::Key(n) | Input::Button(n) => n.clone(),
            Input::Axis(n, d) => format!("{n} {}", if *d < 0 { "-" } else { "+" }),
        }
    }
    fn of_pad(&self) -> bool {
        !matches!(self, Input::Key(_))
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Keyboard,
    /// `uuid` tells the model of a pad; two pads of the same model have the same one and count as one device.
    Pad { uuid: String },
}

/// A keyboard or a pad with its own keys.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlDevice {
    pub id: String,
    pub name: String,
    pub kind: Kind,
    /// The inputs of an action, by [`Action::id`]. An action this version does not know stays in the file.
    #[serde(default)]
    pub map: BTreeMap<String, Vec<Input>>,
}

fn keys(list: &[(Action, &[Input])]) -> BTreeMap<String, Vec<Input>> {
    list.iter().map(|(a, i)| (a.id().to_string(), i.to_vec())).collect()
}

impl ControlDevice {
    /// The keyboard, with the keys of the moves and Stop only: nothing that switches something on until it is asked for.
    pub fn keyboard(name: &str) -> ControlDevice {
        let k = |n: &str| Input::Key(n.to_string());
        let map = keys(&[
            (Action::MoveLeft, &[k("ArrowLeft")]),
            (Action::MoveRight, &[k("ArrowRight")]),
            (Action::MoveForward, &[k("ArrowDown")]),
            (Action::MoveBack, &[k("ArrowUp")]),
            (Action::MoveUp, &[k("PageUp")]),
            (Action::MoveDown, &[k("PageDown")]),
            (Action::Stop, &[k("Escape")]),
        ]);
        ControlDevice { id: KEYBOARD_ID.to_string(), name: name.to_string(), kind: Kind::Keyboard, map }
    }

    /// A pad, with the moves on the cross and the shoulders, and Stop on the right button.
    pub fn pad(uuid: &str, name: &str) -> ControlDevice {
        let b = |n: &str| Input::Button(n.to_string());
        let map = keys(&[
            (Action::MoveLeft, &[b("DPadLeft")]),
            (Action::MoveRight, &[b("DPadRight")]),
            (Action::MoveForward, &[b("DPadDown")]),
            (Action::MoveBack, &[b("DPadUp")]),
            (Action::MoveUp, &[b("RightTrigger")]),
            (Action::MoveDown, &[b("LeftTrigger")]),
            (Action::Stop, &[b("East")]),
        ]);
        ControlDevice { id: format!("pad-{uuid}"), name: name.to_string(), kind: Kind::Pad { uuid: uuid.to_string() }, map }
    }

    /// The first keys of this device again.
    pub fn reset_keys(&mut self) {
        self.map = match &self.kind {
            Kind::Keyboard => ControlDevice::keyboard(&self.name).map,
            Kind::Pad { uuid } => ControlDevice::pad(uuid, &self.name).map,
        };
    }

    pub fn inputs(&self, a: Action) -> &[Input] {
        self.map.get(a.id()).map_or(&[], Vec::as_slice)
    }

    /// What this input does here.
    pub fn action_of(&self, input: &Input) -> Option<Action> {
        Action::ALL.into_iter().find(|a| self.inputs(*a).contains(input))
    }

    /// Give the input to an action; an input does one thing, so another action loses it. Pressing it again takes nothing away.
    pub fn assign(&mut self, a: Action, input: Input) {
        for list in self.map.values_mut() {
            list.retain(|i| *i != input);
        }
        let list = self.map.entry(a.id().to_string()).or_default();
        if list.len() < MAX_INPUTS {
            list.push(input);
        }
        self.map.retain(|_, l| !l.is_empty());
    }

    pub fn unassign(&mut self, a: Action, input: &Input) {
        if let Some(l) = self.map.get_mut(a.id()) {
            l.retain(|i| i != input);
        }
        self.map.retain(|_, l| !l.is_empty());
    }
}

/// All the devices, for all the programs.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlDevices {
    #[serde(default)]
    pub devices: Vec<ControlDevice>,
    /// The device of the machines that have none of their own.
    #[serde(default)]
    pub default: Option<String>,
    /// Devices taken off the list: their keys wait here in case they come back.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub parked: Vec<ControlDevice>,
}

impl ControlDevices {
    pub fn get(&self, id: &str) -> Option<&ControlDevice> {
        self.devices.iter().find(|d| d.id == id)
    }

    /// The device of a machine: its own; the default one when it has none, or when its own was removed.
    pub fn for_machine(&self, own: Option<&str>) -> Option<&ControlDevice> {
        own.and_then(|id| self.get(id)).or_else(|| self.default.as_deref().and_then(|id| self.get(id)))
    }

    pub fn has_pads(&self) -> bool {
        self.devices.iter().any(|d| matches!(d.kind, Kind::Pad { .. }))
    }

    /// Add a device; the first one becomes the default. One that was on the list before comes back with its keys. Nothing happens when
    /// it is there already or the list is full.
    pub fn add(&mut self, d: ControlDevice) {
        if self.get(&d.id).is_none() && self.devices.len() < MAX_DEVICES {
            let d = match self.parked.iter().position(|p| p.id == d.id) {
                Some(i) => self.parked.remove(i),
                None => d,
            };
            if self.default.is_none() && self.devices.is_empty() {
                self.default = Some(d.id.clone());
            }
            self.devices.push(d);
        }
    }

    /// Take a device off the list. It is no longer the default one.
    pub fn remove(&mut self, id: &str) {
        if let Some(i) = self.devices.iter().position(|d| d.id == id) {
            let d = self.devices.remove(i);
            self.parked.retain(|p| p.id != id);
            self.parked.push(d);
            self.parked.truncate(MAX_DEVICES);
        }
        if self.default.as_deref() == Some(id) {
            self.default = None;
        }
    }

    /// What a file or a backup holds, checked: it comes from outside. `None` when it is not such a text at all.
    pub fn parse(text: &str) -> Option<ControlDevices> {
        let mut v: ControlDevices = serde_json::from_str(text).ok()?;
        let mut seen = std::collections::BTreeSet::new();
        v.devices.retain(|d| !d.id.is_empty() && d.id.len() <= 200 && seen.insert(d.id.clone()));
        v.devices.truncate(MAX_DEVICES);
        v.parked.retain(|d| !d.id.is_empty() && d.id.len() <= 200 && seen.insert(d.id.clone()));
        v.parked.truncate(MAX_DEVICES);
        for d in v.devices.iter_mut().chain(v.parked.iter_mut()) {
            d.name = d.name.chars().filter(|c| !c.is_control()).take(MAX_NAME).collect();
            if d.name.trim().is_empty() {
                d.name = d.id.clone();
            }
            let pad = matches!(d.kind, Kind::Pad { .. });
            for l in d.map.values_mut() {
                // A key on a pad, or a button on the keyboard, could never be pressed.
                l.retain(|i| i.of_pad() == pad && i.label().len() <= 80);
                l.truncate(MAX_INPUTS);
            }
            d.map.retain(|_, l| !l.is_empty());
        }
        if v.default.as_deref().is_some_and(|id| v.get(id).is_none()) {
            v.default = None;
        }
        Some(v)
    }

    pub fn to_text(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// What the presses of a device come to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fired {
    Press(Action),
    Release(Action),
}

/// Turns presses and releases into [`Fired`]. An action held by two inputs is let go with the last of them. What is held remembers
/// its action, so it is let go right also when the keys are changed in the meantime.
#[derive(Default)]
pub struct Engine {
    down: BTreeMap<Input, Action>,
}

impl Engine {
    pub fn event(&mut self, dev: &ControlDevice, input: &Input, pressed: bool) -> Option<Fired> {
        if pressed {
            if self.down.contains_key(input) {
                return None;
            }
            let a = dev.action_of(input)?;
            let first = !self.down.values().any(|x| *x == a);
            self.down.insert(input.clone(), a);
            first.then_some(Fired::Press(a))
        } else {
            let a = self.down.remove(input)?;
            (!self.down.values().any(|x| *x == a)).then_some(Fired::Release(a))
        }
    }

    /// Let go of everything (the window lost the focus, the pad went away, the machine changed).
    pub fn release_all(&mut self) -> Vec<Fired> {
        let mut acts: Vec<Action> = self.down.values().copied().collect();
        acts.sort();
        acts.dedup();
        self.down.clear();
        acts.into_iter().map(Fired::Release).collect()
    }

    pub fn is_down(&self, input: &Input) -> bool {
        self.down.contains_key(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key(n: &str) -> Input {
        Input::Key(n.to_string())
    }

    #[test]
    fn a_machine_uses_its_own_device_and_otherwise_the_default_one() {
        let mut l = ControlDevices::default();
        assert!(l.for_machine(None).is_none(), "no device: nothing happens");
        l.add(ControlDevice::keyboard("Keyboard"));
        l.add(ControlDevice::pad("abc", "Pad"));
        assert_eq!(l.default.as_deref(), Some(KEYBOARD_ID), "the first one is the default");
        assert_eq!(l.for_machine(None).unwrap().id, KEYBOARD_ID);
        assert_eq!(l.for_machine(Some("pad-abc")).unwrap().id, "pad-abc");
        assert_eq!(l.for_machine(Some("gone")).unwrap().id, KEYBOARD_ID, "its own was removed");
        l.remove(KEYBOARD_ID);
        assert!(l.default.is_none());
        assert!(l.for_machine(None).is_none());
        assert_eq!(l.for_machine(Some("pad-abc")).unwrap().id, "pad-abc");
    }

    #[test]
    fn a_device_taken_off_the_list_comes_back_with_its_keys_and_is_not_the_default_again() {
        let mut l = ControlDevices::default();
        l.add(ControlDevice::keyboard("Keyboard"));
        l.add(ControlDevice::pad("abc", "Pad"));
        l.devices[0].assign(Action::Spindle, key("S"));
        l.remove(KEYBOARD_ID);
        assert!(l.get(KEYBOARD_ID).is_none() && l.default.is_none());
        assert_eq!(ControlDevices::parse(&l.to_text()).unwrap(), l, "the keys that wait are in the file");
        l.add(ControlDevice::keyboard("Keyboard"));
        assert_eq!(l.get(KEYBOARD_ID).unwrap().action_of(&key("S")), Some(Action::Spindle));
        assert!(l.parked.is_empty());
        assert!(l.default.is_none(), "having no default one was a choice");
    }

    #[test]
    fn nothing_that_switches_something_on_has_a_key_at_first() {
        for d in [ControlDevice::keyboard("K"), ControlDevice::pad("u", "P")] {
            for a in [Action::Spindle, Action::Air, Action::Water, Action::Laser, Action::StartJob, Action::Frame, Action::PenDown] {
                assert!(d.inputs(a).is_empty(), "{a:?} on {}", d.name);
            }
            assert!(!d.inputs(Action::Stop).is_empty());
        }
    }

    #[test]
    fn an_input_does_one_thing_and_each_device_has_its_own_keys() {
        let mut k = ControlDevice::keyboard("K");
        let p = ControlDevice::pad("u", "P");
        k.assign(Action::Spindle, key("ArrowLeft"));
        assert_eq!(k.action_of(&key("ArrowLeft")), Some(Action::Spindle));
        assert!(k.inputs(Action::MoveLeft).is_empty());
        assert_eq!(p.action_of(&Input::Button("DPadLeft".into())), Some(Action::MoveLeft));
        k.reset_keys();
        assert_eq!(k.action_of(&key("ArrowLeft")), Some(Action::MoveLeft));
        k.unassign(Action::MoveLeft, &key("ArrowLeft"));
        assert_eq!(k.action_of(&key("ArrowLeft")), None);
    }

    #[test]
    fn a_held_action_is_let_go_once_with_the_last_input_and_when_everything_is_released() {
        let mut d = ControlDevice::keyboard("K");
        d.assign(Action::MoveLeft, key("A"));
        let mut e = Engine::default();
        assert_eq!(e.event(&d, &key("ArrowLeft"), true), Some(Fired::Press(Action::MoveLeft)));
        assert_eq!(e.event(&d, &key("ArrowLeft"), true), None, "a key that repeats");
        assert_eq!(e.event(&d, &key("A"), true), None, "already held");
        assert_eq!(e.event(&d, &key("ArrowLeft"), false), None);
        assert_eq!(e.event(&d, &key("A"), false), Some(Fired::Release(Action::MoveLeft)));
        assert_eq!(e.event(&d, &key("Z"), true), None, "no action");
        assert_eq!(e.event(&d, &key("Z"), false), None);
        // The keys change while one is held: it still ends what it began.
        e.event(&d, &key("ArrowRight"), true);
        d.unassign(Action::MoveRight, &key("ArrowRight"));
        assert_eq!(e.event(&d, &key("ArrowRight"), false), Some(Fired::Release(Action::MoveRight)));
        e.event(&d, &key("A"), true);
        e.event(&d, &key("Escape"), true);
        assert_eq!(e.release_all(), vec![Fired::Release(Action::MoveLeft), Fired::Release(Action::Stop)]);
        assert!(e.release_all().is_empty());
    }

    #[test]
    fn a_file_goes_there_and_back_and_what_is_wrong_in_it_is_left_out() {
        let mut l = ControlDevices::default();
        l.add(ControlDevice::keyboard("Keyboard"));
        l.add(ControlDevice::pad("abc", "Pad"));
        l.devices[1].assign(Action::Spindle, Input::Axis("LeftStickX".into(), -1));
        assert_eq!(ControlDevices::parse(&l.to_text()).unwrap(), l);
        assert!(ControlDevices::parse("no").is_none());
        let bad = r#"{"devices":[{"id":"a","name":"\u0007","kind":"keyboard","map":{"stop":[{"button":"South"},{"key":"Space"}],"later":[{"key":"F1"}]}},{"id":"a","name":"twice","kind":"keyboard"}],"default":"gone"}"#;
        let v = ControlDevices::parse(bad).unwrap();
        assert_eq!(v.devices.len(), 1);
        assert_eq!(v.devices[0].name, "a");
        assert_eq!(v.devices[0].inputs(Action::Stop), [Input::Key("Space".into())]);
        assert!(v.devices[0].map.contains_key("later"), "an action of a newer version stays");
        assert!(v.default.is_none());
    }
}
