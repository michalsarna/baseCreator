//! Encapsulated PostScript (`.eps`): plain (`%!PS-Adobe-3.0 EPSF-3.0`) or with the binary DOS header that puts a TIFF or WMF preview
//! beside the PostScript (`C5 D0 D3 C6`). Reading its paths into outlines.
//!
//! - An EPS written by VectorCraft keeps its whole document (zlib, ASCII85) between `%VectorCraft_BeginData: native` and
//!   `%VectorCraft_EndData`: that is read as a VectorCraft drawing (`vectorcraft`), with its pictures.
//! - An EPS of the old Illustrator kind (`%AI` comments) is read by `ai`.
//! - Any other is run by a small PostScript interpreter that knows procedures (`def`, `bind`, dictionaries), the stack, the
//!   arithmetic and the path and graphics state operators (`moveto` … `curveto`, `arc`, `rectstroke`, `gsave`, `concat`, `translate`,
//!   `scale`, `rotate`, the colours); every path that is stroked or filled is one outline in its colour. Text and pictures of such a
//!   file are not read.
//!
//! Points have Y up in PostScript: the drawing is read with the top left corner of the bounding box at (0, 0), as an SVG is. The
//! file is untrusted input: sizes, steps and the depth of procedures are limited and nothing panics.
use crate::bezier::{Contour, Node};
use crate::geom::{Pt, Xf};
use crate::svg::Outline;
use std::collections::HashMap;
use std::rc::Rc;

const PT_TO_MM: f64 = 25.4 / 72.0;
const MAX_BYTES: usize = 256 * 1024 * 1024;
/// The most operators run.
const MAX_STEPS: usize = 20_000_000;
const MAX_POINTS: usize = 2_000_000;
const MAX_STACK: usize = 100_000;
const MAX_DEPTH: usize = 200;

/// Whether the bytes are an EPS (plain or with the binary header).
pub fn is_eps(data: &[u8]) -> bool {
    data.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]) || (data.starts_with(b"%!PS-Adobe") && data[..data.len().min(64)].windows(4).any(|w| w == b"EPSF"))
}

/// The PostScript of an EPS: past the binary header when it has one.
pub fn postscript(data: &[u8]) -> &[u8] {
    if data.starts_with(&[0xC5, 0xD0, 0xD3, 0xC6]) && data.len() >= 12 {
        let off = u32::from_le_bytes([data[4], data[5], data[6], data[7]]) as usize;
        let len = u32::from_le_bytes([data[8], data[9], data[10], data[11]]) as usize;
        if let Some(ps) = off.checked_add(len).and_then(|end| data.get(off..end)) {
            return ps;
        }
    }
    data
}

/// The VectorCraft document kept in an EPS that VectorCraft wrote.
pub fn vectorcraft_native(data: &[u8]) -> Option<Vec<u8>> {
    let ps = postscript(data);
    let text = String::from_utf8_lossy(ps);
    let start = text.find("%VectorCraft_BeginData: native")?;
    let body = &text[start..];
    let body = &body[body.find('\n')? + 1..];
    let end = body.find("%VectorCraft_EndData")?;
    let mut a85 = String::new();
    for line in body[..end].lines() {
        let line = line.trim_end_matches('\r');
        a85.push_str(line.strip_prefix("% ").or_else(|| line.strip_prefix('%')).unwrap_or(line).trim());
    }
    let packed = ascii85(&a85)?;
    let mut out = vec![];
    use std::io::Read;
    flate2::read::ZlibDecoder::new(&packed[..]).take(MAX_BYTES as u64).read_to_end(&mut out).ok()?;
    crate::vectorcraft::is_vectorcraft(&out).then_some(out)
}

/// ASCII85 (with or without `<~` … `~>`).
fn ascii85(s: &str) -> Option<Vec<u8>> {
    let s = s.trim();
    let s = s.strip_prefix("<~").unwrap_or(s);
    let s = s.find("~>").map(|i| &s[..i]).unwrap_or(s);
    let mut out = Vec::with_capacity(s.len() * 4 / 5);
    let mut group = [0u8; 5];
    let mut n = 0;
    for c in s.bytes() {
        match c {
            b'z' if n == 0 => out.extend_from_slice(&[0; 4]),
            b'!'..=b'u' => {
                group[n] = c - b'!';
                n += 1;
                if n == 5 {
                    let v = group.iter().fold(0u64, |v, d| v * 85 + *d as u64);
                    if v > u32::MAX as u64 {
                        return None;
                    }
                    out.extend_from_slice(&(v as u32).to_be_bytes());
                    n = 0;
                }
            }
            c if c.is_ascii_whitespace() => {}
            _ => return None,
        }
    }
    if n > 0 {
        if n == 1 {
            return None;
        }
        for d in group.iter_mut().skip(n) {
            *d = 84;
        }
        let v = group.iter().fold(0u64, |v, d| v * 85 + *d as u64);
        if v > u32::MAX as u64 {
            return None;
        }
        out.extend_from_slice(&(v as u32).to_be_bytes()[..n - 1]);
    }
    Some(out)
}

/// Read the paths of an EPS into outlines (millimetres, the top left corner of the bounding box at (0, 0)).
pub fn parse(data: &[u8]) -> Result<Vec<Outline>, String> {
    if data.len() > MAX_BYTES {
        return Err("the EPS file is too big".into());
    }
    if !is_eps(data) {
        return Err("this is not an EPS file".into());
    }
    if let Some(native) = vectorcraft_native(data) {
        return crate::vectorcraft::parse(&native);
    }
    let ps = postscript(data);
    if crate::ai::is_postscript_ai(ps) {
        return crate::ai::parse(ps);
    }
    let text = String::from_utf8_lossy(ps);
    let bbox = bounding_box(&text);
    let mut m = Machine::default();
    m.run_text(&text)?;
    if m.out.is_empty() {
        return Err("the EPS file has no paths that can be read (its text and pictures are not read)".into());
    }
    // From the points of PostScript (Y up) to the millimetres of the drawing (Y down from the top of the box).
    let (x0, top) = match bbox {
        Some([x0, _, _, y1]) => (x0, y1),
        None => {
            let pts = m.out.iter().flat_map(|o| o.contours.iter()).flat_map(|c| c.nodes.iter()).map(|n| n.p);
            let (mut x0, mut top) = (f64::INFINITY, f64::NEG_INFINITY);
            for p in pts {
                x0 = x0.min(p.x);
                top = top.max(p.y);
            }
            if x0.is_finite() { (x0, top) } else { (0.0, 0.0) }
        }
    };
    let xf = Xf { a: PT_TO_MM, b: 0.0, c: 0.0, d: -PT_TO_MM, e: -x0 * PT_TO_MM, f: top * PT_TO_MM };
    Ok(m.out.into_iter().map(|o| Outline { contours: o.contours.into_iter().map(|c| Contour { nodes: c.nodes.iter().map(|n| n.transformed(&xf)).collect(), closed: c.closed }).collect(), ..o }).collect())
}

/// The pictures of an EPS: those of the VectorCraft document in it (others are not read).
pub fn images(data: &[u8]) -> Vec<(crate::image::ImageData, Xf)> {
    vectorcraft_native(data).map(|d| crate::vectorcraft::images(&d)).unwrap_or_default()
}

/// `%%HiResBoundingBox` (or `%%BoundingBox`): x0, y0, x1, y1.
fn bounding_box(text: &str) -> Option<[f64; 4]> {
    let read = |key: &str| -> Option<[f64; 4]> {
        let line = text.lines().take(400).find(|l| l.starts_with(key))?;
        let n: Vec<f64> = line[key.len()..].split_whitespace().filter_map(|t| t.parse().ok()).collect();
        (n.len() == 4 && n.iter().all(|v| v.is_finite())).then(|| [n[0], n[1], n[2], n[3]])
    };
    read("%%HiResBoundingBox:").or_else(|| read("%%BoundingBox:"))
}

#[derive(Clone, Debug)]
enum Tok {
    Num(f64),
    Name(Rc<str>),
    Lit(Rc<str>),
    Str,
    ArrOpen,
    ArrClose,
    Proc(Rc<Vec<Tok>>),
    DictOpen,
    DictClose,
}

#[derive(Clone, Debug)]
enum Val {
    Num(f64),
    Bool(bool),
    Lit(Rc<str>),
    Proc(Rc<Vec<Tok>>),
    Arr(Rc<Vec<Val>>),
    Dict,
    Mark,
    Op(Rc<str>),
    Other,
}

/// The tokens of PostScript; a procedure `{ … }` is one token.
fn tokens(text: &str) -> Vec<Tok> {
    fn group(b: &[u8], i: &mut usize, depth: usize) -> Vec<Tok> {
        let mut out = vec![];
        while *i < b.len() {
            let c = b[*i];
            match c {
                b'%' => {
                    while *i < b.len() && b[*i] != b'\n' && b[*i] != b'\r' {
                        *i += 1;
                    }
                }
                c if c.is_ascii_whitespace() || c == 0 => *i += 1,
                b'{' => {
                    *i += 1;
                    let inner = if depth < MAX_DEPTH { group(b, i, depth + 1) } else { vec![] };
                    out.push(Tok::Proc(Rc::new(inner)));
                }
                b'}' => {
                    *i += 1;
                    if depth > 0 {
                        return out;
                    }
                }
                b'[' => {
                    *i += 1;
                    out.push(Tok::ArrOpen);
                }
                b']' => {
                    *i += 1;
                    out.push(Tok::ArrClose);
                }
                b'(' => {
                    // A string, with nested brackets and escapes.
                    *i += 1;
                    let mut level = 1;
                    while *i < b.len() && level > 0 {
                        match b[*i] {
                            b'\\' => *i += 1,
                            b'(' => level += 1,
                            b')' => level -= 1,
                            _ => {}
                        }
                        *i += 1;
                    }
                    out.push(Tok::Str);
                }
                b'<' if b.get(*i + 1) == Some(&b'<') => {
                    *i += 2;
                    out.push(Tok::DictOpen);
                }
                b'>' if b.get(*i + 1) == Some(&b'>') => {
                    *i += 2;
                    out.push(Tok::DictClose);
                }
                b'<' => {
                    // A hex or an ASCII85 string.
                    let end: &[u8] = if b.get(*i + 1) == Some(&b'~') { b"~>" } else { b">" };
                    *i += 1;
                    while *i < b.len() && !b[*i..].starts_with(end) {
                        *i += 1;
                    }
                    *i = (*i + end.len()).min(b.len());
                    out.push(Tok::Str);
                }
                _ => {
                    let start = *i;
                    if c == b'/' {
                        *i += 1;
                        if b.get(*i) == Some(&b'/') {
                            *i += 1;
                        }
                    }
                    while *i < b.len() && !b[*i].is_ascii_whitespace() && !b"()<>[]{}/%".contains(&b[*i]) {
                        *i += 1;
                    }
                    if *i == start {
                        *i += 1;
                        continue;
                    }
                    let word = String::from_utf8_lossy(&b[start..*i]);
                    if let Some(name) = word.strip_prefix('/') {
                        out.push(Tok::Lit(name.trim_start_matches('/').into()));
                    } else if let Some(n) = number(&word) {
                        out.push(Tok::Num(n));
                    } else {
                        out.push(Tok::Name(word.as_ref().into()));
                    }
                }
            }
        }
        out
    }
    let mut i = 0;
    group(text.as_bytes(), &mut i, 0)
}

/// A PostScript number (also `16#FF` radix numbers).
fn number(w: &str) -> Option<f64> {
    if let Some((radix, digits)) = w.split_once('#') {
        let r: u32 = radix.parse().ok()?;
        return (2..=36).contains(&r).then(|| i64::from_str_radix(digits, r).ok().map(|v| v as f64)).flatten();
    }
    let first = w.as_bytes().first()?;
    if !(first.is_ascii_digit() || matches!(first, b'-' | b'+' | b'.')) {
        return None;
    }
    w.parse::<f64>().ok().filter(|v| v.is_finite())
}

#[derive(Clone)]
struct Gs {
    ctm: Xf,
    colour: [u8; 3],
    path: Vec<Contour>,
    current: Option<Pt>,
}

impl Default for Gs {
    fn default() -> Gs {
        Gs { ctm: Xf::IDENTITY, colour: [0, 0, 0], path: vec![], current: None }
    }
}

#[derive(Default)]
struct Machine {
    stack: Vec<Val>,
    /// The definitions: the bottom one for `def` outside of any `begin`.
    dicts: Vec<HashMap<Rc<str>, Val>>,
    gs: Gs,
    saved: Vec<Gs>,
    out: Vec<Outline>,
    steps: usize,
    points: usize,
    done: bool,
}

fn rgb(v: f64) -> u8 {
    (v.clamp(0.0, 1.0) * 255.0).round() as u8
}

impl Machine {
    fn run_text(&mut self, text: &str) -> Result<(), String> {
        self.dicts = vec![HashMap::new()];
        let toks = tokens(text);
        self.run(&toks, 0)
    }

    fn num(&mut self) -> Option<f64> {
        match self.stack.pop()? {
            Val::Num(n) => Some(n),
            Val::Bool(b) => Some(if b { 1.0 } else { 0.0 }),
            _ => None,
        }
    }

    fn nums<const N: usize>(&mut self) -> Option<[f64; N]> {
        if self.stack.len() < N {
            self.stack.clear();
            return None;
        }
        let mut a = [0.0; N];
        for k in (0..N).rev() {
            a[k] = self.num()?;
        }
        Some(a)
    }

    fn lookup(&self, name: &str) -> Option<Val> {
        self.dicts.iter().rev().find_map(|d| d.get(name).cloned())
    }

    fn push(&mut self, v: Val) -> Result<(), String> {
        if self.stack.len() >= MAX_STACK {
            return Err("the EPS file is too complex".into());
        }
        self.stack.push(v);
        Ok(())
    }

    fn run(&mut self, toks: &[Tok], depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Ok(());
        }
        let mut arrays: Vec<usize> = vec![];
        for t in toks {
            if self.done {
                return Ok(());
            }
            self.steps += 1;
            if self.steps > MAX_STEPS || self.points > MAX_POINTS {
                return Err("the EPS file is too complex".into());
            }
            match t {
                Tok::Num(n) => self.push(Val::Num(*n))?,
                Tok::Lit(n) => self.push(Val::Lit(n.clone()))?,
                Tok::Str => self.push(Val::Other)?,
                Tok::Proc(p) => self.push(Val::Proc(p.clone()))?,
                Tok::ArrOpen | Tok::DictOpen => {
                    arrays.push(self.stack.len());
                    self.push(Val::Mark)?;
                }
                Tok::ArrClose => {
                    let at = self.stack.iter().rposition(|v| matches!(v, Val::Mark));
                    if let Some(at) = at {
                        let items: Vec<Val> = self.stack.drain(at..).skip(1).collect();
                        arrays.pop();
                        self.push(Val::Arr(Rc::new(items)))?;
                    }
                }
                Tok::DictClose => {
                    if let Some(at) = self.stack.iter().rposition(|v| matches!(v, Val::Mark)) {
                        self.stack.truncate(at);
                    }
                    self.push(Val::Dict)?;
                }
                Tok::Name(n) => self.exec_name(n, depth)?,
            }
        }
        Ok(())
    }

    /// Run a value as `exec` does.
    fn exec_val(&mut self, v: Val, depth: usize) -> Result<(), String> {
        match v {
            Val::Proc(p) => self.run(&p, depth + 1),
            Val::Op(n) => self.op(&n, depth),
            other => self.push(other),
        }
    }

    fn exec_name(&mut self, n: &Rc<str>, depth: usize) -> Result<(), String> {
        match self.lookup(n) {
            Some(Val::Proc(p)) => self.run(&p, depth + 1),
            Some(Val::Op(o)) => self.op(&o, depth),
            Some(v) => self.push(v),
            None => self.op(n, depth),
        }
    }

    fn add_point(&mut self, p: Pt, new: bool) {
        let q = self.gs.ctm.apply(p);
        self.points += 1;
        if new || self.gs.path.is_empty() || self.gs.current.is_none() {
            self.gs.path.push(Contour { nodes: vec![Node::corner(q)], closed: false });
        } else if let Some(c) = self.gs.path.last_mut() {
            if c.closed {
                let start = c.nodes[0].p;
                self.gs.path.push(Contour { nodes: vec![Node::corner(start), Node::corner(q)], closed: false });
            } else {
                c.nodes.push(Node::corner(q));
            }
        }
        self.gs.current = Some(p);
    }

    fn curve_to(&mut self, c1: Pt, c2: Pt, p: Pt) {
        if self.gs.current.is_none() {
            self.add_point(c1, true);
        }
        let xf = self.gs.ctm;
        if let Some(c) = self.gs.path.last_mut() {
            if c.closed {
                let start = c.nodes[0].p;
                self.gs.path.push(Contour { nodes: vec![Node::corner(start)], closed: false });
            }
        }
        if let Some(c) = self.gs.path.last_mut() {
            if let Some(last) = c.nodes.last_mut() {
                last.hout = xf.apply(c1);
            }
            let q = xf.apply(p);
            c.nodes.push(Node { p: q, hin: xf.apply(c2), hout: q, smooth: false });
        }
        self.points += 3;
        self.gs.current = Some(p);
    }

    fn close(&mut self) {
        if let Some(c) = self.gs.path.last_mut() {
            if c.nodes.len() > 1 && !c.closed {
                // A last node on the first one is that node.
                let (first, last) = (c.nodes[0], c.nodes[c.nodes.len() - 1]);
                if first.p.dist(last.p) < 1e-9 {
                    c.nodes[0].hin = last.hin;
                    c.nodes.pop();
                }
                c.closed = true;
            }
            self.gs.current = c.nodes.first().map(|n| self.gs.ctm.inverse().map_or(n.p, |i| i.apply(n.p)));
        }
    }

    /// The path drawn (stroked or filled): one outline.
    fn paint(&mut self) {
        let path: Vec<Contour> = std::mem::take(&mut self.gs.path).into_iter().filter(|c| c.nodes.len() > 1).collect();
        self.gs.current = None;
        if !path.is_empty() {
            self.out.push(Outline { contours: path, color: self.gs.colour, layer: None });
        }
    }

    /// A circular arc of the user space, as Bézier curves of at most a quarter each.
    fn arc(&mut self, x: f64, y: f64, r: f64, a1: f64, a2: f64, ccw: bool) {
        let (mut a1, mut a2) = (a1.to_radians(), a2.to_radians());
        if ccw {
            while a2 < a1 {
                a2 += std::f64::consts::TAU;
            }
        } else {
            while a2 > a1 {
                a2 -= std::f64::consts::TAU;
            }
        }
        let at = |a: f64| Pt::new(x + r * a.cos(), y + r * a.sin());
        let start = at(a1);
        let fresh = self.gs.current.is_none();
        self.add_point(start, fresh);
        let n = ((a2 - a1).abs() / std::f64::consts::FRAC_PI_2).ceil().clamp(1.0, 16.0) as usize;
        let step = (a2 - a1) / n as f64;
        let k = 4.0 / 3.0 * (step / 4.0).tan();
        for _ in 0..n {
            let b = a1 + step;
            let (p0, p3) = (at(a1), at(b));
            let c1 = Pt::new(p0.x - k * r * a1.sin(), p0.y + k * r * a1.cos());
            let c2 = Pt::new(p3.x + k * r * b.sin(), p3.y - k * r * b.cos());
            self.curve_to(c1, c2, p3);
            a1 = b;
        }
    }

    fn rect(&mut self, x: f64, y: f64, w: f64, h: f64) {
        self.add_point(Pt::new(x, y), true);
        self.add_point(Pt::new(x + w, y), false);
        self.add_point(Pt::new(x + w, y + h), false);
        self.add_point(Pt::new(x, y + h), false);
        self.close();
    }

    fn matrix(&mut self) -> Option<Xf> {
        match self.stack.pop()? {
            Val::Arr(a) if a.len() == 6 => {
                let n: Vec<f64> = a.iter().map(|v| if let Val::Num(n) = v { Some(*n) } else { None }).collect::<Option<Vec<_>>>()?;
                Some(Xf { a: n[0], b: n[1], c: n[2], d: n[3], e: n[4], f: n[5] })
            }
            _ => None,
        }
    }

    fn op(&mut self, name: &str, depth: usize) -> Result<(), String> {
        let cur = self.gs.current.unwrap_or(Pt::new(0.0, 0.0));
        match name {
            // The stack.
            "pop" => {
                self.stack.pop();
            }
            "exch" => {
                let n = self.stack.len();
                if n >= 2 {
                    self.stack.swap(n - 1, n - 2);
                }
            }
            "dup" => {
                if let Some(v) = self.stack.last().cloned() {
                    self.push(v)?;
                }
            }
            "copy" => {
                if let Some(n) = self.num() {
                    let n = n.max(0.0) as usize;
                    let len = self.stack.len();
                    if n <= len {
                        let top: Vec<Val> = self.stack[len - n..].to_vec();
                        for v in top {
                            self.push(v)?;
                        }
                    }
                }
            }
            "index" => {
                if let Some(n) = self.num() {
                    let len = self.stack.len();
                    let n = n.max(0.0) as usize;
                    if n < len {
                        let v = self.stack[len - 1 - n].clone();
                        self.push(v)?;
                    }
                }
            }
            "roll" => {
                if let Some([n, j]) = self.nums::<2>() {
                    let (n, len) = (n.max(0.0) as usize, self.stack.len());
                    if n > 0 && n <= len {
                        let j = (j as i64).rem_euclid(n as i64) as usize;
                        self.stack[len - n..].rotate_right(j);
                    }
                }
            }
            "clear" => self.stack.clear(),
            "mark" => self.push(Val::Mark)?,
            "cleartomark" => {
                if let Some(at) = self.stack.iter().rposition(|v| matches!(v, Val::Mark)) {
                    self.stack.truncate(at);
                }
            }
            "counttomark" => {
                let n = self.stack.iter().rev().position(|v| matches!(v, Val::Mark)).unwrap_or(self.stack.len());
                self.push(Val::Num(n as f64))?;
            }
            "count" => self.push(Val::Num(self.stack.len() as f64))?,
            // Arithmetic.
            "add" | "sub" | "mul" | "div" | "idiv" | "mod" | "atan" | "exp" => {
                if let Some([a, b]) = self.nums::<2>() {
                    let r = match name {
                        "add" => a + b,
                        "sub" => a - b,
                        "mul" => a * b,
                        "div" => a / b,
                        "idiv" => (a / b).trunc(),
                        "mod" => a % b,
                        "atan" => a.atan2(b).to_degrees().rem_euclid(360.0),
                        _ => a.powf(b),
                    };
                    self.push(Val::Num(if r.is_finite() { r } else { 0.0 }))?;
                }
            }
            "neg" | "abs" | "sqrt" | "sin" | "cos" | "round" | "floor" | "ceiling" | "truncate" | "cvi" | "cvr" => {
                if let Some(a) = self.num() {
                    let r = match name {
                        "neg" => -a,
                        "abs" => a.abs(),
                        "sqrt" => a.max(0.0).sqrt(),
                        "sin" => a.to_radians().sin(),
                        "cos" => a.to_radians().cos(),
                        "round" => a.round(),
                        "floor" => a.floor(),
                        "ceiling" => a.ceil(),
                        "truncate" | "cvi" => a.trunc(),
                        _ => a,
                    };
                    self.push(Val::Num(r))?;
                }
            }
            "eq" | "ne" | "gt" | "ge" | "lt" | "le" => {
                let (b, a) = (self.stack.pop(), self.stack.pop());
                let r = match (a, b) {
                    (Some(Val::Num(a)), Some(Val::Num(b))) => match name {
                        "eq" => a == b,
                        "ne" => a != b,
                        "gt" => a > b,
                        "ge" => a >= b,
                        "lt" => a < b,
                        _ => a <= b,
                    },
                    (Some(Val::Lit(a)), Some(Val::Lit(b))) => (a == b) == (name == "eq"),
                    _ => name == "ne",
                };
                self.push(Val::Bool(r))?;
            }
            "true" => self.push(Val::Bool(true))?,
            "false" => self.push(Val::Bool(false))?,
            "not" => match self.stack.pop() {
                Some(Val::Bool(b)) => self.push(Val::Bool(!b))?,
                Some(Val::Num(n)) => self.push(Val::Num(!(n as i64) as f64))?,
                _ => {}
            },
            "and" | "or" => {
                let (b, a) = (self.stack.pop(), self.stack.pop());
                if let (Some(Val::Bool(a)), Some(Val::Bool(b))) = (a, b) {
                    self.push(Val::Bool(if name == "and" { a && b } else { a || b }))?;
                }
            }
            // Definitions and control.
            "def" => {
                let (v, k) = (self.stack.pop(), self.stack.pop());
                if let (Some(v), Some(Val::Lit(k))) = (v, k) {
                    if let Some(d) = self.dicts.last_mut() {
                        d.insert(k, v);
                    }
                }
            }
            "bind" | "readonly" | "executeonly" | "noaccess" | "cvx" => {}
            "load" => {
                if let Some(Val::Lit(k)) = self.stack.pop() {
                    let v = self.lookup(&k).unwrap_or(Val::Op(k));
                    self.push(v)?;
                }
            }
            "where" => match self.stack.pop() {
                Some(Val::Lit(k)) if self.lookup(&k).is_some() => {
                    self.push(Val::Dict)?;
                    self.push(Val::Bool(true))?;
                }
                _ => self.push(Val::Bool(false))?,
            },
            "known" => {
                let k = self.stack.pop();
                self.stack.pop();
                let known = matches!(k, Some(Val::Lit(ref k)) if self.lookup(k).is_some());
                self.push(Val::Bool(known))?;
            }
            "dict" => {
                self.stack.pop();
                self.push(Val::Dict)?;
            }
            "begin" => {
                self.stack.pop();
                if self.dicts.len() < MAX_DEPTH {
                    self.dicts.push(HashMap::new());
                }
            }
            "end" => {
                if self.dicts.len() > 1 {
                    // What was defined stays known (the dictionaries of a prolog are begun again later).
                    let top = self.dicts.pop().unwrap_or_default();
                    if let Some(d) = self.dicts.last_mut() {
                        d.extend(top);
                    }
                }
            }
            "currentdict" | "userdict" | "globaldict" | "systemdict" | "statusdict" => self.push(Val::Dict)?,
            "exec" => {
                if let Some(v) = self.stack.pop() {
                    self.exec_val(v, depth)?;
                }
            }
            "if" => {
                let (p, c) = (self.stack.pop(), self.stack.pop());
                if let (Some(Val::Proc(p)), Some(Val::Bool(true))) = (p, c) {
                    self.run(&p, depth + 1)?;
                }
            }
            "ifelse" => {
                let (b, a, c) = (self.stack.pop(), self.stack.pop(), self.stack.pop());
                if let (Some(Val::Proc(b)), Some(Val::Proc(a)), Some(Val::Bool(c))) = (b, a, c) {
                    self.run(if c { &a } else { &b }, depth + 1)?;
                }
            }
            "repeat" => {
                let (p, n) = (self.stack.pop(), self.num());
                if let (Some(Val::Proc(p)), Some(n)) = (p, n) {
                    for _ in 0..(n.max(0.0) as usize).min(MAX_STEPS) {
                        self.run(&p, depth + 1)?;
                        if self.done {
                            break;
                        }
                    }
                }
            }
            "for" => {
                let p = self.stack.pop();
                if let (Some(Val::Proc(p)), Some([a, s, b])) = (p, self.nums::<3>()) {
                    if s != 0.0 {
                        let mut i = a;
                        let mut k = 0;
                        while (s > 0.0 && i <= b) || (s < 0.0 && i >= b) {
                            self.push(Val::Num(i))?;
                            self.run(&p, depth + 1)?;
                            i += s;
                            k += 1;
                            if k > MAX_STEPS || self.done {
                                break;
                            }
                        }
                    }
                }
            }
            "forall" => {
                let (p, a) = (self.stack.pop(), self.stack.pop());
                if let (Some(Val::Proc(p)), Some(Val::Arr(a))) = (p, a) {
                    for v in a.iter() {
                        self.push(v.clone())?;
                        self.run(&p, depth + 1)?;
                    }
                }
            }
            "aload" => {
                if let Some(Val::Arr(a)) = self.stack.pop() {
                    for v in a.iter() {
                        self.push(v.clone())?;
                    }
                    self.push(Val::Arr(a))?;
                }
            }
            "length" => {
                let n = match self.stack.pop() {
                    Some(Val::Arr(a)) => a.len(),
                    _ => 0,
                };
                self.push(Val::Num(n as f64))?;
            }
            "array" => {
                let n = self.num().unwrap_or(0.0).clamp(0.0, 1e5) as usize;
                self.push(Val::Arr(Rc::new(vec![Val::Num(0.0); n])))?;
            }
            "matrix" => self.push(Val::Arr(Rc::new([1.0, 0.0, 0.0, 1.0, 0.0, 0.0].map(Val::Num).to_vec())))?,
            "get" => {
                let (i, a) = (self.num(), self.stack.pop());
                if let (Some(i), Some(Val::Arr(a))) = (i, a) {
                    self.push(a.get(i.max(0.0) as usize).cloned().unwrap_or(Val::Num(0.0)))?;
                } else {
                    self.push(Val::Other)?;
                }
            }
            "stop" | "quit" | "showpage" => {
                if name != "showpage" {
                    self.done = true;
                }
            }
            "stopped" => {
                if let Some(v) = self.stack.pop() {
                    self.exec_val(v, depth)?;
                }
                self.push(Val::Bool(false))?;
            }
            // The path.
            "newpath" => {
                self.gs.path.clear();
                self.gs.current = None;
            }
            "moveto" => {
                if let Some([x, y]) = self.nums::<2>() {
                    self.add_point(Pt::new(x, y), true);
                }
            }
            "rmoveto" => {
                if let Some([x, y]) = self.nums::<2>() {
                    self.add_point(Pt::new(cur.x + x, cur.y + y), true);
                }
            }
            "lineto" => {
                if let Some([x, y]) = self.nums::<2>() {
                    self.add_point(Pt::new(x, y), false);
                }
            }
            "rlineto" => {
                if let Some([x, y]) = self.nums::<2>() {
                    self.add_point(Pt::new(cur.x + x, cur.y + y), false);
                }
            }
            "curveto" => {
                if let Some([a, b, c, d, e, f]) = self.nums::<6>() {
                    self.curve_to(Pt::new(a, b), Pt::new(c, d), Pt::new(e, f));
                }
            }
            "rcurveto" => {
                if let Some([a, b, c, d, e, f]) = self.nums::<6>() {
                    self.curve_to(Pt::new(cur.x + a, cur.y + b), Pt::new(cur.x + c, cur.y + d), Pt::new(cur.x + e, cur.y + f));
                }
            }
            "closepath" => self.close(),
            "arc" | "arcn" => {
                if let Some([x, y, r, a1, a2]) = self.nums::<5>() {
                    self.arc(x, y, r, a1, a2, name == "arc");
                }
            }
            "rectstroke" | "rectfill" | "rectclip" => {
                if let Some([x, y, w, h]) = self.nums::<4>() {
                    let saved = std::mem::take(&mut self.gs.path);
                    self.rect(x, y, w, h);
                    if name == "rectclip" {
                        self.gs.path.clear();
                    } else {
                        self.paint();
                    }
                    self.gs.path = saved;
                }
            }
            "stroke" | "fill" | "eofill" => self.paint(),
            "clip" | "eoclip" => {}
            "currentpoint" => {
                self.push(Val::Num(cur.x))?;
                self.push(Val::Num(cur.y))?;
            }
            // The graphics state.
            "gsave" | "save" => {
                if self.saved.len() < MAX_DEPTH {
                    self.saved.push(self.gs.clone());
                }
                if name == "save" {
                    self.push(Val::Other)?;
                }
            }
            "grestore" | "restore" => {
                if name == "restore" {
                    self.stack.pop();
                }
                if let Some(g) = self.saved.pop() {
                    self.gs = g;
                }
            }
            "concat" => {
                if let Some(m) = self.matrix() {
                    self.gs.ctm = m.then(self.gs.ctm);
                }
            }
            "setmatrix" => {
                if let Some(m) = self.matrix() {
                    self.gs.ctm = m;
                }
            }
            "translate" | "scale" => {
                if matches!(self.stack.last(), Some(Val::Arr(_))) {
                    self.stack.pop();
                }
                if let Some([x, y]) = self.nums::<2>() {
                    let m = if name == "translate" { Xf::translate(x, y) } else { Xf::scale(x, y) };
                    self.gs.ctm = m.then(self.gs.ctm);
                }
            }
            "rotate" => {
                if matches!(self.stack.last(), Some(Val::Arr(_))) {
                    self.stack.pop();
                }
                if let Some(a) = self.num() {
                    let (s, c) = a.to_radians().sin_cos();
                    self.gs.ctm = Xf { a: c, b: s, c: -s, d: c, e: 0.0, f: 0.0 }.then(self.gs.ctm);
                }
            }
            "setgray" => {
                if let Some(g) = self.num() {
                    let g = rgb(g);
                    self.gs.colour = [g, g, g];
                }
            }
            "setrgbcolor" | "sethsbcolor" => {
                if let Some([r, g, b]) = self.nums::<3>() {
                    if name == "setrgbcolor" {
                        self.gs.colour = [rgb(r), rgb(g), rgb(b)];
                    }
                }
            }
            "setcmykcolor" => {
                if let Some([c, m, y, k]) = self.nums::<4>() {
                    let ch = |v: f64| rgb((1.0 - v) * (1.0 - k));
                    self.gs.colour = [ch(c), ch(m), ch(y)];
                }
            }
            "setlinewidth" | "setlinecap" | "setlinejoin" | "setmiterlimit" | "setflat" | "setoverprint" | "setstrokeadjust" | "setcolorspace" | "setpacking" | "setglobal" => {
                self.stack.pop();
            }
            "setdash" => {
                self.stack.pop();
                self.stack.pop();
            }
            "setcolor" => {
                // As many numbers as the colour space has: taken as grey, RGB or CMYK by their count.
                let n = self.stack.iter().rev().take_while(|v| matches!(v, Val::Num(_))).count().min(4);
                match n {
                    1 => self.op("setgray", depth)?,
                    3 => self.op("setrgbcolor", depth)?,
                    4 => self.op("setcmykcolor", depth)?,
                    _ => {}
                }
            }
            // Text and pictures are not read: their operands are taken off the stack.
            "show" | "findfont" | "setfont" | "stringwidth" | "ashow" | "image" | "imagemask" | "colorimage" => {
                self.stack.pop();
            }
            "scalefont" | "makefont" | "widthshow" => {
                self.stack.pop();
            }
            _ => {}
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn eps(body: &str) -> Vec<u8> {
        format!("%!PS-Adobe-3.0 EPSF-3.0\n%%BoundingBox: 0 0 100 50\n%%EndComments\n{body}\nshowpage\n%%EOF\n").into_bytes()
    }

    #[test]
    fn the_paths_of_a_plain_eps_are_read_with_procedures_and_transforms() {
        let data = eps("/m {moveto} bind def /l {lineto} bind def\n1 0 0 setrgbcolor 10 10 m 90 10 l 90 40 l closepath stroke\ngsave 50 25 translate 0 0 10 0 360 arc fill grestore");
        let out = parse(&data).unwrap();
        assert_eq!(out.len(), 2);
        assert_eq!(out[0].color, [255, 0, 0]);
        let c = &out[0].contours[0];
        assert!(c.closed && c.nodes.len() == 3);
        // Y down from the top of the box: (10, 10) of PostScript is 40 points from the top.
        let mm = |v: f64| v * PT_TO_MM;
        assert!((c.nodes[0].p.x - mm(10.0)).abs() < 1e-9 && (c.nodes[0].p.y - mm(40.0)).abs() < 1e-9);
        // The circle round (50, 25) with a radius of 10: every node 10 points from the centre.
        let circle = &out[1].contours[0];
        assert!(circle.nodes.len() >= 4);
        for n in &circle.nodes {
            assert!(((n.p.x - mm(50.0)).hypot(n.p.y - mm(25.0)) - mm(10.0)).abs() < 1e-6);
        }
    }

    #[test]
    fn a_binary_eps_is_read_past_its_header_and_bad_input_is_an_error() {
        let ps = eps("0 0 moveto 100 50 lineto stroke");
        let mut data = vec![0xC5, 0xD0, 0xD3, 0xC6];
        data.extend_from_slice(&30u32.to_le_bytes());
        data.extend_from_slice(&(ps.len() as u32).to_le_bytes());
        data.extend_from_slice(&[0; 18]);
        data.extend_from_slice(&ps);
        assert!(is_eps(&data));
        assert_eq!(parse(&data).unwrap().len(), 1);
        assert!(parse(b"not an eps").is_err());
        // A procedure that calls itself, or one that doubles, stops at the limits instead of running away.
        assert!(parse(&eps("/x {x} def x")).err().unwrap().contains("no paths"));
        assert!(parse(&eps("/x {x x} def x")).err().unwrap().contains("too complex"));
        assert!(parse(&eps("{ 0 0 moveto } loop 0 0 moveto 1 1 lineto stroke")).is_ok());
    }

    #[test]
    fn ascii85_is_decoded() {
        assert_eq!(ascii85("87cURD]i,\"Ebo80~>").unwrap(), b"Hello World!");
        assert_eq!(ascii85("z").unwrap(), vec![0; 4]);
    }
}
