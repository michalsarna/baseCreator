//! Adobe Illustrator files of the old PostScript kind (`%!PS-Adobe` with `%AI5_` comments), as LightBurn and many cutters write
//! them: reading their paths into outlines, and writing shapes the same way. A PDF-compatible Illustrator file is read by `pdf`.
//!
//! The paths are in points (1/72 inch) with Y up. They are read with the top left corner of the drawing at (0, 0), as an SVG is,
//! and the Illustrator layers (`(name) Ln`) are kept as the `layer` of each outline. Pictures are not read or written: LightBurn
//! leaves them out of such files too. The file is untrusted input: sizes are limited and nothing panics.
use crate::bezier::{Contour, Node};
use crate::geom::{Pt, Xf};
use crate::scene::Scene;
use crate::svg::Outline;
use std::fmt::Write;

const PT_TO_MM: f64 = 25.4 / 72.0;
/// The biggest file read.
const MAX_BYTES: usize = 64 * 1024 * 1024;
/// The most path points taken from one file.
const MAX_POINTS: usize = 2_000_000;

/// Whether the bytes are an Illustrator file of the PostScript kind (not a PDF).
pub fn is_postscript_ai(data: &[u8]) -> bool {
    let head = &data[..data.len().min(4096)];
    head.starts_with(b"%!PS-Adobe") && (contains(head, b"%AI") || contains(head, b"Adobe_Illustrator") || contains(head, b"Illustrator"))
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

fn cmyk_to_rgb(c: f64, m: f64, y: f64, k: f64) -> [u8; 3] {
    let f = |v: f64| ((1.0 - v.clamp(0.0, 1.0)) * (1.0 - k.clamp(0.0, 1.0)) * 255.0).round() as u8;
    [f(c), f(m), f(y)]
}

/// Read the paths of an Illustrator file of the PostScript kind.
pub fn parse(data: &[u8]) -> Result<Vec<Outline>, String> {
    if data.len() > MAX_BYTES {
        return Err("the Illustrator file is too big".into());
    }
    if !is_postscript_ai(data) {
        return Err("this is not an Illustrator file".into());
    }
    let text = String::from_utf8_lossy(data);
    // The drawing comes after the setup; the prolog before it only names resources.
    let body = text.find("%%EndSetup").or_else(|| text.find("%%EndProlog")).map(|i| &text[i..]).unwrap_or(&text);
    let body = body.find("%%Trailer").map(|i| &body[..i]).unwrap_or(body);
    let mut r = Reader::default();
    for line in body.lines() {
        let line = line.trim_end_matches('\r');
        if line.starts_with('%') {
            if line.starts_with("%AI5_BeginLayer") {
                r.layer = None;
            }
            continue;
        }
        r.line(line);
        if r.points > MAX_POINTS {
            return Err("the Illustrator file has too many points".into());
        }
    }
    let mut out = r.out;
    if out.is_empty() {
        return Err("no vector paths found".into());
    }
    // Y up in points → Y down in millimetres, the top left corner of the drawing at (0, 0).
    let (mut x0, mut y1) = (f64::MAX, f64::MIN);
    for c in out.iter().flat_map(|o| o.contours.iter()) {
        for n in &c.nodes {
            for p in [n.p, n.hin, n.hout] {
                x0 = x0.min(p.x);
                y1 = y1.max(p.y);
            }
        }
    }
    let xf = Xf { a: PT_TO_MM, b: 0.0, c: 0.0, d: -PT_TO_MM, e: -x0 * PT_TO_MM, f: y1 * PT_TO_MM };
    for o in &mut out {
        for c in &mut o.contours {
            *c = c.transformed(&xf);
        }
    }
    Ok(out)
}

#[derive(Default)]
struct Reader {
    stack: Vec<f64>,
    string: Option<String>,
    layer: Option<String>,
    stroke: [u8; 3],
    fill: [u8; 3],
    /// The contours of the path being built (a compound path has several).
    path: Vec<Contour>,
    cur: Option<Contour>,
    /// Inside `*u` … `*U`: the paths are painted as one.
    compound: bool,
    pending: Vec<Contour>,
    out: Vec<Outline>,
    points: usize,
}

impl Reader {
    fn line(&mut self, line: &str) {
        let mut rest = line;
        while !rest.is_empty() {
            rest = rest.trim_start();
            if rest.is_empty() {
                break;
            }
            if let Some(after) = rest.strip_prefix('(') {
                // A string: up to the closing bracket that is not escaped.
                let mut s = String::new();
                let mut chars = after.char_indices();
                let mut end = after.len();
                while let Some((i, ch)) = chars.next() {
                    match ch {
                        '\\' => {
                            if let Some((_, n)) = chars.next() {
                                s.push(n);
                            }
                        }
                        ')' => {
                            end = i + 1;
                            break;
                        }
                        _ => s.push(ch),
                    }
                }
                self.string = Some(s);
                rest = &after[end.min(after.len())..];
                continue;
            }
            let (tok, after) = rest.split_at(rest.find(char::is_whitespace).unwrap_or(rest.len()));
            rest = after;
            if tok.starts_with('[') || tok.starts_with(']') {
                // A dash array (`[]0 d`): not a path.
                self.stack.clear();
                continue;
            }
            if let Ok(v) = tok.parse::<f64>() {
                if v.is_finite() {
                    self.stack.push(v);
                }
                continue;
            }
            self.op(tok);
            self.stack.clear();
        }
    }

    fn take<const N: usize>(&self) -> Option<[f64; N]> {
        let s = &self.stack;
        if s.len() < N {
            return None;
        }
        let mut a = [0.0; N];
        a.copy_from_slice(&s[s.len() - N..]);
        Some(a)
    }

    fn last_point(&self) -> Option<Pt> {
        self.cur.as_ref().and_then(|c| c.nodes.last()).map(|n| n.p)
    }

    fn to(&mut self, hout: Pt, hin: Pt, p: Pt) {
        let Some(c) = self.cur.as_mut() else { return };
        if let Some(last) = c.nodes.last_mut() {
            last.hout = hout;
        }
        c.nodes.push(Node { p, hin, hout: p, smooth: false });
        self.points += 1;
    }

    fn end_subpath(&mut self, close: bool) {
        if let Some(mut c) = self.cur.take() {
            if close && c.nodes.len() > 2 {
                // A closed path that comes back to its start in a straight line or a curve: the last node is the first one.
                let (f, l) = (c.nodes[0].p, c.nodes[c.nodes.len() - 1].p);
                if (f.x - l.x).abs() < 1e-6 && (f.y - l.y).abs() < 1e-6 {
                    let last = c.nodes.pop().expect("the path has nodes");
                    c.nodes[0].hin = last.hin;
                }
                c.closed = true;
            }
            if c.nodes.len() > 1 {
                self.path.push(c);
            }
        }
    }

    fn paint(&mut self, close: bool, draw: bool) {
        self.end_subpath(close);
        let cs = std::mem::take(&mut self.path);
        if !draw {
            return;
        }
        if self.compound {
            self.pending.extend(cs);
        } else if !cs.is_empty() {
            self.out.push(Outline { contours: cs, color: self.stroke, layer: self.layer.clone() });
        }
    }

    fn op(&mut self, op: &str) {
        match op {
            "m" => {
                if let Some([x, y]) = self.take() {
                    self.end_subpath(false);
                    self.cur = Some(Contour { nodes: vec![Node::corner(Pt::new(x, y))], closed: false });
                    self.points += 1;
                }
            }
            "l" | "L" => {
                if let Some([x, y]) = self.take() {
                    let p = Pt::new(x, y);
                    let from = self.last_point().unwrap_or(p);
                    self.to(from, p, p);
                }
            }
            "c" | "C" => {
                if let Some([x1, y1, x2, y2, x3, y3]) = self.take() {
                    self.to(Pt::new(x1, y1), Pt::new(x2, y2), Pt::new(x3, y3));
                }
            }
            "v" | "V" => {
                if let (Some([x2, y2, x3, y3]), Some(from)) = (self.take(), self.last_point()) {
                    self.to(from, Pt::new(x2, y2), Pt::new(x3, y3));
                }
            }
            "y" | "Y" => {
                if let Some([x1, y1, x3, y3]) = self.take() {
                    let p = Pt::new(x3, y3);
                    self.to(Pt::new(x1, y1), p, p);
                }
            }
            // Close and paint: stroke, fill, both.
            "s" | "f" | "b" => self.paint(true, true),
            // Paint without closing (a fill closes by itself).
            "S" => self.paint(false, true),
            "F" | "B" => self.paint(true, true),
            // A path that is not painted (a guide or a mask).
            "n" | "N" | "H" | "h" => self.paint(op == "h" || op == "H", false),
            "*u" => {
                self.compound = true;
                self.pending.clear();
            }
            "*U" => {
                self.compound = false;
                let cs = std::mem::take(&mut self.pending);
                if !cs.is_empty() {
                    self.out.push(Outline { contours: cs, color: self.stroke, layer: self.layer.clone() });
                }
            }
            "Ln" => {
                if let Some(s) = self.string.take() {
                    self.layer = Some(s);
                }
            }
            "K" => {
                if let Some([c, m, y, k]) = self.take() {
                    self.stroke = cmyk_to_rgb(c, m, y, k);
                }
            }
            "k" => {
                if let Some([c, m, y, k]) = self.take() {
                    self.fill = cmyk_to_rgb(c, m, y, k);
                }
            }
            "XA" => {
                if let Some([r, g, b]) = self.take() {
                    self.stroke = [(r.clamp(0.0, 1.0) * 255.0) as u8, (g.clamp(0.0, 1.0) * 255.0) as u8, (b.clamp(0.0, 1.0) * 255.0) as u8];
                }
            }
            "G" => {
                if let Some([g]) = self.take() {
                    let v = (g.clamp(0.0, 1.0) * 255.0) as u8;
                    self.stroke = [v, v, v];
                }
            }
            _ => {}
        }
    }
}

fn rgb_to_cmyk(c: [u8; 3]) -> [f64; 4] {
    let (r, g, b) = (c[0] as f64 / 255.0, c[1] as f64 / 255.0, c[2] as f64 / 255.0);
    let k = 1.0 - r.max(g).max(b);
    if k >= 1.0 {
        return [0.0, 0.0, 0.0, 1.0];
    }
    [(1.0 - r - k) / (1.0 - k), (1.0 - g - k) / (1.0 - k), (1.0 - b - k) / (1.0 - k), k]
}

fn n(v: f64) -> String {
    let s = format!("{v:.5}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.to_string() }
}

/// An Illustrator file (PostScript, as LightBurn writes it) of the shapes `ids` (all when empty): one Illustrator layer for each layer
/// of the project, named by `layer_name`, stroked in its colour; the curves are kept. `area_h` is the height of the work area: the
/// scene counts Y down, Illustrator up. Pictures are left out.
pub fn write(scene: &Scene, ids: &[u64], area_h: f64, layer_name: &dyn Fn(usize) -> String, color: &dyn Fn(usize) -> [u8; 3]) -> String {
    let picked = |x: &&crate::scene::Shape| ids.is_empty() || ids.contains(&x.id);
    let mut used: Vec<usize> = scene.shapes.iter().filter(picked).filter(|x| !x.is_image()).map(|x| x.layer).collect();
    used.sort_unstable();
    used.dedup();
    let to_pt = Xf { a: 1.0 / PT_TO_MM, b: 0.0, c: 0.0, d: -1.0 / PT_TO_MM, e: 0.0, f: area_h / PT_TO_MM };
    let mut body = String::new();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for &l in &used {
        let name: String = layer_name(l).chars().map(|c| if matches!(c, '(' | ')' | '\\') { '_' } else { c }).collect();
        let [c, m, y, k] = rgb_to_cmyk(color(l));
        let _ = write!(body, "%AI5_BeginLayer\n1 1 1 1 0 0 0 0 0 0 Lb\n({name}) Ln\n1 XR\n0 A\n0 O\n0 J 0 j 1 w 10 M []0 d\n");
        for sh in scene.shapes.iter().filter(picked).filter(|x| x.layer == l) {
            for cont in crate::export::contours_of(sh) {
                let cont = cont.transformed(&to_pt);
                let len = cont.nodes.len();
                if len < 2 {
                    continue;
                }
                for nd in &cont.nodes {
                    for p in [nd.p, nd.hin, nd.hout] {
                        x0 = x0.min(p.x);
                        y0 = y0.min(p.y);
                        x1 = x1.max(p.x);
                        y1 = y1.max(p.y);
                    }
                }
                let _ = writeln!(body, "{} {} {} {} K", n(c), n(m), n(y), n(k));
                let _ = writeln!(body, "{} {} m", n(cont.nodes[0].p.x), n(cont.nodes[0].p.y));
                let segs = if cont.closed { len } else { len - 1 };
                for i in 0..segs {
                    let (a, b) = (cont.nodes[i], cont.nodes[(i + 1) % len]);
                    if a.has_out() || b.has_in() {
                        let _ = writeln!(body, "{} {} {} {} {} {} C", n(a.hout.x), n(a.hout.y), n(b.hin.x), n(b.hin.y), n(b.p.x), n(b.p.y));
                    } else {
                        let _ = writeln!(body, "{} {} L", n(b.p.x), n(b.p.y));
                    }
                }
                body.push_str(if cont.closed { "s\n" } else { "S\n" });
            }
        }
        body.push_str("LB\n%AI5_EndLayer--\n");
    }
    if x0 > x1 {
        (x0, y0, x1, y1) = (0.0, 0.0, 0.0, 0.0);
    }
    let mut s = String::new();
    let _ = write!(
        s,
        "%!PS-Adobe-3.0\n%%Creator: {}\n%%Title: \n%%BoundingBox: {} {} {} {}\n%%HiResBoundingBox: {} {} {} {}\n%%DocumentProcessColors: Cyan Magenta Yellow Black\n\
%%DocumentSuppliedResources: procset Adobe_level2_AI5 1.2 0\n%%+ procset Adobe_typography_AI5 1.0 1\n%%+ procset Adobe_ColorImage_AI6 1.1 0\n\
%%+ procset Adobe_Illustrator_AI5 1.2 0\n%%+ procset Adobe_cshow 2.0 8\n%AI3_ColorUsage: Color\n%AI3_DocumentPreview: None\n%AI5_NumLayers: {}\n\
%AI5_FileFormat 2.0\n%%EndComments\n%%BeginProlog\n%%IncludeResource: procset Adobe_level2_AI5 1.2 0\n%%IncludeResource: procset Adobe_typography_AI5 1.0 1\n\
%%IncludeResource: procset Adobe_ColorImage_AI6 1.1 0\n%%IncludeResource: procset Adobe_Illustrator_AI5 1.2 0\n%%IncludeResource: procset Adobe_cshow 2.0 8\n\
%%EndProlog\n%%BeginSetup\nAdobe_level2_AI5 /initialize get exec\nAdobe_cshow /initialize get exec\n\
Adobe_Illustrator_AI5_vars Adobe_Illustrator_AI5 Adobe_typography_AI5 /initialize get exec\nAdobe_ColorImage_AI6 /initialize get exec\n\
Adobe_Illustrator_AI5 /initialize get exec\n%%EndSetup\n",
        "baseCreator",
        x0.floor() as i64,
        y0.floor() as i64,
        x1.ceil() as i64,
        y1.ceil() as i64,
        n(x0),
        n(y0),
        n(x1),
        n(y1),
        used.len()
    );
    s.push_str(&body);
    s.push_str(
        "%%PageTrailer\ngsave annotatepage grestore showpage\n%%Trailer\nAdobe_Illustrator_AI5 /terminate get exec\nAdobe_ColorImage_AI6 /terminate get exec\n\
Adobe_typography_AI5 /terminate get exec\nAdobe_cshow /terminate get exec\nAdobe_level2_AI5 /terminate get exec\n%%EOF\n",
    );
    s
}
