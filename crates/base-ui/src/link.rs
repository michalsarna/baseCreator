//! The link to a controller (GRBL, Marlin, Smoothieware) over a serial port or a TCP socket, running on a worker
//! thread: streaming of jobs with the character-counting protocol, immediate lines, realtime bytes and status polling.
//! Shared by every program that talks to a machine.
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use base_core::machine::StatusPoll;
use base_core::machine::Controller;
use std::time::{Duration, Instant};

/// Where to connect.
#[derive(Clone, Debug)]
pub enum Target {
    Serial { port: String, baud: u32 },
    /// A raw TCP socket, e.g. `ser2net` on a Raspberry Pi.
    Tcp { host: String, port: u16 },
}

impl Target {
    fn label(&self) -> String {
        match self {
            Target::Serial { port, baud } => format!("{port} @ {baud}"),
            Target::Tcp { host, port } => format!("{host}:{port}"),
        }
    }
}

/// A byte stream to the controller: a serial port or a TCP socket.
trait Link: Read + Write + Send {}
impl<T: Read + Write + Send> Link for T {}

fn open_link(t: &Target) -> Result<(Box<dyn Link>, bool), String> {
    match t {
        Target::Serial { port, baud } => serialport::new(port, *baud).timeout(Duration::from_millis(5)).open().map(|p| (Box::new(p) as Box<dyn Link>, false)).map_err(|e| e.to_string()),
        Target::Tcp { host, port } => {
            use std::net::{TcpStream, ToSocketAddrs};
            let addrs = (host.as_str(), *port).to_socket_addrs().map_err(|e| e.to_string())?;
            let mut last = "no address".to_string();
            for a in addrs {
                match TcpStream::connect_timeout(&a, Duration::from_secs(4)) {
                    Ok(s) => {
                        let _ = s.set_nodelay(true);
                        let _ = s.set_read_timeout(Some(Duration::from_millis(5)));
                        let _ = s.set_write_timeout(Some(Duration::from_secs(3)));
                        return Ok((Box::new(s), true));
                    }
                    Err(e) => last = e.to_string(),
                }
            }
            Err(last)
        }
    }
}

pub enum Cmd {
    Connect { target: Target, controller: Controller, streaming: base_core::machine::Streaming },
    Disconnect,
    /// A single immediate line (jog, $H, ...), queued ahead of any running job.
    Line(String),
    /// Stream a whole job.
    Job(Vec<String>),
    /// Single-byte realtime command: `?`, `!`, `~`, 0x18.
    Realtime(u8),
    /// Soft reset and drop everything queued.
    Abort,
    /// Stop the job at its pauses (M0, M6) instead of sending them: the controller finishes what it has, the link tells
    /// the program (`Evt::Barrier`) and waits for `Continue`. Off by default.
    Barriers(bool),
    /// Go on with a job that stopped at a pause.
    Continue,
}

/// Direction of a console line.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Dir {
    /// Message from the application itself.
    Info,
    /// Sent to the controller.
    Tx,
    /// Received from the controller.
    Rx,
}

#[derive(Clone)]
pub struct ConsoleLine {
    pub dir: Dir,
    pub text: String,
    /// Part of the periodic `?` status polling (hidden by default in the console).
    pub poll: bool,
}

impl ConsoleLine {
    pub fn info(text: impl Into<String>) -> Self {
        ConsoleLine { dir: Dir::Info, text: text.into(), poll: false }
    }
}

pub enum Evt {
    Log(String),
    /// A line that crossed the serial port.
    Traffic(ConsoleLine),
    Connected(bool),
    Status { state: String, x: f64, y: f64, z: f64 },
    Progress { done: usize, total: usize },
    /// The full status report of a GRBL controller (the simple `Status` is sent too).
    Report(base_core::grbl_status::Report),
    /// The job reached a pause and waits: `text` is the line (`M0 (Change the bit to: V-bit)`), `line` its number.
    Barrier { text: String, line: usize },
    /// A job is running, the controller has not answered for `secs` and the machine is not moving.
    Stalled { secs: u64, state: String },
}

pub struct LaserLink {
    tx: Sender<Cmd>,
    rx: Receiver<Evt>,
}

impl LaserLink {
    pub fn spawn(ctx: egui::Context) -> Self {
        Self::spawn_with(ctx, STALL_AFTER)
    }
    /// `stall` is how long a job may wait for an answer, with the machine standing still, before it is reported.
    pub fn spawn_with(ctx: egui::Context, stall: Duration) -> Self {
        let (tx, wrx) = channel();
        let (wtx, rx) = channel();
        std::thread::spawn(move || worker(wrx, wtx, ctx, stall));
        Self { tx, rx }
    }
    pub fn send(&self, c: Cmd) {
        let _ = self.tx.send(c);
    }
    pub fn poll(&self) -> Vec<Evt> {
        let mut v = vec![];
        while let Ok(e) = self.rx.try_recv() {
            v.push(e);
        }
        v
    }
}

/// A serial port with the USB ids of the device behind it, when it is a USB one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortInfo {
    pub name: String,
    pub usb: Option<(u16, u16)>,
}

impl PortInfo {
    /// "/dev/ttyUSB0 (1a86:7523)".
    pub fn label(&self) -> String {
        match self.usb {
            Some((v, p)) => format!("{} ({:04x}:{:04x})", self.name, v, p),
            None => self.name.clone(),
        }
    }
}

pub fn list_port_info() -> Vec<PortInfo> {
    serialport::available_ports()
        .map(|p| {
            p.into_iter()
                .map(|x| PortInfo {
                    usb: match &x.port_type {
                        serialport::SerialPortType::UsbPort(u) => Some((u.vid, u.pid)),
                        _ => None,
                    },
                    name: x.port_name,
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The port to open: `spec` is a port name, or `VID:PID`; `usb_id` is the USB device the profile is bound to. A bound
/// profile keeps its port while that still has the right ids, and otherwise follows the ids to the port that has them
/// now (the name changes after a restart or another socket).
pub fn resolve_port(spec: &str, usb_id: &str, ports: &[PortInfo]) -> String {
    let by_id = |id: (u16, u16)| ports.iter().find(|p| p.usb == Some(id)).map(|p| p.name.clone());
    if let Some(id) = base_core::machine::parse_usb_id(spec) {
        return by_id(id).unwrap_or_else(|| spec.to_string());
    }
    if let Some(id) = base_core::machine::parse_usb_id(usb_id) {
        if ports.iter().any(|p| p.name == spec && p.usb == Some(id)) {
            return spec.to_string();
        }
        if let Some(name) = by_id(id) {
            return name;
        }
    }
    spec.to_string()
}

/// Remove comments/blank lines so only real commands get streamed.
pub fn clean_gcode(text: &str) -> Vec<String> {
    text.lines()
        .map(|l| l.split(';').next().unwrap_or("").trim().to_string())
        .filter(|l| !l.is_empty())
        .collect()
}

/// Where the layers of a job begin, as line numbers of the lines sent to the machine (`clean_gcode` of the text): the lines that start
/// with a `(Layer:` comment. The first layer is not in the list (it begins with the job), so the n-th number is also where the n-th layer
/// is done.
pub fn layer_marks(lines: &[String]) -> Vec<usize> {
    lines.iter().enumerate().filter(|(_, l)| l.starts_with("(Layer:")).map(|(i, _)| i).skip(1).collect()
}

/// The same for a G-code text with its comments: a `(Layer: …)` line or a `; layer …` comment starts a layer; the numbers are those of the
/// lines `clean_gcode` leaves.
pub fn layer_marks_in_text(text: &str) -> Vec<usize> {
    let (mut kept, mut marks) = (0usize, vec![]);
    for raw in text.lines() {
        let t = raw.trim_start();
        let semi = t.strip_prefix(';').is_some_and(|c| c.trim_start().to_ascii_lowercase().starts_with("layer"));
        if semi || t.starts_with("(Layer:") {
            marks.push(kept);
        }
        if !raw.split(';').next().unwrap_or("").trim().is_empty() {
            kept += 1;
        }
    }
    marks.into_iter().skip(1).collect()
}

fn realtime_name(b: u8) -> String {
    match b {
        0x18 => "0x18 (soft reset)".into(),
        0x85 => "0x85 (jog cancel)".into(),
        b'!' => "! (feed hold)".into(),
        b'~' => "~ (cycle start)".into(),
        b'?' => "?".into(),
        _ => format!("0x{b:02X}"),
    }
}

const RX_BUFFER: usize = 120; // GRBL has 128 bytes; keep a margin.
/// Longest wait for GRBL's start-up banner after a soft reset before streaming anyway.
const BANNER_TIMEOUT: Duration = Duration::from_millis(2500);
/// How long a running job may wait for an `ok` while the machine stands still before it is reported as stalled.
const STALL_AFTER: Duration = Duration::from_secs(10);

/// Why a line is in the queue; decides how its `ok` is shown in the console.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Job,
    Immediate,
    Poll,
}

fn worker(rx: Receiver<Cmd>, tx: Sender<Evt>, ctx: egui::Context, stall_after: Duration) {
    let emit = |e: Evt| {
        let _ = tx.send(e);
        ctx.request_repaint();
    };
    let traffic = |dir: Dir, text: String, poll: bool| Evt::Traffic(ConsoleLine { dir, text, poll });
    let mut port: Option<Box<dyn Link>> = None;
    let mut is_tcp = false;
    let mut controller = Controller::Grbl;
    let mut queue: VecDeque<(String, Kind)> = VecDeque::new();
    let mut pending: VecDeque<(usize, Kind)> = VecDeque::new();
    let mut inbuf = Vec::<u8>::new();
    let (mut total, mut done) = (0usize, 0usize);
    let mut last_poll = Instant::now();
    // Set after a soft reset: GRBL drops everything it receives until it has restarted and printed its banner
    // (`Grbl 1.1h ['$' for help]`), so queued lines are held back until then.
    let mut restarting: Option<Instant> = None;
    let mut simple = false;
    // Since when a line has been waiting for its `ok`, and the machine state from the last status report.
    let mut waiting_since: Option<Instant> = None;
    let mut stall_reported = false;
    let mut machine_state = String::new();
    let mut barriers = false;
    let mut held = false;
    loop {
        loop {
            match rx.try_recv() {
                Ok(Cmd::Connect { target, controller: c, streaming }) => {
                    let label = target.label();
                    match open_link(&target) {
                        Ok((mut p, tcp)) => {
                            is_tcp = tcp;
                            controller = c;
                            simple = streaming == base_core::machine::Streaming::Simple;
                            waiting_since = None;
                            stall_reported = false;
                            restarting = None;
                            if let Some(b) = c.on_connect() {
                                let _ = p.write_all(&[b]);
                                restarting = Some(Instant::now());
                            }
                            port = Some(p);
                            queue.clear();
                            pending.clear();
                            emit(Evt::Log(crate::i18n::trf("Connected to {}", &[&label])));
                            emit(Evt::Connected(true));
                        }
                        Err(e) => emit(Evt::Log(crate::i18n::trf("Cannot open {}: {}", &[&label, &e]))),
                    }
                }
                Ok(Cmd::Disconnect) => {
                    port = None;
                    queue.clear();
                    pending.clear();
                    held = false;
                    emit(Evt::Connected(false));
                    emit(Evt::Log(crate::i18n::tr("Disconnected").into()));
                }
                Ok(Cmd::Barriers(b)) => barriers = b,
                Ok(Cmd::Continue) => held = false,
                Ok(Cmd::Line(l)) => {
                    // Immediate lines go ahead of job lines.
                    let at = queue.iter().position(|(_, k)| *k == Kind::Job).unwrap_or(queue.len());
                    queue.insert(at, (l, Kind::Immediate));
                }
                Ok(Cmd::Job(lines)) => {
                    total = lines.len();
                    done = 0;
                    queue.extend(lines.into_iter().map(|l| (l, Kind::Job)));
                    emit(Evt::Progress { done, total });
                }
                Ok(Cmd::Realtime(b)) => {
                    if let Some(p) = port.as_mut() {
                        let _ = p.write_all(&[b]);
                        emit(traffic(Dir::Tx, realtime_name(b), false));
                    }
                }
                Ok(Cmd::Abort) => {
                    queue.clear();
                    pending.clear();
                    held = false;
                    total = 0;
                    if let Some(p) = port.as_mut() {
                        if let Some(b) = controller.abort_byte() {
                            let _ = p.write_all(&[b]);
                            emit(traffic(Dir::Tx, realtime_name(b), false));
                            restarting = Some(Instant::now());
                        }
                        if let Some(l) = controller.abort_line() {
                            queue.push_back((l.to_string(), Kind::Immediate));
                        }
                    }
                    emit(Evt::Progress { done: 0, total: 0 });
                    emit(Evt::Log(crate::i18n::tr("Aborted (soft reset)").into()));
                }
                Err(TryRecvError::Empty) => break,
                Err(TryRecvError::Disconnected) => return,
            }
        }
        let mut lost = false;
        if let Some(p) = port.as_mut() {
            // Read
            let mut buf = [0u8; 256];
            match p.read(&mut buf) {
                Ok(n) if n > 0 => inbuf.extend_from_slice(&buf[..n]),
                // A TCP peer that closed the socket reads as zero bytes.
                Ok(_) if is_tcp => lost = true,
                Err(e) if !matches!(e.kind(), std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock | std::io::ErrorKind::Interrupted) => lost = true,
                _ => {}
            }
            while let Some(i) = inbuf.iter().position(|&b| b == b'\n') {
                let line: Vec<u8> = inbuf.drain(..=i).collect();
                let line = String::from_utf8_lossy(&line).trim().to_string();
                if line.is_empty() {
                    continue;
                }
                if restarting.is_some() && line.starts_with("Grbl ") {
                    // Restarted: anything sent before the reset is gone, and so are its acknowledgements.
                    restarting = None;
                    pending.clear();
                    emit(traffic(Dir::Rx, line, false));
                } else if let Some((state, x, y)) = base_core::machine::parse_status(&line) {
                    emit(traffic(Dir::Rx, line.clone(), true));
                    machine_state = state.clone();
                    emit(Evt::Status { state, x, y, z: base_core::machine::parse_status_z(&line).unwrap_or(f64::NAN) });
                    let report = if controller == Controller::GrblOld { base_core::grbl_status::parse_legacy(&line) } else { base_core::grbl_status::parse(&line) };
                    if let Some(r) = report {
                        emit(Evt::Report(r));
                    }
                } else if restarting.is_some() && (line == "ok" || line.starts_with("error")) {
                    // Start-up noise (e.g. `error:7` from an empty EEPROM) or a late reply to a line sent before the
                    // reset: not the acknowledgement of anything queued now.
                    emit(traffic(Dir::Rx, line, false));
                } else if line == "ok" || line.starts_with("error") {
                    let kind = pending.pop_front().map(|(_, k)| k);
                    waiting_since = (!pending.is_empty()).then(Instant::now);
                    stall_reported = false;
                    emit(traffic(Dir::Rx, line.clone(), kind == Some(Kind::Poll)));
                    if kind == Some(Kind::Job) {
                        done += 1;
                        emit(Evt::Progress { done, total });
                    }
                } else {
                    emit(traffic(Dir::Rx, line, false));
                }
            }
            if restarting.is_some_and(|t| t.elapsed() > BANNER_TIMEOUT) {
                // No banner (a clone that does not print one?): stream anyway rather than hang.
                restarting = None;
                pending.clear();
            }
            // Write while the controller's RX buffer has room.
            while let Some((l, front_kind)) = queue.front().filter(|_| restarting.is_none()) {
                let front_kind = *front_kind;
                // A job that stopped at a pause sends nothing more of itself; immediate lines still go.
                if held && front_kind == Kind::Job {
                    break;
                }
                if barriers && front_kind == Kind::Job && base_core::resume::is_pause(l) {
                    // Wait until everything before it has been done, then stop here and tell the program.
                    let idle = machine_state.split(':').next().unwrap_or("") == "Idle";
                    if !pending.is_empty() || !idle {
                        break;
                    }
                    let (text, _) = queue.pop_front().unwrap();
                    emit(Evt::Log(text.clone()));
                    emit(Evt::Barrier { text, line: done });
                    done += 1;
                    emit(Evt::Progress { done, total });
                    held = true;
                    break;
                }
                let used: usize = pending.iter().map(|(n, _)| n).sum();
                // Simple streaming keeps one line in flight; counting fills the controller's buffer.
                if used + l.len() + 1 > RX_BUFFER || (simple && !pending.is_empty()) {
                    break;
                }
                let (l, kind) = queue.pop_front().unwrap();
                emit(traffic(Dir::Tx, l.clone(), kind == Kind::Poll));
                if p.write_all(format!("{l}\n").as_bytes()).is_err() {
                    lost = true;
                    break;
                }
                if pending.is_empty() {
                    waiting_since = Some(Instant::now());
                }
                pending.push_back((l.len() + 1, kind));
            }
            // A job that waits for an answer while the machine stands still: say so, ask for the state, and leave the
            // decision to the user (nothing is resumed or sent on its own).
            let moving = matches!(machine_state.split(':').next().unwrap_or(""), "Run" | "Hold" | "Home" | "Jog" | "Door" | "Check");
            let running_job = total > 0 && done < total;
            if let (Some(t), true) = (waiting_since, running_job && !pending.is_empty() && !moving && !stall_reported) {
                if t.elapsed() >= stall_after {
                    stall_reported = true;
                    let secs = t.elapsed().as_secs();
                    emit(Evt::Log(crate::i18n::trf("No answer from the controller for {} s while a job is running (state: {}).", &[&secs, &machine_state])));
                    emit(Evt::Stalled { secs, state: machine_state.clone() });
                    if let StatusPoll::Byte(b) = controller.status_poll() {
                        let _ = p.write_all(&[b]);
                        emit(traffic(Dir::Tx, (b as char).to_string(), true));
                    }
                }
            }
            if last_poll.elapsed() > Duration::from_millis(controller.poll_interval_ms()) {
                last_poll = Instant::now();
                match controller.status_poll() {
                    StatusPoll::Byte(b) => {
                        let _ = p.write_all(&[b]);
                        emit(traffic(Dir::Tx, (b as char).to_string(), true));
                    }
                    // Line-based polling only when nothing else is waiting, so jobs are not slowed down.
                    StatusPoll::Line(l) if queue.is_empty() && pending.is_empty() => queue.push_back((l.to_string(), Kind::Poll)),
                    _ => {}
                }
            }
        }
        if lost {
            port = None;
            queue.clear();
            pending.clear();
            emit(Evt::Connected(false));
            emit(Evt::Log(crate::i18n::tr("Connection lost").into()));
        }
        std::thread::sleep(Duration::from_millis(if port.is_some() { 2 } else { 30 }));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpListener;

    fn info(name: &str, usb: Option<(u16, u16)>) -> PortInfo {
        PortInfo { name: name.into(), usb }
    }

    #[test]
    fn the_layers_of_a_job_are_found_in_the_lines_and_in_the_text_with_the_same_numbers() {
        let text = "(MillCreator: t)\nG21 G90\n(Layer: cut)\nG0 X1\nG1 X2\n(Layer: pocket)\nG0 X3\n; comment\n(Layer: engrave)\nG1 X4\nM30\n";
        let lines = clean_gcode(text);
        let from_lines = layer_marks(&lines);
        assert_eq!(from_lines.len(), 2, "the first layer is not a mark: {from_lines:?}");
        assert_eq!(lines[from_lines[0]], "(Layer: pocket)");
        assert_eq!(layer_marks_in_text(text), from_lines);
        // A laser job: `; layer` comments are gone from the lines, the numbers still point at the first line of the next layer.
        let laser = "; job\nG21\n; layer Cut (line) speed 20\nG1 X1\nG1 X2\n; pass 2/2\nG1 X3\n; layer Fill (fill) speed 90\nG1 Y1\nM5\n";
        let marks = layer_marks_in_text(laser);
        assert_eq!(marks.len(), 1);
        assert_eq!(clean_gcode(laser)[marks[0]], "G1 Y1");
        assert!(layer_marks(&[]).is_empty() && layer_marks_in_text("G0 X1\n").is_empty());
    }

    #[test]
    fn ports_are_found_by_usb_id_when_their_names_change() {
        let ports = vec![info("/dev/ttyUSB1", Some((0x1a86, 0x7523))), info("/dev/ttyACM0", Some((0x0403, 0x6001))), info("/dev/ttyS0", None)];
        // The port field may itself be a VID:PID.
        assert_eq!(resolve_port("0403:6001", "", &ports), "/dev/ttyACM0");
        assert_eq!(resolve_port("1A86:7523", "", &ports), "/dev/ttyUSB1");
        // A bound profile whose old name is gone (or now belongs to another device) follows the ids.
        assert_eq!(resolve_port("/dev/ttyUSB0", "1a86:7523", &ports), "/dev/ttyUSB1");
        assert_eq!(resolve_port("/dev/ttyACM0", "1a86:7523", &ports), "/dev/ttyUSB1");
        // While the name still has the right ids it stays.
        assert_eq!(resolve_port("/dev/ttyUSB1", "1a86:7523", &ports), "/dev/ttyUSB1");
        // Without a binding the name is used as it is, also when it is not there.
        assert_eq!(resolve_port("/dev/ttyUSB9", "", &ports), "/dev/ttyUSB9");
        assert_eq!(resolve_port("0403:9999", "", &ports), "0403:9999");
        // Nonsense ids are not ids.
        assert_eq!(base_core::machine::parse_usb_id("zz:12"), None);
        assert_eq!(base_core::machine::parse_usb_id("12345:1"), None);
        assert_eq!(base_core::machine::parse_usb_id("0403:6001"), Some((0x0403, 0x6001)));
        assert_eq!(info("COM3", Some((0x1a86, 0x7523))).label(), "COM3 (1a86:7523)");
    }

    /// A stand-in for `ser2net` + GRBL: answers every line with `ok`, every `?` with a status report and a soft reset
    /// with the start-up banner.
    fn fake_grbl(listener: TcpListener, close_after_ms: u64) {
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = s.set_read_timeout(Some(Duration::from_millis(20)));
            let start = Instant::now();
            let mut buf = Vec::new();
            let mut chunk = [0u8; 64];
            while start.elapsed() < Duration::from_millis(close_after_ms) {
                if let Ok(n) = s.read(&mut chunk) {
                    for &b in &chunk[..n] {
                        match b {
                            b'?' => {
                                let _ = s.write_all(b"<Idle|MPos:1.000,2.000,0.000|FS:0,0>\n");
                            }
                            b'\n' => {
                                buf.clear();
                                let _ = s.write_all(b"ok\n");
                            }
                            0x18 => {
                                // Like GRBL after a soft reset.
                                buf.clear();
                                let _ = s.write_all(b"\r\nGrbl 1.1h ['$' for help]\r\n");
                            }
                            _ => buf.push(b),
                        }
                    }
                }
            }
            // Dropping the socket closes the connection.
        });
    }

    fn wait_for(link: &LaserLink, secs: f32, mut pred: impl FnMut(&Evt) -> bool) -> bool {
        let t = Instant::now();
        while t.elapsed().as_secs_f32() < secs {
            for e in link.poll() {
                if pred(&e) {
                    return true;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        false
    }

    #[test]
    fn streams_gcode_over_tcp_like_ser2net() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        fake_grbl(listener, 1500);
        let link = LaserLink::spawn(egui::Context::default());
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port }, controller: Controller::Grbl, streaming: base_core::machine::Streaming::Counting });
        assert!(wait_for(&link, 3.0, |e| matches!(e, Evt::Connected(true))), "connected");
        link.send(Cmd::Job(vec!["G21".into(), "G0 X5 Y5".into(), "M5".into()]));
        let mut done = false;
        assert!(wait_for(&link, 3.0, |e| {
            if let Evt::Progress { done: d, total } = e {
                done = *d == 3 && *total == 3;
            }
            done
        }), "three lines acknowledged");
        // A status report from the poll arrives too.
        assert!(wait_for(&link, 3.0, |e| matches!(e, Evt::Status { x, y, .. } if (*x - 1.0).abs() < 1e-9 && (*y - 2.0).abs() < 1e-9)), "status");
        // The server closes the socket: the link reports the loss.
        assert!(wait_for(&link, 4.0, |e| matches!(e, Evt::Connected(false))), "connection lost detected");
    }

    /// A controller that acknowledges lines slowly (or not at all) and reports how many lines it had in flight.
    fn slow_grbl(listener: TcpListener, ack_delay_ms: u64, silent_from: Option<usize>, state: &'static str) -> std::sync::mpsc::Receiver<usize> {
        let (tx, rx) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let (mut s, _) = listener.accept().unwrap();
            let _ = s.set_read_timeout(Some(Duration::from_millis(10)));
            let start = Instant::now();
            let (mut buf, mut chunk) = (Vec::<u8>::new(), [0u8; 64]);
            let (mut inflight, mut seen, mut max) = (0usize, 0usize, 0usize);
            let mut ack_at: Option<Instant> = None;
            while start.elapsed() < Duration::from_secs(6) {
                let read = s.read(&mut chunk);
                // The worker closed the socket: done.
                if matches!(read, Ok(0)) {
                    break;
                }
                if let Ok(n) = read {
                    for &b in &chunk[..n] {
                        match b {
                            b'?' => {
                                let _ = s.write_all(format!("<{state}|MPos:0.000,0.000,0.000|FS:0,0>\n").as_bytes());
                            }
                            0x18 => {
                                let _ = s.write_all(b"\r\nGrbl 1.1h ['$' for help]\r\n");
                            }
                            b'\n' => {
                                buf.clear();
                                seen += 1;
                                inflight += 1;
                                max = max.max(inflight);
                                if ack_at.is_none() {
                                    ack_at = Some(Instant::now() + Duration::from_millis(ack_delay_ms));
                                }
                            }
                            _ => buf.push(b),
                        }
                    }
                }
                // Acknowledge the oldest line when its time has come, unless the controller has gone quiet.
                if inflight > 0 && ack_at.is_some_and(|t| Instant::now() >= t) && silent_from.map_or(true, |n| seen <= n) {
                    inflight -= 1;
                    let _ = s.write_all(b"ok\n");
                    ack_at = (inflight > 0).then(|| Instant::now() + Duration::from_millis(ack_delay_ms));
                }
            }
            let _ = tx.send(max);
        });
        rx
    }

    fn run_job_on(streaming: base_core::machine::Streaming, lines: usize, ack_ms: u64) -> usize {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let max = slow_grbl(listener, ack_ms, None, "Idle");
        let link = LaserLink::spawn(egui::Context::default());
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port }, controller: Controller::Grbl, streaming });
        assert!(wait_for(&link, 3.0, |e| matches!(e, Evt::Connected(true))));
        link.send(Cmd::Job((0..lines).map(|i| format!("G1 X{i}")).collect()));
        let mut finished = false;
        assert!(wait_for(&link, 6.0, |e| {
            if let Evt::Progress { done, total } = e {
                finished = *done == lines && *total == lines;
            }
            finished
        }), "the job finishes");
        link.send(Cmd::Disconnect);
        max.recv_timeout(Duration::from_secs(8)).unwrap()
    }

    #[test]
    fn counting_keeps_several_lines_in_flight_and_simple_keeps_one() {
        assert!(run_job_on(base_core::machine::Streaming::Counting, 6, 40) > 1, "counting fills the buffer");
        assert_eq!(run_job_on(base_core::machine::Streaming::Simple, 6, 40), 1, "simple waits for every ok");
    }

    #[test]
    fn a_job_that_waits_with_the_machine_standing_still_is_reported_once() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let _max = slow_grbl(listener, 5, Some(2), "Idle");
        let link = LaserLink::spawn_with(egui::Context::default(), Duration::from_millis(400));
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port }, controller: Controller::Grbl, streaming: base_core::machine::Streaming::Counting });
        assert!(wait_for(&link, 3.0, |e| matches!(e, Evt::Connected(true))));
        // The third and later lines are never acknowledged.
        link.send(Cmd::Job((0..6).map(|i| format!("G1 X{i}")).collect()));
        let mut stalled = 0;
        let mut state = String::new();
        wait_for(&link, 3.0, |e| {
            if let Evt::Stalled { state: s, .. } = e {
                stalled += 1;
                state = s.clone();
            }
            false
        });
        assert_eq!(stalled, 1, "reported exactly once");
        assert_eq!(state, "Idle");
    }

    #[test]
    fn a_machine_that_is_moving_is_not_called_stalled() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        let _max = slow_grbl(listener, 5, Some(2), "Run");
        let link = LaserLink::spawn_with(egui::Context::default(), Duration::from_millis(400));
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port }, controller: Controller::Grbl, streaming: base_core::machine::Streaming::Counting });
        assert!(wait_for(&link, 3.0, |e| matches!(e, Evt::Connected(true))));
        link.send(Cmd::Job((0..6).map(|i| format!("G1 X{i}")).collect()));
        assert!(!wait_for(&link, 2.0, |e| matches!(e, Evt::Stalled { .. })));
    }

    #[test]
    fn a_job_stops_at_a_pause_lets_immediate_lines_through_and_goes_on_when_told() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        fake_grbl(listener, 4000);
        let link = LaserLink::spawn(egui::Context::default());
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port }, controller: Controller::Grbl, streaming: base_core::machine::Streaming::Counting });
        assert!(wait_for(&link, 3.0, |e| matches!(e, Evt::Connected(true))));
        link.send(Cmd::Barriers(true));
        link.send(Cmd::Job(vec!["G0 X1".into(), "M0 (Change the bit to: V-bit)".into(), "G0 X2".into(), "G0 X3".into()]));
        let mut barrier = None;
        assert!(wait_for(&link, 3.0, |e| {
            if let Evt::Barrier { text, line } = e {
                barrier = Some((text.clone(), *line));
            }
            barrier.is_some()
        }), "the job stops at M0");
        assert_eq!(barrier, Some(("M0 (Change the bit to: V-bit)".to_string(), 1)));
        // While it waits, an immediate line is answered and the job does not move on.
        link.send(Cmd::Line("G10 L20 P1 Z0".into()));
        let mut oks_after = 0;
        let mut progressed = false;
        wait_for(&link, 1.0, |e| {
            match e {
                Evt::Traffic(l) if l.dir == Dir::Tx && l.text == "G10 L20 P1 Z0" => oks_after += 1,
                Evt::Progress { done, .. } if *done > 2 => progressed = true,
                _ => {}
            }
            false
        });
        assert_eq!(oks_after, 1, "the immediate line was sent");
        assert!(!progressed, "the job waits");
        link.send(Cmd::Continue);
        let mut finished = false;
        assert!(wait_for(&link, 3.0, |e| {
            if let Evt::Progress { done, total } = e {
                finished = *done == 4 && *total == 4;
            }
            finished
        }), "the job goes on after Continue");
    }

    #[test]
    fn pauses_are_sent_to_the_controller_when_barriers_are_off() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = listener.local_addr().unwrap().port();
        fake_grbl(listener, 2500);
        let link = LaserLink::spawn(egui::Context::default());
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port }, controller: Controller::Grbl, streaming: base_core::machine::Streaming::Counting });
        assert!(wait_for(&link, 3.0, |e| matches!(e, Evt::Connected(true))));
        link.send(Cmd::Job(vec!["G0 X1".into(), "M0".into(), "G0 X2".into()]));
        let mut finished = false;
        let mut barrier = false;
        assert!(wait_for(&link, 3.0, |e| {
            match e {
                Evt::Barrier { .. } => barrier = true,
                Evt::Progress { done, total } => finished = *done == 3 && *total == 3,
                _ => {}
            }
            finished
        }));
        assert!(!barrier);
    }

    #[test]
    fn unreachable_host_reports_an_error() {
        let link = LaserLink::spawn(egui::Context::default());
        // Port 1 on localhost is closed.
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port: 1 }, controller: Controller::Grbl, streaming: base_core::machine::Streaming::Counting });
        assert!(wait_for(&link, 5.0, |e| matches!(e, Evt::Log(s) if s.contains("127.0.0.1:1"))));
    }
}
