//! A viewer for G-code files, the same in all programs: a view that covers the whole work area with the toolpath of a file, flat
//! (2D) for a laser file and in space (3D) when Z changes. Drag to move (3D: to rotate, right button or Shift to move), the wheel
//! zooms. The program keeps an `Option<GcodeViewer>`, draws [`panel`] in the place of the editor while it is `Some`, and opens a
//! file with [`open`] (File > View G-code file…, or a dropped file that [`is_gcode`]).
use crate::i18n::{tr, trf};
use crate::theme;
use base_core::gcode_view::{self, Toolpath};
use egui::{Color32, Pos2, Rect, Sense, Shape, Stroke, Vec2};
use std::path::{Path, PathBuf};

/// Extensions that are taken for G-code when a file is opened or dropped.
pub const EXTENSIONS: [&str; 7] = ["gcode", "nc", "gc", "gco", "ngc", "tap", "cnc"];

pub fn is_gcode(p: &Path) -> bool {
    p.extension().is_some_and(|e| EXTENSIONS.iter().any(|x| e.eq_ignore_ascii_case(x)))
}

/// Segments drawn at most; a bigger file is thinned out (every n-th) so that moving the view stays smooth.
const DRAW_LIMIT: usize = 300_000;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Cam {
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    pub pan: Vec2,
}

impl Cam {
    pub fn home(is_3d: bool) -> Cam {
        // Seen from above for a flat file; from the front-left and above for a 3D one.
        if is_3d {
            Cam { yaw: -0.6, pitch: 0.95, zoom: 1.0, pan: Vec2::ZERO }
        } else {
            Cam { yaw: 0.0, pitch: 0.0, zoom: 1.0, pan: Vec2::ZERO }
        }
    }
}

pub struct GcodeViewer {
    pub path: PathBuf,
    pub tp: Toolpath,
    pub show_rapid: bool,
    pub by_height: bool,
    pub cam: Cam,
}

impl GcodeViewer {
    pub fn load(path: PathBuf) -> Result<GcodeViewer, String> {
        let bytes = std::fs::read(&path).map_err(|e| e.to_string())?;
        // Old programs write G-code in odd encodings: what is not UTF-8 is read lossily, the comments are not drawn anyway.
        let text = String::from_utf8_lossy(&bytes);
        let tp = gcode_view::parse(&text);
        if tp.segs.is_empty() {
            return Err("no moves".into());
        }
        let cam = Cam::home(tp.is_3d);
        Ok(GcodeViewer { path, tp, show_rapid: true, by_height: true, cam })
    }

    /// The scale that fits the file into a rectangle (the camera at home).
    fn fit_scale(&self, rect: Rect) -> f32 {
        let d = [self.tp.max[0] - self.tp.min[0], self.tp.max[1] - self.tp.min[1], self.tp.max[2] - self.tp.min[2]];
        let r = if self.tp.is_3d {
            0.5 * (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt()
        } else {
            // A flat file fits by its two sides.
            0.5 * d[0].max(d[1] * rect.width() / rect.height().max(1.0))
        };
        let side = if self.tp.is_3d { rect.width().min(rect.height()) } else { rect.width() };
        0.88 * side / (2.0 * r.max(0.001))
    }

    /// A point of the file on the screen.
    fn project(&self, p: [f32; 3], centre: Pos2, scale: f32) -> Pos2 {
        let c = [(self.tp.min[0] + self.tp.max[0]) / 2.0, (self.tp.min[1] + self.tp.max[1]) / 2.0, (self.tp.min[2] + self.tp.max[2]) / 2.0];
        let q = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
        let (sy, cy) = self.cam.yaw.sin_cos();
        let (sp, cp) = self.cam.pitch.sin_cos();
        let x1 = q[0] * cy - q[1] * sy;
        let y1 = q[0] * sy + q[1] * cy;
        let up = y1 * cp + q[2] * sp;
        centre + self.cam.pan + Vec2::new(x1, -up) * scale
    }
}

fn height_colour(t: f32) -> Color32 {
    // Deep blue at the bottom to yellow at the top.
    let t = t.clamp(0.0, 1.0);
    let lerp = |a: f32, b: f32| (a + (b - a) * t) as u8;
    Color32::from_rgb(lerp(60.0, 255.0), lerp(120.0, 210.0), lerp(255.0, 40.0))
}

/// A dialog that asks for a G-code file.
pub fn pick_file() -> Option<PathBuf> {
    rfd::FileDialog::new().add_filter("G-code", &EXTENSIONS).pick_file()
}

/// Opens a file into the viewer. The error is a line for the status bar.
pub fn open(view: &mut Option<GcodeViewer>, p: PathBuf) -> Result<(), String> {
    match GcodeViewer::load(p) {
        Ok(v) => {
            *view = Some(v);
            Ok(())
        }
        Err(e) => Err(trf("Open failed: {}", &[&e])),
    }
}

/// The viewer in the place of the work area; false when there is none (the editor is drawn then) or it was closed.
pub fn panel(view: &mut Option<GcodeViewer>, ui: &mut egui::Ui) -> bool {
    let Some(mut v) = view.take() else { return false };
    let mut close = ui.input(|i| i.key_pressed(egui::Key::Escape));
    egui::Frame::new().fill(theme::panel()).inner_margin(egui::Margin::symmetric(8, 4)).show(ui, |ui| {
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(format!("{} · {}", tr("G-code viewer"), v.path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default())).strong());
            ui.separator();
            if ui.button(tr("Reset view")).clicked() {
                v.cam = Cam::home(v.tp.is_3d);
            }
            ui.checkbox(&mut v.show_rapid, tr("Show travel moves"));
            if v.tp.is_3d {
                ui.checkbox(&mut v.by_height, tr("Colour by height"));
            }
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(tr("Close")).clicked() {
                    close = true;
                }
            });
        });
        ui.horizontal_wrapped(|ui| {
            let t = &v.tp;
            ui.label(egui::RichText::new(format!(
                "{} · X {:.1}…{:.1}  Y {:.1}…{:.1}{}",
                if t.is_3d { "3D" } else { "2D" },
                t.min[0],
                t.max[0],
                t.min[1],
                t.max[1],
                if t.is_3d { format!("  Z {:.1}…{:.1}", t.min[2], t.max[2]) } else { String::new() }
            )).color(theme::text_dim()));
            ui.label(egui::RichText::new(format!(
                "{} {:.0} mm · {} {} · {}",
                tr("cut length"),
                t.cut_len,
                tr("Time"),
                crate::widgets::fmt_time(t.est_seconds),
                trf("{} moves", &[&t.segs.len()])
            )).color(theme::text_dim()));
            ui.separator();
            ui.label(egui::RichText::new(tr(if v.tp.is_3d { "Drag: rotate · right drag or Shift+drag: move · wheel: zoom" } else { "Drag: move · wheel: zoom" })).color(theme::text_dim()));
        });
    });
    let rect = ui.available_rect_before_wrap();
    let resp = ui.allocate_rect(rect, Sense::click_and_drag());
    let scale = v.fit_scale(rect) * v.cam.zoom;
    // Moving, rotating and zooming.
    let shift = ui.input(|i| i.modifiers.shift);
    let secondary = ui.input(|i| i.pointer.button_down(egui::PointerButton::Secondary) || i.pointer.button_down(egui::PointerButton::Middle));
    if resp.dragged() {
        let d = resp.drag_delta();
        if v.tp.is_3d && !shift && !secondary {
            v.cam.yaw -= d.x * 0.01;
            v.cam.pitch = (v.cam.pitch + d.y * 0.01).clamp(0.0, std::f32::consts::FRAC_PI_2);
        } else {
            v.cam.pan += d;
        }
    }
    if resp.hovered() {
        let scroll = ui.input(|i| i.smooth_scroll_delta.y + (i.zoom_delta() - 1.0) * 200.0);
        if scroll != 0.0 {
            let f = (scroll * 0.004).exp();
            let new_zoom = (v.cam.zoom * f).clamp(0.02, 2000.0);
            let f = new_zoom / v.cam.zoom;
            // The point under the pointer stays where it is.
            if let Some(p) = resp.hover_pos() {
                let off = p - rect.center();
                v.cam.pan = off - (off - v.cam.pan) * f;
            }
            v.cam.zoom = new_zoom;
        }
    }
    let painter = ui.painter_at(rect);
    painter.rect_filled(rect, 0.0, Color32::from_rgb(18, 20, 24));
    let scale = scale.max(1e-6);
    let centre = rect.center();
    let n = v.tp.segs.len();
    let step = n.div_ceil(DRAW_LIMIT).max(1);
    let z_span = (v.tp.max[2] - v.tp.min[2]).max(1e-6);
    let mut shapes = Vec::with_capacity(n.min(DRAW_LIMIT) + 8);
    let (mut rapids, mut cuts) = (vec![], vec![]);
    for (i, s) in v.tp.segs.iter().enumerate() {
        if step > 1 && i % step != 0 && s.cut {
            continue;
        }
        if !s.cut && (!v.show_rapid || (step > 1 && i % (step * 4) != 0)) {
            continue;
        }
        let a = v.project(s.a, centre, scale);
        let b = v.project(s.b, centre, scale);
        if !rect.expand(50.0).intersects(Rect::from_two_pos(a, b)) {
            continue;
        }
        if s.cut {
            let c = if v.tp.is_3d && v.by_height {
                height_colour((s.a[2] + s.b[2]) / 2.0 / z_span - v.tp.min[2] / z_span)
            } else {
                // A laser file: the harder the beam, the brighter.
                let k = 0.35 + 0.65 * s.power;
                Color32::from_rgb((255.0 * k) as u8, (120.0 * k) as u8, (40.0 * k) as u8)
            };
            cuts.push((a, b, c));
        } else {
            rapids.push((a, b));
        }
    }
    for (a, b) in rapids {
        shapes.push(Shape::line_segment([a, b], Stroke::new(0.8, Color32::from_rgba_unmultiplied(110, 140, 200, 110))));
    }
    for (a, b, c) in cuts {
        shapes.push(Shape::line_segment([a, b], Stroke::new(1.3, c)));
    }
    // The axes at the origin.
    let len = (v.tp.max[0] - v.tp.min[0]).max(v.tp.max[1] - v.tp.min[1]).max(1.0) * 0.12;
    for (d, col) in [([len, 0.0, 0.0], Color32::from_rgb(230, 80, 80)), ([0.0, len, 0.0], Color32::from_rgb(90, 200, 90)), ([0.0, 0.0, len], Color32::from_rgb(90, 140, 240))] {
        if d[2] > 0.0 && !v.tp.is_3d {
            continue;
        }
        shapes.push(Shape::line_segment([v.project([0.0; 3], centre, scale), v.project(d, centre, scale)], Stroke::new(1.5, col)));
    }
    painter.extend(shapes);
    if close {
        return false;
    }
    *view = Some(v);
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extensions_are_recognised_without_regard_to_case() {
        assert!(is_gcode(Path::new("a/b.NC")));
        assert!(is_gcode(Path::new("job.gcode")));
        assert!(!is_gcode(Path::new("drawing.svg")));
    }

    #[test]
    fn the_camera_starts_flat_for_a_laser_file_and_tilted_for_a_3d_one() {
        assert_eq!(Cam::home(false).pitch, 0.0);
        assert!(Cam::home(true).pitch > 0.5);
    }

    #[test]
    fn reset_returns_the_camera_home_and_a_file_without_moves_is_refused() {
        let dir = std::env::temp_dir().join(format!("lc-gview-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.nc");
        std::fs::write(&p, "G0 Z5\nG1 Z-1 F100\nG1 X10\n").unwrap();
        let mut v = GcodeViewer::load(p).unwrap();
        assert!(v.tp.is_3d);
        v.cam.zoom = 5.0;
        v.cam = Cam::home(v.tp.is_3d);
        assert_eq!(v.cam, Cam::home(true));
        let q = dir.join("b.nc");
        std::fs::write(&q, "; nothing\n").unwrap();
        assert!(GcodeViewer::load(q).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn zooming_keeps_the_point_under_the_pointer() {
        // pan' = off - (off - pan) * f maps the point at screen offset `off` to itself.
        let (off, pan, f) = (Vec2::new(40.0, -10.0), Vec2::new(5.0, 7.0), 2.5);
        let new_pan = off - (off - pan) * f;
        // A model point at screen offset q = pan + s*m before; after: new_pan + s*f*m.
        let m = (off - pan) / 3.0;
        assert!(((new_pan + 3.0 * f * m) - off).length() < 1e-4);
    }
}
