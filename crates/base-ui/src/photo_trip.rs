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
