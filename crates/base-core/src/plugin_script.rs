//! Running the scripts of plugins (Rhai). A script can only compute: it has no access to files, the network, the machine
//! or the system. Its time and memory are limited. What it returns is data (shapes, text lines) that the program checks
//! before it uses it.
//!
//! The functions a script defines:
//!
//! * importer: `fn read(text, ctx)` returns shapes
//! * generator: `fn generate(p, ctx)` returns shapes (`p` holds the values of the form)
//! * postprocessor: `fn post(lines, ctx)` returns the lines of the program
//!
//! `ctx` is `#{ app, area_w, area_h }` (work area in mm). Shapes are built with `rect`, `rounded_rect`, `circle`,
//! `ellipse`, `line`, `polyline`, `polygon`, `path` and `text`, moved with `translate`, `rotate`, `scale` and put on a
//! layer with `s.layer = n`; `group([...])` makes shapes move together. `warn("...")` adds a message.
use crate::bezier::{Contour, Node};
use crate::geom::{Pt, Xf};
use crate::plugins::{GeneratorDef, ImporterDef, ParamKind, Plugin, PostDef};
use crate::scene::{Kind, Scene};
use crate::text::TextData;
use rhai::{Array, Dynamic, Engine, EvalAltResult, ImmutableString, Map, Scope, AST, FLOAT, INT};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::{Duration, Instant};

pub const MAX_SECONDS: u64 = 10;
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
        let mut m = Map::new();
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

/// The engine with limits, the shape functions and the collected messages.
struct Run {
    engine: Engine,
    warnings: Rc<RefCell<Vec<String>>>,
}

fn engine() -> Run {
    let mut e = Engine::new();
    e.set_max_operations(200_000_000);
    e.set_max_call_levels(48);
    e.set_max_expr_depths(64, 32);
    e.set_max_string_size(2_000_000);
    e.set_max_array_size(500_000);
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
    let mut scope = Scope::new();
    match run.engine.call_fn::<Dynamic>(&mut scope, ast, name, (first.clone(), Dynamic::from(ctx.map()))) {
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

/// Read `bytes` with the importer `def` of `plugin`.
pub fn import(plugin: &Plugin, def: &ImporterDef, bytes: &[u8], ctx: &Ctx) -> Result<Shapes, String> {
    if bytes.len() > 64_000_000 {
        return Err("the file is too big".into());
    }
    let run = engine();
    let ast = compile(&run, plugin, &def.script)?;
    let text = String::from_utf8_lossy(bytes).to_string();
    let v = call(&run, &ast, "read", Dynamic::from(text), ctx)?;
    into_shapes(&run, v)
}

/// The values of a form as the script gets them: numbers (limited to min and max), bools, texts and choices.
pub fn form_values(def: &GeneratorDef, given: &serde_json::Map<String, serde_json::Value>) -> Map {
    let mut m = Map::new();
    for p in &def.params {
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
    let mut m = serde_json::Map::new();
    for p in &def.params {
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
