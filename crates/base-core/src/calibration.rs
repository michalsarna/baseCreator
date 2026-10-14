//! Numbers for the calibration tools: steps per millimetre from a measured move, and how far out of square the axes are
//! from the two diagonals of a rectangle.

/// The new value of `$100` / `$101` / `$102`: the machine was told to move `commanded` mm and moved `measured` mm.
pub fn corrected_steps(current: f64, commanded: f64, measured: f64) -> Option<f64> {
    if ![current, commanded, measured].iter().all(|v| v.is_finite() && *v > 0.0) {
        return None;
    }
    let v = current * commanded / measured;
    (v.is_finite() && v > 0.0).then(|| (v * 1000.0).round() / 1000.0)
}

/// How far the axes are from a right angle.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Skew {
    /// Degrees; positive when the diagonal from the front left to the back right corner is the longer one.
    pub angle_deg: f64,
    /// How much one end of the Y axis is shifted along X over the whole height, mm.
    pub shift_mm: f64,
}

/// A rectangle `width` × `height` was cut (or marked) and its two diagonals measured: `d1` from the front left corner
/// to the back right one, `d2` from the front right to the back left.
pub fn skew(width: f64, height: f64, d1: f64, d2: f64) -> Option<Skew> {
    if ![width, height, d1, d2].iter().all(|v| v.is_finite() && *v > 0.0) {
        return None;
    }
    let d = (d1 + d2) / 2.0;
    // d1² - d2² = 4·W·H·sinθ for axes that are 90° + θ apart.
    let sin = (d1 * d1 - d2 * d2) / (4.0 * width * height);
    if sin.abs() > 0.5 {
        return None;
    }
    let _ = d;
    let theta = sin.asin();
    Some(Skew { angle_deg: theta.to_degrees(), shift_mm: height * theta.tan() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_follow_the_measured_distance() {
        // Asked for 100 mm, got 98: the axis needs more steps per mm.
        assert_eq!(corrected_steps(800.0, 100.0, 98.0), Some(816.327));
        assert_eq!(corrected_steps(800.0, 100.0, 100.0), Some(800.0));
        assert_eq!(corrected_steps(800.0, 100.0, 0.0), None);
        assert_eq!(corrected_steps(f64::NAN, 100.0, 99.0), None);
        assert_eq!(corrected_steps(800.0, -1.0, 99.0), None);
    }

    #[test]
    fn a_square_machine_has_no_skew_and_a_tilted_one_shows_it() {
        let (w, h) = (300.0f64, 200.0f64);
        let d = (w * w + h * h).sqrt();
        let s = skew(w, h, d, d).unwrap();
        assert!(s.angle_deg.abs() < 1e-9 && s.shift_mm.abs() < 1e-9);
        // Build the diagonals of a parallelogram whose Y axis leans by 0.5°.
        let th = 0.5f64.to_radians();
        let d1 = ((w + h * th.sin()).powi(2) + (h * th.cos()).powi(2)).sqrt();
        let d2 = ((w - h * th.sin()).powi(2) + (h * th.cos()).powi(2)).sqrt();
        let s = skew(w, h, d1, d2).unwrap();
        assert!((s.angle_deg - 0.5).abs() < 0.01, "{s:?}");
        assert!((s.shift_mm - h * th.tan()).abs() < 0.05, "{s:?}");
        let flipped = skew(w, h, d2, d1).unwrap();
        assert!((flipped.angle_deg + 0.5).abs() < 0.01);
        assert!(skew(w, h, 0.0, d).is_none() && skew(0.0, h, d, d).is_none());
        assert!(skew(w, h, d * 2.0, d).is_none(), "a measurement that cannot be right");
    }
}
