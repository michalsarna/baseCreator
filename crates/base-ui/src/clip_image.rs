//! Pictures in the clipboard of the system: egui hands the programs only text, so a picture is read here.

/// A picture of the clipboard: RGBA pixels, row by row.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipImage {
    pub w: u32,
    pub h: u32,
    pub rgba: Vec<u8>,
}

impl ClipImage {
    /// A short sum of the picture, to tell whether the clipboard got another one.
    pub fn fingerprint(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (self.w, self.h).hash(&mut h);
        // Every 97th byte is plenty to tell two pictures apart and keeps a large screenshot quick.
        self.rgba.iter().step_by(97).for_each(|b| b.hash(&mut h));
        h.finish()
    }
}

/// The picture in the clipboard of the system, if there is one.
pub fn read() -> Option<ClipImage> {
    let img = arboard::Clipboard::new().ok()?.get_image().ok()?;
    (img.width > 0 && img.height > 0).then(|| ClipImage { w: img.width as u32, h: img.height as u32, rgba: img.bytes.into_owned() })
}
