//! Shared core of LightCreator and MillCreator: geometry, Bezier paths, boolean operations, text outlines,
//! bitmaps, tracing, camera calibration and the GRBL / ser2net helpers. No GUI, no machine-specific types.
pub mod arcs;
pub mod backup;
pub mod bezier;
pub mod calibration;
pub mod camimg;
pub mod clipart;
pub mod control;
pub mod diag;
pub mod fillet;
pub mod firmware;
pub mod geom;
pub mod grbl_codes;
pub mod grbl_settings;
pub mod grbl_status;
pub mod image;
pub mod machine;
pub mod lightburn;
pub mod ops;
pub mod paths;
pub mod pdf;
pub mod presets;
pub mod project;
pub mod resume;
pub mod scene;
pub mod ser2net;
pub mod snap;
pub mod svg;
pub mod text;
pub mod trace;
pub mod units;

pub use bezier::*;
pub use camimg::{CameraKind, CameraSource, CameraView, ImageSettings, Lens, RefPoint};
pub use geom::*;
pub use image::{Adjust, ImageData};
pub use scene::{Kind, Scene, Shape, DEFAULT_FLATTEN_TOL};
pub use text::TextData;
pub use units::{SpeedUnit, Units};

#[cfg(test)]
mod tests;
