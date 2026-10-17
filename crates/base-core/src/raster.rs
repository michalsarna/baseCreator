//! Turning a picture into dots: the dithering algorithms (how grey levels become black and white dots) and the grid on which a
//! picture shape is sampled, with a scan angle. The same code serves the laser (power along the scan lines), the router
//! (depth or width of an engraving) and the plotter (hatching and dots).
use crate::geom::{Pt, Rect, Xf};
use crate::image::ImageData;
use crate::scene::Shape;
use serde::{Deserialize, Serialize};

/// How a grayscale image is turned into laser dots.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Dither {
    /// Black or white at 50 %.
    Threshold,
    #[default]
    FloydSteinberg,
    Jarvis,
    Stucki,
    Atkinson,
    /// 8x8 Bayer matrix.
    Ordered,
    /// Continuous power between `min_power` and `power`.
    Grayscale,
}

impl Dither {
    pub const ALL: [Dither; 7] = [Dither::Threshold, Dither::FloydSteinberg, Dither::Jarvis, Dither::Stucki, Dither::Atkinson, Dither::Ordered, Dither::Grayscale];
    pub fn label(self) -> &'static str {
        match self {
            Dither::Threshold => "Threshold",
            Dither::FloydSteinberg => "Floyd-Steinberg",
            Dither::Jarvis => "Jarvis",
            Dither::Stucki => "Stucki",
            Dither::Atkinson => "Atkinson",
            Dither::Ordered => "Ordered",
            Dither::Grayscale => "Grayscale",
        }
    }

    /// A short description of what the method does.
    pub fn hint(self) -> &'static str {
        match self {
            Dither::Threshold => "Black or white only, split at half brightness. Sharp: good for line art and text.",
            Dither::FloydSteinberg => "Spreads the rounding error to the neighbouring dots. A good all-round choice for photos.",
            Dither::Jarvis => "Spreads the error over a wider area: smoother tones, a little slower.",
            Dither::Stucki => "Like Jarvis, with crisper detail.",
            Dither::Atkinson => "Keeps highlights clean and contrast high; loses some of the darkest and lightest detail.",
            Dither::Ordered => "A fixed 8×8 pattern of dots: regular and fast, without noise.",
            Dither::Grayscale => "No dots: every pixel gets a strength between the minimum and the full strength of the layer.",
        }
    }
}

/// Error diffusion kernels: (dx, dy, weight) with a common divisor.
fn kernel(d: Dither) -> (&'static [(i32, i32, f32)], f32) {
    match d {
        Dither::Jarvis => (
            &[(1, 0, 7.0), (2, 0, 5.0), (-2, 1, 3.0), (-1, 1, 5.0), (0, 1, 7.0), (1, 1, 5.0), (2, 1, 3.0), (-2, 2, 1.0), (-1, 2, 3.0), (0, 2, 5.0), (1, 2, 3.0), (2, 2, 1.0)],
            48.0,
        ),
        Dither::Stucki => (
            &[(1, 0, 8.0), (2, 0, 4.0), (-2, 1, 2.0), (-1, 1, 4.0), (0, 1, 8.0), (1, 1, 4.0), (2, 1, 2.0), (-2, 2, 1.0), (-1, 2, 2.0), (0, 2, 4.0), (1, 2, 2.0), (2, 2, 1.0)],
            42.0,
        ),
        Dither::Atkinson => (&[(1, 0, 1.0), (2, 0, 1.0), (-1, 1, 1.0), (0, 1, 1.0), (1, 1, 1.0), (0, 2, 1.0)], 8.0),
        _ => (&[(1, 0, 7.0), (-1, 1, 3.0), (0, 1, 5.0), (1, 1, 1.0)], 16.0),
    }
}

const BAYER8: [[u8; 8]; 8] = [
    [0, 32, 8, 40, 2, 34, 10, 42],
    [48, 16, 56, 24, 50, 18, 58, 26],
    [12, 44, 4, 36, 14, 46, 6, 38],
    [60, 28, 52, 20, 62, 30, 54, 22],
    [3, 35, 11, 43, 1, 33, 9, 41],
    [51, 19, 59, 27, 49, 17, 57, 25],
    [15, 47, 7, 39, 13, 45, 5, 37],
    [63, 31, 55, 23, 61, 29, 53, 21],
];

/// Turn darkness values (0..1, row-major) into levels: 0 = off, otherwise a fraction of the power range.
pub fn dither(d: &mut [f32], cols: usize, rows: usize, mode: Dither) {
    match mode {
        Dither::Grayscale => {}
        Dither::Threshold => d.iter_mut().for_each(|v| *v = if *v >= 0.5 { 1.0 } else { 0.0 }),
        Dither::Ordered => {
            for r in 0..rows {
                for c in 0..cols {
                    let t = (BAYER8[r % 8][c % 8] as f32 + 0.5) / 64.0;
                    let v = &mut d[r * cols + c];
                    *v = if *v > t { 1.0 } else { 0.0 };
                }
            }
        }
        _ => {
            let (k, div) = kernel(mode);
            for r in 0..rows {
                for c in 0..cols {
                    let old = d[r * cols + c];
                    let new = if old >= 0.5 { 1.0 } else { 0.0 };
                    d[r * cols + c] = new;
                    let err = old - new;
                    if err == 0.0 {
                        continue;
                    }
                    for &(dx, dy, w) in k {
                        let (cc, rr) = (c as i32 + dx, r as i32 + dy);
                        if cc >= 0 && (cc as usize) < cols && (rr as usize) < rows {
                            d[rr as usize * cols + cc as usize] += err * w / div;
                        }
                    }
                }
            }
        }
    }
}

/// The turn from the frame of the scan lines to the work area: the scan angle in degrees.
pub fn scan_rotation(angle_deg: f64) -> Xf {
    if angle_deg.is_finite() && angle_deg != 0.0 {
        Xf::rotate(angle_deg.clamp(-180.0, 180.0).to_radians())
    } else {
        Xf::IDENTITY
    }
}

/// The most cells of a grid; a finer interval is made coarser until the grid fits.
pub const MAX_CELLS: usize = 20_000_000;

/// A picture shape sampled on a grid of square cells and dithered. The grid lies in a frame turned by the scan angle (`rot`
/// takes a point of the frame to the work area); `min` is the corner of the grid in that frame, `step` the size of a cell.
#[derive(Clone, Debug)]
pub struct Grid {
    pub cols: usize,
    pub rows: usize,
    pub step: f64,
    pub min: Pt,
    pub rot: Xf,
    /// Row by row from the top: 0 = nothing, otherwise how dark the cell is (0..1; only 0 and 1 unless the method is grayscale).
    pub levels: Vec<f32>,
}

impl Grid {
    /// The level of a cell.
    pub fn at(&self, col: usize, row: usize) -> f32 {
        self.levels[row * self.cols + col]
    }

    /// The centre of a cell in the frame of the grid.
    pub fn centre(&self, col: usize, row: usize) -> Pt {
        Pt::new(self.min.x + (col as f64 + 0.5) * self.step, self.min.y + (row as f64 + 0.5) * self.step)
    }

    /// The centre of a cell in the work area.
    pub fn centre_in_area(&self, col: usize, row: usize) -> Pt {
        self.rot.apply(self.centre(col, row))
    }
}

/// Sample a picture shape on a grid with the cell size `interval` (mm) and the scan angle `angle` (degrees), and dither it.
/// Nothing when the shape has no size or cannot be inverted.
pub fn sample(shape: &Shape, im: &ImageData, interval: f64, angle: f64, mode: Dither) -> Option<Grid> {
    let wb = shape.bounds()?;
    let inv = shape.xf.inverse()?;
    // The scan lines may run at an angle: the grid is made in a turned frame, the writer turns the points back.
    let rot = scan_rotation(angle);
    let b = if angle == 0.0 || !angle.is_finite() {
        wb
    } else {
        let back = rot.inverse().unwrap_or(Xf::IDENTITY);
        Rect::from_pts([wb.min, Pt::new(wb.max.x, wb.min.y), wb.max, Pt::new(wb.min.x, wb.max.y)].map(|p| back.apply(p))).unwrap_or(wb)
    };
    let mut step = if interval.is_finite() { interval.max(0.02) } else { 0.1 };
    loop {
        let (c, r) = ((b.width() / step).ceil() as usize, (b.height() / step).ceil() as usize);
        if c.saturating_mul(r) <= MAX_CELLS {
            break;
        }
        step *= 1.5;
    }
    let cols = ((b.width() / step).ceil() as usize).max(1);
    let rows = ((b.height() / step).ceil() as usize).max(1);
    let mut levels = vec![0.0f32; cols * rows];
    for r in 0..rows {
        let y = b.min.y + (r as f64 + 0.5) * step;
        for c in 0..cols {
            let x = b.min.x + (c as f64 + 0.5) * step;
            let l = inv.apply(rot.apply(Pt::new(x, y)));
            levels[r * cols + c] = im.darkness_at(l.x, l.y);
        }
    }
    dither(&mut levels, cols, rows, mode);
    Some(Grid { cols, rows, step, min: b.min, rot, levels })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Kind, Scene};

    fn mean(v: &[f32]) -> f32 {
        v.iter().sum::<f32>() / v.len() as f32
    }

    #[test]
    fn every_method_gives_dots_that_keep_the_average_darkness() {
        for mode in Dither::ALL {
            let mut d = vec![0.3f32; 64 * 64];
            dither(&mut d, 64, 64, mode);
            if mode == Dither::Grayscale {
                assert!(d.iter().all(|v| (*v - 0.3).abs() < 1e-6), "grayscale keeps the levels");
                continue;
            }
            assert!(d.iter().all(|v| *v == 0.0 || *v == 1.0), "{mode:?} gives only dots");
            if mode == Dither::Threshold {
                assert_eq!(mean(&d), 0.0, "threshold turns 30 % into white");
            } else {
                // The share of dots follows the darkness (Atkinson spreads only 3/4 of the error, so it loses a little).
                let tolerance = if mode == Dither::Atkinson { 0.08 } else { 0.05 };
                assert!((mean(&d) - 0.3).abs() < tolerance, "{mode:?}: {}", mean(&d));
            }
        }
    }

    #[test]
    fn the_extremes_stay_and_a_dark_half_stays_dark() {
        for mode in Dither::ALL {
            let mut d: Vec<f32> = (0..16 * 16).map(|i| if (i % 16) < 8 { 1.0 } else { 0.0 }).collect();
            let before = d.clone();
            dither(&mut d, 16, 16, mode);
            assert_eq!(d, before, "{mode:?}: black and white stay as they are");
        }
    }

    fn picture(w: f64, h: f64, dark: bool) -> (Shape, ImageData) {
        let im = ImageData { name: "t".into(), px_w: 8, px_h: 8, gray: vec![if dark { 0 } else { 255 }; 64], w, h, invert: false };
        let mut scene = Scene::default();
        let id = scene.add(0, Kind::Image(im.clone()), Xf::translate(10.0, 20.0));
        (scene.shape(id).unwrap().clone(), im)
    }

    #[test]
    fn a_picture_is_sampled_on_a_grid_in_the_area_it_covers() {
        let (shape, im) = picture(10.0, 5.0, true);
        let g = sample(&shape, &im, 1.0, 0.0, Dither::Threshold).unwrap();
        assert_eq!((g.cols, g.rows), (10, 5));
        assert!(g.levels.iter().all(|v| *v == 1.0), "a black picture is all dots");
        let c = g.centre_in_area(0, 0);
        assert!((c.x - 10.5).abs() < 1e-9 && (c.y - 20.5).abs() < 1e-9, "{c:?}");
        let (shape, im) = picture(10.0, 5.0, false);
        let g = sample(&shape, &im, 1.0, 0.0, Dither::FloydSteinberg).unwrap();
        assert!(g.levels.iter().all(|v| *v == 0.0), "a white picture is empty");
    }

    #[test]
    fn a_scan_angle_turns_the_grid_and_a_fine_interval_is_made_coarser() {
        let (shape, im) = picture(10.0, 10.0, true);
        let flat = sample(&shape, &im, 1.0, 0.0, Dither::Threshold).unwrap();
        let turned = sample(&shape, &im, 1.0, 45.0, Dither::Threshold).unwrap();
        assert!(turned.cols > flat.cols, "a turned frame is wider than the picture");
        assert!(turned.levels.iter().any(|v| *v == 0.0) && turned.levels.iter().any(|v| *v == 1.0), "the corners are empty");
        let (big, im) = picture(5000.0, 5000.0, true);
        let g = sample(&big, &im, 0.02, 0.0, Dither::Threshold).unwrap();
        assert!(g.cols * g.rows <= MAX_CELLS && g.step > 0.02);
    }
}
