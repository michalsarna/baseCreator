//! Shared GUI of LightCreator and MillCreator: colour schemes, fonts, the translation layer, a small Markdown
//! viewer, unit-aware number fields and the camera helpers.
pub mod camera;
pub mod camera_stream;
pub mod clip_net;
pub mod clipart_ui;
pub mod console_ui;
pub mod description_ui;
pub mod dialogs;
pub mod editor;
pub mod fonts;
pub mod i18n;
pub mod icons;
pub mod link;
pub mod markdown_ui;
pub mod menu;
pub mod playback;
#[cfg(target_os = "macos")]
pub mod native_menu;
pub mod start;
pub mod texts;
pub mod theme;
pub mod units_ui;
pub mod window;
pub mod widgets;
