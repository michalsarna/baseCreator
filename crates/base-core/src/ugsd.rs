//! Reads the design files of Universal G-code Sender's Designer (`.ugsd`, JSON). Only the format is followed; the
//! code is our own. The files describe milling: every shape carries a cut type, depth and feed rate, which the
//! programs turn into their own layers.
//!
//! UGS counts Y up from the bottom of the work area, the scene counts down from the top, so the shapes are flipped.
use crate::bezier::{Contour, Node};
use crate::geom::{Pt, Xf};
use crate::image::ImageData;
use crate::scene::{Kind, Scene};
use crate::text::TextData;
use base64::Engine;
use serde_json::Value;

pub const MAX_BYTES: usize = 256 * 1024 * 1024;
pub const MAX_SHAPES: usize = 200_000;
const MAX_DEPTH: usize = 32;
/// The size of the font the Designer draws texts with, in its units (millimetres).
const TEXT_SIZE: f64 = 18.0;
/// Radius of a point entity.
const POINT_RADIUS: f64 = 1.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutType {
    None,
    Pocket,
    VCarve,
    Surface,
    OnPath,
    InsidePath,
    OutsidePath,
    CenterDrill,
    LaserOnPath,
    LaserFill,
    LaserRaster,
    PlotterOnPath,
    PlotterFill,
    HeightMap,
}

impl CutType {
    fn parse(s: &str) -> CutType {
        match s {
            "POCKET" => CutType::Pocket,
            "VCARVE" => CutType::VCarve,
            "SURFACE" => CutType::Surface,
            "ON_PATH" => CutType::OnPath,
            "INSIDE_PATH" => CutType::InsidePath,
            "OUTSIDE_PATH" => CutType::OutsidePath,
            "CENTER_DRILL" => CutType::CenterDrill,
            "LASER_ON_PATH" => CutType::LaserOnPath,
            "LASER_FILL" => CutType::LaserFill,
            "LASER_RASTER" => CutType::LaserRaster,
            "PLOTTER_ON_PATH" => CutType::PlotterOnPath,
            "PLOTTER_FILL" => CutType::PlotterFill,
            "HEIGHT_MAP" => CutType::HeightMap,
            _ => CutType::None,
        }
    }
    /// Is the inside of the shapes worked, not only their outlines?
    pub fn is_fill(self) -> bool {
        matches!(self, CutType::Pocket | CutType::Surface | CutType::LaserFill | CutType::PlotterFill | CutType::VCarve)
    }
}

/// One group of shapes that are cut the same way; `index` is the scene layer they were put on.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    pub index: usize,
    pub cut: CutType,
    /// Depths in millimetres, positive downwards.
    pub start_depth: f64,
    pub cut_depth: f64,
    pub feed: f64,
    /// Percent of the spindle's top speed.
    pub spindle_pct: f64,
    pub passes: u32,
}

/// What the file says about the tool and the machine; all optional (many files have no settings).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub tool_diameter: Option<f64>,
    pub plunge_speed: Option<f64>,
    pub depth_per_pass: Option<f64>,
}

pub struct Import {
    pub scene: Scene,
    pub cuts: Vec<Cut>,
    pub settings: Settings,
    pub warnings: Vec<String>,
}

fn n(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(Value::as_f64).filter(|x| x.is_finite())
}

fn parse_xf(v: &Value) -> Xf {
    let Some(t) = v.get("transform") else { return Xf::IDENTITY };
    let g = |k: &str, d: f64| n(t, k).unwrap_or(d);
    Xf { a: g("m00", 1.0), b: g("m10", 0.0), c: g("m01", 0.0), d: g("m11", 1.0), e: g("m02", 0.0), f: g("m12", 0.0) }
}

fn pt(v: &Value) -> Option<Pt> {
    let a = v.as_array()?;
    let (x, y) = (a.first()?.as_f64()?, a.get(1)?.as_f64()?);
    (x.is_finite() && y.is_finite()).then(|| Pt::new(x, y))
}

fn cubic_ctrl(p0: Pt, c: Pt) -> Pt {
    Pt::new(p0.x + 2.0 / 3.0 * (c.x - p0.x), p0.y + 2.0 / 3.0 * (c.y - p0.y))
}

/// The contours of a PATH entity's segments, in the entity's own coordinates.
fn path_contours(v: &Value) -> Result<Vec<Contour>, String> {
    let segs = v.get("segments").and_then(Value::as_array).ok_or("a path without segments")?;
    let mut out: Vec<Contour> = vec![];
    let mut cur: Vec<Node> = vec![];
    let mut start: Option<Pt> = None;
    fn end(out: &mut Vec<Contour>, cur: &mut Vec<Node>, closed: bool) {
        let mut nodes = std::mem::take(cur);
        if closed && nodes.len() > 1 && nodes[0].p.dist(nodes[nodes.len() - 1].p) < 1e-9 {
            // The last point lands on the first one: the closing segment is that one.
            let last = nodes.pop().unwrap();
            nodes[0].hin = last.hin;
        }
        if nodes.len() >= 2 {
            out.push(Contour { nodes, closed });
        }
    }
    for s in segs {
        let ty = s.get("type").and_then(Value::as_str).unwrap_or("");
        let pts: Vec<Pt> = s.get("coordinates").and_then(Value::as_array).map(|a| a.iter().filter_map(pt).collect()).unwrap_or_default();
        match ty {
            "MOVE_TO" => {
                end(&mut out, &mut cur, false);
                let p = *pts.first().ok_or("a MOVE_TO without a point")?;
                start = Some(p);
                cur.push(Node::corner(p));
            }
            "LINE_TO" => {
                let p = *pts.first().ok_or("a LINE_TO without a point")?;
                if cur.is_empty() {
                    start = Some(p);
                }
                cur.push(Node::corner(p));
            }
            "QUAD_TO" | "CUBIC_TO" => {
                let need = if ty == "QUAD_TO" { 2 } else { 3 };
                if pts.len() < need || cur.is_empty() {
                    return Err("a curve without enough points".into());
                }
                let l = cur.last().unwrap().p;
                let (c1, c2, p) = if need == 2 { (cubic_ctrl(l, pts[0]), cubic_ctrl(pts[1], pts[0]), pts[1]) } else { (pts[0], pts[1], pts[2]) };
                cur.last_mut().unwrap().hout = c1;
                cur.push(Node { p, hin: c2, hout: p, smooth: false });
            }
            "CLOSE" => {
                end(&mut out, &mut cur, true);
                if let Some(p) = start {
                    // A new sub-path may go on from the same start without a MOVE_TO.
                    cur.push(Node::corner(p));
                }
            }
            _ => {}
        }
    }
    end(&mut out, &mut cur, false);
    Ok(out)
}

#[derive(Default)]
struct Ctx {
    scene: Scene,
    flip: Xf,
    cuts: Vec<Cut>,
    warnings: Vec<String>,
    count: usize,
    max_layers: usize,
    merged: bool,
    skipped: std::collections::BTreeSet<String>,
    unsupported: std::collections::BTreeSet<&'static str>,
}

impl Ctx {
    fn cut_index(&mut self, v: &Value) -> usize {
        let cut = CutType::parse(v.get("cutType").and_then(Value::as_str).unwrap_or(""));
        let c = Cut {
            index: 0,
            cut,
            start_depth: n(v, "startDepth").unwrap_or(0.0),
            cut_depth: n(v, "cutDepth").unwrap_or(0.0),
            feed: n(v, "feedRate").unwrap_or(0.0),
            spindle_pct: n(v, "spindleSpeed").unwrap_or(100.0),
            passes: n(v, "passes").unwrap_or(1.0).max(1.0) as u32,
        };
        match cut {
            CutType::VCarve => {
                self.unsupported.insert("V-carving");
            }
            CutType::Surface => {
                self.unsupported.insert("surfacing");
            }
            CutType::HeightMap => {
                self.unsupported.insert("height maps");
            }
            _ => {}
        }
        if c.start_depth > 0.0 {
            self.unsupported.insert("start depths");
        }
        if v.get("tabs").and_then(Value::as_array).is_some_and(|t| !t.is_empty()) || n(v, "tabCount").is_some_and(|t| t > 0.0) {
            self.unsupported.insert("tabs");
        }
        if let Some(i) = self.cuts.iter().position(|o| Cut { index: o.index, ..c.clone() } == *o) {
            return self.cuts[i].index;
        }
        if self.cuts.len() >= self.max_layers {
            self.merged = true;
            return self.max_layers - 1;
        }
        let index = self.cuts.len();
        self.cuts.push(Cut { index, ..c });
        index
    }

    fn add(&mut self, layer: usize, kind: Kind, xf: Xf, group: Option<u64>) {
        let id = self.scene.add(layer, kind, xf);
        if let Some(g) = group {
            if let Some(s) = self.scene.shape_mut(id) {
                s.group = Some(g);
            }
        }
    }

    fn entity(&mut self, v: &Value, group: Option<u64>, depth: usize) {
        if self.count >= MAX_SHAPES || depth > MAX_DEPTH {
            return;
        }
        self.count += 1;
        let ty = v.get("type").and_then(Value::as_str).unwrap_or("");
        if ty == "GROUP" {
            let g = group.unwrap_or_else(|| self.scene.new_group_id());
            if let Some(children) = v.get("children").and_then(Value::as_array) {
                for c in children {
                    self.entity(c, Some(g), depth + 1);
                }
            }
            return;
        }
        let xf = parse_xf(v);
        let flip = self.flip;
        match ty {
            "PATH" => match path_contours(v) {
                Ok(cs) if !cs.is_empty() => {
                    let layer = self.cut_index(v);
                    let total = xf.then(flip);
                    self.add(layer, Kind::Bezier(cs.iter().map(|c| c.transformed(&total)).collect()), Xf::IDENTITY, group);
                }
                Ok(_) => {}
                Err(e) => self.warnings.push(format!("a path was left out: {e}")),
            },
            "RECTANGLE" => {
                // A unit square scaled by the transform; the corner radius is in millimetres on the work area.
                let (w, h) = (xf.a.hypot(xf.b), xf.c.hypot(xf.d));
                let r = n(v, "cornerRadius").unwrap_or(0.0);
                let layer = self.cut_index(v);
                if r > 0.0 && w > 0.0 && h > 0.0 {
                    let (rx, ry) = (r / w, r / h);
                    // Different radii on the two axes of the unit square make the corners circular after scaling.
                    let c = rounded_unit_square(rx, ry);
                    let total = Xf::translate(0.5, 0.5).then(xf).then(flip);
                    self.add(layer, Kind::Bezier(vec![c.transformed(&total)]), Xf::IDENTITY, group);
                } else {
                    let local = xf.then(flip);
                    // Local coordinates start at the corner (0, 0) of the unit square.
                    self.add(layer, Kind::Rect { w: 1.0, h: 1.0 }, local, group);
                }
            }
            "ELLIPSE" => {
                let layer = self.cut_index(v);
                // Ellipse2D(0, 0, 10, 10): ten units across.
                self.add(layer, Kind::Ellipse { w: 10.0, h: 10.0 }, xf.then(flip), group);
            }
            "POINT" => {
                let layer = self.cut_index(v);
                let local = Xf::translate(-POINT_RADIUS, -POINT_RADIUS).then(xf).then(flip);
                self.add(layer, Kind::Ellipse { w: 2.0 * POINT_RADIUS, h: 2.0 * POINT_RADIUS }, local, group);
            }
            "TEXT" => {
                let text = v.get("text").and_then(Value::as_str).unwrap_or("").to_string();
                if text.is_empty() {
                    return;
                }
                let name = v.get("fontName").and_then(Value::as_str).unwrap_or("");
                let lower = name.to_lowercase();
                let t = TextData {
                    text,
                    family: if crate::text::families().iter().any(|f| f.eq_ignore_ascii_case(name)) { name.to_string() } else { crate::text::DEFAULT_FAMILY.to_string() },
                    size: TEXT_SIZE,
                    bold: lower.contains("bold"),
                    italic: lower.contains("italic"),
                    ..TextData::default()
                };
                let layer = self.cut_index(v);
                // The text's outline has its baseline at the origin and Y up; our text box has its top-left at the origin.
                let asc = crate::text::ascent(&t);
                let local = Xf { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: 0.0, f: asc }.then(xf).then(flip);
                self.add(layer, Kind::Text(t), local, group);
            }
            "RASTER" => self.raster(v, xf, group),
            other => {
                self.skipped.insert(other.to_string());
            }
        }
    }

    fn raster(&mut self, v: &Value, xf: Xf, group: Option<u64>) {
        let Some(data) = v.get("image").and_then(Value::as_str) else { return self.warnings.push("a picture without its data was left out".into()) };
        let data = data.rsplit(',').next().unwrap_or(data);
        let clean: String = data.chars().filter(|c| !c.is_whitespace()).collect();
        let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(clean) else { return self.warnings.push("a picture with broken data was left out".into()) };
        let name = v.get("name").and_then(Value::as_str).filter(|s| !s.is_empty()).unwrap_or("picture");
        let probe = match ImageData::from_bytes(&bytes, name, 1.0) {
            Ok(i) => i,
            Err(e) => return self.warnings.push(format!("a picture could not be read: {e}")),
        };
        // The picture is as big as its pixels in the Designer's units before the transform scales it.
        let (px_w, px_h) = (probe.px_w as f64, probe.px_h as f64);
        let (sx, sy) = (xf.a.hypot(xf.b), xf.c.hypot(xf.d));
        if px_w <= 0.0 || px_h <= 0.0 || sx <= 0.0 || sy <= 0.0 {
            return self.warnings.push("a picture with a wrong size was left out".into());
        }
        let (mm_w, mm_h) = (px_w * sx, px_h * sy);
        let mut img = probe;
        img.w = mm_w;
        img.h = mm_h;
        let (kx, ky) = (px_w / mm_w, px_h / mm_h);
        // Rows run down in the picture and up in the Designer.
        let to_ugs = Xf { a: kx, b: 0.0, c: 0.0, d: -ky, e: 0.0, f: px_h };
        let local = to_ugs.then(xf).then(self.flip);
        let layer = self.cut_index(v);
        self.add(layer, Kind::Image(img), local, group);
    }
}

/// The unit square, centred on the origin, with corners of the radii `rx` and `ry`.
fn rounded_unit_square(rx: f64, ry: f64) -> Contour {
    let (rx, ry) = (rx.min(0.5), ry.min(0.5));
    const K: f64 = 0.552_284_749_830_793_4;
    let (kx, ky) = (rx * K, ry * K);
    let (l, r, t, b) = (-0.5, 0.5, -0.5, 0.5);
    let mut nodes = vec![];
    let mut push = |p: Pt, hin: Pt, hout: Pt| nodes.push(Node { p, hin, hout, smooth: false });
    push(Pt::new(l + rx, t), Pt::new(l + rx - kx, t), Pt::new(l + rx, t));
    push(Pt::new(r - rx, t), Pt::new(r - rx, t), Pt::new(r - rx + kx, t));
    push(Pt::new(r, t + ry), Pt::new(r, t + ry - ky), Pt::new(r, t + ry));
    push(Pt::new(r, b - ry), Pt::new(r, b - ry), Pt::new(r, b - ry + ky));
    push(Pt::new(r - rx, b), Pt::new(r - rx + kx, b), Pt::new(r - rx, b));
    push(Pt::new(l + rx, b), Pt::new(l + rx, b), Pt::new(l + rx - kx, b));
    push(Pt::new(l, b - ry), Pt::new(l, b - ry + ky), Pt::new(l, b - ry));
    push(Pt::new(l, t + ry), Pt::new(l, t + ry), Pt::new(l, t + ry - ky));
    Contour { nodes, closed: true }
}

/// Read a Designer file. `area_h` is the height of the work area in millimetres; the shapes are put on at most
/// `max_layers` layers (the rest of the ways to cut share the last one).
pub fn import(text: &str, area_h: f64, max_layers: usize) -> Result<Import, String> {
    if text.len() > MAX_BYTES {
        return Err("the UGS design file is too big".into());
    }
    let root: Value = serde_json::from_str(text).map_err(|e| format!("this is not a UGS design: {e}"))?;
    let entities = root.get("entities").and_then(Value::as_array).ok_or("this is not a UGS design")?;
    match root.get("version").and_then(|v| v.as_str().map(str::to_string).or_else(|| v.as_i64().map(|i| i.to_string()))) {
        Some(v) if v == "1" => {}
        Some(v) => return Err(format!("this version of the UGS design file ({v}) is not supported")),
        None => return Err("this is not a UGS design".into()),
    }
    let settings = root.get("settings").map(|s| Settings { tool_diameter: n(s, "toolDiameter").filter(|d| *d > 0.0), plunge_speed: n(s, "plungeSpeed").filter(|d| *d > 0.0), depth_per_pass: n(s, "depthPerPass").filter(|d| *d > 0.0) }).unwrap_or_default();
    let area_h = if area_h.is_finite() { area_h } else { 0.0 };
    let mut cx = Ctx { max_layers: max_layers.max(1), flip: Xf { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: 0.0, f: area_h }, ..Default::default() };
    for e in entities {
        cx.entity(e, None, 0);
    }
    if !cx.skipped.is_empty() {
        cx.warnings.push(format!("left out (not supported yet): {}", cx.skipped.iter().cloned().collect::<Vec<_>>().join(", ")));
    }
    if !cx.unsupported.is_empty() {
        cx.warnings.push(format!("not supported yet, imported as plain cuts: {}", cx.unsupported.iter().cloned().collect::<Vec<_>>().join(", ")));
    }
    if cx.merged {
        cx.warnings.push("there are more ways to cut than layers; the last ones share the last layer".into());
    }
    if cx.count >= MAX_SHAPES {
        cx.warnings.push("the design has more shapes than are read; the rest was left out".into());
    }
    Ok(Import { scene: cx.scene, cuts: cx.cuts, settings, warnings: cx.warnings })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DESIGN: &str = r#"{"version":"1","entities":[
      {"type":"RECTANGLE","cutType":"OUTSIDE_PATH","cutDepth":15,"feedRate":1000,"spindleSpeed":100,"transform":{"m00":264,"m10":0,"m01":0,"m11":130,"m02":3,"m12":2},"cornerRadius":0},
      {"type":"GROUP","children":[
        {"type":"ELLIPSE","cutType":"POCKET","cutDepth":5,"feedRate":800,"transform":{"m00":2,"m10":0,"m01":0,"m11":2,"m02":10,"m12":20}},
        {"type":"PATH","cutType":"POCKET","cutDepth":5,"feedRate":800,"transform":{"m00":1,"m10":0,"m01":0,"m11":1,"m02":0,"m12":0},
         "segments":[{"type":"MOVE_TO","coordinates":[[0,0]]},{"type":"LINE_TO","coordinates":[[10,0]]},{"type":"QUAD_TO","coordinates":[[10,10],[0,10]]},{"type":"CLOSE","coordinates":[]}]},
        {"type":"PATH","isHidden":true,"cutType":"ON_PATH","segments":[{"type":"MOVE_TO","coordinates":[[0,0]]},{"type":"LINE_TO","coordinates":[[1,1]]}]}
      ]}]}"#;

    #[test]
    fn shapes_land_on_the_work_area_upright() {
        let imp = import(DESIGN, 300.0, 16).unwrap();
        assert_eq!(imp.scene.shapes.len(), 4);
        // The rectangle: 264 x 130 at (3, 2) counted from the bottom of a 300 mm high area.
        let b = imp.scene.shapes[0].bounds().unwrap();
        assert!((b.width() - 264.0).abs() < 1e-6 && (b.height() - 130.0).abs() < 1e-6);
        assert!((b.min.x - 3.0).abs() < 1e-6 && (b.min.y - (300.0 - 132.0)).abs() < 1e-6, "{b:?}");
        // The ellipse: 20 mm across, its box starting at (10, 20) from the bottom.
        let e = imp.scene.shapes[1].bounds().unwrap();
        assert!((e.width() - 20.0).abs() < 0.2 && (e.min.y - (300.0 - 40.0)).abs() < 0.2, "{e:?}");
    }

    #[test]
    fn cuts_are_grouped_and_hidden_shapes_are_drawn() {
        let imp = import(DESIGN, 300.0, 16).unwrap();
        assert_eq!(imp.cuts.len(), 3);
        assert_eq!(imp.cuts[0].cut, CutType::OutsidePath);
        assert_eq!(imp.cuts[1].cut, CutType::Pocket);
        assert_eq!(imp.scene.shapes[1].layer, 1);
        assert_eq!(imp.scene.shapes[2].layer, 1);
        assert_eq!(imp.cuts[2].cut, CutType::OnPath, "the hidden path is drawn too");
        assert!(imp.warnings.is_empty());
        assert_eq!(imp.scene.shapes[1].group, imp.scene.shapes[2].group);
    }

    #[test]
    fn a_closed_path_has_a_quadratic_curve_as_a_cubic() {
        let imp = import(DESIGN, 300.0, 16).unwrap();
        let Kind::Bezier(cs) = &imp.scene.shapes[2].kind else { panic!() };
        assert_eq!(cs.len(), 1);
        assert!(cs[0].closed);
        assert_eq!(cs[0].nodes.len(), 3);
    }

    #[test]
    fn wrong_files_are_refused() {
        assert!(import("nope", 100.0, 16).is_err());
        assert!(import(r#"{"version":"2","entities":[]}"#, 100.0, 16).is_err());
        assert!(import(r#"{"hello":1}"#, 100.0, 16).is_err());
    }

    #[test]
    fn rounded_rectangle_corners_are_circular() {
        let d = r#"{"version":"1","entities":[{"type":"RECTANGLE","cutType":"ON_PATH","cornerRadius":5,"transform":{"m00":100,"m10":0,"m01":0,"m11":20,"m02":0,"m12":0}}]}"#;
        let imp = import(d, 50.0, 16).unwrap();
        let b = imp.scene.shapes[0].bounds().unwrap();
        assert!((b.width() - 100.0).abs() < 1e-6 && (b.height() - 20.0).abs() < 1e-6);
        let Kind::Bezier(cs) = &imp.scene.shapes[0].kind else { panic!() };
        // The first straight edge starts five millimetres from the left edge.
        assert!((cs[0].nodes[0].p.x - 5.0).abs() < 1e-6);
    }

    #[test]
    fn examples_of_the_user_are_read_when_present() {
        let dir = std::path::Path::new(&std::env::var("HOME").unwrap_or_default()).join("Documents/uGS-examples");
        let Ok(rd) = std::fs::read_dir(dir) else { return };
        for f in rd.flatten().filter(|f| f.path().extension().is_some_and(|e| e == "ugsd")) {
            let imp = import(&std::fs::read_to_string(f.path()).unwrap(), 400.0, 16).unwrap();
            assert!(!imp.scene.shapes.is_empty(), "{:?}", f.path());
        }
    }
}
