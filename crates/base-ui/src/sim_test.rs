//! End-to-end tests of the serial worker (`link`) against the real GRBL 1.1h firmware, compiled for the PC by grbl-sim.
//!
//! The harness (`Sim`, `Session`) is public behind the `sim-tests` feature, so that every program can test its own jobs
//! against the simulator; the tests that do not depend on a program (streaming, feed hold, abort, settings, pauses) are
//! here. They are `#[ignore]`d because they need the simulator binary. Build it once and run them with:
//!
//! ```sh
//! tools/grbl-sim/build.sh
//! cargo test -p base-ui --features sim-tests sim_test -- --ignored --test-threads=1
//! ```
//!
//! A program uses it from its own tests with `base-ui = { …, features = ["sim-tests"] }` in `[dev-dependencies]`; the
//! simulator is looked for in `target/grbl-sim` of the workspace (build it with the `grbl-sim` action of baseCreator in
//! CI) or at `GRBL_SIM_EXE`. See docs/grbl-sim.md.
use crate::link::{Cmd, Dir, Evt, LaserLink, Target};
use base_core::machine::{Controller, Streaming};
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

/// The bridge that exposes the simulator on a TCP port (see `tools/grbl-sim/serve.py`).
const SERVE_PY: &str = include_str!("../../../tools/grbl-sim/serve.py");

/// Where the simulator binary is: `GRBL_SIM_EXE`, else `target/grbl-sim/…` of the workspace of the test (found from the
/// directory of the crate being tested, upwards).
fn sim_exe() -> PathBuf {
    if let Some(p) = std::env::var_os("GRBL_SIM_EXE").or_else(|| std::env::var_os("LC_GRBL_SIM_EXE")) {
        return PathBuf::from(p);
    }
    let rel = "target/grbl-sim/grbl/grbl/sim/grbl_sim.exe";
    let start = std::env::var_os("CARGO_MANIFEST_DIR").map(PathBuf::from).unwrap_or_else(|| std::env::current_dir().unwrap());
    start.ancestors().map(|d| d.join(rel)).find(|p| p.exists()).unwrap_or_else(|| start.join(rel))
}

static COUNT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// `serve.py` on a free port with a fresh simulator behind it per connection, killed when dropped.
pub struct Sim {
    child: Child,
    pub port: u16,
    script: PathBuf,
}

impl Sim {
    pub fn start() -> Sim {
        let exe = sim_exe();
        assert!(exe.exists(), "grbl-sim not found at {}; run tools/grbl-sim/build.sh first (or set GRBL_SIM_EXE)", exe.display());
        let script = std::env::temp_dir().join(format!("grbl-sim-serve-{}-{}.py", std::process::id(), COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        std::fs::write(&script, SERVE_PY).expect("write serve.py");
        let mut child = Command::new("python3")
            .arg(&script)
            .args(["--port", "0", "--sim"])
            .arg(&exe)
            .stdout(Stdio::piped())
            .spawn()
            .expect("python3 is needed to run tools/grbl-sim/serve.py");
        let mut line = String::new();
        BufReader::new(child.stdout.take().unwrap()).read_line(&mut line).unwrap();
        let port = line.trim().rsplit(':').next().and_then(|p| p.parse().ok()).unwrap_or_else(|| panic!("unexpected bridge output: {line:?}"));
        Sim { child, port, script }
    }
}

impl Drop for Sim {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.script);
    }
}

/// Everything the worker reported so far, collected from its event channel.
#[derive(Default)]
pub struct Seen {
    pub connected: Option<bool>,
    pub rx: Vec<String>,
    /// The lines the worker sent (not the status polls), in the order of its replies: reply n answers line n.
    pub tx: Vec<String>,
    pub state: String,
    pub pos: (f64, f64),
    pub progress: (usize, usize),
    /// The pauses the job stopped at (`Evt::Barrier`): the line and its number.
    pub barriers: Vec<(String, usize)>,
    /// The last status reports and progress changes with the time they came, to see how the machine got where it is.
    pub trail: Vec<String>,
    started: Option<Instant>,
    /// Replies before this index are ignored by `errors()` (start-up output, see `wait_banner`).
    pub mark: usize,
    /// The number of lines sent before the banner (see `mark`).
    pub mark_tx: usize,
}

pub struct Session {
    pub link: LaserLink,
    pub seen: Seen,
    _sim: Sim,
}

impl Session {
    /// Connect the worker to a fresh simulator, the same way the applications connect to a ser2net machine.
    pub fn connect() -> Session {
        Self::connect_with(Controller::Grbl, Streaming::Counting)
    }

    pub fn connect_with(controller: Controller, streaming: Streaming) -> Session {
        let sim = Sim::start();
        let link = LaserLink::spawn(egui::Context::default());
        link.send(Cmd::Connect { target: Target::Tcp { host: "127.0.0.1".into(), port: sim.port }, controller, streaming });
        let mut s = Session { link, seen: Seen::default(), _sim: sim };
        s.wait("connected", 5.0, |v| v.connected == Some(true));
        s
    }

    pub fn pump(&mut self) {
        for e in self.link.poll() {
            match e {
                Evt::Connected(c) => self.seen.connected = Some(c),
                Evt::Traffic(l) if l.dir == Dir::Rx && !l.poll => self.seen.rx.push(l.text),
                Evt::Traffic(l) if l.dir == Dir::Tx && !l.poll => self.seen.tx.push(l.text),
                Evt::Barrier { text, line } => self.seen.barriers.push((text, line)),
                Evt::Status { state, x, y, .. } => {
                    let t = self.seen.started.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
                    if self.seen.trail.last().map(|l| !l.ends_with(&format!("{state} {x:.3},{y:.3}"))).unwrap_or(true) {
                        self.seen.trail.push(format!("{t:.1}s {state} {x:.3},{y:.3}"));
                    }
                    self.seen.state = state;
                    self.seen.pos = (x, y);
                }
                Evt::Progress { done, total } => {
                    let t = self.seen.started.get_or_insert_with(Instant::now).elapsed().as_secs_f32();
                    if done + 3 >= total || done % 250 == 0 {
                        self.seen.trail.push(format!("{t:.1}s progress {done}/{total}"));
                    }
                    self.seen.progress = (done, total)
                }
                _ => {}
            }
        }
    }

    /// Wait until `pred` holds. `secs` is the time the wait is expected to take; it is given up when nothing happens for that
    /// long (at most 30 s: no reply, no new state, position or progress), or after four times as long in any case. A slow
    /// runner makes a job take longer, which is not a failure as long as the machine keeps moving.
    pub fn wait(&mut self, what: &str, secs: f32, mut pred: impl FnMut(&Seen) -> bool) {
        let start = Instant::now();
        let (mut news, mut since) = (self.news(), Instant::now());
        loop {
            self.pump();
            if pred(&self.seen) {
                return;
            }
            let now = self.news();
            if now != news {
                (news, since) = (now, Instant::now());
            }
            if since.elapsed().as_secs_f32() >= secs.min(30.0) || start.elapsed().as_secs_f32() >= secs * 4.0 {
                panic!("timed out waiting for {what}; {}", self.report());
            }
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    /// Something that changes whenever the machine or the worker does anything worth waiting for.
    #[allow(clippy::type_complexity)]
    fn news(&self) -> (usize, String, (i64, i64), (usize, usize), usize) {
        let v = &self.seen;
        (v.rx.len(), v.state.clone(), ((v.pos.0 * 100.0) as i64, (v.pos.1 * 100.0) as i64), v.progress, v.barriers.len())
    }

    /// Everything that helps to tell a lost line from a lost reply when a wait fails: the state, the progress, how many
    /// `ok` were seen and every reply that is not a plain `ok` (an `ok` torn by another reply shows up there).
    pub fn report(&self) -> String {
        let oks = self.seen.rx.iter().filter(|l| *l == "ok").count();
        let others: Vec<&String> = self.seen.rx.iter().filter(|l| *l != "ok").take(20).collect();
        format!(
            "state {:?} at {:?}, progress {:?}, {} x ok, other replies {:?}, last replies {:?}, trail {:?}",
            self.seen.state, self.seen.pos, self.seen.progress, oks, others, last(&self.seen.rx, 12), last(&self.seen.trail, 25)
        )
    }

    /// Wait for GRBL's banner after the soft reset the worker sends on connect. Replies up to here are not checked.
    pub fn wait_banner(&mut self) {
        self.wait("GRBL banner", 5.0, |v| v.rx.iter().any(|l| l.starts_with("Grbl ")));
        self.seen.mark = self.seen.rx.len();
        self.seen.mark_tx = self.seen.tx.len();
    }

    /// Fast machine settings so jobs run in a few seconds of (roughly real) simulated time (all three axes), then `extra`
    /// settings of the program (`$32=1` for a laser).
    pub fn fast_machine(&mut self, extra: &[&str]) {
        let lines: Vec<&str> = ["$110=30000", "$111=30000", "$112=30000", "$120=5000", "$121=5000", "$122=5000"].into_iter().chain(extra.iter().copied()).collect();
        let before = self.ok_count();
        for l in &lines {
            self.link.send(Cmd::Line((*l).into()));
        }
        self.wait("settings accepted", 5.0, |v| v.rx.iter().filter(|l| *l == "ok").count() >= before + lines.len());
    }

    pub fn ok_count(&mut self) -> usize {
        self.pump();
        self.seen.rx.iter().filter(|l| *l == "ok").count()
    }

    pub fn run_job(&mut self, lines: Vec<String>, secs: f32) {
        let n = lines.len();
        self.link.send(Cmd::Job(lines));
        // Give up early when nothing moves any more: the machine idle and no new acknowledgement for a while means a
        // line or its reply got lost, and waiting for the whole time limit only hides that.
        let (mut last_done, mut since) = (0, Instant::now());
        self.wait("every job line acknowledged", secs, |v| {
            if v.progress.0 != last_done {
                (last_done, since) = (v.progress.0, Instant::now());
            }
            assert!(!(v.state == "Idle" && v.progress.0 > 0 && since.elapsed().as_secs_f32() > 20.0), "the job stopped at {:?} of {n} lines with the machine idle", v.progress);
            v.progress == (n, n)
        });
    }

    pub fn wait_idle_at(&mut self, x: f64, y: f64, secs: f32) {
        self.wait("machine idle at the end position", secs, |v| v.state == "Idle" && (v.pos.0 - x).abs() < 0.01 && (v.pos.1 - y).abs() < 0.01);
    }

    /// The errors with the line each one answers (the n-th `ok` or `error` answers the n-th line sent): a garbled line
    /// shows up here as what was sent and what GRBL made of it.
    pub fn errors_described(&self) -> Vec<String> {
        let mut n = 0;
        let mut out = vec![];
        for l in &self.seen.rx {
            if l == "ok" || l.starts_with("error") {
                if l.starts_with("error") && n >= self.seen.mark_tx {
                    out.push(format!("{l} for line {n} {:?}, around it {:?}", self.seen.tx.get(n), self.seen.tx.get(n.saturating_sub(2)..(n + 2).min(self.seen.tx.len()))));
                }
                n += 1;
            }
        }
        out
    }

    pub fn errors(&self) -> Vec<&String> {
        self.seen.rx[self.seen.mark..].iter().filter(|l| l.starts_with("error") || l.starts_with("ALARM")).collect()
    }

    /// The settings report of `$$`, as the lines GRBL printed before its `ok`.
    pub fn read_settings(&mut self) -> Vec<String> {
        let from = self.seen.rx.len();
        self.link.send(Cmd::Line("$$".into()));
        // The "ok" that closes the report, not one that answers an earlier line still in flight.
        self.wait("settings report", 5.0, |v| ok_after(&v.rx[from..], "$"));
        self.seen.rx[from..].to_vec()
    }

    /// The parser state (`$G`): `[GC:G0 G54 G17 G21 G90 G94 M5 M9 T0 F0 S0]`.
    pub fn parser_state(&mut self) -> String {
        let from = self.seen.rx.len();
        self.link.send(Cmd::Line("$G".into()));
        self.wait("parser state", 5.0, |v| ok_after(&v.rx[from..], "[GC:"));
        self.seen.rx[from..].iter().find(|l| l.starts_with("[GC:")).cloned().unwrap_or_default()
    }
}

fn last(v: &[String], n: usize) -> &[String] {
    &v[v.len().saturating_sub(n)..]
}

/// Is there an "ok" after a line that starts with `prefix`?
pub fn ok_after(lines: &[String], prefix: &str) -> bool {
    lines.iter().position(|l| l.starts_with(prefix)).is_some_and(|i| lines[i..].iter().any(|l| l == "ok"))
}

/// Many short moves, as a dense fill makes them: the window of the character counting is full all the time.
pub fn dense_job(lines: usize) -> Vec<String> {
    let mut job: Vec<String> = ["G21", "G90", "G0 X0 Y0", "G1 F6000"].iter().map(|l| l.to_string()).collect();
    for i in 0..lines {
        let (row, col) = (i / 40, i % 40);
        let x = if row % 2 == 0 { col } else { 39 - col } as f64 * 0.5;
        job.push(format!("G1 X{x:.3} Y{:.3}", 1.0 + row as f64 * 0.1));
    }
    job.push("G0 X0 Y0".into());
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    use base_core::grbl_settings as gs;

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_keeps_up_with_real_time() {
        // The other tests assume the simulator runs at roughly real speed; this one says so plainly when it does not.
        let mut s = Session::connect();
        s.wait_banner();
        // 10 mm at GRBL's defaults (500 mm/min, 10 mm/s²): 0.83 s up, 0.37 s cruise, 0.83 s down = 2.0 s.
        let t = Instant::now();
        s.run_job(["G21", "G90", "G1 X10 F600"].iter().map(|l| l.to_string()).collect(), 20.0);
        s.wait_idle_at(10.0, 0.0, 60.0);
        let secs = t.elapsed().as_secs_f32();
        eprintln!("grbl-sim: a 2.0 s move took {secs:.1} s");
        assert!(secs < 5.0, "grbl-sim runs {:.1}x slower than real time", secs / 2.0);
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_long_job_keeps_character_counting_in_sync() {
        // Hundreds of short lines: the 120-byte RX window is full all the time. A wrong byte count would overflow GRBL's
        // 128-byte buffer (lost or garbled lines -> errors) or stall the stream.
        let mut s = Session::connect();
        s.wait_banner();
        s.fast_machine(&[]);
        let job = dense_job(1200);
        let n = job.len();
        s.run_job(job, 240.0);
        // Not the end position: over a run of this length the simulator now and then ends a few millimetres away from the
        // target although every line was acknowledged (its two threads share the step counters); the shorter jobs check it.
        s.wait("machine idle after the job", 60.0, |v| v.state == "Idle" && v.progress == (n, n));
        assert!(s.errors().is_empty(), "GRBL rejected lines: {:?}", s.errors_described());
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_rejected_line_does_not_desync_the_stream() {
        let mut s = Session::connect();
        s.wait_banner();
        s.fast_machine(&[]);
        let job: Vec<String> = ["G21", "G90", "G1 X5 Y5 F3000", "G999", "G1 X10 Y10", "G0 X0 Y0"].iter().map(|l| l.to_string()).collect();
        s.run_job(job, 20.0);
        s.wait_idle_at(0.0, 0.0, 20.0);
        // GRBL answers the unsupported command with error:20 and keeps going; the worker counts it as acknowledged.
        assert_eq!(s.errors(), vec!["error:20"]);
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_feed_hold_and_resume() {
        let mut s = Session::connect();
        s.wait_banner();
        // Slow moves so the hold lands mid-job.
        let job: Vec<String> = ["G21", "G90", "G1 X40 Y0 F600", "G1 X40 Y10", "G0 X0 Y0"].iter().map(|l| l.to_string()).collect();
        s.link.send(Cmd::Job(job));
        s.wait("moving", 10.0, |v| v.state == "Run" && v.pos.0 > 1.0);
        s.link.send(Cmd::Realtime(b'!'));
        s.wait("held", 10.0, |v| v.state.starts_with("Hold:0"));
        let held_at = s.seen.pos;
        std::thread::sleep(Duration::from_millis(600));
        s.pump();
        assert!((s.seen.pos.0 - held_at.0).abs() < 1e-6, "machine moved while held");
        s.link.send(Cmd::Realtime(b'~'));
        s.wait("job finished", 60.0, |v| v.progress == (5, 5));
        s.wait_idle_at(0.0, 0.0, 60.0);
        assert!(s.errors().is_empty(), "{:?}", s.errors_described());
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_abort_then_new_job_runs() {
        let mut s = Session::connect();
        s.wait_banner();
        s.fast_machine(&[]);
        s.link.send(Cmd::Job(dense_job(400)));
        s.wait("job started", 20.0, |v| v.progress.0 > 20);
        s.link.send(Cmd::Abort);
        // Real GRBL 1.1 comes back in alarm (ALARM:3, position lost) after a reset while moving and needs $X;
        // grbl-sim comes back idle, where $X is a harmless `ok`.
        s.wait("reset banner", 20.0, |v| v.rx.iter().filter(|l| l.starts_with("Grbl ")).count() >= 2);
        s.seen.mark = s.seen.rx.len();
        s.seen.mark_tx = s.seen.tx.len();
        s.link.send(Cmd::Line("$X".into()));
        s.link.send(Cmd::Line("G10 L20 P1 X0 Y0".into()));
        let job: Vec<String> = ["G21", "G90", "G1 X3 Y4 F3000", "M5"].iter().map(|l| l.to_string()).collect();
        s.run_job(job, 20.0);
        s.wait("idle", 20.0, |v| v.state == "Idle");
        assert!(s.errors().is_empty(), "{:?}", s.errors_described());
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_job_sent_right_after_connect_is_not_lost() {
        // The app lets the user press Start as soon as the link reports "connected", while GRBL is still restarting from
        // the soft reset (0x18) the worker sends on connect and throws away whatever it receives. The worker has to hold
        // the job back until the banner, or the lines are lost and the job never finishes.
        let mut s = Session::connect();
        let job: Vec<String> = ["G21", "G90", "G0 X2 Y3", "M5"].iter().map(|l| l.to_string()).collect();
        s.run_job(job, 15.0);
        s.wait_idle_at(2.0, 3.0, 15.0);
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_job_stops_at_a_pause_and_goes_on_when_told() {
        // A milling job asks for another bit with M0 (a plotter job for another pen): the link finishes what the
        // controller has, tells the program, sends nothing more of the job, and goes on when it is told to.
        let mut s = Session::connect();
        s.wait_banner();
        s.fast_machine(&[]);
        s.link.send(Cmd::Barriers(true));
        let job: Vec<String> = ["G21", "G90", "G1 X5 Y0 F3000", "M0 (Change the bit to: V-bit)", "G1 X10 Y0", "M5"].iter().map(|l| l.to_string()).collect();
        s.link.send(Cmd::Job(job));
        s.wait("the pause", 20.0, |v| !v.barriers.is_empty());
        assert_eq!(s.seen.barriers[0].0, "M0 (Change the bit to: V-bit)");
        s.wait_idle_at(5.0, 0.0, 20.0);
        // It really waits: nothing moves and the job does not go on.
        std::thread::sleep(Duration::from_millis(800));
        s.pump();
        assert!(s.seen.pos.0 < 5.01 && s.seen.progress.0 < s.seen.progress.1, "{:?} {:?}", s.seen.pos, s.seen.progress);
        s.link.send(Cmd::Continue);
        s.wait("job finished", 20.0, |v| v.progress.0 == v.progress.1);
        s.wait_idle_at(10.0, 0.0, 20.0);
        assert!(s.errors().is_empty(), "{:?}", s.errors_described());
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_settings_can_be_read_changed_backed_up_and_restored() {
        let mut s = Session::connect();
        s.wait_banner();
        // The whole report: the settings the page knows are all there.
        let all = gs::parse_report(&s.read_settings());
        for id in [0, 1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 20, 21, 22, 23, 24, 25, 26, 27, 30, 31, 32, 100, 101, 102, 110, 111, 112, 120, 121, 122, 130, 131, 132] {
            assert!(all.contains_key(&id), "${id} is missing from the report: {all:?}");
        }
        // Change one setting and read it back.
        let old = all[&110].clone();
        s.link.send(Cmd::Line(gs::write_line(110, "1234")));
        let changed = gs::parse_report(&s.read_settings());
        assert!(gs::same_value(&changed[&110], "1234"), "{}", changed[&110]);
        // The backup text holds it, reads back strictly, and restoring it brings the old value back.
        let backup = gs::backup_text(&all, &[], "test");
        let file = gs::parse_backup(&backup).expect("the backup is readable");
        for (id, v) in &file {
            s.link.send(Cmd::Line(gs::write_line(*id, v)));
        }
        let restored = gs::parse_report(&s.read_settings());
        assert!(gs::same_value(&restored[&110], &old), "{} vs {old}", restored[&110]);
        assert!(s.errors().is_empty(), "{:?}", s.errors_described());
    }

    #[test]
    #[ignore = "needs grbl-sim: tools/grbl-sim/build.sh"]
    fn grbl_sim_spindle_and_arcs_run_and_end_where_expected() {
        // What a milling job is made of: spindle on with a speed, a dwell, a plunge in Z, an arc, and back up.
        let mut s = Session::connect();
        s.wait_banner();
        s.fast_machine(&[]);
        let job: Vec<String> = ["G21", "G90", "M3 S12000", "G4 P0.2", "G0 X10 Y10", "G1 Z-1 F600", "G2 X10 Y10 I5 J0 F3000", "G0 Z5", "M5", "G0 X0 Y0"].iter().map(|l| l.to_string()).collect();
        s.run_job(job, 60.0);
        s.wait_idle_at(0.0, 0.0, 30.0);
        assert!(s.parser_state().contains("M5"), "the spindle is off");
        assert!(s.errors().is_empty(), "{:?}", s.errors_described());
    }
}
