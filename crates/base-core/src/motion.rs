//! How long a machine takes for a list of straight moves when it speeds up and slows down the way GRBL does: every move
//! has its own acceleration (the slowest of the axes it uses), a corner is taken at the speed the junction deviation
//! allows, the speeds are planned backwards (to be able to stop) and forwards (from where the machine can get to), and only
//! the next [`LOOKAHEAD`] moves are known when one starts, so a long chain of tiny moves cannot go faster than the machine
//! can stop within them. Without the accelerations of the machine every move simply takes its length at its feed.
use serde::{Deserialize, Serialize};

/// The junction deviation of GRBL when the device does not say (`$11`), mm.
pub const GRBL_JUNCTION: f64 = 0.01;
/// The moves GRBL plans ahead (its block buffer of 16 on an Arduino, one of them being run).
pub const LOOKAHEAD: usize = 15;

/// What the machine does when it changes speed. Kept with a device; all zero is "not known".
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    /// The acceleration of X, Y and Z (mm/s², `$120` – `$122`); 0 = not known.
    #[serde(default)]
    pub accel: [f64; 3],
    /// How far a corner may be cut (mm, `$11`); 0 = GRBL's 0.01 mm.
    #[serde(default)]
    pub junction: f64,
}

impl Motion {
    /// The same acceleration on every axis (a laser that only knows one).
    pub fn flat(accel: f64) -> Motion {
        Motion { accel: [accel; 3], junction: 0.0 }
    }

    /// Is enough known to count the speeding up and slowing down (X and Y)?
    pub fn known(&self) -> bool {
        self.accel[0] > 0.0 && self.accel[1] > 0.0
    }

    pub fn is_unset(&self) -> bool {
        *self == Motion::default()
    }

    pub fn junction_mm(&self) -> f64 {
        if self.junction > 0.0 { self.junction } else { GRBL_JUNCTION }
    }

    /// Are the numbers usable (from a file)?
    pub fn check(&self) -> Result<(), String> {
        let ok = |v: f64, hi: f64| v.is_finite() && (0.0..=hi).contains(&v);
        if !self.accel.iter().all(|a| ok(*a, 100_000.0)) || !ok(self.junction, 10.0) {
            return Err("the acceleration or the junction deviation is out of range".into());
        }
        Ok(())
    }

    /// Take what the controller reports (`$120` – `$122`, `$11`).
    pub fn take(&mut self, s: &crate::firmware::Suggestion) {
        if let Some(a) = s.accel {
            self.accel = a.map(|v| v.clamp(0.0, 100_000.0));
        }
        if let Some(j) = s.junction {
            self.junction = j.clamp(0.0, 10.0);
        }
    }

    /// The acceleration of an axis, the smallest known one standing in for one that is not known.
    fn axis(&self, i: usize) -> f64 {
        if self.accel[i] > 0.0 {
            return self.accel[i];
        }
        self.accel.iter().copied().filter(|a| *a > 0.0).fold(f64::INFINITY, f64::min)
    }
}

/// A straight move to `to` at `feed` (mm/min). `stop`: the machine comes to rest before it (a pause, the spindle or the pen
/// changing, a dwell).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Block {
    pub to: [f64; 3],
    pub feed: f64,
    pub stop: bool,
}

impl Block {
    pub fn new(to: [f64; 3], feed: f64) -> Block {
        Block { to, feed, stop: false }
    }
}

struct Plan {
    /// The block this is.
    at: usize,
    len: f64,
    u: [f64; 3],
    /// The speed asked for (mm/s), squared.
    v2: f64,
    /// The acceleration along the move (mm/s²); infinite when nothing is known.
    a: f64,
    /// The fastest the move may start, squared.
    entry2: f64,
}

/// The seconds the moves take from `start`, at rest at the start and the end.
pub fn seconds(start: [f64; 3], blocks: &[Block], m: &Motion) -> f64 {
    durations(start, blocks, m).iter().sum()
}

/// The seconds of every move (as many as `blocks`), planned with GRBL's lookahead.
pub fn durations(start: [f64; 3], blocks: &[Block], m: &Motion) -> Vec<f64> {
    durations_with(start, blocks, m, LOOKAHEAD)
}

/// The seconds of every move for a controller that plans `lookahead` moves ahead (at least 2).
pub fn durations_with(start: [f64; 3], blocks: &[Block], m: &Motion, lookahead: usize) -> Vec<f64> {
    let lookahead = lookahead.max(2);
    let mut out = vec![0.0; blocks.len()];
    if !m.known() {
        let mut at = start;
        for (b, t) in blocks.iter().zip(out.iter_mut()) {
            let len = dist(at, b.to);
            if b.feed > 0.0 {
                *t = len / (b.feed / 60.0);
            }
            at = b.to;
        }
        return out;
    }
    let jd = m.junction_mm();
    let mut ps: Vec<Plan> = Vec::with_capacity(blocks.len());
    let mut at = start;
    let mut stop = true;
    for (i, b) in blocks.iter().enumerate() {
        let len = dist(at, b.to);
        stop |= b.stop || !(b.feed > 0.0);
        if len < 1e-9 || !(b.feed > 0.0) {
            at = b.to;
            continue;
        }
        let u = [(b.to[0] - at[0]) / len, (b.to[1] - at[1]) / len, (b.to[2] - at[2]) / len];
        let a = (0..3).filter(|&i| u[i].abs() > 1e-9).map(|i| m.axis(i) / u[i].abs()).fold(f64::INFINITY, f64::min);
        let v = b.feed / 60.0;
        let entry2 = match ps.last() {
            Some(p) if !stop => {
                let cos = -(p.u[0] * u[0] + p.u[1] * u[1] + p.u[2] * u[2]);
                let vj2 = if cos > 0.999_999 {
                    0.0
                } else if cos < -0.999_999 {
                    f64::INFINITY
                } else {
                    let sin_half = (0.5 * (1.0 - cos)).sqrt();
                    p.a.min(a) * jd * sin_half / (1.0 - sin_half)
                };
                vj2.min(v * v).min(p.v2)
            }
            _ => 0.0,
        };
        ps.push(Plan { at: i, len, u, v2: v * v, a, entry2 });
        at = b.to;
        stop = false;
    }
    let n = ps.len();
    if n == 0 {
        return out;
    }
    // Only the next moves are known: every move must be able to stop by the end of the ones planned with it.
    let mut window: f64 = ps.iter().take(lookahead - 1).map(|p| p.len).sum();
    for k in 0..n {
        let reach = 2.0 * ps[k].a * window;
        if reach < ps[k].entry2 {
            ps[k].entry2 = reach;
        }
        window -= ps[k].len;
        if let Some(p) = ps.get(k + lookahead - 1) {
            window += p.len;
        }
    }
    // Backwards: slow enough to stop at the end (and to make every next start).
    let mut exit2 = 0.0;
    for p in ps.iter_mut().rev() {
        p.entry2 = p.entry2.min(exit2 + 2.0 * p.a * p.len);
        exit2 = p.entry2;
    }
    // Forwards: no faster than the machine can get from the start of the move before.
    for k in 1..n {
        let reach = ps[k - 1].entry2 + 2.0 * ps[k - 1].a * ps[k - 1].len;
        if reach < ps[k].entry2 {
            ps[k].entry2 = reach;
        }
    }
    for k in 0..n {
        let p = &ps[k];
        let v0 = p.entry2.sqrt();
        let v1 = ps.get(k + 1).map_or(0.0, |q| q.entry2).sqrt();
        out[p.at] = move_time(p.len, v0, v1, p.v2.sqrt(), p.a);
    }
    out
}

/// The seconds of one move of `len` mm that starts at `v0`, ends at `v1`, may go up to `vmax` (mm/s) and speeds up and slows
/// down at `a` (mm/s²): a trapezoid, or a triangle when the move is too short to reach `vmax`.
pub fn move_time(len: f64, v0: f64, v1: f64, vmax: f64, a: f64) -> f64 {
    if !(vmax > 0.0) {
        return 0.0;
    }
    if !a.is_finite() || a <= 0.0 {
        return len / vmax;
    }
    let (v0, v1) = (v0.min(vmax), v1.min(vmax));
    let up = (vmax * vmax - v0 * v0) / (2.0 * a);
    let down = (vmax * vmax - v1 * v1) / (2.0 * a);
    if up + down <= len {
        (vmax - v0) / a + (vmax - v1) / a + (len - up - down) / vmax
    } else {
        let peak = ((2.0 * a * len + v0 * v0 + v1 * v1) / 2.0).sqrt().max(v0).max(v1);
        (peak - v0) / a + (peak - v1) / a
    }
}

fn dist(a: [f64; 3], b: [f64; 3]) -> f64 {
    ((b[0] - a[0]).powi(2) + (b[1] - a[1]).powi(2) + (b[2] - a[2]).powi(2)).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn line(to: [f64; 3], feed: f64) -> Block {
        Block::new(to, feed)
    }

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn without_the_accelerations_every_move_is_its_length_at_its_feed() {
        let t = seconds([0.0; 3], &[line([100.0, 0.0, 0.0], 600.0), line([100.0, 50.0, 0.0], 3000.0)], &Motion::default());
        assert!(close(t, 10.0 + 1.0, 1e-9), "{t}");
    }

    #[test]
    fn a_move_from_rest_to_rest_as_grbl_runs_it() {
        // 10 mm at 600 mm/min with 10 mm/s²: 1 s up, 1 s down would be 10 mm already, so 0.83 + 0.37 + 0.83 s ≈ 2.0 s
        // (what the grbl-sim test measures).
        let t = seconds([0.0; 3], &[line([10.0, 0.0, 0.0], 600.0)], &Motion::flat(10.0));
        assert!(close(t, 2.0, 0.01), "{t}");
        // A long line: the cruise plus the two ramps.
        let t = seconds([0.0; 3], &[line([100.0, 0.0, 0.0], 600.0)], &Motion::flat(500.0));
        assert!(close(t, 10.0 + 0.02, 1e-6), "{t}");
    }

    #[test]
    fn many_tiny_moves_in_a_line_take_as_long_as_one_line() {
        let m = Motion::flat(500.0);
        let one = seconds([0.0; 3], &[line([100.0, 0.0, 0.0], 600.0)], &m);
        let many: Vec<Block> = (1..=1000).map(|i| line([i as f64 * 0.1, 0.0, 0.0], 600.0)).collect();
        let t = seconds([0.0; 3], &many, &m);
        assert!(close(t, one, one * 0.01), "{t} vs {one}");
        // Counted from rest to rest every one of them would take much longer.
        let rest: f64 = (0..1000).map(|_| move_time(0.1, 0.0, 0.0, 10.0, 500.0)).sum();
        assert!(rest > 2.0 * t, "{rest} vs {t}");
    }

    #[test]
    fn sharp_corners_slow_the_machine_down() {
        let m = Motion::flat(200.0);
        let straight: Vec<Block> = (1..=20).map(|i| line([i as f64 * 5.0, 0.0, 0.0], 3000.0)).collect();
        let zigzag: Vec<Block> = (1..=20).map(|i| line([i as f64 * 5.0 / 2f64.sqrt(), if i % 2 == 1 { 5.0 / 2f64.sqrt() } else { 0.0 }, 0.0], 3000.0)).collect();
        let (a, b) = (seconds([0.0; 3], &straight, &m), seconds([0.0; 3], &zigzag, &m));
        assert!(b > a * 1.3, "zigzag {b} vs straight {a}");
        // A turn back stops the machine.
        let back = seconds([0.0; 3], &[line([10.0, 0.0, 0.0], 3000.0), line([0.0, 0.0, 0.0], 3000.0)], &m);
        let two = 2.0 * seconds([0.0; 3], &[line([10.0, 0.0, 0.0], 3000.0)], &m);
        assert!(close(back, two, 1e-9));
    }

    #[test]
    fn a_short_move_never_reaches_its_feed() {
        // 1 mm at 6000 mm/min with 100 mm/s²: up to 10 mm/s at the middle and down again, 0.2 s (not 0.01 s).
        let t = seconds([0.0; 3], &[line([1.0, 0.0, 0.0], 6000.0)], &Motion::flat(100.0));
        assert!(close(t, 0.2, 1e-6), "{t}");
    }

    #[test]
    fn a_stop_brings_the_machine_to_rest() {
        let m = Motion::flat(50.0);
        let go = seconds([0.0; 3], &[line([50.0, 0.0, 0.0], 3000.0), line([100.0, 0.0, 0.0], 3000.0)], &m);
        let stop = seconds([0.0; 3], &[line([50.0, 0.0, 0.0], 3000.0), Block { to: [100.0, 0.0, 0.0], feed: 3000.0, stop: true }], &m);
        assert!(stop > go + 0.5, "{stop} vs {go}");
    }

    #[test]
    fn the_short_lookahead_holds_back_a_chain_of_tiny_fast_moves() {
        // 0.1 mm moves at 6000 mm/min with 500 mm/s²: GRBL sees 1.5 mm ahead, so it goes about sqrt(2 · 500 · 1.4) ≈ 37 mm/s.
        let m = Motion::flat(500.0);
        let many: Vec<Block> = (1..=1000).map(|i| line([i as f64 * 0.1, 0.0, 0.0], 6000.0)).collect();
        let t = seconds([0.0; 3], &many, &m);
        assert!(t > 2.4 && t < 3.0, "{t}");
        // A controller that plans far ahead runs them at the feed.
        let far: f64 = durations_with([0.0; 3], &many, &m, 100_000).iter().sum();
        assert!(far < 1.3, "{far}");
    }

    #[test]
    fn every_move_has_its_own_time() {
        let m = Motion::flat(100.0);
        let b = [line([10.0, 0.0, 0.0], 600.0), line([10.0, 0.0, 0.0], 600.0), Block { to: [20.0, 0.0, 0.0], feed: 0.0, stop: false }, line([30.0, 0.0, 0.0], 600.0)];
        let d = durations([0.0; 3], &b, &m);
        assert_eq!(d.len(), 4);
        assert!(d[0] > 0.0 && d[1] == 0.0 && d[2] == 0.0 && d[3] > 0.0, "{d:?}");
        assert!((d.iter().sum::<f64>() - seconds([0.0; 3], &b, &m)).abs() < 1e-12);
    }

    #[test]
    fn a_slow_z_makes_plunges_take_longer() {
        let fast = seconds([0.0; 3], &[line([0.0, 0.0, -5.0], 600.0)], &Motion::flat(500.0));
        let slow = seconds([0.0; 3], &[line([0.0, 0.0, -5.0], 600.0)], &Motion { accel: [500.0, 500.0, 5.0], junction: 0.0 });
        assert!(slow > fast + 1.0, "{slow} vs {fast}");
        // An unknown Z stands on the slowest known axis.
        let unknown = seconds([0.0; 3], &[line([0.0, 0.0, -5.0], 600.0)], &Motion { accel: [500.0, 300.0, 0.0], junction: 0.0 });
        let as_y = seconds([0.0; 3], &[line([0.0, 0.0, -5.0], 600.0)], &Motion::flat(300.0));
        assert!(close(unknown, as_y, 1e-9));
    }

    #[test]
    fn the_settings_are_checked_and_old_files_read() {
        assert!(Motion::default().check().is_ok());
        assert!(Motion { accel: [f64::NAN, 1.0, 1.0], junction: 0.0 }.check().is_err());
        assert!(Motion { accel: [1.0; 3], junction: 11.0 }.check().is_err());
        let m: Motion = serde_json::from_str("{}").unwrap();
        assert!(m.is_unset() && !m.known());
    }
}
