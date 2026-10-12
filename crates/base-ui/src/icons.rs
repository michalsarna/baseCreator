//! Icons. Every picture in the interface is an SVG file from `assets/ui-icons` (Lucide, plus two drawn in the
//! same style), embedded in the program and tinted with the colour of the current theme.
use crate::editor::Tool;
use egui::{Color32, Rect, Ui};
use std::collections::HashMap;
use std::sync::{Arc, OnceLock};

// Every file of `assets/ui-icons`, collected by `build.rs`.
include!(concat!(env!("OUT_DIR"), "/ui_icon_files.rs"));

/// SVG bytes by name; strokes use `currentColor`, which is replaced by white so a tint can recolour the icon.
fn table() -> &'static HashMap<&'static str, Arc<[u8]>> {
    static T: OnceLock<HashMap<&'static str, Arc<[u8]>>> = OnceLock::new();
    T.get_or_init(|| {
        FILES
            .iter()
            .map(|(n, b)| {
                let text = String::from_utf8_lossy(b).replace("currentColor", "#ffffff");
                (*n, Arc::from(text.into_bytes().into_boxed_slice()))
            })
            .collect()
    })
}

/// Names of all embedded icons.
pub fn all_names() -> Vec<&'static str> {
    FILES.iter().map(|(n, _)| *n).collect()
}

/// The icon as an image of `size` points, tinted with `color`. An unknown name gives `None`.
pub fn image(name: &str, size: f32, color: Color32) -> Option<egui::Image<'static>> {
    let (key, bytes) = table().get_key_value(name)?;
    Some(egui::Image::from_bytes(format!("bytes://lc-icon/{key}.svg"), bytes.clone()).fit_to_exact_size(egui::vec2(size, size)).tint(color))
}

/// Paint the icon filling `rect`.
pub fn paint(ui: &Ui, rect: Rect, name: &str, color: Color32) {
    if let Some(img) = image(name, rect.width().min(rect.height()), color) {
        img.paint_at(ui, rect);
    }
}

pub fn tool_icon(t: Tool) -> &'static str {
    match t {
        Tool::Select => "mouse-pointer-2",
        Tool::Node => "node-edit",
        Tool::Rect => "square",
        Tool::Ellipse => "circle",
        Tool::Triangle => "triangle",
        Tool::Star => "star",
        Tool::Polygon => "hexagon",
        Tool::Heart => "heart",
        Tool::Spiral => "spiral",
        Tool::Line => "minus",
        Tool::Pen => "pen-tool",
        Tool::Text => "type",
        Tool::Pan => "hand",
        Tool::Zoom => "zoom-in",
        Tool::ZoomOut => "zoom-out",
    }
}

/// Icon of a menu title (File, Edit ...), by its English name.
pub fn menu_title_icon(title: &str) -> Option<&'static str> {
    Some(match title {
        "File" => "file",
        "Edit" => "pencil",
        "Arrange" => "shapes",
        "View" => "eye",
        "Laser" => "zap",
        "Settings" => "settings",
        "Help" => "circle-question-mark",
        _ => return None,
    })
}

/// Icon of a submenu title, by its English name.
pub fn submenu_icon(title: &str) -> Option<&'static str> {
    Some(match title {
        "Import" => "download",
        "Export" => "upload",
        "Open recent" => "history",
        "Language" => "languages",
        "Colour scheme" => "palette",
        _ => return None,
    })
}

/// A leading icon for menu entries: the icon, or a transparent placeholder so that texts line up.
pub fn slot(name: Option<&str>, color: Color32) -> egui::Image<'static> {
    match name.and_then(|n| image(n, 16.0, color)) {
        Some(img) => img,
        None => image("check", 16.0, Color32::TRANSPARENT).unwrap_or_else(|| egui::Image::from_bytes("bytes://base-ui/blank.svg", &b"<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"16\" height=\"16\"/>"[..]).fit_to_exact_size(egui::vec2(16.0, 16.0))),
    }
}

/// The original SVG text of an icon (strokes still `currentColor`), for the native macOS menu bar.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
pub fn svg_text(name: &str) -> Option<String> {
    FILES.iter().find(|(n, _)| *n == name).map(|(_, b)| String::from_utf8_lossy(b).to_string())
}

/// Make the program able to draw the icons (SVG and picture loaders of egui). Call once at start.
pub fn install(ctx: &egui::Context) {
    egui_extras::install_image_loaders(ctx);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Every icon the shared code asks for must exist in the assets folder (a missing one would draw nothing).
    #[test]
    fn the_icons_of_tools_and_menus_exist() {
        let have: std::collections::HashSet<&str> = all_names().into_iter().collect();
        for (tool, _, _) in Tool::ALL {
            assert!(have.contains(tool_icon(tool)), "tool icon {}", tool_icon(tool));
        }
        for t in ["File", "Edit", "Arrange", "View", "Laser", "Settings", "Help"] {
            assert!(have.contains(menu_title_icon(t).unwrap()), "{t}");
        }
        for t in ["Import", "Export", "Open recent", "Language", "Colour scheme"] {
            assert!(have.contains(submenu_icon(t).unwrap()), "{t}");
        }
        assert!(image("check", 16.0, Color32::WHITE).is_some());
        assert!(image("no-such-icon", 16.0, Color32::WHITE).is_none());
    }
}
