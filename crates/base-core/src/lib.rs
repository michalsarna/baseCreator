//! Shared core of LightCreator and MillCreator: geometry, Bezier paths, boolean operations, text outlines,
//! bitmaps, tracing, camera calibration and the GRBL / ser2net helpers. No GUI, no machine-specific types.
pub mod arcs;
pub mod bezier;
pub mod camimg;
pub mod clipart;
pub mod fillet;
pub mod geom;
pub mod grbl_settings;
pub mod image;
pub mod ops;
pub mod pdf;
pub mod scene;
pub mod ser2net;
pub mod snap;
pub mod svg;
pub mod text;
pub mod trace;

pub use bezier::*;
pub use camimg::{CameraKind, CameraSource, CameraView, ImageSettings, Lens, RefPoint};
pub use geom::*;
pub use image::{Adjust, ImageData};
pub use scene::{Kind, Scene, Shape, DEFAULT_FLATTEN_TOL};
pub use text::TextData;

#[cfg(test)]
mod tests;
