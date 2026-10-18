//! Running the scripts of plugins (Rhai). A script can only compute: it has no access to files, the network, the machine
//! or the system. Its time and memory are limited. What it returns is data (shapes, text lines) that the program checks
//! before it uses it.
//!
//! The functions a script defines:
//!
//! * importer: `fn read(text, ctx)` returns shapes (`data` is a blob of bytes when the importer is `binary`; the values of its
//!   form are in `ctx.p`; an importer with `form` also has `fn form(data, ctx)` that returns more fields of the form)
//! * generator: `fn generate(p, ctx)` returns shapes (`p` holds the values of the form)
//! * postprocessor: `fn post(lines, ctx)` returns the lines of the program
//!
//! `ctx` is `#{ app, area_w, area_h }` (work area in mm). Shapes are built with `rect`, `rounded_rect`, `circle`,
//! `ellipse`, `line`, `polyline`, `polygon`, `path` and `text`, moved with `translate`, `rotate`, `scale` and put on a
//! layer with `s.layer = n`; `group([...])` makes shapes move together. `warn("...")` adds a message. `union_polygons(polys)`
//! merges any number of overlapping closed polygons (`[[x, y], ...]`) into the outlines of the area they cover. `read_f32(data, pos)` and
//! `read_f64(data, pos)` read a little-endian float out of a blob (integers: `data.parse_le_int(pos, len)`). `read_stl(data)` reads an
//! STL mesh; it has `tri_count()`, `bounds()`, `rotated(yaw, pitch)` (the mesh seen from another side), `groups(angle, dist)`, `project()`, `project(ids)` and `section(z)` (rings as arrays of
//! `[x, y]`), and `xf_rings(rings, sx, sy, dx, dy)` moves and mirrors rings.
use crate::bezier::{Contour, Node};
use crate::geom::{Pt, Xf};
use crate::plugins::{GeneratorDef, ImporterDef, ParamDef, ParamKind, Plugin, PostDef};
use crate::scene::{Kind, Scene};
use crate::text::TextData;
use rhai::{Array, Blob, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Scope, AST, FLOAT, INT};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub const MAX_SECONDS: u64 = 10;
/// The biggest file an importer is given.
pub const MAX_IMPORT_BYTES: u64 = 64_000_000;
pub const MAX_SHAPES: usize = 200_000;
pub const MAX_NODES: usize = 2_000_000;
pub const MAX_LINES: usize = 5_000_000;
pub const MAX_LINE_LEN: usize = 240;
const MAX_COORD: f64 = 1.0e7;
const MAX_DEPTH: usize = 16;

/// What an importer or a generator gives back.
#[derive(Debug)]
pub struct Shapes {
    pub scene: Scene,
    pub warnings: Vec<String>,
}

/// The facts a script may look at.
#[derive(Clone, Debug)]
pub struct Ctx {
    pub app: String,
    pub area_w: f64,
    pub area_h: f64,
}

impl Ctx {
    fn map(&self) -> Map {
        self.map_with(Map::new())
    }

    /// The map the script gets, with the values of the form in `p`.
    fn map_with(&self, p: Map) -> Map {
        let mut m = Map::new();
        m.insert("p".into(), Dynamic::from(p));
        m.insert("app".into(), Dynamic::from(self.app.clone()));
        m.insert("area_w".into(), Dynamic::from_float(self.area_w));
        m.insert("area_h".into(), Dynamic::from_float(self.area_h));
        m
    }
}

fn num(d: &Dynamic) -> Option<f64> {
    d.as_float().ok().or_else(|| d.as_int().ok().map(|i| i as f64)).filter(|v| v.is_finite() && v.abs() <= MAX_COORD)
}

fn field(m: &Map, k: &str) -> Result<f64, String> {
    m.get(k).and_then(num).ok_or_else(|| format!("{k:?} must be a number"))
}

fn field_or(m: &Map, k: &str, d: f64) -> Result<f64, String> {
    match m.get(k) {
        None => Ok(d),
        Some(v) => num(v).ok_or_else(|| format!("{k:?} must be a number")),
    }
}

fn pt_of(d: &Dynamic) -> Option<Pt> {
    let a = d.clone().try_cast::<Array>()?;
    Some(Pt::new(num(a.first()?)?, num(a.get(1)?)?))
}

fn shape(kind: &str, pairs: &[(&str, Dynamic)]) -> Map {
    let mut m = Map::new();
    m.insert("type".into(), Dynamic::from(kind.to_string()));
    for (k, v) in pairs {
        m.insert((*k).into(), v.clone());
    }
    m
}

fn f(v: FLOAT) -> Dynamic {
    Dynamic::from_float(v)
}

fn compose(s: &mut Map, xf: Xf) {
    let cur = xf_of(s.get("xf")).unwrap_or(Xf::IDENTITY);
    let n = cur.then(xf);
    let arr: Array = [n.a, n.b, n.c, n.d, n.e, n.f].iter().map(|v| f(*v)).collect();
    s.insert("xf".into(), Dynamic::from(arr));
}

fn xf_of(d: Option<&Dynamic>) -> Option<Xf> {
    let a = d?.clone().try_cast::<Array>()?;
    if a.len() != 6 {
        return None;
    }
    let v: Vec<f64> = a.iter().filter_map(num).collect();
    (v.len() == 6).then(|| Xf { a: v[0], b: v[1], c: v[2], d: v[3], e: v[4], f: v[5] })
}

/// A mesh read by `read_stl`, as the script holds it. The planes found by `groups` are kept for `project`.
#[derive(Clone)]
struct MeshH {
    mesh: Rc<crate::mesh::Mesh>,
    groups: Rc<RefCell<Vec<crate::mesh::Group>>>,
}

fn ring_array(rings: &[crate::geom::Polyline]) -> Array {
    rings.iter().map(|r| Dynamic::from(r.pts.iter().map(|q| Dynamic::from(vec![f(q.x), f(q.y)])).collect::<Array>())).collect()
}

/// The engine with limits, the shape functions and the collected messages.
struct Run {
    engine: Engine,
    warnings: Rc<RefCell<Vec<String>>>,
}

fn engine() -> Run {
    engine_for(0)
}

/// The most elements of an array or a blob a script may make; a binary importer may be given a bigger blob.
const MAX_ARRAY: usize = 500_000;

/// The engine for a file of `file_bytes` bytes (a binary file is given to the script as a blob, which counts as an array).
fn engine_for(file_bytes: usize) -> Run {
    let mut e = Engine::new();
    e.set_max_operations(200_000_000);
    e.set_max_call_levels(48);
    e.set_max_expr_depths(64, 32);
    e.set_max_string_size(2_000_000);
    e.set_max_array_size(MAX_ARRAY.max(file_bytes));
    e.set_max_map_size(20_000);
    e.disable_symbol("eval");
    let start = Instant::now();
    e.on_progress(move |ops| {
        if ops & 0x3ff == 0 && start.elapsed() > Duration::from_secs(MAX_SECONDS) {
            return Some(Dynamic::from("the script ran too long".to_string()));
        }
        None
    });
    let warnings: Rc<RefCell<Vec<String>>> = Rc::new(RefCell::new(vec![]));
    let w = warnings.clone();
    e.on_print(move |s| {
        let mut w = w.borrow_mut();
        if w.len() < 100 {
            w.push(s.chars().take(300).collect());
        }
    });
    let w = warnings.clone();
    e.register_fn("warn", move |s: ImmutableString| {
        let mut w = w.borrow_mut();
        if w.len() < 100 {
            w.push(s.chars().take(300).collect());
        }
    });
    e.register_fn("rect", |x: FLOAT, y: FLOAT, w: FLOAT, h: FLOAT| shape("rect", &[("x", f(x)), ("y", f(y)), ("w", f(w)), ("h", f(h))]));
    e.register_fn("rounded_rect", |x: FLOAT, y: FLOAT, w: FLOAT, h: FLOAT, r: FLOAT| shape("rounded_rect", &[("x", f(x)), ("y", f(y)), ("w", f(w)), ("h", f(h)), ("r", f(r))]));
    e.register_fn("circle", |cx: FLOAT, cy: FLOAT, r: FLOAT| shape("ellipse", &[("x", f(cx - r)), ("y", f(cy - r)), ("w", f(2.0 * r)), ("h", f(2.0 * r))]));
    e.register_fn("ellipse", |cx: FLOAT, cy: FLOAT, rx: FLOAT, ry: FLOAT| shape("ellipse", &[("x", f(cx - rx)), ("y", f(cy - ry)), ("w", f(2.0 * rx)), ("h", f(2.0 * ry))]));
    e.register_fn("line", |x1: FLOAT, y1: FLOAT, x2: FLOAT, y2: FLOAT| {
        let pts: Array = vec![Dynamic::from(vec![f(x1), f(y1)]), Dynamic::from(vec![f(x2), f(y2)])];
        shape("polyline", &[("points", Dynamic::from(pts)), ("closed", Dynamic::from(false))])
    });
    e.register_fn("polyline", |pts: Array| shape("polyline", &[("points", Dynamic::from(pts)), ("closed", Dynamic::from(false))]));
    e.register_fn("polygon", |pts: Array| shape("polyline", &[("points", Dynamic::from(pts)), ("closed", Dynamic::from(true))]));
    e.register_fn("path", |segs: Array| shape("path", &[("segments", Dynamic::from(segs))]));
    e.register_fn("text", |s: ImmutableString, x: FLOAT, y: FLOAT, size: FLOAT| shape("text", &[("text", Dynamic::from(s.to_string())), ("x", f(x)), ("y", f(y)), ("size", f(size))]));
    e.register_fn("group", |children: Array| shape("group", &[("children", Dynamic::from(children))]));
    e.register_fn("translate", |mut s: Map, dx: FLOAT, dy: FLOAT| {
        compose(&mut s, Xf::translate(dx, dy));
        s
    });
    e.register_fn("rotate", |mut s: Map, deg: FLOAT, cx: FLOAT, cy: FLOAT| {
        compose(&mut s, Xf::rotate_about(deg.to_radians(), Pt::new(cx, cy)));
        s
    });
    e.register_fn("scale", |mut s: Map, sx: FLOAT, sy: FLOAT| {
        compose(&mut s, Xf::scale(sx, sy));
        s
    });
    // Meshes (STL): the loops over the triangles are native; the script chooses what to ask for.
    e.register_type_with_name::<MeshH>("Mesh");
    e.register_fn("read_stl", |data: Blob| -> Result<MeshH, Box<EvalAltResult>> {
        let mesh = crate::mesh::Mesh::from_stl(&data).map_err(|e| e.to_string())?;
        Ok(MeshH { mesh: Rc::new(mesh), groups: Rc::new(RefCell::new(vec![])) })
    });
    e.register_fn("tri_count", |m: &mut MeshH| m.mesh.len() as INT);
    e.register_fn("bounds", |m: &mut MeshH| -> Array { m.mesh.bounds().map(|b| b.iter().map(|v| f(*v)).collect()).unwrap_or_default() });
    // The planes of the mesh, largest first, as maps #{ id, nx, ny, nz, d, area, tris, min_z, max_z } (at most 1000; the
    // ids are what `project` takes). `angle` is the tolerance of the normals in degrees, `dist` of the planes in mm.
    e.register_fn("groups", |m: &mut MeshH, angle: FLOAT, dist: FLOAT| -> Array {
        let mut g = m.mesh.plane_groups(angle, dist);
        g.truncate(1000);
        let out = g
            .iter()
            .enumerate()
            .map(|(i, x)| {
                let mut d = Map::new();
                for (k, v) in [("id", Dynamic::from(i as INT)), ("nx", f(x.n[0])), ("ny", f(x.n[1])), ("nz", f(x.n[2])), ("d", f(x.d)), ("area", f(x.area)), ("tris", Dynamic::from(x.tris.len() as INT)), ("min_z", f(x.min_z)), ("max_z", f(x.max_z))] {
                    d.insert(k.into(), v);
                }
                Dynamic::from(d)
            })
            .collect();
        *m.groups.borrow_mut() = g;
        out
    });
    // Move and mirror rings (arrays of [x, y]): x' = sx * x + dx, y' = sy * y + dy, in one native pass.
    e.register_fn("xf_rings", |rings: Array, sx: FLOAT, sy: FLOAT, dx: FLOAT, dy: FLOAT| -> Result<Array, Box<EvalAltResult>> {
        let mut out = Array::with_capacity(rings.len());
        for r in rings {
            let pts = r.try_cast::<Array>().ok_or("xf_rings: a ring is an array of [x, y] points")?;
            let mut moved = Array::with_capacity(pts.len());
            for q in &pts {
                let p = pt_of(q).ok_or("xf_rings: a point is [x, y]")?;
                moved.push(Dynamic::from(vec![f(sx * p.x + dx), f(sy * p.y + dy)]));
            }
            out.push(Dynamic::from(moved));
        }
        Ok(out)
    });
    // The mesh as seen from another side (the planes found before keep their triangles).
    e.register_fn("rotated", |m: &mut MeshH, yaw: FLOAT, pitch: FLOAT| MeshH { mesh: Rc::new(m.mesh.rotated(yaw, pitch)), groups: m.groups.clone() });
    e.register_fn("project", |m: &mut MeshH| -> Array { ring_array(&m.mesh.project(None)) });
    e.register_fn("project", |m: &mut MeshH, ids: Array| -> Result<Array, Box<EvalAltResult>> {
        let groups = m.groups.borrow();
        let mut tris: Vec<u32> = vec![];
        for id in &ids {
            let i = id.as_int().map_err(|_| "project: the ids are the numbers groups() gave")?;
            let g = usize::try_from(i).ok().and_then(|i| groups.get(i)).ok_or("project: a group that groups() did not make")?;
            tris.extend_from_slice(&g.tris);
        }
        Ok(ring_array(&m.mesh.project(Some(&tris))))
    });
    let w = warnings.clone();
    e.register_fn("section", move |m: &mut MeshH, z: FLOAT| -> Array {
        let (rings, open) = m.mesh.section(z);
        if open > 0 {
            let mut w = w.borrow_mut();
            if w.len() < 100 {
                w.push(format!("section: {open} line(s) of the cut do not close (the mesh has holes); they are left out"));
            }
        }
        ring_array(&rings)
    });
    // Numbers out of a blob: Rhai's own parse_le_float reads a 4-byte number into the low bytes of a double, so the floats of
    // binary formats (STL, ...) come from these.
    e.register_fn("read_f32", |data: &mut Blob, pos: INT| -> Result<FLOAT, Box<EvalAltResult>> {
        let at = usize::try_from(pos).map_err(|_| "read_f32: a negative position")?;
        let b: [u8; 4] = data.get(at..at.saturating_add(4)).and_then(|s| s.try_into().ok()).ok_or("read_f32: past the end of the data")?;
        Ok(f32::from_le_bytes(b) as FLOAT)
    });
    e.register_fn("read_f64", |data: &mut Blob, pos: INT| -> Result<FLOAT, Box<EvalAltResult>> {
        let at = usize::try_from(pos).map_err(|_| "read_f64: a negative position")?;
        let b: [u8; 8] = data.get(at..at.saturating_add(8)).and_then(|s| s.try_into().ok()).ok_or("read_f64: past the end of the data")?;
        Ok(f64::from_le_bytes(b))
    });
    // The union of many polygons: a mesh projected on a plane is thousands of overlapping triangles.
    e.register_fn("union_polygons", |polys: Array| -> Result<Array, Box<EvalAltResult>> {
        let mut rings: Vec<crate::geom::Polyline> = Vec::with_capacity(polys.len());
        for p in &polys {
            let pts = p.clone().try_cast::<Array>().ok_or("a polygon is an array of [x, y] points")?;
            let pts: Vec<Pt> = pts.iter().map(|q| pt_of(q).ok_or("a point is [x, y]")).collect::<Result<_, _>>()?;
            if pts.len() >= 3 {
                rings.push(crate::geom::Polyline::new(pts, true));
            }
        }
        let out = crate::ops::union_all(&rings);
        Ok(out.into_iter().map(|r| Dynamic::from(r.pts.iter().map(|q| Dynamic::from(vec![f(q.x), f(q.y)])).collect::<Array>())).collect())
    });
    // Integers where the script has a whole number: `rect(0, 0, 10, 5)` works too.
    e.register_fn("rect", |x: INT, y: INT, w: INT, h: INT| shape("rect", &[("x", f(x as f64)), ("y", f(y as f64)), ("w", f(w as f64)), ("h", f(h as f64))]));
    e.register_fn("circle", |cx: INT, cy: INT, r: INT| shape("ellipse", &[("x", f((cx - r) as f64)), ("y", f((cy - r) as f64)), ("w", f(2.0 * r as f64)), ("h", f(2.0 * r as f64))]));
    Run { engine: e, warnings }
}

fn compile(run: &Run, plugin: &Plugin, rel: &str) -> Result<AST, String> {
    let text = plugin.read_text(rel)?;
    run.engine.compile(&text).map_err(|e| format!("{rel}: {e}"))
}

/// Call `name` with `args`; when the script wrote the function without the `ctx` parameter, call it without.
fn call(run: &Run, ast: &AST, name: &str, first: Dynamic, ctx: &Ctx) -> Result<Dynamic, String> {
    call_with(run, ast, name, first, ctx.map())
}

fn call_with(run: &Run, ast: &AST, name: &str, first: Dynamic, ctx: Map) -> Result<Dynamic, String> {
    let mut scope = Scope::new();
    match run.engine.call_fn::<Dynamic>(&mut scope, ast, name, (first.clone(), Dynamic::from(ctx))) {
        Ok(v) => Ok(v),
        Err(e) if matches!(*e, EvalAltResult::ErrorFunctionNotFound(ref s, _) if s.starts_with(name)) => {
            let mut scope = Scope::new();
            run.engine.call_fn::<Dynamic>(&mut scope, ast, name, (first,)).map_err(|e2| match *e2 {
                EvalAltResult::ErrorFunctionNotFound(..) => format!("the script has no function {name}(…)"),
                other => other.to_string(),
            })
        }
        Err(e) => Err(e.to_string()),
    }
}

struct Build {
    scene: Scene,
    nodes: usize,
    warnings: Vec<String>,
}

impl Build {
    fn add_all(&mut self, v: &Dynamic, group: Option<u64>, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Err("shapes are nested too deep".into());
        }
        if let Some(a) = v.clone().try_cast::<Array>() {
            for x in &a {
                self.add_all(x, group, depth + 1)?;
            }
            return Ok(());
        }
        let Some(m) = v.clone().try_cast::<Map>() else { return Err(format!("expected shapes, got {}", v.type_name())) };
        self.add(&m, group, Xf::IDENTITY, depth)
    }

    fn add(&mut self, m: &Map, group: Option<u64>, parent: Xf, depth: usize) -> Result<(), String> {
        if self.scene.shapes.len() >= MAX_SHAPES {
            return Err("the script made too many shapes".into());
        }
        let ty = m.get("type").and_then(|t| t.clone().into_string().ok()).ok_or("a shape without a type (build shapes with rect(), circle(), ...)")?;
        let own = match m.get("xf") {
            None => Xf::IDENTITY,
            Some(d) => xf_of(Some(d)).ok_or("xf must be six numbers")?,
        };
        let xf = own.then(parent);
        let layer = field_or(m, "layer", 0.0)?;
        if !(0.0..=255.0).contains(&layer) {
            return Err("layer must be 0-255".into());
        }
        let layer = layer as usize;
        if ty == "group" {
            let g = self.scene.new_group_id();
            let children = m.get("children").and_then(|c| c.clone().try_cast::<Array>()).ok_or("a group needs children")?;
            for c in &children {
                let cm = c.clone().try_cast::<Map>().ok_or("a group holds shapes")?;
                if depth > MAX_DEPTH {
                    return Err("groups are nested too deep".into());
                }
                self.add(&cm, Some(g), xf, depth + 1)?;
            }
            return Ok(());
        }
        let kind = match ty.as_str() {
            "rect" => {
                let (x, y, w, h) = (field(m, "x")?, field(m, "y")?, field(m, "w")?, field(m, "h")?);
                if w <= 0.0 || h <= 0.0 {
                    return Err("a rect needs a positive width and height".into());
                }
                self.finish(layer, Kind::Rect { w, h }, Xf::translate(x, y).then(xf), group);
                return Ok(());
            }
            "rounded_rect" => {
                let (x, y, w, h, r) = (field(m, "x")?, field(m, "y")?, field(m, "w")?, field(m, "h")?, field(m, "r")?);
                if w <= 0.0 || h <= 0.0 {
                    return Err("a rect needs a positive width and height".into());
                }
                let c = crate::lightburn::rounded_rect(w, h, r);
                Kind::Bezier(vec![c.transformed(&Xf::translate(x + w / 2.0, y + h / 2.0).then(xf))]).clone_into_pair(self, layer, group)?;
                return Ok(());
            }
            "ellipse" => {
                let (x, y, w, h) = (field(m, "x")?, field(m, "y")?, field(m, "w")?, field(m, "h")?);
                if w <= 0.0 || h <= 0.0 {
                    return Err("an ellipse needs a positive size".into());
                }
                self.finish(layer, Kind::Ellipse { w, h }, Xf::translate(x, y).then(xf), group);
                return Ok(());
            }
            "polyline" => {
                let pts: Vec<Pt> = m.get("points").and_then(|p| p.clone().try_cast::<Array>()).ok_or("a polyline needs points")?.iter().map(|p| pt_of(p).ok_or("a point is [x, y]")).collect::<Result<_, _>>()?;
                let closed = m.get("closed").and_then(|c| c.as_bool().ok()).unwrap_or(false);
                if pts.len() < 2 {
                    return Ok(());
                }
                let c = Contour { nodes: pts.iter().map(|p| Node::corner(*p)).collect(), closed };
                Kind::Bezier(vec![c.transformed(&xf)])
            }
            "path" => {
                let segs = m.get("segments").and_then(|p| p.clone().try_cast::<Array>()).ok_or("a path needs segments")?;
                let cs = path_contours(&segs)?;
                Kind::Bezier(cs.iter().map(|c| c.transformed(&xf)).collect())
            }
            "text" => {
                let text = m.get("text").and_then(|t| t.clone().into_string().ok()).unwrap_or_default();
                let size = field(m, "size")?;
                if text.is_empty() || text.len() > 5000 || !(0.1..=10_000.0).contains(&size) {
                    return Err("a text needs some text (up to 5000 characters) and a size of 0.1-10000 mm".into());
                }
                let mut t = TextData { text, size, ..TextData::default() };
                if let Some(fam) = m.get("family").and_then(|t| t.clone().into_string().ok()) {
                    t.family = fam;
                }
                t.bold = m.get("bold").and_then(|b| b.as_bool().ok()).unwrap_or(false);
                t.italic = m.get("italic").and_then(|b| b.as_bool().ok()).unwrap_or(false);
                let (x, y) = (field_or(m, "x", 0.0)?, field_or(m, "y", 0.0)?);
                self.finish(layer, Kind::Text(t), Xf::translate(x, y).then(xf), group);
                return Ok(());
            }
            other => return Err(format!("unknown shape type {other:?}")),
        };
        kind.clone_into_pair(self, layer, group)
    }

    fn finish(&mut self, layer: usize, kind: Kind, xf: Xf, group: Option<u64>) {
        let id = self.scene.add(layer, kind, xf);
        if let Some(g) = group {
            if let Some(s) = self.scene.shape_mut(id) {
                s.group = Some(g);
            }
        }
    }
}

trait IntoPair {
    fn clone_into_pair(self, b: &mut Build, layer: usize, group: Option<u64>) -> Result<(), String>;
}

impl IntoPair for Kind {
    fn clone_into_pair(self, b: &mut Build, layer: usize, group: Option<u64>) -> Result<(), String> {
        if let Kind::Bezier(cs) = &self {
            b.nodes += cs.iter().map(|c| c.nodes.len()).sum::<usize>();
            if b.nodes > MAX_NODES {
                return Err("the script made too many points".into());
            }
        }
        b.finish(layer, self, Xf::IDENTITY, group);
        Ok(())
    }
}

/// `["M", x, y]`, `["L", x, y]`, `["Q", cx, cy, x, y]`, `["C", c1x, c1y, c2x, c2y, x, y]`, `["Z"]`.
fn path_contours(segs: &Array) -> Result<Vec<Contour>, String> {
    let mut out: Vec<Contour> = vec![];
    let mut cur: Vec<Node> = vec![];
    fn end(out: &mut Vec<Contour>, cur: &mut Vec<Node>, closed: bool) {
        let mut nodes = std::mem::take(cur);
        if closed && nodes.len() > 1 && nodes[0].p.dist(nodes[nodes.len() - 1].p) < 1e-9 {
            let last = nodes.pop().unwrap();
            nodes[0].hin = last.hin;
        }
        if nodes.len() >= 2 {
            out.push(Contour { nodes, closed });
        }
    }
    for s in segs {
        let a = s.clone().try_cast::<Array>().ok_or("a path segment is an array like [\"L\", x, y]")?;
        let cmd = a.first().and_then(|c| c.clone().into_string().ok()).ok_or("a path segment starts with a letter")?;
        let n: Vec<f64> = a[1..].iter().map(|v| num(v).ok_or("a path number is wrong")).collect::<Result<_, _>>()?;
        match (cmd.as_str(), n.len()) {
            ("M", 2) => {
                end(&mut out, &mut cur, false);
                cur.push(Node::corner(Pt::new(n[0], n[1])));
            }
            ("L", 2) => {
                if cur.is_empty() {
                    return Err("a path starts with M".into());
                }
                cur.push(Node::corner(Pt::new(n[0], n[1])));
            }
            ("Q", 4) | ("C", 6) => {
                let l = cur.last().ok_or("a path starts with M")?.p;
                let (c1, c2, p) = if n.len() == 4 {
                    let c = Pt::new(n[0], n[1]);
                    let p = Pt::new(n[2], n[3]);
                    (Pt::new(l.x + 2.0 / 3.0 * (c.x - l.x), l.y + 2.0 / 3.0 * (c.y - l.y)), Pt::new(p.x + 2.0 / 3.0 * (c.x - p.x), p.y + 2.0 / 3.0 * (c.y - p.y)), p)
                } else {
                    (Pt::new(n[0], n[1]), Pt::new(n[2], n[3]), Pt::new(n[4], n[5]))
                };
                cur.last_mut().unwrap().hout = c1;
                cur.push(Node { p, hin: c2, hout: p, smooth: false });
            }
            ("Z", 0) => end(&mut out, &mut cur, true),
            (c, k) => return Err(format!("a path segment {c:?} with {k} numbers is not valid")),
        }
    }
    end(&mut out, &mut cur, false);
    Ok(out)
}

fn into_shapes(run: &Run, value: Dynamic) -> Result<Shapes, String> {
    let mut b = Build { scene: Scene::default(), nodes: 0, warnings: vec![] };
    b.add_all(&value, None, 0)?;
    b.warnings.extend(run.warnings.borrow().iter().cloned());
    Ok(Shapes { scene: b.scene, warnings: b.warnings })
}

/// Read `bytes` with the importer `def` of `plugin` (the form, if it has one, with its start values).
pub fn import(plugin: &Plugin, def: &ImporterDef, bytes: &[u8], ctx: &Ctx) -> Result<Shapes, String> {
    import_with(plugin, def, bytes, &serde_json::Map::new(), &[], ctx)
}

/// What the script of an importer gets as the file: a blob of bytes for a `binary` importer, a text otherwise.
fn file_arg(def: &ImporterDef, bytes: &[u8]) -> Dynamic {
    if def.binary {
        Dynamic::from_blob(bytes.to_vec())
    } else {
        Dynamic::from(String::from_utf8_lossy(bytes).to_string())
    }
}

/// Read `bytes` with the importer `def`, with the values `given` for its form (`def.params` and the `extra` fields the script
/// made with `form`); the values the person did not give are the start values.
pub fn import_with(plugin: &Plugin, def: &ImporterDef, bytes: &[u8], given: &serde_json::Map<String, serde_json::Value>, extra: &[ParamDef], ctx: &Ctx) -> Result<Shapes, String> {
    if bytes.len() as u64 > MAX_IMPORT_BYTES {
        return Err("the file is too big".into());
    }
    let run = engine_for(if def.binary { bytes.len() } else { 0 });
    let ast = compile(&run, plugin, &def.script)?;
    let mut all: Vec<ParamDef> = def.params.iter().chain(extra.iter()).cloned().collect();
    if def.view3d {
        all.extend(view_params());
    }
    let p = values_map(&all, given);
    let v = call_with(&run, &ast, "read", file_arg(def, bytes), ctx.map_with(p))?;
    into_shapes(&run, v)
}

/// The two numbers the 3D view of an importer gives the script: the turn about Z and the tilt (degrees), 0 and 0 from above.
fn view_params() -> Vec<ParamDef> {
    ["view_yaw", "view_pitch"].iter().map(|id| ParamDef { id: id.to_string(), label: String::new(), kind: ParamKind::Number, default: serde_json::json!(0.0), min: Some(-360.0), max: Some(360.0), options: vec![], unit: String::new() }).collect()
}

/// The most fields the script of an importer may add to the form.
pub const MAX_EXTRA_FIELDS: usize = 100;

/// The fields the script of an importer adds to the form, made from the contents of the file (`fn form(data, ctx)` returns an
/// array of maps `#{ id, label, kind, default, min, max, options, unit }`). Nothing for an importer without `form`.
pub fn form_for(plugin: &Plugin, def: &ImporterDef, bytes: &[u8], ctx: &Ctx) -> Result<Vec<ParamDef>, String> {
    if !def.form {
        return Ok(vec![]);
    }
    if bytes.len() as u64 > MAX_IMPORT_BYTES {
        return Err("the file is too big".into());
    }
    let run = engine_for(if def.binary { bytes.len() } else { 0 });
    let ast = compile(&run, plugin, &def.script)?;
    let v = call_with(&run, &ast, "form", file_arg(def, bytes), ctx.map())?;
    let arr = v.try_cast::<Array>().ok_or("form() returns an array of fields")?;
    if arr.len() > MAX_EXTRA_FIELDS {
        return Err(format!("form() made more than {MAX_EXTRA_FIELDS} fields"));
    }
    let mut out = vec![];
    for d in arr {
        let m = d.try_cast::<Map>().ok_or("a field of the form is a map #{ id, label, kind, ... }")?;
        out.push(param_of(&m)?);
    }
    // The same checks as for the fields of a manifest, and no id that the manifest has already.
    let all: Vec<ParamDef> = def.params.iter().chain(out.iter()).cloned().collect();
    crate::plugins::check_params("form()", &all[..]).map_err(|e| e.replace("at most 40 parameters", "too many fields"))?;
    Ok(out)
}

fn text_of(m: &Map, k: &str) -> String {
    m.get(k).and_then(|v| v.clone().into_string().ok()).unwrap_or_default().chars().take(200).collect()
}

fn param_of(m: &Map) -> Result<ParamDef, String> {
    let kind = match text_of(m, "kind").as_str() {
        "number" => ParamKind::Number,
        "bool" => ParamKind::Bool,
        "text" => ParamKind::Text,
        "choice" => ParamKind::Choice,
        other => return Err(format!("a field has the kind {other:?}; use number, bool, text or choice")),
    };
    // `default` is a keyword of Rhai: a script writes it as "default": ... (or uses `start`).
    let default = match (m.get("default").or_else(|| m.get("start")), kind) {
        (None, _) => serde_json::Value::Null,
        (Some(d), ParamKind::Number) => serde_json::json!(num(d).ok_or("the default of a number must be a number")?),
        (Some(d), ParamKind::Bool) => serde_json::Value::Bool(d.as_bool().map_err(|_| "the default of a bool must be true or false")?),
        (Some(d), _) => serde_json::Value::String(d.clone().into_string().map_err(|_| "the default must be a text")?.chars().take(1000).collect()),
    };
    let options: Vec<String> = m.get("options").and_then(|o| o.clone().try_cast::<Array>()).map(|a| a.iter().filter_map(|o| o.clone().into_string().ok()).map(|s| s.chars().take(200).collect()).collect()).unwrap_or_default();
    Ok(ParamDef { id: text_of(m, "id"), label: text_of(m, "label"), kind, default, min: m.get("min").and_then(num), max: m.get("max").and_then(num), options, unit: text_of(m, "unit") })
}

/// The values of a form as the script gets them: numbers (limited to min and max), bools, texts and choices.
pub fn form_values(def: &GeneratorDef, given: &serde_json::Map<String, serde_json::Value>) -> Map {
    values_map(&def.params, given)
}

/// The values of a form (a list of fields) as a script gets them.
pub fn values_map(params: &[ParamDef], given: &serde_json::Map<String, serde_json::Value>) -> Map {
    let mut m = Map::new();
    for p in params {
        let g = given.get(&p.id).unwrap_or(&p.default);
        let v: Dynamic = match p.kind {
            ParamKind::Number => {
                let mut x = g.as_f64().or(p.default.as_f64()).unwrap_or(0.0);
                if !x.is_finite() {
                    x = 0.0;
                }
                if let Some(lo) = p.min {
                    x = x.max(lo);
                }
                if let Some(hi) = p.max {
                    x = x.min(hi);
                }
                f(x)
            }
            ParamKind::Bool => Dynamic::from(g.as_bool().or(p.default.as_bool()).unwrap_or(false)),
            ParamKind::Text => Dynamic::from(g.as_str().or(p.default.as_str()).unwrap_or("").chars().take(1000).collect::<String>()),
            ParamKind::Choice => {
                let s = g.as_str().unwrap_or("");
                Dynamic::from(if p.options.iter().any(|o| o == s) { s.to_string() } else { p.options.first().cloned().unwrap_or_default() })
            }
        };
        m.insert(p.id.as_str().into(), v);
    }
    m
}

/// The start values of the form of a generator.
pub fn defaults(def: &GeneratorDef) -> serde_json::Map<String, serde_json::Value> {
    defaults_of(&def.params)
}

/// The start values of a list of fields.
pub fn defaults_of(params: &[ParamDef]) -> serde_json::Map<String, serde_json::Value> {
    let mut m = serde_json::Map::new();
    for p in params {
        let v = match (&p.kind, &p.default) {
            (ParamKind::Choice, serde_json::Value::Null) => serde_json::Value::String(p.options.first().cloned().unwrap_or_default()),
            (ParamKind::Number, serde_json::Value::Null) => serde_json::json!(p.min.unwrap_or(0.0)),
            (ParamKind::Bool, serde_json::Value::Null) => serde_json::json!(false),
            (ParamKind::Text, serde_json::Value::Null) => serde_json::json!(""),
            (_, d) => d.clone(),
        };
        m.insert(p.id.clone(), v);
    }
    m
}

/// Run a generator with the values of its form.
pub fn generate(plugin: &Plugin, def: &GeneratorDef, values: &serde_json::Map<String, serde_json::Value>, ctx: &Ctx) -> Result<Shapes, String> {
    let run = engine();
    let ast = compile(&run, plugin, &def.script)?;
    let v = call(&run, &ast, "generate", Dynamic::from(form_values(def, values)), ctx)?;
    into_shapes(&run, v)
}

/// Rewrite the lines of a G-code program with the postprocessor `def`. The result is checked with [`check_output`]; a
/// program that fails the check is an error, never partly used.
pub fn postprocess(plugin: &Plugin, def: &PostDef, lines: &[String], ctx: &Ctx) -> Result<(Vec<String>, Vec<String>), String> {
    if lines.len() > MAX_LINES {
        return Err("the program is too long for a postprocessor".into());
    }
    let run = engine();
    let ast = compile(&run, plugin, &def.script)?;
    let input: Array = lines.iter().map(|l| Dynamic::from(l.clone())).collect();
    let v = call(&run, &ast, "post", Dynamic::from(input), ctx)?;
    let out: Vec<String> = if let Some(a) = v.clone().try_cast::<Array>() {
        a.into_iter().map(|d| d.into_string().map_err(|t| format!("a line must be a text, not {t}"))).collect::<Result<_, _>>()?
    } else if let Ok(s) = v.clone().into_string() {
        s.lines().map(|l| l.to_string()).collect()
    } else {
        return Err("a postprocessor returns an array of lines".into());
    };
    check_output(&out)?;
    let warnings = run.warnings.borrow().clone();
    Ok((out, warnings))
}

/// What a postprocessor may send to a machine: plain lines of G-code and nothing that changes the controller itself.
/// No controller commands (`$...`, `~`, `!`, `?`, `^X`), no control characters, no long lines.
pub fn check_output(lines: &[String]) -> Result<(), String> {
    if lines.len() > MAX_LINES {
        return Err("the result is too long".into());
    }
    for (i, l) in lines.iter().enumerate() {
        if l.len() > MAX_LINE_LEN {
            return Err(format!("line {}: longer than {MAX_LINE_LEN} characters", i + 1));
        }
        if let Some(c) = l.chars().find(|c| !(c.is_ascii_graphic() || *c == ' ' || *c == '\t') && !(*c as u32 >= 0x80)) {
            return Err(format!("line {}: the control character {:?} is not allowed", i + 1, c));
        }
        let t = l.trim_start();
        if t.starts_with(['$', '~', '!', '?']) {
            return Err(format!("line {}: controller commands ($, ~, !, ?) are not allowed in a program", i + 1));
        }
    }
    Ok(())
}
