//! The picture of the overlay (a photo or a frame of the camera, shown under the design) as the project file keeps it: the picture as a
//! JPEG, where its corners are on the work area, how see-through it is, and whether it is shown. It is a section of the project
//! (`overlay`), so the file brings the picture back to the machine and the person that opens it.
use crate::geom::Pt;
use base64::Engine;
use serde::{Deserialize, Serialize};

/// The name of the section of the project file.
pub const OVERLAY_SECTION: &str = "overlay";
const FORMAT: u32 = 1;
/// The quality of the JPEG that is kept: a photo to look at while placing work, not an archive.
const JPEG_QUALITY: u8 = 88;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OverlayData {
    pub format: u32,
    pub name: String,
    /// Work-area positions (mm) of the picture's top-left, top-right, bottom-right and bottom-left corners.
    pub corners: [Pt; 4],
    pub opacity: f64,
    /// Is it shown on the work area?
    pub visible: bool,
    /// The picture: a JPEG in base64.
    pub jpeg: String,
}

impl OverlayData {
    /// The data of a picture given as RGB bytes (`w * h * 3`).
    pub fn new(name: &str, corners: [Pt; 4], opacity: f64, visible: bool, rgb: &[u8], w: u32, h: u32) -> Result<OverlayData, String> {
        if w == 0 || h == 0 || rgb.len() != (w as usize) * (h as usize) * 3 {
            return Err("the picture of the overlay has no size".into());
        }
        let mut out = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY).encode(rgb, w, h, image::ExtendedColorType::Rgb8).map_err(|e| e.to_string())?;
        Ok(OverlayData { format: FORMAT, name: name.to_string(), corners, opacity, visible, jpeg: base64::engine::general_purpose::STANDARD.encode(out) })
    }

    /// The picture as RGBA bytes with its size.
    pub fn decode(&self) -> Result<(u32, u32, Vec<u8>), String> {
        let bytes = base64::engine::general_purpose::STANDARD.decode(self.jpeg.as_bytes()).map_err(|e| e.to_string())?;
        let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg).map_err(|e| e.to_string())?.to_rgba8();
        Ok((img.width(), img.height(), img.into_raw()))
    }

    /// Does this program read it (a newer file may have another format)?
    pub fn is_readable(&self) -> bool {
        self.format <= FORMAT && self.corners.iter().all(|c| c.x.is_finite() && c.y.is_finite()) && self.opacity.is_finite()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rgb(w: u32, h: u32) -> Vec<u8> {
        (0..w * h * 3).map(|i| (i % 251) as u8).collect()
    }

    #[test]
    fn a_picture_comes_back_with_its_size_and_a_bad_one_is_not_taken() {
        let d = OverlayData::new("x", [Pt::default(); 4], 0.5, true, &rgb(64, 48), 64, 48).unwrap();
        let (w, h, px) = d.decode().unwrap();
        assert_eq!((w, h, px.len()), (64, 48, 64 * 48 * 4));
        assert!(d.is_readable());
        assert!(OverlayData::new("x", [Pt::default(); 4], 0.5, true, &[1, 2, 3], 4, 4).is_err());
        assert!(OverlayData::new("x", [Pt::default(); 4], 0.5, true, &[], 0, 0).is_err());
        let newer = OverlayData { format: 99, ..d.clone() };
        assert!(!newer.is_readable());
        let broken = OverlayData { jpeg: "not base64!!".into(), ..d };
        assert!(broken.decode().is_err());
    }
}
