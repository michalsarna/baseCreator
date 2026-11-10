//! A photo of a time-lapse with the head moved aside, while a job runs: the job stops at a line where it can go on
//! (`base_core::photo_park`), the head goes to the park position, the machine stands still for a moment, the camera takes the
//! picture, the head comes back and the job goes on. The same in every program that sends jobs; the program hands the
//! commands to its link and takes the picture.
use crate::link::Cmd;
use base_core::photo_park::{stop_points, trip, ParkSettings, StopKind};
use std::time::{Duration, Instant};

/// How the machine of a program makes the trip.
#[derive(Clone, Debug, PartialEq)]
pub struct TripSetup {
    pub park: ParkSettings,
    pub kind: StopKind,
    /// mm above the work zero.
    pub safe_z: f64,
    /// Seconds the spindle needs to come to speed again.
    pub spin_up: f64,
    /// The spindle (or the beam) is stopped for the trip: a laser, a mill that does not keep it running. A pen: false.
    pub spindle_off: bool,
    /// An HPGL plotter with its own commands for the way aside.
    pub hpgl_park: bool,
    /// A picture at every stop point (every lift), not only when one is due.
    pub every_lift: bool,
}

#[derive(Clone, Debug, PartialEq)]
enum Phase {
    /// The job runs; `armed`: it stops at its next stop point.
    Running { armed: bool },
    /// The head is on its way aside; the lines that bring it back.
    Away { back: Vec<String> },
    /// The head stands aside; the picture is taken from a picture of the camera that came after `from`.
    Settling { from: Instant, back: Vec<String> },
}

/// Longest wait for a new picture of the camera once the machine stands: then the last one is taken.
const FRAME_WAIT: Duration = Duration::from_secs(5);

/// One job with photos taken aside.
#[derive(Clone, Debug)]
pub struct PhotoTrip {
    setup: TripSetup,
    lines: Vec<String>,
    phase: Phase,
    /// Pictures taken on this job.
    pub shots: usize,
}

impl PhotoTrip {
    /// A job starts: the trip and the commands for the link (where the job may stop, sent after its `Job`).
    pub fn start(lines: &[String], setup: TripSetup) -> (PhotoTrip, Vec<Cmd>) {
        let stops = stop_points(lines, setup.kind, setup.safe_z);
        let armed = setup.every_lift;
        let cmds = vec![Cmd::PhotoStops(stops), Cmd::PhotoArm(armed)];
        (PhotoTrip { setup, lines: lines.to_vec(), phase: Phase::Running { armed }, shots: 0 }, cmds)
    }

    /// The head is away from the job (on its way, standing aside).
    pub fn away(&self) -> bool {
        !matches!(self.phase, Phase::Running { .. })
    }

    /// A picture is due: the job stops at its next stop point.
    pub fn request(&mut self) -> Vec<Cmd> {
        match &mut self.phase {
            Phase::Running { armed } if !*armed => {
                *armed = true;
                vec![Cmd::PhotoArm(true)]
            }
            _ => vec![],
        }
    }

    /// The job stopped for the photo before its line `line`: the way aside.
    pub fn stopped(&mut self, line: usize) -> Vec<Cmd> {
        let s = &self.setup;
        match trip(&self.lines, line, &s.park, s.safe_z, s.spin_up, s.spindle_off) {
            Ok(t) => {
                let mut cmds: Vec<Cmd> = if s.hpgl_park { vec![Cmd::PlotterPark] } else { t.away.into_iter().map(Cmd::Line).collect() };
                cmds.push(Cmd::PhotoWait);
                self.phase = Phase::Away { back: t.back };
                cmds
            }
            // The stop points are lines where the job can go on; should one not be, the job goes on without the picture.
            Err(_) => self.go_on(vec![]),
        }
    }

    /// The machine stands aside.
    pub fn ready(&mut self, now: Instant) {
        if let Phase::Away { back } = &mut self.phase {
            let back = std::mem::take(back);
            self.phase = Phase::Settling { from: now + Duration::from_secs_f64(self.setup.park.settle), back };
        }
    }

    /// Every frame: `frame_at` is when the camera's latest picture came. Returns the commands, and whether to take the picture
    /// now (before the commands are sent).
    pub fn tick(&mut self, now: Instant, frame_at: Option<Instant>) -> (Vec<Cmd>, bool) {
        let Phase::Settling { from, back } = &mut self.phase else { return (vec![], false) };
        let from = *from;
        let fresh = frame_at.is_some_and(|t| t >= from);
        if now < from || !(fresh || now >= from + FRAME_WAIT) {
            return (vec![], false);
        }
        let back = std::mem::take(back);
        self.shots += 1;
        (self.go_on(back), true)
    }

    /// A picture at every stop point, or only when one is due (a time-lapse in the "after every lift" mode runs, or not).
    pub fn set_every_lift(&mut self, on: bool) -> Vec<Cmd> {
        if self.setup.every_lift == on {
            return vec![];
        }
        self.setup.every_lift = on;
        match &mut self.phase {
            Phase::Running { armed } if *armed != on => {
                *armed = on;
                vec![Cmd::PhotoArm(on)]
            }
            _ => vec![],
        }
    }

    fn go_on(&mut self, back: Vec<String>) -> Vec<Cmd> {
        let mut cmds: Vec<Cmd> = back.into_iter().map(Cmd::Line).collect();
        cmds.push(Cmd::PhotoContinue);
        let armed = self.setup.every_lift;
        if armed {
            cmds.push(Cmd::PhotoArm(true));
        }
        self.phase = Phase::Running { armed };
        cmds
    }
}

/// What a program keeps for the photos aside of the job it sends: the trip of the job, when the device moves aside for
/// photos. `send` hands a command to the link (`Machine::photo_cmd`, or the link itself).
#[derive(Clone, Debug, Default)]
pub struct Trips {
    pub trip: Option<PhotoTrip>,
}

impl Trips {
    /// A job starts with these lines (all of them, as sent). `setup` is `None` for a job without photos aside (a check run).
    pub fn start(&mut self, lines: &[String], setup: Option<TripSetup>, send: &mut dyn FnMut(Cmd)) {
        self.trip = setup.filter(|s| s.park.enabled).map(|s| {
            let (t, cmds) = PhotoTrip::start(lines, s);
            cmds.into_iter().for_each(&mut *send);
            t
        });
    }

    /// The job ended or was stopped.
    pub fn stop(&mut self) {
        self.trip = None;
    }

    /// `Evt::PhotoStop` / `Notice::PhotoStop`.
    pub fn stopped(&mut self, line: usize, send: &mut dyn FnMut(Cmd)) {
        match &mut self.trip {
            Some(t) => t.stopped(line).into_iter().for_each(send),
            // No trip (the job was started again in between): just go on.
            None => send(Cmd::PhotoContinue),
        }
    }

    /// `Evt::PhotoReady` / `Notice::PhotoReady`.
    pub fn ready(&mut self, now: Instant) {
        if let Some(t) = &mut self.trip {
            t.ready(now);
        }
    }

    /// Every frame, for one camera: see `frame_all`.
    pub fn frame(&mut self, rec: &mut crate::camera_rec::Recorder, now: Instant, send: &mut dyn FnMut(Cmd)) {
        self.frame_all(vec![rec], now, send);
    }

    /// Every frame, for the recorders of all the cameras: a picture due on any of them stops the job at its next stop point; once the
    /// head stands aside, every camera whose time-lapse runs takes its picture (after a picture of each that came after the wait).
    pub fn frame_all(&mut self, mut recs: Vec<&mut crate::camera_rec::Recorder>, now: Instant, send: &mut dyn FnMut(Cmd)) {
        for r in recs.iter_mut() {
            r.parked = self.trip.is_some();
        }
        let Some(t) = &mut self.trip else {
            // Asked for while a job ran that has ended since (its last layer): taken where the head is.
            for r in recs.iter_mut() {
                if r.take_trip_request() {
                    r.trip_shot(now);
                }
            }
            return;
        };
        let every = recs.iter().any(|r| r.lapse_shots().is_some() && r.settings.mode == crate::camera_rec::LapseMode::EveryLift);
        t.set_every_lift(every).into_iter().for_each(&mut *send);
        let mut wanted = false;
        for r in recs.iter_mut() {
            wanted |= r.take_trip_request();
        }
        if wanted {
            t.request().into_iter().for_each(&mut *send);
        }
        // The oldest of the newest pictures of the cameras that take one; none of them: nothing to wait for.
        let lapsing: Vec<Option<Instant>> = recs.iter().filter(|r| r.lapsing()).map(|r| r.frame_at()).collect();
        let frame_at = if lapsing.is_empty() { Some(now) } else if lapsing.iter().any(Option::is_none) { None } else { lapsing.into_iter().flatten().min() };
        let (cmds, shoot) = t.tick(now, frame_at);
        if shoot {
            for r in recs.iter_mut().filter(|r| r.lapsing()) {
                r.trip_shot(now);
            }
        }
        cmds.into_iter().for_each(send);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(c: &[Cmd]) -> Vec<String> {
        c.iter()
            .map(|c| match c {
                Cmd::Line(l) => l.clone(),
                Cmd::PhotoStops(s) => format!("stops {}", s.iter().filter(|b| **b).count()),
                Cmd::PhotoArm(a) => format!("arm {a}"),
                Cmd::PhotoWait => "wait".into(),
                Cmd::PlotterPark => "plotter park".into(),
                Cmd::PhotoContinue => "continue".into(),
                _ => "?".into(),
            })
            .collect()
    }

    fn setup(every_lift: bool, hpgl_park: bool) -> TripSetup {
        TripSetup { park: ParkSettings { enabled: true, x: -2.0, y: -3.0, settle: 1.0, keep_spindle: false }, kind: StopKind::Lift, safe_z: 5.0, spin_up: 0.0, spindle_off: true, hpgl_park, every_lift }
    }

    const MILL: &str = "G21 G90\nG0 Z5\nG0 X10 Y10\nS9000 M3\nF300\nG1 Z-1\nG1 X50\nG0 Z5\nG0 X60 Y10\nG1 Z-1\nG1 X70\nG0 Z5\nG0 X0 Y0\nM5";

    fn prog() -> Vec<String> {
        MILL.lines().map(String::from).collect()
    }

    #[test]
    fn a_due_picture_arms_the_job_the_trip_goes_aside_waits_for_a_new_picture_and_comes_back() {
        let (mut t, cmds) = PhotoTrip::start(&prog(), setup(false, false));
        assert_eq!(names(&cmds), ["stops 2", "arm false"]);
        assert_eq!(names(&t.request()), ["arm true"]);
        assert!(t.request().is_empty(), "armed once");
        assert_eq!(names(&t.stopped(8)), ["G21", "G90", "M5", "G53 G0 X-2 Y-3", "wait"]);
        assert!(t.away());
        let now = Instant::now();
        t.ready(now);
        // Still settling, then waiting for a picture newer than the settling.
        assert_eq!(t.tick(now, Some(now)).1, false);
        let later = now + Duration::from_millis(1100);
        assert_eq!(t.tick(later, Some(now)).1, false, "an old picture is not taken");
        let (cmds, shoot) = t.tick(later, Some(later));
        assert!(shoot);
        let n = names(&cmds);
        assert!(n.contains(&"G0 X50 Y10".to_string()) && n.contains(&"S9000 M3".to_string()), "{n:?}");
        assert_eq!(n.last().unwrap(), "continue");
        assert!(!t.away() && t.shots == 1);
        // No new picture at all: after a while the last one is taken anyway.
        t.request();
        t.stopped(12);
        t.ready(now);
        assert!(t.tick(now + Duration::from_secs(7), None).1);
    }

    #[test]
    fn every_lift_follows_the_time_lapse_and_trips_start_only_when_moving_aside_is_on() {
        let (mut t, _) = PhotoTrip::start(&prog(), setup(false, false));
        assert_eq!(names(&t.set_every_lift(true)), ["arm true"]);
        assert!(t.set_every_lift(true).is_empty());
        assert_eq!(names(&t.set_every_lift(false)), ["arm false"]);
        let mut trips = Trips::default();
        let mut sent = vec![];
        trips.start(&prog(), Some(TripSetup { park: ParkSettings::default(), ..setup(false, false) }), &mut |c| sent.push(c));
        assert!(trips.trip.is_none() && sent.is_empty(), "moving aside is off");
        trips.start(&prog(), Some(setup(false, false)), &mut |c| sent.push(c));
        assert_eq!(names(&sent), ["stops 2", "arm false"]);
        // A stop without a trip (a new job in between) just goes on.
        trips.stop();
        sent.clear();
        trips.stopped(8, &mut |c| sent.push(c));
        assert_eq!(names(&sent), ["continue"]);
    }

    #[test]
    fn with_two_cameras_a_picture_due_on_one_parks_the_head_and_both_take_a_new_picture() {
        use crate::camera_rec::{LapseMode, RecSettings, Recorder};
        let dir = std::env::temp_dir().join(format!("trip-two-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let t0 = Instant::now();
        let mk = |sub: &str| {
            let mut r = Recorder::new(RecSettings { dir: Some(dir.join(sub)), interval: 1000.0, mode: LapseMode::PerLayer, format: Default::default() });
            r.feed(t0, 4, 2, vec![50; 24]);
            r.parked = true;
            r.start_lapse(t0);
            r
        };
        let (mut a, mut b) = (mk("a"), mk("b"));
        let mut trips = Trips::default();
        let mut sent = vec![];
        trips.start(&prog(), Some(setup(false, false)), &mut |c| sent.push(c));
        a.layer_done(t0);
        sent.clear();
        trips.frame_all(vec![&mut a, &mut b], t0, &mut |c| sent.push(c));
        assert_eq!(names(&sent), ["arm true"], "one camera asks, the job stops");
        trips.stopped(8, &mut |c| sent.push(c));
        let ready = t0 + Duration::from_millis(10);
        trips.ready(ready);
        // Camera a has a new picture, b not yet: wait.
        let later = ready + Duration::from_millis(1100);
        a.feed(later, 4, 2, vec![60; 24]);
        sent.clear();
        trips.frame_all(vec![&mut a, &mut b], later, &mut |c| sent.push(c));
        assert!(sent.is_empty() && a.lapse_shots() == Some(0));
        b.feed(later, 4, 2, vec![70; 24]);
        trips.frame_all(vec![&mut a, &mut b], later, &mut |c| sent.push(c));
        assert_eq!((a.lapse_shots(), b.lapse_shots()), (Some(1), Some(1)), "both cameras");
        assert_eq!(names(&sent).last().unwrap(), "continue");
        std::fs::remove_dir_all(dir).ok();
    }

    #[test]
    fn every_lift_stays_armed_and_an_hpgl_plotter_goes_aside_with_its_own_commands() {
        let (mut t, cmds) = PhotoTrip::start(&prog(), setup(true, true));
        assert_eq!(names(&cmds), ["stops 2", "arm true"]);
        assert_eq!(names(&t.stopped(8)), ["plotter park", "wait"]);
        let now = Instant::now();
        t.ready(now);
        let (cmds, shoot) = t.tick(now + Duration::from_secs(2), Some(now + Duration::from_secs(2)));
        assert!(shoot);
        assert_eq!(names(&cmds[cmds.len() - 2..]), ["continue", "arm true"]);
        // A stop where the job could not go on: no trip, it just goes on.
        assert_eq!(names(&t.stopped(0)), ["continue", "arm true"]);
    }
}
