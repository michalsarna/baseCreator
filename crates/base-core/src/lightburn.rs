//! Reading LightBurn projects (`.lbrn2`, and the older `.lbrn` of the same XML shape) and MillMage projects (`.mage`, the same
//! shapes with `<Layer>` in place of `<CutSetting>` and `LayerIndex` in place of `CutIndex`) into a `Scene` and the settings
//! of its cut layers; and writing them (`write`).
//!
//! LightBurn writes coordinates the way its work area shows them: millimetres, Y up from the bottom left corner of
//! a machine with that origin. The scene of the programs here counts Y down from the top, so the height of the work
//! area is needed to turn the drawing over (`import`). Every shape of the project comes out as a shape of the scene:
//! rectangles, ellipses and bitmaps keep their kind, the rest (polygons, stars, paths, rounded rectangles, the
//! outlines of text) become editable Bézier contours. Shapes of a type this reader does not know are left out and
//! named in `Import::warnings`. The file is untrusted input: sizes are limited and nothing panics.
use crate::bezier::{Contour, Node};
use crate::geom::{Pt, Xf};
use crate::image::ImageData;
use crate::scene::{Kind, Scene};
use base64::Engine;

/// The extensions of the project files this module reads: LightBurn's and MillMage's.
pub const EXTENSIONS: [&str; 3] = ["lbrn2", "lbrn", "mage"];
/// The biggest file read, in bytes.
pub const MAX_BYTES: usize = 256 * 1024 * 1024;
/// The most shapes taken from one file.
pub const MAX_SHAPES: usize = 200_000;
/// The deepest nesting of groups followed.
const MAX_DEPTH: usize = 32;
/// The highest layer a scene shape gets (the programs have 30 layers, 0..=29).
pub const LAST_LAYER: usize = 29;

/// How a LightBurn layer cuts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CutKind {
    /// Follow the outlines (LightBurn: Line / Cut).
    Line,
    /// Fill the insides with scan lines (LightBurn: Fill / Scan).
    Fill,
    /// Concentric rings (LightBurn: Offset Fill).
    Offset,
    /// Engrave bitmaps (LightBurn: Image).
    Image,
}

/// The settings of one layer of the project.
#[derive(Clone, Debug, PartialEq)]
pub struct Cut {
    /// LightBurn's layer number (C00 is 0).
    pub index: usize,
    pub name: String,
    pub kind: CutKind,
    /// Percent.
    pub power_max: f64,
    pub power_min: f64,
    /// Millimetres per second.
    pub speed_mm_s: f64,
    pub passes: u32,
    /// Distance between scan lines in millimetres, when the file says.
    pub interval_mm: Option<f64>,
}

/// What a project file holds that these programs can use.
#[derive(Debug, Default)]
pub struct Import {
    pub scene: Scene,
    pub cuts: Vec<Cut>,
    /// The name of the machine the project was made for (`No Machine` when none).
    pub device_name: String,
    /// Things left out or changed, in English, for the user.
    pub warnings: Vec<String>,
}

fn num(s: Option<&str>) -> Option<f64> {
    s.and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite())
}

fn child_value<'a>(n: roxmltree::Node<'a, '_>, name: &str) -> Option<&'a str> {
    n.children().find(|c| c.has_tag_name(name)).and_then(|c| c.attribute("Value"))
}

fn parse_cut(n: roxmltree::Node) -> Option<Cut> {
    let index = num(child_value(n, "index"))? as i64;
    if !(0..=63).contains(&index) {
        return None;
    }
    let kind = match n.attribute("type").unwrap_or("Cut") {
        "Scan" | "Scan+Cut" => CutKind::Fill,
        "Offset" => CutKind::Offset,
        "Image" => CutKind::Image,
        _ => CutKind::Line,
    };
    let clamp = |v: f64, lo: f64, hi: f64| v.max(lo).min(hi);
    Some(Cut {
        index: index as usize,
        name: child_value(n, "name").unwrap_or("").to_string(),
        kind,
        power_max: clamp(num(child_value(n, "maxPower")).unwrap_or(20.0), 0.0, 100.0),
        power_min: clamp(num(child_value(n, "minPower")).unwrap_or(0.0), 0.0, 100.0),
        speed_mm_s: clamp(num(child_value(n, "speed")).unwrap_or(10.0), 0.0, 100_000.0),
        passes: clamp(num(child_value(n, "numPasses")).unwrap_or(1.0), 1.0, 1000.0) as u32,
        interval_mm: num(child_value(n, "interval")).filter(|v| *v > 0.0 && *v < 100.0),
    })
}

/// `a b c d e f` of an `XForm` element (the same meaning as `Xf`: x' = a x + c y + e, y' = b x + d y + f).
fn parse_xf(n: roxmltree::Node) -> Xf {
    let v: Vec<f64> = n.children().find(|c| c.has_tag_name("XForm")).and_then(|c| c.text()).map(|t| t.split_whitespace().filter_map(|w| w.parse().ok()).collect()).unwrap_or_default();
    if v.len() == 6 && v.iter().all(|x| x.is_finite()) {
        Xf { a: v[0], b: v[1], c: v[2], d: v[3], e: v[4], f: v[5] }
    } else {
        Xf::IDENTITY
    }
}

/// The scanner over a `VertList` text: `V x y c0x.. c0y.. c1x.. c1y..` repeated, a control point missing (written as a
/// lone `c0x1` or `c1x1`) when the vertex has none on that side.
struct Verts {
    p: Vec<Pt>,
    c0: Vec<Option<Pt>>,
    c1: Vec<Option<Pt>>,
}

fn read_number(b: &[u8], i: &mut usize) -> Option<f64> {
    while *i < b.len() && (b[*i] as char).is_whitespace() {
        *i += 1;
    }
    let start = *i;
    if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
        *i += 1;
    }
    while *i < b.len() && (b[*i].is_ascii_digit() || b[*i] == b'.') {
        *i += 1;
    }
    if *i < b.len() && (b[*i] == b'e' || b[*i] == b'E') {
        let save = *i;
        *i += 1;
        if *i < b.len() && (b[*i] == b'-' || b[*i] == b'+') {
            *i += 1;
        }
        if *i < b.len() && b[*i].is_ascii_digit() {
            while *i < b.len() && b[*i].is_ascii_digit() {
                *i += 1;
            }
        } else {
            *i = save;
        }
    }
    std::str::from_utf8(&b[start..*i]).ok()?.parse::<f64>().ok().filter(|v| v.is_finite())
}

fn parse_verts(text: &str) -> Result<Verts, String> {
    let b = text.as_bytes();
    let mut v = Verts { p: vec![], c0: vec![], c1: vec![] };
    let mut i = 0;
    while i < b.len() {
        match b[i] {
            b'V' => {
                i += 1;
                let x = read_number(b, &mut i).ok_or("a vertex has no x")?;
                let y = read_number(b, &mut i).ok_or("a vertex has no y")?;
                v.p.push(Pt::new(x, y));
                v.c0.push(None);
                v.c1.push(None);
            }
            b'c' if i + 2 < b.len() && !v.p.is_empty() => {
                let which = b[i + 1];
                let axis = b[i + 2];
                i += 3;
                let val = read_number(b, &mut i).ok_or("a control point has no value")?;
                let last = v.p.len() - 1;
                let slot = match which {
                    b'0' => &mut v.c0[last],
                    b'1' => &mut v.c1[last],
                    _ => continue,
                };
                // `c0x1` alone (no `c0y` after it) means "no control point"; both coordinates make one.
                match axis {
                    b'x' => *slot = Some(Pt::new(val, f64::NAN)),
                    b'y' => {
                        if let Some(q) = slot {
                            q.y = val;
                        }
                    }
                    _ => {}
                }
            }
            _ => i += 1,
        }
    }
    for s in v.c0.iter_mut().chain(v.c1.iter_mut()) {
        if s.is_some_and(|q| q.y.is_nan()) {
            *s = None;
        }
    }
    Ok(v)
}

/// The segments of a `PrimList`: (is a curve, from vertex, to vertex).
fn parse_prims(text: &str) -> Vec<(bool, usize, usize)> {
    let b = text.as_bytes();
    let mut out = vec![];
    let mut i = 0;
    while i < b.len() {
        let kind = b[i];
        if kind == b'L' || kind == b'B' {
            i += 1;
            let (Some(a), Some(c)) = (read_number(b, &mut i), read_number(b, &mut i)) else { break };
            if a >= 0.0 && c >= 0.0 {
                out.push((kind == b'B', a as usize, c as usize));
            }
        } else {
            i += 1;
        }
    }
    out
}

/// The contours drawn by `prims` over `verts`: a new one starts where a segment does not continue the last, and a
/// contour that ends where it began is closed.
fn contours(verts: &Verts, prims: &[(bool, usize, usize)]) -> Vec<Contour> {
    let mut out: Vec<Contour> = vec![];
    let mut cur: Option<(Contour, Vec<usize>)> = None;
    let finish = |c: Option<(Contour, Vec<usize>)>, out: &mut Vec<Contour>| {
        if let Some((mut c, ids)) = c {
            if ids.len() > 2 && ids.first() == ids.last() {
                c.nodes.pop();
                c.closed = true;
            }
            if c.nodes.len() > 1 {
                out.push(c);
            }
        }
    };
    let n = verts.p.len();
    let node = |i: usize| Node::corner(verts.p[i]);
    for &(curve, a, b) in prims {
        if a >= n || b >= n {
            continue;
        }
        let continues = cur.as_ref().is_some_and(|(_, ids)| ids.last() == Some(&a)) && cur.as_ref().is_some_and(|(_, ids)| !(ids.len() > 2 && ids.first() == ids.last()));
        if !continues {
            finish(cur.take(), &mut out);
            cur = Some((Contour { nodes: vec![node(a)], closed: false }, vec![a]));
        }
        let (c, ids) = cur.as_mut().expect("a contour is open");
        let last = c.nodes.len() - 1;
        c.nodes[last].hout = if curve { verts.c0[a].unwrap_or(verts.p[a]) } else { verts.p[a] };
        let mut nb = node(b);
        nb.hin = if curve { verts.c1[b].unwrap_or(verts.p[b]) } else { verts.p[b] };
        c.nodes.push(nb);
        ids.push(b);
    }
    finish(cur.take(), &mut out);
    out
}

/// The contours of a `Path` shape, or an error when the lists are broken.
fn path_contours(shape: roxmltree::Node, pool: &mut Pool) -> Result<Vec<Contour>, String> {
    let vid: Option<usize> = shape.attribute("VertID").and_then(|s| s.parse().ok());
    let pid: Option<usize> = shape.attribute("PrimID").and_then(|s| s.parse().ok());
    let vert_text = shape.children().find(|c| c.has_tag_name("VertList")).and_then(|c| c.text());
    let prim_text = shape.children().find(|c| c.has_tag_name("PrimList")).and_then(|c| c.text());
    // A shape may leave out a list that an earlier shape with the same number already gave.
    let verts = match (vert_text, vid) {
        (Some(t), id) => {
            let v = std::rc::Rc::new(parse_verts(t)?);
            if let Some(id) = id {
                pool.verts.insert(id, v.clone());
            }
            v
        }
        (None, Some(id)) => pool.verts.get(&id).cloned().ok_or("a path refers to vertices that are not in the file")?,
        _ => return Err("a path has no vertices".into()),
    };
    let prims = match (prim_text, pid) {
        (Some(t), id) => {
            let p = std::rc::Rc::new(parse_prims(t));
            if let Some(id) = id {
                pool.prims.insert(id, p.clone());
            }
            p
        }
        (None, Some(id)) => pool.prims.get(&id).cloned().ok_or("a path refers to segments that are not in the file")?,
        _ => return Err("a path has no segments".into()),
    };
    Ok(contours(&verts, &prims))
}

#[derive(Default)]
struct Pool {
    verts: std::collections::HashMap<usize, std::rc::Rc<Verts>>,
    prims: std::collections::HashMap<usize, std::rc::Rc<Vec<(bool, usize, usize)>>>,
}

pub(crate) fn rounded_rect(w: f64, h: f64, r: f64) -> Contour {
    let (hw, hh) = (w / 2.0, h / 2.0);
    let r = r.max(0.0).min(hw).min(hh);
    if r <= 0.0 {
        return Contour { nodes: vec![Node::corner(Pt::new(-hw, -hh)), Node::corner(Pt::new(hw, -hh)), Node::corner(Pt::new(hw, hh)), Node::corner(Pt::new(-hw, hh))], closed: true };
    }
    const K: f64 = 0.552_284_749_830_793_4;
    let k = r * K;
    let mut nodes = vec![];
    // Round the corners one after the other, starting after the top left one: two nodes per straight edge end.
    let mut push = |p: Pt, hin: Pt, hout: Pt| nodes.push(Node { p, hin, hout, smooth: false });
    push(Pt::new(-hw + r, -hh), Pt::new(-hw + r - k, -hh), Pt::new(-hw + r, -hh));
    push(Pt::new(hw - r, -hh), Pt::new(hw - r, -hh), Pt::new(hw - r + k, -hh));
    push(Pt::new(hw, -hh + r), Pt::new(hw, -hh + r - k), Pt::new(hw, -hh + r));
    push(Pt::new(hw, hh - r), Pt::new(hw, hh - r), Pt::new(hw, hh - r + k));
    push(Pt::new(hw - r, hh), Pt::new(hw - r + k, hh), Pt::new(hw - r, hh));
    push(Pt::new(-hw + r, hh), Pt::new(-hw + r, hh), Pt::new(-hw + r - k, hh));
    push(Pt::new(-hw, hh - r), Pt::new(-hw, hh - r + k), Pt::new(-hw, hh - r));
    push(Pt::new(-hw, -hh + r), Pt::new(-hw, -hh + r), Pt::new(-hw, -hh + r - k));
    Contour { nodes, closed: true }
}

/// The corners of a regular polygon or a star: the first one at the local `(0, -ry)`, which LightBurn turns up with
/// the mirror in the `XForm` it writes.
fn star_polygon(n: usize, rx: f64, ry: f64, inner: Option<f64>) -> Contour {
    let mut nodes = vec![];
    let step = std::f64::consts::TAU / n as f64;
    for k in 0..n {
        let t = step * k as f64;
        nodes.push(Node::corner(Pt::new(rx * t.sin(), -ry * t.cos())));
        if let Some(ratio) = inner {
            let t2 = t + step / 2.0;
            nodes.push(Node::corner(Pt::new(rx * ratio * t2.sin(), -ry * ratio * t2.cos())));
        }
    }
    Contour { nodes, closed: true }
}

struct Ctx<'a> {
    scene: Scene,
    flip: Xf,
    warnings: Vec<String>,
    pool: Pool,
    count: usize,
    skipped: std::collections::BTreeSet<String>,
    dropped_layers: bool,
    marker: std::marker::PhantomData<&'a ()>,
}

impl Ctx<'_> {
    fn layer(&mut self, shape: roxmltree::Node) -> usize {
        let i = num(shape.attribute("CutIndex").or_else(|| shape.attribute("LayerIndex"))).unwrap_or(0.0).max(0.0) as usize;
        if i > LAST_LAYER {
            self.dropped_layers = true;
        }
        i.min(LAST_LAYER)
    }

    fn add(&mut self, layer: usize, kind: Kind, xf: Xf, group: Option<u64>) {
        let id = self.scene.add(layer, kind, xf);
        if let Some(g) = group {
            if let Some(s) = self.scene.shape_mut(id) {
                s.group = Some(g);
            }
        }
    }

    fn bezier(&mut self, layer: usize, contours: Vec<Contour>, xf: Xf, group: Option<u64>) {
        let total = xf.then(self.flip);
        let cs: Vec<Contour> = contours.iter().map(|c| c.transformed(&total)).collect();
        if !cs.is_empty() {
            self.add(layer, Kind::Bezier(cs), Xf::IDENTITY, group);
        }
    }

    fn shape(&mut self, n: roxmltree::Node, parent: Xf, group: Option<u64>, depth: usize) {
        if self.count >= MAX_SHAPES || depth > MAX_DEPTH {
            return;
        }
        self.count += 1;
        let ty = n.attribute("Type").unwrap_or("");
        let own = parse_xf(n);
        let xf = own.then(parent);
        let layer = self.layer(n);
        let f = |k: &str| num(n.attribute(k));
        match ty {
            "Group" => {
                let g = group.unwrap_or_else(|| self.scene.new_group_id());
                if let Some(children) = n.children().find(|c| c.has_tag_name("Children")) {
                    for c in children.children().filter(|c| c.has_tag_name("Shape")) {
                        self.shape(c, xf, Some(g), depth + 1);
                    }
                }
            }
            "Rect" => {
                let (Some(w), Some(h)) = (f("W"), f("H")) else { return self.bad("Rect") };
                let r = f("Cr").unwrap_or(0.0);
                if r > 0.0 {
                    self.bezier(layer, vec![rounded_rect(w, h, r)], xf, group);
                } else {
                    let local = Xf::translate(-w / 2.0, -h / 2.0).then(xf).then(self.flip);
                    self.add(layer, Kind::Rect { w, h }, local, group);
                }
            }
            "Ellipse" => {
                let (Some(rx), Some(ry)) = (f("Rx"), f("Ry")) else { return self.bad("Ellipse") };
                let local = Xf::translate(-rx, -ry).then(xf).then(self.flip);
                self.add(layer, Kind::Ellipse { w: 2.0 * rx, h: 2.0 * ry }, local, group);
            }
            "Polygon" | "Star" => {
                let (Some(rx), Some(ry)) = (f("Rx"), f("Ry")) else { return self.bad(ty) };
                let corners = f("N").unwrap_or(0.0) as usize;
                if !(3..=1000).contains(&corners) {
                    return self.bad(ty);
                }
                let inner = if ty == "Star" { Some(f("Ra").unwrap_or(0.5)) } else { None };
                self.bezier(layer, vec![star_polygon(corners, rx, ry, inner)], xf, group);
            }
            "Path" => match path_contours(n, &mut self.pool) {
                Ok(cs) => self.bezier(layer, cs, xf, group),
                Err(e) => self.warnings.push(format!("a path was left out: {e}")),
            },
            "Text" => {
                // The file carries the outlines the text had when it was saved; they look the same everywhere.
                match n.children().find(|c| c.has_tag_name("BackupPath")) {
                    Some(b) => match path_contours(b, &mut self.pool) {
                        Ok(cs) => {
                            let bxf = parse_xf(b).then(parent);
                            self.bezier(layer, cs, bxf, group)
                        }
                        Err(e) => self.warnings.push(format!("a text was left out: {e}")),
                    },
                    None => self.warnings.push(format!("the text \"{}\" has no outlines in the file and was left out", n.attribute("Str").unwrap_or(""))),
                }
            }
            "Bitmap" => self.bitmap(n, xf, layer, group),
            other => {
                self.skipped.insert(other.to_string());
            }
        }
    }

    fn bad(&mut self, ty: &str) {
        self.warnings.push(format!("a {ty} with missing or wrong numbers was left out"));
    }

    fn bitmap(&mut self, n: roxmltree::Node, xf: Xf, layer: usize, group: Option<u64>) {
        let (Some(w), Some(h)) = (num(n.attribute("W")), num(n.attribute("H"))) else { return self.bad("Bitmap") };
        let Some(data) = n.attribute("Data") else { return self.warnings.push("a bitmap without its picture was left out".into()) };
        let clean: String = data.chars().filter(|c| !c.is_whitespace()).collect();
        let bytes = match base64::engine::general_purpose::STANDARD.decode(clean) {
            Ok(b) => b,
            Err(_) => return self.warnings.push("a bitmap with a broken picture was left out".into()),
        };
        let name = n.attribute("File").and_then(|f| f.rsplit(['/', '\\']).next()).filter(|s| !s.is_empty()).unwrap_or("picture");
        // The size on the work area: the picture's size in LightBurn's local units times what the transform scales.
        let sx = xf.a.hypot(xf.b);
        let sy = xf.c.hypot(xf.d);
        if w <= 0.0 || h <= 0.0 || sx <= 0.0 || sy <= 0.0 {
            return self.bad("Bitmap");
        }
        let (mm_w, mm_h) = (w * sx, h * sy);
        let mut img = match ImageData::from_bytes(&bytes, name, mm_w.max(mm_h)) {
            Ok(i) => i,
            Err(e) => return self.warnings.push(format!("a bitmap could not be read: {e}")),
        };
        img.w = mm_w;
        img.h = mm_h;
        // LightBurn's picture is centred on its local origin and upright when the file's Y axis points up; the scene's
        // Y points down, so the rows run the other way.
        let k_x = w / mm_w;
        let k_y = h / mm_h;
        let to_lb = Xf { a: k_x, b: 0.0, c: 0.0, d: -k_y, e: -w / 2.0, f: h / 2.0 };
        let local = to_lb.then(xf).then(self.flip);
        self.add(layer, Kind::Image(img), local, group);
    }
}

/// Read a LightBurn project. `area_h` is the height of the work area in millimetres: LightBurn's Y counts up from the
/// bottom, the scene's counts down from the top.
pub fn import(text: &str, area_h: f64) -> Result<Import, String> {
    if text.len() > MAX_BYTES {
        return Err("the LightBurn file is too big".into());
    }
    let opts = roxmltree::ParsingOptions { allow_dtd: false, nodes_limit: 5_000_000, ..Default::default() };
    let doc = roxmltree::Document::parse_with_options(text, opts).map_err(|e| format!("this is not a LightBurn project: {e}"))?;
    let root = doc.root_element();
    if !root.has_tag_name("LightBurnProject") && !root.has_tag_name("MillMageProject") {
        return Err("this is not a LightBurn project".into());
    }
    let mut cuts: Vec<Cut> = root.children().filter(|c| c.has_tag_name("CutSetting") || c.has_tag_name("CutSetting_Img") || c.has_tag_name("Layer")).filter_map(parse_cut).collect();
    cuts.sort_by_key(|c| c.index);
    cuts.dedup_by_key(|c| c.index);
    let area_h = if area_h.is_finite() { area_h } else { 0.0 };
    let mut cx = Ctx { scene: Scene::default(), flip: Xf { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: 0.0, f: area_h }, warnings: vec![], pool: Pool::default(), count: 0, skipped: Default::default(), dropped_layers: false, marker: std::marker::PhantomData };
    for s in root.children().filter(|c| c.has_tag_name("Shape")) {
        cx.shape(s, Xf::IDENTITY, None, 0);
    }
    if !cx.skipped.is_empty() {
        cx.warnings.push(format!("left out (not supported yet): {}", cx.skipped.iter().cloned().collect::<Vec<_>>().join(", ")));
    }
    if cx.dropped_layers {
        cx.warnings.push("shapes on LightBurn's tool layers T1 and T2 were put on the last layer".into());
    }
    if cx.count >= MAX_SHAPES {
        cx.warnings.push("the project has more shapes than are read; the rest was left out".into());
    }
    Ok(Import { scene: cx.scene, cuts, device_name: root.attribute("DeviceName").unwrap_or("").to_string(), warnings: cx.warnings })
}

/// Which project file `write` makes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Flavor {
    /// LightBurn's older `.lbrn` (`FormatVersion="0"`).
    Lbrn,
    /// LightBurn's `.lbrn2`.
    Lbrn2,
    /// MillMage's `.mage`: `<Layer>` in place of `<CutSetting>`, the size of the material from the work area.
    Mage,
}

/// What `write` needs to know of a layer of the project besides its shapes.
pub struct LayerInfo<'a> {
    pub name: &'a dyn Fn(usize) -> String,
    pub color: &'a dyn Fn(usize) -> [u8; 3],
}

fn xml_attr(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

fn fmt_num(v: f64) -> String {
    let s = format!("{v:.5}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.to_string() }
}

/// The `VertList` and `PrimList` of contours already in LightBurn's coordinates.
fn vert_prim_lists(cs: &[Contour]) -> (String, String) {
    use std::fmt::Write;
    let (mut v, mut p) = (String::new(), String::new());
    let mut base = 0usize;
    for c in cs {
        let n = c.nodes.len();
        if n < 2 {
            continue;
        }
        for nd in &c.nodes {
            let _ = write!(v, "V{} {}", fmt_num(nd.p.x), fmt_num(nd.p.y));
            if nd.has_out() {
                let _ = write!(v, "c0x{}c0y{}", fmt_num(nd.hout.x), fmt_num(nd.hout.y));
            } else {
                v.push_str("c0x1");
            }
            if nd.has_in() {
                let _ = write!(v, "c1x{}c1y{}", fmt_num(nd.hin.x), fmt_num(nd.hin.y));
            } else {
                v.push_str("c1x1");
            }
        }
        let segs = if c.closed { n } else { n - 1 };
        for i in 0..segs {
            let j = (i + 1) % n;
            let curve = c.nodes[i].has_out() || c.nodes[j].has_in();
            if !p.is_empty() {
                p.push(' ');
            }
            let _ = write!(p, "{}{} {}", if curve { 'B' } else { 'L' }, base + i, base + j);
        }
        base += n;
    }
    (v, p)
}

/// A grey picture as PNG bytes, as it is shown (a negative is turned over).
pub fn png_of(im: &ImageData) -> Option<Vec<u8>> {
    let gray: Vec<u8> = if im.invert { im.gray.iter().map(|g| 255 - g).collect() } else { im.gray.clone() };
    let img = image::GrayImage::from_raw(im.px_w, im.px_h, gray)?;
    let mut out = std::io::Cursor::new(Vec::new());
    img.write_to(&mut out, image::ImageFormat::Png).ok()?;
    Some(out.into_inner())
}

/// A LightBurn or MillMage project of the shapes `ids` (all when empty): every shape as a path (curves kept) or a bitmap, on the
/// layer of the same number, one cut setting (or MillMage layer) for each layer used. `area` is the work area (width, height) in
/// millimetres: LightBurn's Y counts up from the bottom.
pub fn write(scene: &Scene, ids: &[u64], area: (f64, f64), layers: &LayerInfo, flavor: Flavor) -> String {
    use std::fmt::Write;
    let picked = |x: &&crate::scene::Shape| ids.is_empty() || ids.contains(&x.id);
    let mut used: Vec<usize> = scene.shapes.iter().filter(picked).map(|x| x.layer).collect();
    used.sort_unstable();
    used.dedup();
    let flip = Xf { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: 0.0, f: area.1 };
    let mut s = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    let (root, idx) = match flavor {
        Flavor::Mage => ("MillMageProject", "LayerIndex"),
        _ => ("LightBurnProject", "CutIndex"),
    };
    let version = if flavor == Flavor::Lbrn { 0 } else { 1 };
    let _ = writeln!(s, "<{root} AppVersion=\"2.2.00\" DeviceName=\"No Machine\" FormatVersion=\"{version}\" MaterialHeight=\"0\" MirrorX=\"False\" MirrorY=\"False\" AskForSendName=\"True\" CurrentPage=\"0\" Page0Name=\"Page 1\">");
    if flavor == Flavor::Mage {
        let _ = writeln!(s, "    <CNCProjectSetup>\n        <SafeZ Value=\"20\"/>\n        <RetractZ Value=\"5\"/>\n        <UseRetractZ Value=\"1\"/>\n        <StockWidth Value=\"{}\"/>\n        <StockHeight Value=\"{}\"/>\n        <ProjectOrigin Value=\"0\"/>\n        <ProjUnits Value=\"1\"/>\n        <Z_Zero Value=\"0\"/>\n    </CNCProjectSetup>", fmt_num(area.0), fmt_num(area.1));
    }
    for &l in &used {
        let name = xml_attr(&(layers.name)(l));
        let only_pictures = scene.shapes.iter().filter(picked).filter(|x| x.layer == l).all(|x| x.is_image());
        match flavor {
            Flavor::Mage => {
                let c = (layers.color)(l);
                let _ = writeln!(s, "    <Layer>\n        <index Value=\"{l}\"/>\n        <color r=\"{}\" g=\"{}\" b=\"{}\"/>\n        <name Value=\"{name}\"/>\n        <hide Value=\"0\"/>\n        <fade Value=\"0\"/>\n        <type Value=\"1\"/>\n    </Layer>", c[0], c[1], c[2]);
            }
            _ if only_pictures => {
                let _ = writeln!(s, "    <CutSetting_Img type=\"Image\">\n        <index Value=\"{l}\"/>\n        <name Value=\"{name}\"/>\n        <priority Value=\"{l}\"/>\n    </CutSetting_Img>");
            }
            _ => {
                let _ = writeln!(s, "    <CutSetting type=\"Cut\">\n        <index Value=\"{l}\"/>\n        <name Value=\"{name}\"/>\n        <priority Value=\"{l}\"/>\n    </CutSetting>");
            }
        }
    }
    for (op, sh) in scene.shapes.iter().filter(picked).enumerate() {
        let op_attr = if flavor == Flavor::Mage { format!(" OpIndex=\"{op}\"") } else { String::new() };
        if let Kind::Image(im) = &sh.kind {
            let Some(png) = png_of(im) else { continue };
            // The picture is centred on its local origin with Y up; its size on the work area is in W and H, the rest of the
            // transform only turns, mirrors and moves it.
            let (sx, sy) = (sh.xf.a.hypot(sh.xf.b), sh.xf.c.hypot(sh.xf.d));
            if sx <= 0.0 || sy <= 0.0 {
                continue;
            }
            let (w, h) = (im.w * sx, im.h * sy);
            let from_lb = Xf { a: im.w / w, b: 0.0, c: 0.0, d: -im.h / h, e: im.w / 2.0, f: im.h / 2.0 };
            let x = from_lb.then(sh.xf).then(flip);
            let data = base64::engine::general_purpose::STANDARD.encode(png);
            let file = xml_attr(if im.name.is_empty() { "picture.png" } else { &im.name });
            let _ = writeln!(s, "    <Shape Type=\"Bitmap\"{op_attr} {idx}=\"{}\" W=\"{}\" H=\"{}\" Gamma=\"1\" Contrast=\"0\" Brightness=\"0\" EnhanceAmount=\"0\" EnhanceRadius=\"0\" EnhanceDenoise=\"0\" File=\"{file}\" SourceHash=\"0\" Data=\"{data}\">", sh.layer, fmt_num(w), fmt_num(h));
            let _ = writeln!(s, "        <XForm>{} {} {} {} {} {}</XForm>\n    </Shape>", fmt_num(x.a), fmt_num(x.b), fmt_num(x.c), fmt_num(x.d), fmt_num(x.e), fmt_num(x.f));
            continue;
        }
        let cs: Vec<Contour> = crate::export::contours_of(sh).iter().map(|c| c.transformed(&flip)).collect();
        let (v, p) = vert_prim_lists(&cs);
        if p.is_empty() {
            continue;
        }
        let _ = writeln!(s, "    <Shape Type=\"Path\"{op_attr} {idx}=\"{}\">\n        <XForm>1 0 0 1 0 0</XForm>\n        <VertList>{v}</VertList>\n        <PrimList>{p}</PrimList>\n    </Shape>", sh.layer);
    }
    let _ = writeln!(s, "    <Notes ShowOnLoad=\"0\" Notes=\"\"/>\n</{root}>");
    s
}
