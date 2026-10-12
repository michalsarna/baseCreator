//! Raster images for engraving: stored as 8-bit grayscale (white = no burn).
use serde::{Deserialize, Serialize};

mod b64 {
    use base64::Engine;
    use serde::{Deserialize, Deserializer, Serializer};
    pub fn serialize<S: Serializer>(v: &Vec<u8>, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&base64::engine::general_purpose::STANDARD.encode(v))
    }
    pub fn deserialize<'de, D: Deserializer<'de>>(d: D) -> Result<Vec<u8>, D::Error> {
        let s = String::deserialize(d)?;
        base64::engine::general_purpose::STANDARD.decode(s).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageData {
    pub name: String,
    pub px_w: u32,
    pub px_h: u32,
    /// Row-major luminance, `px_w * px_h` bytes.
    #[serde(with = "b64")]
    pub gray: Vec<u8>,
    /// Size on the work area, millimetres.
    pub w: f64,
    pub h: f64,
    /// Swap black and white (engrave the light parts).
    #[serde(default)]
    pub invert: bool,
}

impl ImageData {
    /// Decode a PNG / JPEG / BMP / GIF / WebP file. Transparent pixels become white. The longer side is
    /// `longest_mm` millimetres.
    pub fn from_bytes(bytes: &[u8], name: &str, longest_mm: f64) -> Result<ImageData, String> {
        let img = image::load_from_memory(bytes).map_err(|e| e.to_string())?.to_rgba8();
        let (w, h) = (img.width(), img.height());
        if w == 0 || h == 0 {
            return Err("empty image".into());
        }
        let gray: Vec<u8> = img
            .pixels()
            .map(|p| {
                let [r, g, b, a] = p.0;
                let l = 0.299 * r as f32 + 0.587 * g as f32 + 0.114 * b as f32;
                let a = a as f32 / 255.0;
                (l * a + 255.0 * (1.0 - a)).round().clamp(0.0, 255.0) as u8
            })
            .collect();
        let k = longest_mm / w.max(h) as f64;
        Ok(ImageData { name: name.to_string(), px_w: w, px_h: h, gray, w: w as f64 * k, h: h as f64 * k, invert: false })
    }

    /// Darkness (0 = white / nothing, 1 = black) at the local point `(x, y)` millimetres, bilinear.
    pub fn darkness_at(&self, x: f64, y: f64) -> f32 {
        if x < 0.0 || y < 0.0 || x >= self.w || y >= self.h {
            return 0.0;
        }
        let fx = (x / self.w * self.px_w as f64 - 0.5).clamp(0.0, self.px_w as f64 - 1.0);
        let fy = (y / self.h * self.px_h as f64 - 0.5).clamp(0.0, self.px_h as f64 - 1.0);
        let (x0, y0) = (fx.floor() as usize, fy.floor() as usize);
        let (x1, y1) = ((x0 + 1).min(self.px_w as usize - 1), (y0 + 1).min(self.px_h as usize - 1));
        let (tx, ty) = ((fx - x0 as f64) as f32, (fy - y0 as f64) as f32);
        let g = |x: usize, y: usize| self.gray[y * self.px_w as usize + x] as f32;
        let v = g(x0, y0) * (1.0 - tx) * (1.0 - ty) + g(x1, y0) * tx * (1.0 - ty) + g(x0, y1) * (1.0 - tx) * ty + g(x1, y1) * tx * ty;
        let d = 1.0 - v / 255.0;
        if self.invert {
            1.0 - d
        } else {
            d
        }
    }
}

/// Import-time adjustments of a bitmap.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Adjust {
    /// Quarter turns clockwise (0..=3).
    pub quarter_turns: u8,
    /// Extra free rotation in degrees (the canvas grows, new corners are white).
    pub angle_deg: f32,
    pub flip_h: bool,
    pub flip_v: bool,
    /// -1..=1
    pub brightness: f32,
    /// -1..=1
    pub contrast: f32,
    /// 0.2..=3; above 1 brightens the midtones.
    pub gamma: f32,
    /// Stretch the darkest and lightest pixels to the full range.
    pub auto_levels: bool,
}

impl Default for Adjust {
    fn default() -> Self {
        Adjust { quarter_turns: 0, angle_deg: 0.0, flip_h: false, flip_v: false, brightness: 0.0, contrast: 0.0, gamma: 1.0, auto_levels: false }
    }
}

impl ImageData {
    /// Shrink so the longer side has at most `max_px` pixels (nearest neighbour is enough for previews).
    pub fn downscaled(&self, max_px: u32) -> ImageData {
        let m = self.px_w.max(self.px_h);
        if m <= max_px {
            return self.clone();
        }
        let k = max_px as f64 / m as f64;
        let (nw, nh) = (((self.px_w as f64 * k).round() as u32).max(1), ((self.px_h as f64 * k).round() as u32).max(1));
        let mut gray = Vec::with_capacity((nw * nh) as usize);
        for y in 0..nh {
            for x in 0..nw {
                let (sx, sy) = ((x as f64 / k) as u32, (y as f64 / k) as u32);
                gray.push(self.gray[(sy.min(self.px_h - 1) * self.px_w + sx.min(self.px_w - 1)) as usize]);
            }
        }
        ImageData { gray, px_w: nw, px_h: nh, ..self.clone() }
    }

    fn rotated_quarters(&self, q: u8) -> ImageData {
        let (w, h) = (self.px_w as usize, self.px_h as usize);
        match q % 4 {
            0 => self.clone(),
            2 => ImageData { gray: self.gray.iter().rev().copied().collect(), ..self.clone() },
            k => {
                let mut g = vec![255u8; w * h];
                for y in 0..h {
                    for x in 0..w {
                        // New image is h wide and w tall.
                        let (nx, ny) = if k == 1 { (h - 1 - y, x) } else { (y, w - 1 - x) };
                        g[ny * h + nx] = self.gray[y * w + x];
                    }
                }
                ImageData { gray: g, px_w: self.px_h, px_h: self.px_w, w: self.h, h: self.w, ..self.clone() }
            }
        }
    }

    fn rotated_free(&self, deg: f32) -> ImageData {
        let (w, h) = (self.px_w as f32, self.px_h as f32);
        let (s, c) = deg.to_radians().sin_cos();
        let (nw, nh) = ((w * c.abs() + h * s.abs()).ceil().max(1.0), (w * s.abs() + h * c.abs()).ceil().max(1.0));
        let (cx, cy, ncx, ncy) = (w / 2.0, h / 2.0, nw / 2.0, nh / 2.0);
        let mut g = vec![255u8; (nw * nh) as usize];
        let at = |x: i32, y: i32| -> f32 { if x < 0 || y < 0 || x >= self.px_w as i32 || y >= self.px_h as i32 { 255.0 } else { self.gray[(y as u32 * self.px_w + x as u32) as usize] as f32 } };
        for y in 0..nh as u32 {
            for x in 0..nw as u32 {
                let (dx, dy) = (x as f32 + 0.5 - ncx, y as f32 + 0.5 - ncy);
                // Inverse rotation.
                let (sx, sy) = (dx * c + dy * s + cx - 0.5, -dx * s + dy * c + cy - 0.5);
                let (x0, y0) = (sx.floor() as i32, sy.floor() as i32);
                let (tx, ty) = (sx - x0 as f32, sy - y0 as f32);
                let v = at(x0, y0) * (1.0 - tx) * (1.0 - ty) + at(x0 + 1, y0) * tx * (1.0 - ty) + at(x0, y0 + 1) * (1.0 - tx) * ty + at(x0 + 1, y0 + 1) * tx * ty;
                g[(y * nw as u32 + x) as usize] = v.round().clamp(0.0, 255.0) as u8;
            }
        }
        let k = self.w / self.px_w as f64;
        ImageData { gray: g, px_w: nw as u32, px_h: nh as u32, w: nw as f64 * k, h: nh as f64 * k, ..self.clone() }
    }

    /// A processed copy: rotate / flip, then levels, brightness, contrast and gamma.
    pub fn adjusted(&self, a: &Adjust) -> ImageData {
        let mut im = self.rotated_quarters(a.quarter_turns);
        if a.angle_deg.abs() > 0.01 {
            im = im.rotated_free(a.angle_deg);
        }
        let (w, h) = (im.px_w as usize, im.px_h as usize);
        if a.flip_h {
            for row in im.gray.chunks_mut(w) {
                row.reverse();
            }
        }
        if a.flip_v {
            let mut g = Vec::with_capacity(w * h);
            for row in im.gray.chunks(w).rev() {
                g.extend_from_slice(row);
            }
            im.gray = g;
        }
        let (lo, hi) = if a.auto_levels {
            (*im.gray.iter().min().unwrap_or(&0) as f32, *im.gray.iter().max().unwrap_or(&255) as f32)
        } else {
            (0.0, 255.0)
        };
        let span = (hi - lo).max(1.0);
        let k = ((a.contrast.clamp(-0.99, 0.99) + 1.0) * std::f32::consts::FRAC_PI_4).tan();
        let inv_gamma = 1.0 / a.gamma.clamp(0.2, 3.0);
        let lut: Vec<u8> = (0..256)
            .map(|i| {
                let mut v = ((i as f32 - lo) / span).clamp(0.0, 1.0);
                v = v.powf(inv_gamma);
                v = (v - 0.5) * k + 0.5 + a.brightness.clamp(-1.0, 1.0);
                (v.clamp(0.0, 1.0) * 255.0).round() as u8
            })
            .collect();
        im.gray.iter_mut().for_each(|g| *g = lut[*g as usize]);
        im
    }
}
