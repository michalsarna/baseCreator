//! Pictures from a camera: brightness, contrast, white balance and denoising, correction of lens distortion, and the
//! homography that maps points of the picture onto the work area. All in plain Rust, on RGB bytes.
use crate::geom::Pt;
use serde::{Deserialize, Serialize};

/// Where a camera's picture comes from.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum CameraKind {
    /// An MJPEG stream or a JPEG snapshot address (http / https).
    Url(String),
    /// A camera of the computer, by its number in the list the system gives.
    Local(u32),
}

/// How the picture is adjusted before it is shown. The defaults change nothing.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageSettings {
    /// -100 (darker) to 100 (brighter).
    #[serde(default)]
    pub brightness: i32,
    /// 0 (as it is) to 100 (twice the contrast).
    #[serde(default)]
    pub contrast: u32,
    /// Colour temperature in kelvin the picture is balanced for, 2500 to 10000; `None` = leave it to the camera.
    #[serde(default)]
    pub white_balance: Option<u32>,
    /// 0.0 (none) to 0.95: how much of a blurred copy is mixed in.
    #[serde(default)]
    pub denoise: f32,
    /// Ask a local camera for the uncompressed YUYV format where the backend can (less processing, more data).
    #[serde(default)]
    pub prefer_yuyv: bool,
}

impl Default for ImageSettings {
    fn default() -> Self {
        ImageSettings { brightness: 0, contrast: 0, white_balance: None, denoise: 0.0, prefer_yuyv: false }
    }
}

impl ImageSettings {
    /// Does it change the picture at all?
    pub fn is_neutral(&self) -> bool {
        self.brightness == 0 && self.contrast == 0 && self.white_balance.is_none() && self.denoise <= 0.0
    }

    /// The settings with every value inside its range.
    pub fn clamped(&self) -> ImageSettings {
        ImageSettings {
            brightness: self.brightness.clamp(-100, 100),
            contrast: self.contrast.min(100),
            white_balance: self.white_balance.map(|k| k.clamp(2500, 10_000)),
            denoise: if self.denoise.is_finite() { self.denoise.clamp(0.0, 0.95) } else { 0.0 },
            prefer_yuyv: self.prefer_yuyv,
        }
    }
}

/// The coefficients of the lens distortion (the Brown-Conrady model) for coordinates normalised by the longer side of
/// the picture, measured from its centre: radial `k1..k3`, tangential `p1`, `p2`. All zero = no correction.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Lens {
    #[serde(default)]
    pub k1: f64,
    #[serde(default)]
    pub k2: f64,
    #[serde(default)]
    pub k3: f64,
    #[serde(default)]
    pub p1: f64,
    #[serde(default)]
    pub p2: f64,
}

impl Lens {
    pub fn is_none(&self) -> bool {
        *self == Lens::default()
    }

    /// Every coefficient finite and small enough to mean a lens.
    pub fn is_sane(&self) -> bool {
        [self.k1, self.k2, self.k3, self.p1, self.p2].iter().all(|v| v.is_finite() && v.abs() <= 5.0)
    }
}

/// A point of the picture (pixels) known to be at a place of the work area (millimetres, design coordinates).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct RefPoint {
    pub px: (f64, f64),
    pub mm: (f64, f64),
}

/// One camera of a machine.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraSource {
    pub name: String,
    #[serde(default)]
    pub enabled: bool,
    pub kind: CameraKind,
    /// Quarter turns clockwise applied to the picture (0..=3).
    #[serde(default)]
    pub rotation: u8,
    #[serde(default)]
    pub image: ImageSettings,
    #[serde(default)]
    pub lens: Lens,
    /// Points that fix the picture to the work area; at least three (four recommended) align it.
    #[serde(default)]
    pub reference: Vec<RefPoint>,
    /// Where the live view of this camera opens: 0 = floating window, 1 = separate window, 2 = tab of the side panel, 3 = tab of the work area. `None`
    /// until chosen (the way the device's view was last left, `CameraView::mode`, is used then).
    #[serde(default)]
    pub view_mode: Option<u8>,
}

impl CameraSource {
    pub fn url(name: &str, url: &str, rotation: u8) -> CameraSource {
        CameraSource { name: name.into(), enabled: true, kind: CameraKind::Url(url.into()), rotation: rotation % 4, image: ImageSettings::default(), lens: Lens::default(), reference: vec![], view_mode: None }
    }

    /// Is the address a usable http / https URL (or a local camera chosen)? A camera that is not valid cannot be
    /// switched on.
    pub fn is_valid(&self) -> bool {
        match &self.kind {
            CameraKind::Local(_) => true,
            CameraKind::Url(u) => valid_url(u),
        }
    }
}

/// An http or https address with a host, without spaces or control characters.
pub fn valid_url(u: &str) -> bool {
    let u = u.trim();
    let rest = u.strip_prefix("http://").or_else(|| u.strip_prefix("https://"));
    let Some(rest) = rest else { return false };
    let host = rest.split(['/', '?', '#']).next().unwrap_or("");
    let name = host.rsplit('@').next().unwrap_or("");
    let name = if name.starts_with('[') { name } else { name.split(':').next().unwrap_or("") };
    !name.is_empty() && !u.chars().any(|c| c.is_whitespace() || c.is_control()) && u.len() <= 500
}

// ---------------------------------------------------------------- picture adjustments

/// The colour of black-body light at `kelvin` as RGB in 0..1 (Tanner Helland's approximation).
fn kelvin_rgb(kelvin: f64) -> [f64; 3] {
    let t = kelvin.clamp(1000.0, 40_000.0) / 100.0;
    let r = if t <= 66.0 { 255.0 } else { (329.698_727_446 * (t - 60.0).powf(-0.133_204_759_2)).clamp(0.0, 255.0) };
    let g = if t <= 66.0 { (99.470_802_586_1 * t.ln() - 161.119_568_166_1).clamp(0.0, 255.0) } else { (288.122_169_528_3 * (t - 60.0).powf(-0.075_514_849_2)).clamp(0.0, 255.0) };
    let b = if t >= 66.0 { 255.0 } else if t <= 19.0 { 0.0 } else { (138.517_731_223_1 * (t - 10.0).ln() - 305.044_792_730_7).clamp(0.0, 255.0) };
    [r / 255.0, g / 255.0, b / 255.0]
}

/// Adjust the picture in place: brightness and contrast, white balance, denoising.
pub fn adjust(rgb: &mut [u8], w: usize, h: usize, s: &ImageSettings) {
    let s = s.clamped();
    if s.is_neutral() || rgb.len() < w * h * 3 {
        return;
    }
    // Colour balance: the picture was taken under light of `kelvin`; make it look as under daylight (6500 K).
    let gain = match s.white_balance {
        Some(k) => {
            let (lit, day) = (kelvin_rgb(f64::from(k)), kelvin_rgb(6500.0));
            let g = [day[0] / lit[0].max(0.05), day[1] / lit[1].max(0.05), day[2] / lit[2].max(0.05)];
            let m = g.iter().cloned().fold(f64::MIN, f64::max);
            [g[0] / m, g[1] / m, g[2] / m]
        }
        None => [1.0; 3],
    };
    let contrast = 1.0 + f64::from(s.contrast) / 100.0;
    let shift = f64::from(s.brightness) * 2.55;
    for px in rgb[..w * h * 3].chunks_exact_mut(3) {
        for c in 0..3 {
            let v = f64::from(px[c]) * gain[c];
            px[c] = ((v - 128.0) * contrast + 128.0 + shift).round().clamp(0.0, 255.0) as u8;
        }
    }
    if s.denoise > 0.0 && w >= 3 && h >= 3 {
        let src = rgb[..w * h * 3].to_vec();
        let mix = f64::from(s.denoise);
        for y in 0..h {
            for x in 0..w {
                for c in 0..3 {
                    let mut sum = 0.0;
                    let mut n = 0.0;
                    for dy in -1i32..=1 {
                        for dx in -1i32..=1 {
                            let (xx, yy) = ((x as i32 + dx).clamp(0, w as i32 - 1) as usize, (y as i32 + dy).clamp(0, h as i32 - 1) as usize);
                            sum += f64::from(src[(yy * w + xx) * 3 + c]);
                            n += 1.0;
                        }
                    }
                    let orig = f64::from(src[(y * w + x) * 3 + c]);
                    rgb[(y * w + x) * 3 + c] = (orig * (1.0 - mix) + sum / n * mix).round().clamp(0.0, 255.0) as u8;
                }
            }
        }
    }
}

/// The picture with the lens distortion taken out. For every pixel of the result the place it has in the distorted
/// picture is computed with the model, and the colour is taken from there (bilinear). Zero coefficients return the
/// picture as it is.
pub fn undistort(rgb: &[u8], w: usize, h: usize, lens: &Lens) -> Vec<u8> {
    if lens.is_none() || !lens.is_sane() || rgb.len() < w * h * 3 || w == 0 || h == 0 {
        return rgb.to_vec();
    }
    let f = w.max(h) as f64;
    let (cx, cy) = (w as f64 / 2.0, h as f64 / 2.0);
    let mut out = vec![0u8; w * h * 3];
    for y in 0..h {
        for x in 0..w {
            // The ideal (undistorted) point, normalised, then where the lens put it.
            let (xn, yn) = ((x as f64 + 0.5 - cx) / f, (y as f64 + 0.5 - cy) / f);
            let r2 = xn * xn + yn * yn;
            let radial = 1.0 + lens.k1 * r2 + lens.k2 * r2 * r2 + lens.k3 * r2 * r2 * r2;
            let xd = xn * radial + 2.0 * lens.p1 * xn * yn + lens.p2 * (r2 + 2.0 * xn * xn);
            let yd = yn * radial + lens.p1 * (r2 + 2.0 * yn * yn) + 2.0 * lens.p2 * xn * yn;
            let (sx, sy) = (xd * f + cx - 0.5, yd * f + cy - 0.5);
            if sx < 0.0 || sy < 0.0 || sx > (w - 1) as f64 || sy > (h - 1) as f64 {
                continue; // outside the picture: black
            }
            let (x0, y0) = (sx.floor() as usize, sy.floor() as usize);
            let (x1, y1) = ((x0 + 1).min(w - 1), (y0 + 1).min(h - 1));
            let (fx, fy) = (sx - x0 as f64, sy - y0 as f64);
            for c in 0..3 {
                let p = |xx: usize, yy: usize| f64::from(rgb[(yy * w + xx) * 3 + c]);
                let v = p(x0, y0) * (1.0 - fx) * (1.0 - fy) + p(x1, y0) * fx * (1.0 - fy) + p(x0, y1) * (1.0 - fx) * fy + p(x1, y1) * fx * fy;
                out[(y * w + x) * 3 + c] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
    }
    out
}

// ---------------------------------------------------------------- homography

/// A 3×3 matrix, row by row, that maps picture points to work-area points.
pub type Homography = [f64; 9];

pub fn apply(h: &Homography, p: (f64, f64)) -> Option<(f64, f64)> {
    let w = h[6] * p.0 + h[7] * p.1 + h[8];
    if w.abs() < 1e-12 {
        return None;
    }
    Some(((h[0] * p.0 + h[1] * p.1 + h[2]) / w, (h[3] * p.0 + h[4] * p.1 + h[5]) / w))
}

/// Solve the linear system `a x = b` (n × n) by elimination with pivoting.
fn solve(mut a: Vec<Vec<f64>>, mut b: Vec<f64>) -> Option<Vec<f64>> {
    let n = b.len();
    for col in 0..n {
        let piv = (col..n).max_by(|&i, &j| a[i][col].abs().partial_cmp(&a[j][col].abs()).unwrap_or(std::cmp::Ordering::Equal))?;
        if a[piv][col].abs() < 1e-12 {
            return None;
        }
        a.swap(col, piv);
        b.swap(col, piv);
        for r in col + 1..n {
            let k = a[r][col] / a[col][col];
            for c in col..n {
                a[r][c] -= k * a[col][c];
            }
            b[r] -= k * b[col];
        }
    }
    let mut x = vec![0.0; n];
    for i in (0..n).rev() {
        let s: f64 = (i + 1..n).map(|j| a[i][j] * x[j]).sum();
        x[i] = (b[i] - s) / a[i][i];
    }
    Some(x)
}

/// The transform from the picture to the work area that fits the reference points best (least squares). With three
/// points it is an affine map (no perspective); with four or more a full homography. `None` when the points do not
/// fix a map (fewer than three, or all on a line).
pub fn homography(points: &[RefPoint]) -> Option<Homography> {
    if points.len() < 3 || points.iter().any(|p| ![p.px.0, p.px.1, p.mm.0, p.mm.1].iter().all(|v| v.is_finite())) {
        return None;
    }
    // Scale the picture coordinates so the equations are well conditioned.
    let s = points.iter().map(|p| p.px.0.abs().max(p.px.1.abs())).fold(1.0, f64::max);
    let affine = points.len() == 3;
    let n = if affine { 6 } else { 8 };
    let mut ata = vec![vec![0.0; n]; n];
    let mut atb = vec![0.0; n];
    let mut add = |row: Vec<f64>, rhs: f64| {
        for i in 0..n {
            for j in 0..n {
                ata[i][j] += row[i] * row[j];
            }
            atb[i] += row[i] * rhs;
        }
    };
    for p in points {
        let (u, v) = (p.px.0 / s, p.px.1 / s);
        let (x, y) = p.mm;
        if affine {
            add(vec![u, v, 1.0, 0.0, 0.0, 0.0], x);
            add(vec![0.0, 0.0, 0.0, u, v, 1.0], y);
        } else {
            add(vec![u, v, 1.0, 0.0, 0.0, 0.0, -u * x, -v * x], x);
            add(vec![0.0, 0.0, 0.0, u, v, 1.0, -u * y, -v * y], y);
        }
    }
    let sol = solve(ata, atb)?;
    let mut h = if affine { [sol[0], sol[1], sol[2], sol[3], sol[4], sol[5], 0.0, 0.0, 1.0] } else { [sol[0], sol[1], sol[2], sol[3], sol[4], sol[5], sol[6], sol[7], 1.0] };
    // Back from the scaled picture coordinates: (u, v) = (px / s, py / s).
    h[0] /= s;
    h[1] /= s;
    h[3] /= s;
    h[4] /= s;
    h[6] /= s;
    h[7] /= s;
    Some(h)
}

/// The work-area positions of the four corners of a picture of `w × h` pixels under `h`: the corners the overlay uses.
pub fn corners(h: &Homography, w: f64, ht: f64) -> Option<[Pt; 4]> {
    let c = |p: (f64, f64)| apply(h, p).map(|(x, y)| Pt::new(x, y));
    Some([c((0.0, 0.0))?, c((w, 0.0))?, c((w, ht))?, c((0.0, ht))?])
}

/// Reference points from the four corners of an overlay (the older way of aligning a picture): the corners of a
/// picture of `w × h` pixels and the work-area positions they have.
pub fn points_from_corners(c: &[Pt; 4], w: f64, h: f64) -> Vec<RefPoint> {
    [(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)].iter().zip(c).map(|(px, p)| RefPoint { px: *px, mm: (p.x, p.y) }).collect()
}

/// How the camera view of a device opens: the same way every time.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CameraView {
    /// 0 = floating window, 1 = separate window, 2 = tab of the side panel, 3 = tab of the work area.
    #[serde(default)]
    pub mode: u8,
    /// 1.0 = the whole picture.
    #[serde(default = "one")]
    pub zoom: f32,
    /// Position and size of the floating window, and of the separate window, as the program last saw them.
    #[serde(default)]
    pub float_pos: Option<[f32; 2]>,
    #[serde(default)]
    pub float_size: Option<[f32; 2]>,
    #[serde(default)]
    pub win_pos: Option<[f32; 2]>,
    #[serde(default)]
    pub win_size: Option<[f32; 2]>,
}

fn one() -> f32 {
    1.0
}

impl Default for CameraView {
    fn default() -> Self {
        CameraView { mode: 0, zoom: 1.0, float_pos: None, float_size: None, win_pos: None, win_size: None }
    }
}

impl CameraView {
    /// Are the numbers sane (for a profile that comes from a file)?
    pub fn is_valid(&self) -> bool {
        let pair = |p: &Option<[f32; 2]>, lo: f32, hi: f32| p.is_none_or(|v| v.iter().all(|x| x.is_finite() && *x >= lo && *x <= hi));
        self.mode <= 3 && self.zoom.is_finite() && (1.0..=8.0).contains(&self.zoom) && pair(&self.float_pos, -20_000.0, 20_000.0) && pair(&self.win_pos, -20_000.0, 20_000.0) && pair(&self.float_size, 100.0, 20_000.0) && pair(&self.win_size, 100.0, 20_000.0)
    }
}
