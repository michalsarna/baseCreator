//! The machine as the programs see it: the link to a GRBL controller and everything that follows from what it says,
//! the positions and pins, the firmware and its settings, the job that is running (with its pauses and its errors),
//! the probe, macros and jogging. No window code here: the panels are in `control_ui` and `firmware_ui`.
//! Shared by every program that drives a machine.
use crate::console_ui::ConsoleState;
use crate::i18n::{tr, trf};
use crate::link::{self, Cmd, ConsoleLine, Dir, Evt, LaserLink, PortInfo, Target};
use base_core::control::{self, Axis, Change, Control, FwCopies, Noted, Tls};
use base_core::firmware::{self, FirmwareInfo};
use base_core::grbl_codes;
use base_core::grbl_settings as gs;
use base_core::grbl_status;
use base_core::machine::{Controller, Streaming};
use egui::Context;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

/// A pause of the job: the program waits for the person (a tool or pen change).
#[derive(Clone, Debug, PartialEq)]
pub struct Barrier {
    /// The line of the program, `M0 (Change the bit to: V-bit 60°)`.
    pub text: String,
    pub line: usize,
}

impl Barrier {
    /// The words in the comment of the line, or the line itself.
    pub fn message(&self) -> String {
        let t = &self.text;
        match (t.find('('), t.rfind(')')) {
            (Some(a), Some(b)) if b > a => t[a + 1..b].trim().to_string(),
            _ => t.trim().to_string(),
        }
    }
}

/// An alarm or an error that came while a job ran.
#[derive(Clone, Debug, PartialEq)]
pub struct Issue {
    /// The line of the job (0-based) that was being acknowledged.
    pub line: usize,
    pub code: u32,
    pub alarm: bool,
}

/// What the program should react to; returned by `Machine::poll`.
#[derive(Debug, PartialEq)]
pub enum Notice {
    Connected,
    /// The link went away; `lost` is true when the person did not ask for it.
    Disconnected {
        lost: bool,
    },
    /// The time to open a lost link again has come.
    ReconnectDue,
    Barrier(Barrier),
    /// A job ran to its end. `seconds` is the time it took; `issues` the errors on the way; `checked` says it was a check run.
    JobFinished {
        seconds: u64,
        issues: usize,
        checked: bool,
    },
    Alarm(u32),
    Error {
        code: u32,
        line: Option<usize>,
    },
    Stalled {
        secs: u64,
        state: String,
    },
    /// The firmware and the settings were read.
    Read,
    /// The controller answered a probe move.
    Probed {
        pos: [f64; 3],
        touched: bool,
    },
    /// The reference tool touched the tool length sensor: the work Z of its tip at that moment.
    TlsReference { wz: f64 },
    /// A tool was measured and the work Z was set from the reference; `delta` is how much longer the tool is than the
    /// reference tool (negative: shorter).
    TlsTool { delta: f64 },
    /// A tool was measured and nothing was changed.
    TlsTest { wz: f64, delta: Option<f64> },
    /// The measurement did not work (no contact, an alarm).
    TlsFailed,
}

/// What a measurement at the tool length sensor is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TlsMode {
    /// The tool in the machine is the reference: remember where its tip touched.
    Reference,
    /// A new tool: make its tip touch at the same work Z as the reference tool's.
    Tool,
    /// Only measure and say how it differs from the reference.
    Test,
}

struct TlsRun {
    mode: TlsMode,
    tls: Tls,
    wcs: u8,
    hits: Vec<([f64; 3], bool)>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ask {
    Info,
    Settings,
    Parser,
}

struct Capture {
    ask: Ask,
    lines: Vec<String>,
    started: Instant,
}

/// The page of the settings of the controller (`$$`): what was read and what is being edited.
#[derive(Default)]
pub struct Fw {
    pub values: BTreeMap<u32, String>,
    pub typed: BTreeMap<u32, String>,
    pub info: FirmwareInfo,
    pub reading: bool,
    /// Read the settings again at this time (after a write, to see what the controller made of it).
    pub reread_at: Option<Instant>,
    /// What an import wrote, to compare with the controller's answer once it has been read back.
    pub verify: Option<Vec<(u32, String)>>,
    pub report: Vec<String>,
    pub import: Option<Vec<(u32, String)>>,
    pub write_ask: Option<(u32, String)>,
    pub reset_open: bool,
    pub reset_text: String,
    pub search: String,
    pub only_changed: bool,
    /// Where the settings are saved before the first write of a session.
    pub backup_dir: Option<std::path::PathBuf>,
    pub backed_up: bool,
    /// What differed from the newest copy kept with the device when the settings were last read.
    pub changes: Vec<Change>,
    /// A write was made here: the next reading is the result of it, not a surprise.
    pub own_change: bool,
}

/// The word that has to be typed before the factory settings are restored.
pub const RESET_WORD: &str = "RESET";

/// How long an answer to `$$`, `$I` or `$G` may take.
const ASK_TIMEOUT: Duration = Duration::from_secs(6);

/// Why the machine stopped, for the window that asks for the person's attention.
#[derive(Clone, Debug, PartialEq)]
pub struct Problem {
    /// An alarm: the window goes away by itself when the alarm is cleared.
    pub alarm: bool,
    pub title: String,
    pub text: String,
    /// The line of the job that was running, when there was one.
    pub line: Option<String>,
}

pub struct Machine {
    /// The controller the next connection talks to (`Controller::GrblOld` or `Grbl07` for the older GRBLs).
    controller: Controller,
    /// What the person has to look at now (an alarm, an error in a job, a controller that does not answer).
    pub problem: Option<Problem>,
    link: LaserLink,
    pub connected: bool,
    /// The user asked for the link (so a lost one is worth opening again).
    pub want_link: bool,
    pub reconnect_at: Option<Instant>,
    /// The state word of the last status report (`Idle`, `Run`, `Hold:0`, `Alarm`…).
    pub state: String,
    /// The position shown: the work position when it is known, else the machine position.
    pub pos: [f64; 3],
    pub mpos: [f64; 3],
    pub wpos: [f64; 3],
    wco: Option<[f64; 3]>,
    /// The active input pins (`XYZPDHRS`).
    pub pins: String,
    pub feed: f64,
    pub spindle: f64,
    /// The work coordinate system, 1 for G54 … 6 for G59.
    pub wcs: u8,
    /// Where the last probe move stopped.
    pub probe: Option<([f64; 3], bool)>,
    /// Lines of the job acknowledged and the number of lines of the job.
    pub progress: (usize, usize),
    pub console: ConsoleState,
    pub ports: Vec<PortInfo>,
    pub fw: Fw,
    pub barrier: Option<Barrier>,
    pub issues: Vec<Issue>,
    /// The last alarm that was seen and not yet cleared by an unlock.
    pub alarm: Option<u32>,
    job: Option<Job>,
    tls_run: Option<TlsRun>,
    asks: VecDeque<Ask>,
    capture: Option<Capture>,
    /// The tool-change dialog and the other windows keep their own state in the programs.
    pub keyboard_jog: bool,
}

struct Job {
    started: Instant,
    check: bool,
    after: Vec<String>,
}

impl Machine {
    pub fn new(ctx: &Context) -> Machine {
        Machine {
            controller: Controller::Grbl,
            problem: None,
            link: LaserLink::spawn(ctx.clone()),
            connected: false,
            want_link: false,
            reconnect_at: None,
            state: String::new(),
            pos: [f64::NAN; 3],
            mpos: [f64::NAN; 3],
            wpos: [f64::NAN; 3],
            wco: None,
            pins: String::new(),
            feed: 0.0,
            spindle: 0.0,
            wcs: 1,
            probe: None,
            progress: (0, 0),
            console: Default::default(),
            ports: vec![],
            fw: Fw::default(),
            barrier: None,
            issues: vec![],
            alarm: None,
            job: None,
            tls_run: None,
            asks: VecDeque::new(),
            capture: None,
            keyboard_jog: false,
        }
    }

    pub fn refresh_ports(&mut self) {
        self.ports = link::list_port_info();
    }

    pub fn push(&mut self, l: ConsoleLine) {
        self.console.push_capped(l);
    }

    pub fn info(&mut self, text: impl Into<String>) {
        self.push(ConsoleLine::info(text));
    }

    pub fn job_running(&self) -> bool {
        self.progress.1 > 0 && self.progress.0 < self.progress.1
    }

    /// Is a job on, counting the time it waits at a pause?
    pub fn job_active(&self) -> bool {
        self.job.is_some()
    }

    /// The machine is at work: a job (also while it waits at a pause), a jog, homing or a held move.
    pub fn working(&self) -> bool {
        self.job_running() || self.job_active() || matches!(self.state.split(':').next().unwrap_or(""), "Run" | "Hold" | "Jog" | "Home")
    }

    pub fn checking(&self) -> bool {
        self.job.as_ref().is_some_and(|j| j.check)
    }

    pub fn is_alarm(&self) -> bool {
        self.state.starts_with("Alarm")
    }

    // ---- the link ----

    pub fn connect(&mut self, target: Target, streaming: Streaming) {
        self.want_link = true;
        self.link.send(Cmd::Connect {
            target,
            controller: self.controller,
            streaming,
        });
    }

    /// Which GRBL the next connection talks to (see `GrblVersion`).
    pub fn set_grbl_version(&mut self, dialect: base_core::machine::GrblVersion) {
        self.controller = dialect.controller();
    }

    pub fn disconnect(&mut self) {
        self.want_link = false;
        self.reconnect_at = None;
        self.link.send(Cmd::Disconnect);
    }

    pub fn line(&mut self, l: impl Into<String>) {
        self.link.send(Cmd::Line(l.into()));
    }

    pub fn lines<I: IntoIterator<Item = String>>(&mut self, ls: I) {
        for l in link::clean_gcode(&ls.into_iter().collect::<Vec<_>>().join("\n")) {
            self.link.send(Cmd::Line(l));
        }
    }

    pub fn realtime(&mut self, b: u8) {
        self.link.send(Cmd::Realtime(b));
    }

    pub fn pause(&mut self) {
        self.realtime(b'!');
    }

    pub fn resume(&mut self) {
        self.realtime(b'~');
    }

    pub fn home(&mut self) {
        self.line("$H");
    }

    /// The S value that makes the spindle run at `rpm`, when the machine's top speed is `max_rpm`: the controller's own
    /// range (`$30`, when it is known) is spread over the spindle's, so 12000 of 24000 rpm is half of the range whatever it is.
    /// Without `$30`, or when it is the same as the top speed, the S value is the speed.
    pub fn spindle_s(&self, rpm: f64, max_rpm: f64) -> f64 {
        let s_max = self.fw.values.get(&30).and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| *v > 0.0);
        match s_max {
            Some(s) if max_rpm > 0.0 && (s - max_rpm).abs() > max_rpm * 0.01 => (rpm / max_rpm * s).round().clamp(0.0, s),
            _ => rpm,
        }
    }

    pub fn unlock(&mut self) {
        self.alarm = None;
        self.line("$X");
    }

    // ---- reading what the controller says ----

    /// Ask for the build info (`$I`), the settings (`$$`) and the parser state (`$G`), one after the other.
    pub fn read_all(&mut self) {
        if !self.connected {
            return;
        }
        for a in [Ask::Info, Ask::Settings, Ask::Parser] {
            if !self.asks.contains(&a) && self.capture.as_ref().map(|c| c.ask) != Some(a) {
                self.asks.push_back(a);
            }
        }
        self.fw.reading = true;
    }

    pub fn read_settings(&mut self) {
        if self.connected
            && !self.asks.contains(&Ask::Settings)
            && self.capture.as_ref().map(|c| c.ask) != Some(Ask::Settings)
        {
            self.asks.push_back(Ask::Settings);
            self.fw.reading = true;
        }
    }

    pub fn read_parser(&mut self) {
        if self.connected && !self.asks.contains(&Ask::Parser) {
            self.asks.push_back(Ask::Parser);
        }
    }

    fn pump_asks(&mut self) {
        if let Some(c) = &self.capture {
            if c.started.elapsed() > ASK_TIMEOUT {
                // No answer: give up on this one.
                let ask = c.ask;
                self.capture = None;
                if ask == Ask::Settings {
                    self.fw.reading = false;
                }
            }
        }
        if self.capture.is_none() && self.connected && !self.job_running() {
            if let Some(a) = self.asks.pop_front() {
                let l = match a {
                    Ask::Info => "$I",
                    Ask::Settings => "$$",
                    Ask::Parser => "$G",
                };
                self.link.send(Cmd::Line(l.to_string()));
                self.capture = Some(Capture {
                    ask: a,
                    lines: vec![],
                    started: Instant::now(),
                });
            }
        }
    }

    fn finish_capture(&mut self, notices: &mut Vec<Notice>) {
        let Some(c) = self.capture.take() else { return };
        match c.ask {
            Ask::Info => {
                let info = firmware::parse_info(&c.lines);
                if info.is_known() {
                    self.fw.info = info;
                }
            }
            Ask::Settings => {
                self.fw.reading = false;
                let values = gs::parse_report(&c.lines);
                if !values.is_empty() {
                    self.fw.values = values;
                    self.fw.typed.clear();
                    if let Some(want) = self.fw.verify.take() {
                        let bad: Vec<String> = want
                            .iter()
                            .filter(|(id, v)| {
                                self.fw
                                    .values
                                    .get(id)
                                    .map_or(true, |now| !gs::same_value(now, v))
                            })
                            .map(|(id, v)| {
                                format!(
                                    "${id}: {} {}, {} {}",
                                    tr("File"),
                                    v,
                                    tr("Controller"),
                                    self.fw
                                        .values
                                        .get(id)
                                        .cloned()
                                        .unwrap_or_else(|| "–".into())
                                )
                            })
                            .collect();
                        self.fw.report = if bad.is_empty() {
                            vec![trf(
                                "All {} settings of the file are in the controller now.",
                                &[&want.len()],
                            )]
                        } else {
                            std::iter::once(trf(
                                "{} of {} settings differ after writing:",
                                &[&bad.len(), &want.len()],
                            ))
                            .chain(bad)
                            .collect()
                        };
                    }
                    notices.push(Notice::Read);
                }
            }
            Ask::Parser => {
                if let Some(w) = c.lines.iter().find_map(|l| grbl_status::parser_wcs(l)) {
                    self.wcs = w;
                }
            }
        }
    }

    /// What the settings say about the machine (empty until they have been read).
    pub fn suggestion(&self) -> firmware::Suggestion {
        firmware::suggest(&self.fw.values)
    }

    // ---- taking what the worker reports ----

    /// Take what the worker reports; call it every frame.
    pub fn poll(&mut self) -> Vec<Notice> {
        let mut notices = vec![];
        for e in self.link.poll() {
            match e {
                Evt::Log(s) => self.info(s),
                Evt::Traffic(l) => self.traffic(l, &mut notices),
                Evt::Connected(c) => {
                    let lost = self.connected && !c;
                    self.connected = c;
                    if c {
                        self.reconnect_at = None;
                        self.link.send(Cmd::Barriers(true));
                        notices.push(Notice::Connected);
                    } else {
                        self.state.clear();
                        self.progress = (0, 0);
                        self.job = None;
                        self.tls_run = None;
                        self.barrier = None;
                        self.capture = None;
                        self.asks.clear();
                        self.fw.reading = false;
                        self.pins.clear();
                        notices.push(Notice::Disconnected {
                            lost: lost && self.want_link,
                        });
                    }
                }
                Evt::Status { state, .. } => {
                    if state.starts_with("Alarm") && self.alarm.is_none() {
                        self.alarm = Some(0);
                    }
                    self.state = state;
                }
                Evt::Report(r) => self.report(&r),
                Evt::Progress { done, total } => {
                    self.progress = (done, total);
                    if total > 0 && done >= total {
                        if let Some(j) = self.job.take() {
                            let seconds = j.started.elapsed().as_secs();
                            let issues = self.issues.len();
                            if !j.after.is_empty() {
                                for l in &j.after {
                                    self.link.send(Cmd::Line(l.clone()));
                                }
                            }
                            notices.push(Notice::JobFinished {
                                seconds,
                                issues,
                                checked: j.check,
                            });
                        }
                    }
                }
                Evt::Barrier { text, line } => {
                    let b = Barrier { text, line };
                    self.barrier = Some(b.clone());
                    notices.push(Notice::Barrier(b));
                }
                Evt::Stalled { secs, state } => notices.push(Notice::Stalled { secs, state }),
            }
        }
        if let Some(at) = self.reconnect_at {
            if at <= Instant::now() {
                self.reconnect_at = None;
                if self.want_link && !self.connected {
                    notices.push(Notice::ReconnectDue);
                }
            }
        }
        if let Some(at) = self.fw.reread_at {
            if Instant::now() >= at {
                self.fw.reread_at = None;
                self.read_settings();
            }
        }
        self.pump_asks();
        notices
    }

    /// Ask to be woken in a while when something is waiting for a timer.
    pub fn wants_repaint(&self) -> bool {
        self.capture.is_some()
            || !self.asks.is_empty()
            || self.fw.reread_at.is_some()
            || self.reconnect_at.is_some()
    }

    fn report(&mut self, r: &grbl_status::Report) {
        if let Some(w) = r.wco {
            self.wco = Some(w);
        }
        if let Some(m) = grbl_status::machine_position(r, self.wco) {
            self.mpos = m;
        }
        if let Some(w) = grbl_status::work_position(r, self.wco) {
            self.wpos = w;
        }
        self.pos = if self.wpos.iter().all(|v| v.is_finite()) {
            self.wpos
        } else {
            self.mpos
        };
        self.pins = r.pins.clone().unwrap_or_default();
        if let Some((f, s)) = r.feed_spindle {
            self.feed = f;
            self.spindle = s;
        }
    }

    fn traffic(&mut self, l: ConsoleLine, notices: &mut Vec<Notice>) {
        let rx = l.dir == Dir::Rx && !l.poll;
        let text = l.text.clone();
        self.push(l);
        if !rx {
            return;
        }
        if let Some(c) = self.capture.as_mut() {
            if text == "ok" || text.starts_with("error") {
                self.finish_capture(notices);
            } else if text.starts_with("ALARM") {
                // An alarm in the middle of a read: the answer is lost.
                self.capture = None;
                self.fw.reading = false;
            } else {
                c.lines.push(text.clone());
                return;
            }
        }
        if let Some(p) = grbl_status::parse_probe(&text) {
            self.probe = Some(p);
            notices.push(Notice::Probed {
                pos: p.0,
                touched: p.1,
            });
            self.tls_hit(p, notices);
        }
        if let Some(w) = grbl_status::parser_wcs(&text) {
            self.wcs = w;
        }
        if let Some(x) = grbl_codes::explain(&text) {
            let line = self.job_running().then_some(self.progress.0);
            match x.kind {
                grbl_codes::Kind::Alarm => {
                    if self.tls_run.take().is_some() {
                        notices.push(Notice::TlsFailed);
                    }
                    self.alarm = Some(x.code);
                    notices.push(Notice::Alarm(x.code));
                    if line.is_some() {
                        self.issues.push(Issue {
                            line: self.progress.0,
                            code: x.code,
                            alarm: true,
                        });
                    }
                }
                grbl_codes::Kind::Error => {
                    notices.push(Notice::Error { code: x.code, line });
                    if let Some(n) = line {
                        self.issues.push(Issue {
                            line: n,
                            code: x.code,
                            alarm: false,
                        });
                    }
                }
            }
            if let Some(t) = x.text {
                self.info(format!("   ↳ {}", tr(t)));
            }
        }
    }

    // ---- jobs ----

    /// Send a whole job. `before` and `after` are run around it (the macros of the machine); `check` runs the program in
    /// GRBL's check mode: nothing moves, the controller only reports the lines it does not understand.
    pub fn start_job(
        &mut self,
        lines: Vec<String>,
        before: Vec<String>,
        after: Vec<String>,
        check: bool,
    ) {
        self.issues.clear();
        self.barrier = None;
        let mut after = after;
        if check {
            self.line("$C");
            after.insert(0, "$C".into());
        }
        for l in before {
            self.line(l);
        }
        self.job = Some(Job {
            started: Instant::now(),
            check,
            after,
        });
        self.link.send(Cmd::Job(lines));
    }

    /// Continue a job that stopped at a pause.
    pub fn continue_job(&mut self) {
        self.barrier = None;
        self.link.send(Cmd::Continue);
    }

    /// Stop the job: a soft reset. The `after` lines of the job are still run.
    pub fn stop_job(&mut self) {
        self.barrier = None;
        let after = self.job.take().map(|j| j.after).unwrap_or_default();
        self.link.send(Cmd::Abort);
        for l in after {
            self.link.send(Cmd::Line(l));
        }
        self.alarm = None;
    }

    // ---- moving ----

    /// One jog move of `dx`, `dy` and `dz` millimetres (relative). Zero parts are left out.
    pub fn jog(&mut self, dx: f64, dy: f64, dz: f64, feed: f64) {
        let mut words = String::new();
        for (l, v) in [('X', dx), ('Y', dy), ('Z', dz)] {
            if v != 0.0 {
                words.push_str(&format!(" {l}{}", num(v)));
            }
        }
        if words.is_empty() {
            return;
        }
        if matches!(self.controller, Controller::GrblOld | Controller::Grbl07) {
            // No `$J=` before GRBL 1.1: the move is an ordinary one, relative for just this line.
            self.line("G91 G21");
            self.line(format!("G1{words} F{}", num(feed)));
            self.line("G90");
        } else {
            self.line(format!("$J=G91 G21{words} F{}", num(feed)));
        }
    }

    /// Stop the jog that is running. GRBL 0.9 cannot: its moves run to the end.
    pub fn cancel_jog(&mut self) {
        if self.controller == Controller::Grbl {
            self.realtime(0x85);
        }
    }

    pub fn select_wcs(&mut self, n: u8) {
        let n = n.clamp(1, 6);
        self.wcs = n;
        self.line(format!("G{}", 53 + n));
        self.read_parser();
    }

    /// Set the work position of the axes (`G10 L20`) in the active coordinate system.
    pub fn set_position(&mut self, values: &[(Axis, f64)]) {
        let wcs = self.wcs;
        for l in control::set_position_lines(wcs, values) {
            self.line(l);
        }
    }

    /// Is the probe input closed right now? (A probe that is closed before a move would fail.)
    pub fn probe_closed(&self) -> bool {
        self.pins.contains('P')
    }

    /// The variables of a macro: positions, the box of the job and the last probe.
    pub fn macro_vars(
        &self,
        safe_z: f64,
        job_box: Option<([f64; 3], [f64; 3])>,
    ) -> BTreeMap<String, f64> {
        let mut v = BTreeMap::new();
        for (i, a) in ["x", "y", "z"].iter().enumerate() {
            v.insert(format!("pos{a}"), self.wpos[i]);
            v.insert(format!("mpos{a}"), self.mpos[i]);
            v.insert(format!("{a}min"), job_box.map_or(f64::NAN, |b| b.0[i]));
            v.insert(format!("{a}max"), job_box.map_or(f64::NAN, |b| b.1[i]));
            v.insert(format!("prb{a}"), self.probe.map_or(f64::NAN, |p| p.0[i]));
        }
        v.insert("safez".into(), safe_z);
        v
    }

    /// Run a macro; `Err` names what is wrong with it.
    pub fn run_macro(
        &mut self,
        code: &str,
        safe_z: f64,
        job_box: Option<([f64; 3], [f64; 3])>,
    ) -> Result<usize, String> {
        let vars = self.macro_vars(safe_z, job_box);
        let lines = control::expand(code, &vars)?;
        let n = lines.len();
        self.lines(lines);
        Ok(n)
    }

    /// Run a probe cycle. Refused when the probe is already closed (the tool would be pressed against the plate).
    pub fn probe(
        &mut self,
        c: &Control,
        axis: Axis,
        dir: i8,
        tool_diameter: f64,
    ) -> Result<(), String> {
        if self.probe_closed() {
            return Err(tr(
                "The probe input is already closed. Check the clip, the plate and the wiring.",
            )
            .to_string());
        }
        c.probe.check().map_err(|e| tr(e).to_string())?;
        let lines = control::probe_lines(&c.probe, axis, dir, tool_diameter, self.wcs);
        self.probe = None;
        self.lines(lines);
        Ok(())
    }

    /// The offset of the work coordinates from the machine ones, once the controller has reported it.
    pub fn wco(&self) -> Option<[f64; 3]> {
        self.wco
    }

    // ---- the tool length sensor ----

    pub fn tls_running(&self) -> bool {
        self.tls_run.is_some()
    }

    /// Start a measurement at the tool length sensor. `Err` names why it cannot be done now.
    pub fn tls_start(&mut self, tls: &Tls, mode: TlsMode) -> Result<(), String> {
        if !self.connected {
            return Err(tr("Connect to the machine first (Device tab)").to_string());
        }
        if self.tls_run.is_some() {
            return Err(tr("A measurement is already running.").to_string());
        }
        if !tls.set {
            return Err(tr("Set the position of the sensor first.").to_string());
        }
        tls.check().map_err(|e| tr(e).to_string())?;
        if mode == TlsMode::Tool && tls.ref_wz.is_none() {
            return Err(tr("Measure the reference tool first.").to_string());
        }
        if self.probe_closed() {
            return Err(tr("The probe input is already closed. Check the clip, the plate and the wiring.").to_string());
        }
        if self.wco.is_none() || !self.mpos.iter().all(|v| v.is_finite()) {
            return Err(tr("The controller has not reported the position yet.").to_string());
        }
        if (self.job_running() || self.job_active()) && self.barrier.is_none() {
            return Err(tr("A job is running.").to_string());
        }
        if !self.state.starts_with("Idle") {
            return Err(tr("The machine is not idle.").to_string());
        }
        self.tls_run = Some(TlsRun { mode, tls: tls.clone(), wcs: self.wcs, hits: vec![] });
        self.probe = None;
        self.lines(tls.measure_lines());
        Ok(())
    }

    fn tls_hit(&mut self, hit: ([f64; 3], bool), notices: &mut Vec<Notice>) {
        let Some(run) = self.tls_run.as_mut() else { return };
        if !hit.1 {
            self.tls_run = None;
            notices.push(Notice::TlsFailed);
            return;
        }
        run.hits.push(hit);
        if run.hits.len() < 2 {
            return;
        }
        let run = self.tls_run.take().unwrap();
        let Some(wco) = self.wco else {
            notices.push(Notice::TlsFailed);
            return;
        };
        // The machine Z of the second, slow touch, as a work Z.
        let wz = run.hits[1].0[2] - wco[2];
        match run.mode {
            TlsMode::Reference => {
                self.lines(run.tls.leave_lines());
                notices.push(Notice::TlsReference { wz });
            }
            TlsMode::Tool => {
                let reference = run.tls.ref_wz.unwrap_or(wz);
                self.lines(run.tls.adopt_lines(run.wcs, reference));
                notices.push(Notice::TlsTool { delta: wz - reference });
            }
            TlsMode::Test => {
                self.lines(run.tls.leave_lines());
                notices.push(Notice::TlsTest { wz, delta: run.tls.ref_wz.map(|r| wz - r) });
            }
        }
    }

    // ---- copies of the settings ----

    /// The settings were read: compare them with the newest copy kept with the device, make a new copy when they
    /// differ, and say so. Returns the line for the status bar; the differences are in `fw.changes` and in the console.
    pub fn compare_copies(&mut self, copies: &mut FwCopies) -> Option<String> {
        let own = std::mem::take(&mut self.fw.own_change);
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let firmware = if self.fw.info.is_known() { self.fw.info.label() } else { String::new() };
        let noted = copies.note(secs, &firmware, &self.fw.values, own);
        match noted {
            Noted::First => {
                self.fw.changes.clear();
                Some(trf("A copy of the {} settings of the controller was saved with the device.", &[&self.fw.values.len()]))
            }
            Noted::Same => {
                self.fw.changes.clear();
                None
            }
            Noted::Changed(ch) => {
                let line = trf("Settings that differ from the copy saved with the device: {}. A new copy was saved.", &[&ch.len()]);
                self.info(line.clone());
                for c in ch.iter().take(12) {
                    let (o, n) = (c.old.clone().unwrap_or_else(|| "–".into()), c.new.clone().unwrap_or_else(|| "–".into()));
                    self.info(format!("   ${}: {} → {}", c.id, o, n));
                }
                if ch.len() > 12 {
                    self.info(trf("… and {} more", &[&(ch.len() - 12)]));
                }
                self.fw.changes = ch;
                Some(line)
            }
            Noted::Own(ch) => {
                self.fw.changes.clear();
                let line = trf("A new copy of the settings was saved after your change ({} settings differ from the one before).", &[&ch.len()]);
                self.info(line.clone());
                Some(line)
            }
            Noted::Nothing => None,
        }
    }

    // ---- firmware settings ----

    /// Write one setting; the settings are read again a moment later. The first write of a session saves them first.
    pub fn write_setting(&mut self, id: u32, value: &str) -> Option<String> {
        let note = self.backup_before_write();
        self.fw.own_change = true;
        self.line(gs::write_line(id, value));
        self.fw.reread_at = Some(Instant::now() + Duration::from_millis(1200));
        note
    }

    /// Write the settings of a file, one line each; they are read back afterwards and compared.
    pub fn write_settings(&mut self, settings: Vec<(u32, String)>) -> Option<String> {
        let note = self.backup_before_write();
        self.fw.own_change = true;
        for (id, v) in &settings {
            self.line(gs::write_line(*id, v));
        }
        self.fw.verify = Some(settings);
        self.fw.reread_at = Some(Instant::now() + Duration::from_millis(1500));
        note
    }

    /// `$RST=$`: every setting back to the firmware's defaults.
    pub fn reset_settings(&mut self) -> Option<String> {
        let note = self.backup_before_write();
        self.fw.own_change = true;
        self.line("$RST=$");
        self.fw.values.clear();
        self.fw.typed.clear();
        self.fw.reread_at = Some(Instant::now() + Duration::from_millis(2500));
        note
    }

    /// Save the settings that were read to the backup folder, once per session. The line to show, if anything was saved.
    fn backup_before_write(&mut self) -> Option<String> {
        if self.fw.backed_up || self.fw.values.is_empty() {
            return None;
        }
        let dir = self.fw.backup_dir.clone()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = format!(
            "grbl-settings-{}-{:02}{:02}{:02}.txt",
            base_core::backup::ymd(now),
            now / 3600 % 24,
            now / 60 % 60,
            now % 60
        );
        let text = gs::backup_text(
            &self.fw.values,
            &self.fw.info.raw,
            &base_core::backup::ymd(now),
        );
        let _ = std::fs::create_dir_all(&dir);
        match std::fs::write(dir.join(&name), text) {
            Ok(_) => {
                self.fw.backed_up = true;
                Some(trf(
                    "The settings were saved before the first change: {}",
                    &[&dir.join(&name).display()],
                ))
            }
            Err(_) => None,
        }
    }
}

fn num(v: f64) -> String {
    let s = format!("{:.3}", v);
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-" || s == "-0" {
        "0".into()
    } else {
        s.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;

    /// A GRBL that answers `$I`, `$$`, `$G`, errors for `G99`, a probe line and polls with a status report that has
    /// the work offset and a pin.
    fn grbl(listener: TcpListener) {
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = s.set_read_timeout(Some(Duration::from_millis(10)));
            let start = Instant::now();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 64];
            while start.elapsed() < Duration::from_secs(8) {
                match s.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        for &b in &chunk[..n] {
                            match b {
                                b'?' => {
                                    let _ = s.write_all(b"<Idle|MPos:-10.000,-20.000,-5.000|FS:0,0|WCO:-10.000,-20.000,0.000|Pn:P>\n");
                                }
                                0x18 => {
                                    let _ = s.write_all(b"\r\nGrbl 1.1h ['$' for help]\r\n");
                                }
                                b'\n' => {
                                    let line = String::from_utf8_lossy(&buf).to_string();
                                    buf.clear();
                                    let reply: &[u8] = match line.as_str() {
                                        "$I" => b"[VER:1.1h.20190825:]\n[OPT:V,15,128]\nok\n",
                                        "$$" => b"$30=24000.000\n$31=1000.000\n$110=3000.000\n$111=3000.000\n$130=300.000\n$131=180.000\n$132=45.000\nok\n",
                                        "$G" => b"[GC:G0 G56 G17 G21 G90 G94 M5 M9 T0 F0 S0]\nok\n",
                                        "G99" => b"error:20\n",
                                        "G38.2 Z-1" => b"[PRB:1.000,2.000,-3.000:1]\nok\n",
                                        _ => b"ok\n",
                                    };
                                    let _ = s.write_all(reply);
                                }
                                _ => buf.push(b),
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    fn connect(m: &mut Machine, port: u16) {
        m.connect(
            Target::Tcp {
                host: "127.0.0.1".into(),
                port,
            },
            Streaming::Counting,
        );
    }

    fn wait(
        m: &mut Machine,
        secs: f64,
        mut f: impl FnMut(&mut Machine, &[Notice]) -> bool,
    ) -> bool {
        let end = Instant::now() + Duration::from_secs_f64(secs);
        while Instant::now() < end {
            let n = m.poll();
            if f(m, &n) {
                return true;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn the_firmware_the_settings_and_the_parser_state_are_read_after_connecting() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        grbl(listener);
        let mut m = Machine::new(&Context::default());
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected));
        m.read_all();
        assert!(m.fw.reading);
        assert!(
            wait(&mut m, 4.0, |m, _| !m.fw.reading
                && m.wcs == 3
                && m.fw.info.is_known()),
            "all three answers are in"
        );
        assert_eq!(m.fw.info.label(), "Grbl 1.1h");
        assert_eq!(m.fw.values.len(), 7);
        let s = m.suggestion();
        assert_eq!(s.area, Some([300.0, 180.0, 45.0]));
        assert_eq!(s.spindle, Some((1000.0, 24000.0)));
        // Their lines did not leak into the console as unexplained noise: they are all there as traffic.
        assert!(m.console.iter().any(|l| l.text == "$130=300.000"));
    }

    /// A GRBL 0.9: the status has commas, there is no `$J=`; it records what it is sent.
    fn grbl09(listener: TcpListener, seen: std::sync::Arc<std::sync::Mutex<Vec<String>>>) {
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = s.set_read_timeout(Some(Duration::from_millis(10)));
            let start = Instant::now();
            let (mut buf, mut chunk) = (Vec::new(), [0u8; 64]);
            while start.elapsed() < Duration::from_secs(8) {
                match s.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        for &b in &chunk[..n] {
                            match b {
                                b'?' => {
                                    let _ = s.write_all(b"<Idle,MPos:5.529,0.560,7.000,WPos:1.529,-5.440,2.000>\n");
                                }
                                0x18 => {
                                    let _ = s.write_all(b"\r\nGrbl 0.9i ['$' for help]\r\n");
                                }
                                b'\n' => {
                                    let line = String::from_utf8_lossy(&buf).to_string();
                                    buf.clear();
                                    let reply: &[u8] = if line.starts_with("$J=") { b"error: Unsupported command\n" } else { b"ok\n" };
                                    seen.lock().unwrap().push(line);
                                    let _ = s.write_all(reply);
                                }
                                _ => buf.push(b),
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    #[test]
    fn grbl_09_reports_with_commas_and_jogs_with_plain_moves() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        grbl09(listener, seen.clone());
        let mut m = Machine::new(&Context::default());
        m.set_grbl_version(base_core::machine::GrblVersion::Grbl09);
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected));
        assert!(wait(&mut m, 3.0, |m, _| m.state == "Idle" && m.pos[0] == 1.529), "the work position of the report");
        assert_eq!((m.mpos, m.wpos), ([5.529, 0.56, 7.0], [1.529, -5.44, 2.0]));
        m.jog(1.0, 0.0, -0.5, 3000.0);
        m.cancel_jog();
        assert!(wait(&mut m, 3.0, |_, _| seen.lock().unwrap().iter().any(|l| l == "G90")));
        let lines = seen.lock().unwrap().clone();
        let at = lines.iter().position(|l| l == "G91 G21").expect("relative mode");
        assert_eq!(lines[at + 1], "G1 X1 Z-0.5 F3000");
        assert_eq!(lines[at + 2], "G90");
        assert!(!lines.iter().any(|l| l.starts_with("$J=")), "{lines:?}");
    }

    #[test]
    fn grbl_07_is_never_asked_for_a_status_and_still_jogs() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let seen = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
        grbl09(listener, seen.clone());
        let mut m = Machine::new(&Context::default());
        m.set_grbl_version(base_core::machine::GrblVersion::Grbl07);
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected));
        m.jog(2.0, 3.0, 0.0, 600.0);
        assert!(wait(&mut m, 3.0, |_, _| seen.lock().unwrap().iter().any(|l| l == "G90")));
        // The fake would answer a `?` with a report; GRBL 0.7 has none, so none was asked for.
        assert!(m.state.is_empty() && m.pos.iter().all(|v| v.is_nan()), "state {:?} pos {:?}", m.state, m.pos);
        let lines = seen.lock().unwrap().clone();
        assert!(lines.contains(&"G1 X2 Y3 F600".to_string()), "{lines:?}");
    }

    #[test]
    fn positions_pins_and_the_work_offset_come_from_the_reports() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        grbl(listener);
        let mut m = Machine::new(&Context::default());
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected));
        assert!(wait(&mut m, 3.0, |m, _| m
            .wpos
            .iter()
            .all(|v| v.is_finite())));
        assert_eq!(m.mpos, [-10.0, -20.0, -5.0]);
        assert_eq!(m.wpos, [0.0, 0.0, -5.0]);
        assert_eq!(m.pos, m.wpos);
        assert!(m.probe_closed());
        // A probe cycle is refused while the probe input is closed.
        let c = Control::default();
        assert!(m.probe(&c, Axis::Z, -1, 3.0).is_err());
    }

    #[test]
    fn errors_are_explained_counted_and_tied_to_the_line_of_the_job() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        grbl(listener);
        let mut m = Machine::new(&Context::default());
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected));
        let mut seen = vec![];
        m.start_job(
            vec!["G0 X1".into(), "G99".into(), "G0 X2".into()],
            vec![],
            vec![],
            false,
        );
        assert!(wait(&mut m, 4.0, |_, n| {
            for x in n {
                if let Notice::JobFinished { .. } | Notice::Error { .. } = x {
                    seen.push(format!("{x:?}"));
                }
            }
            seen.iter().any(|s| s.starts_with("JobFinished"))
        }));
        assert!(
            seen.iter()
                .any(|s| s.contains("Error { code: 20, line: Some(1) }")),
            "{seen:?}"
        );
        assert_eq!(
            m.issues,
            vec![Issue {
                line: 1,
                code: 20,
                alarm: false
            }]
        );
        assert!(
            m.console
                .iter()
                .any(|l| l.dir == Dir::Info && l.text.contains("not supported")),
            "the explanation is in the console"
        );
        assert!(!m.job_active());
    }

    #[test]
    fn a_job_waits_at_m0_and_runs_the_macros_around_it() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        grbl(listener);
        let mut m = Machine::new(&Context::default());
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected));
        m.start_job(
            vec![
                "G0 X1".into(),
                "M0 (Change the bit to: V-bit 60°)".into(),
                "G0 X2".into(),
            ],
            vec!["G4 P0".into()],
            vec!["M5".into()],
            false,
        );
        assert!(wait(&mut m, 4.0, |m, _| m.barrier.is_some()));
        assert_eq!(
            m.barrier.as_ref().unwrap().message(),
            "Change the bit to: V-bit 60°"
        );
        assert!(m.job_active());
        m.continue_job();
        let mut fin = None;
        assert!(wait(&mut m, 4.0, |_, n| {
            for x in n {
                if let Notice::JobFinished { checked, .. } = x {
                    fin = Some(*checked);
                }
            }
            fin.is_some()
        }));
        assert_eq!(fin, Some(false));
        assert!(
            wait(&mut m, 3.0, |m, _| m
                .console
                .iter()
                .any(|l| l.dir == Dir::Tx && l.text == "M5")),
            "the line after the job goes out"
        );
        // The line before the job and the line after it both went out.
        let sent: Vec<&str> = m
            .console
            .iter()
            .filter(|l| l.dir == Dir::Tx && !l.poll)
            .map(|l| l.text.as_str())
            .collect();
        let p = |s: &str| sent.iter().position(|l| *l == s);
        assert!(p("G4 P0").unwrap() < p("G0 X1").unwrap(), "{sent:?}");
        assert!(p("M5").unwrap() > p("G0 X2").unwrap(), "{sent:?}");
    }

    #[test]
    fn writes_are_backed_up_once_and_read_back() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        grbl(listener);
        let mut m = Machine::new(&Context::default());
        let dir = std::env::temp_dir().join(format!("bc-fw-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        m.fw.backup_dir = Some(dir.clone());
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected));
        m.read_all();
        assert!(wait(&mut m, 4.0, |m, _| !m.fw.reading && !m.fw.values.is_empty()));
        let note = m.write_setting(110, "5000");
        assert!(note.is_some_and(|n| n.contains("grbl-settings-")));
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        assert!(
            m.write_setting(111, "5000").is_none(),
            "only the first change of a session saves"
        );
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 1);
        let text = std::fs::read_to_string(
            std::fs::read_dir(&dir)
                .unwrap()
                .next()
                .unwrap()
                .unwrap()
                .path(),
        )
        .unwrap();
        assert!(text.contains("$130=300.000"));
        assert!(m.fw.reread_at.is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn jog_and_position_lines_are_built() {
        let mut m = Machine::new(&Context::default());
        m.jog(1.0, 0.0, -0.5, 3000.0);
        m.jog(0.0, 0.0, 0.0, 100.0);
        m.wcs = 2;
        m.set_position(&[(Axis::X, 0.0), (Axis::Y, 0.0)]);
        // Nothing is connected, so the lines wait in the worker; the console has no copy of them.
        assert_eq!(m.console.iter().filter(|l| l.dir == Dir::Tx).count(), 0);
        assert_eq!(num(1.0), "1");
        assert_eq!(num(-0.0004), "0");
        assert_eq!(
            Barrier {
                text: "M6 T2".into(),
                line: 0
            }
            .message(),
            "M6 T2"
        );
        let v = m.macro_vars(5.0, Some(([0.0, 1.0, 2.0], [3.0, 4.0, 5.0])));
        assert_eq!((v["xmin"], v["ymax"], v["safez"]), (0.0, 4.0, 5.0));
        assert!(m.run_macro("G0 Z[safez]", 5.0, None).is_ok());
        assert!(
            m.run_macro("G0 X[posx]", 5.0, None).is_err(),
            "no position yet"
        );
    }

    /// A GRBL with a tool length sensor: its `G38.2` lines answer with the machine Z where the tool touched, one value
    /// after the other from `hits`.
    fn grbl_with_sensor(listener: TcpListener, hits: Vec<f64>) {
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = s.set_read_timeout(Some(Duration::from_millis(10)));
            let start = Instant::now();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 64];
            let mut hits = hits.into_iter();
            while start.elapsed() < Duration::from_secs(8) {
                match s.read(&mut chunk) {
                    Ok(0) => break,
                    Ok(n) => {
                        for &b in &chunk[..n] {
                            match b {
                                b'?' => {
                                    let _ = s.write_all(b"<Idle|MPos:-10.000,-20.000,-5.000|WCO:-10.000,-20.000,-30.000>\n");
                                }
                                b'\n' => {
                                    let line = String::from_utf8_lossy(&buf).to_string();
                                    buf.clear();
                                    if line.starts_with("G38.2") {
                                        let z = hits.next().unwrap_or(-99.0);
                                        let _ = s.write_all(format!("[PRB:-10.000,-20.000,{z:.3}:1]\nok\n").as_bytes());
                                    } else {
                                        let _ = s.write_all(b"ok\n");
                                    }
                                }
                                _ => buf.push(b),
                            }
                        }
                    }
                    _ => {}
                }
            }
        });
    }

    #[test]
    fn the_tool_length_sensor_gives_a_new_tool_the_work_z_of_the_reference() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        // The reference tool touches at machine Z -61.0 (quick) and -61.3 (slow); the next tool, 3.1 mm longer, at -58.0 and -58.2.
        grbl_with_sensor(listener, vec![-61.0, -61.3, -58.0, -58.2, -61.0, -61.3]);
        let mut m = Machine::new(&Context::default());
        connect(&mut m, port);
        assert!(wait(&mut m, 3.0, |m, _| m.connected && m.wco().is_some() && m.state.starts_with("Idle")));
        let mut tls = Tls { set: true, x: -10.0, y: -20.0, start_z: -40.0, ..Default::default() };
        // The tool has to be measured against a reference first.
        assert!(m.tls_start(&tls, TlsMode::Tool).is_err());
        m.tls_start(&tls, TlsMode::Reference).unwrap();
        assert!(m.tls_running() && m.tls_start(&tls, TlsMode::Reference).is_err(), "one at a time");
        let mut reference = None;
        assert!(wait(&mut m, 4.0, |_, n| {
            for x in n {
                if let Notice::TlsReference { wz } = x {
                    reference = Some(*wz);
                }
            }
            reference.is_some()
        }));
        // Work Z of the tip = machine Z of the touch minus the offset of the work coordinates.
        assert!((reference.unwrap() - (-31.3)).abs() < 1e-9);
        tls.ref_wz = reference;
        m.tls_start(&tls, TlsMode::Tool).unwrap();
        let mut delta = None;
        assert!(wait(&mut m, 4.0, |_, n| {
            for x in n {
                if let Notice::TlsTool { delta: d } = x {
                    delta = Some(*d);
                }
            }
            delta.is_some()
        }));
        assert!((delta.unwrap() - 3.1).abs() < 1e-9, "{delta:?}");
        // The offset was set so that this place has the Z of the reference, and the tool went up.
        assert!(wait(&mut m, 3.0, |m, _| m.console.iter().any(|l| l.dir == Dir::Tx && l.text == "G53 G0 Z-5") && m.console.iter().any(|l| l.dir == Dir::Tx && l.text == "G10 L20 P1 Z-31.3")));
        // A test only measures.
        m.tls_start(&tls, TlsMode::Test).unwrap();
        let mut test = None;
        assert!(wait(&mut m, 4.0, |_, n| {
            for x in n {
                if let Notice::TlsTest { delta, .. } = x {
                    test = Some(*delta);
                }
            }
            test.is_some()
        }));
        assert_eq!(test.flatten().map(|d| (d * 10.0).round()), Some(0.0));
    }

    #[test]
    fn a_sensor_that_is_not_set_or_a_closed_probe_is_refused() {
        let mut m = Machine::new(&Context::default());
        assert!(m.tls_start(&Tls::default(), TlsMode::Reference).is_err(), "not connected");
        m.connected = true;
        assert!(m.tls_start(&Tls::default(), TlsMode::Reference).is_err(), "not set");
        let tls = Tls { set: true, ..Default::default() };
        m.pins = "P".into();
        assert!(m.tls_start(&tls, TlsMode::Reference).is_err(), "probe closed");
        m.pins.clear();
        assert!(m.tls_start(&tls, TlsMode::Reference).is_err(), "no position known yet");
    }

    #[test]
    fn what_the_controller_reports_is_compared_with_the_copies_kept_with_the_device() {
        let mut m = Machine::new(&Context::default());
        let mut copies = FwCopies::default();
        m.fw.values = [(110u32, "3000.000"), (130, "300.000")].into_iter().map(|(k, v)| (k, v.to_string())).collect();
        assert!(m.compare_copies(&mut copies).unwrap().contains("saved with the device"));
        assert_eq!(copies.0.len(), 1);
        assert!(m.compare_copies(&mut copies).is_none(), "the same again");
        // Somebody changed a setting from another program: the person is told and the old copy stays.
        m.fw.values.insert(110, "5000.000".into());
        let line = m.compare_copies(&mut copies).expect("a change");
        assert!(line.contains('1'), "{line}");
        assert_eq!(copies.0.len(), 2);
        assert_eq!(m.fw.changes.len(), 1);
        assert!(m.console.iter().any(|l| l.text.contains("$110") && l.text.contains("3000.000") && l.text.contains("5000.000")));
        assert_eq!(copies.0[0].values.get(&110).map(String::as_str), Some("3000.000"), "the earlier copy is kept");
        // A change made here is not a surprise: a copy is made without the alarm.
        m.write_setting(111, "4000");
        m.fw.values.insert(111, "4000.000".into());
        let own = m.compare_copies(&mut copies).expect("own");
        assert!(m.fw.changes.is_empty() && !m.fw.own_change, "{own}");
        assert_eq!(copies.newest().unwrap().why, "own");
    }
}
