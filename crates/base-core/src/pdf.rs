//! Import of PDF-compatible Adobe Illustrator (`.ai`) files, and plain PDFs: vector paths of the first page.
//! Illustrator files saved with "Create PDF Compatible File" (the default since CS) carry a PDF page; very old
//! PostScript-only files are not supported. The Illustrator layers (optional content groups) are kept as the `layer` of each outline.
use crate::bezier::{Contour, Node};
use crate::geom::*;
use lopdf::content::Content;
use lopdf::{Dictionary, Document as Pdf, Object, ObjectId};

const PT_TO_MM: f64 = 25.4 / 72.0;

/// The biggest file read (a PDF can be made to unpack to much more than its size, so the file itself is kept small).
const MAX_FILE: usize = 64_000_000;

#[derive(Clone)]
struct State {
    ctm: Xf,
    stroke: [u8; 3],
    fill: [u8; 3],
}

fn num(o: &Object) -> f64 {
    o.as_float().map(|v| v as f64).or_else(|_| o.as_i64().map(|v| v as f64)).unwrap_or(0.0)
}

fn nums(ops: &[Object]) -> Vec<f64> {
    ops.iter().map(num).collect()
}

fn gray(g: f64) -> [u8; 3] {
    let v = (g.clamp(0.0, 1.0) * 255.0) as u8;
    [v, v, v]
}

fn rgb(c: &[f64]) -> [u8; 3] {
    let f = |i: usize| (c.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0) * 255.0) as u8;
    [f(0), f(1), f(2)]
}

fn cmyk(c: &[f64]) -> [u8; 3] {
    let g = |i: usize| c.get(i).copied().unwrap_or(0.0).clamp(0.0, 1.0);
    let k = 1.0 - g(3);
    [((1.0 - g(0)) * k * 255.0) as u8, ((1.0 - g(1)) * k * 255.0) as u8, ((1.0 - g(2)) * k * 255.0) as u8]
}

fn color_from(c: &[f64]) -> [u8; 3] {
    match c.len() {
        1 => gray(c[0]),
        3 => rgb(c),
        4 => cmyk(c),
        _ => [0, 0, 0],
    }
}

fn matrix(a: &[f64]) -> Xf {
    if a.len() >= 6 {
        Xf { a: a[0], b: a[1], c: a[2], d: a[3], e: a[4], f: a[5] }
    } else {
        Xf::IDENTITY
    }
}

/// Builds Bézier contours from path-construction operators.
#[derive(Default)]
struct PathBuilder {
    contours: Vec<Contour>,
    cur: Vec<Node>,
    start: Pt,
}

impl PathBuilder {
    fn end(&mut self, closed: bool) {
        let mut nodes = std::mem::take(&mut self.cur);
        if closed && nodes.len() > 2 && nodes[0].p.dist(nodes[nodes.len() - 1].p) < 1e-6 {
            let last = nodes.pop().unwrap_or(nodes[0]);
            nodes[0].hin = last.hin;
        }
        if nodes.len() >= 2 {
            self.contours.push(Contour { nodes, closed });
        }
    }
    fn move_to(&mut self, p: Pt) {
        self.end(false);
        self.start = p;
        self.cur.push(Node::corner(p));
    }
    fn line_to(&mut self, p: Pt) {
        if self.cur.is_empty() {
            self.cur.push(Node::corner(p));
            self.start = p;
        } else {
            self.cur.push(Node::corner(p));
        }
    }
    fn curve_to(&mut self, c1: Pt, c2: Pt, p: Pt) {
        if let Some(n) = self.cur.last_mut() {
            n.hout = c1;
            self.cur.push(Node { p, hin: c2, hout: p, smooth: false });
        } else {
            self.move_to(p);
        }
    }
    fn last(&self) -> Pt {
        self.cur.last().map(|n| n.p).unwrap_or(self.start)
    }
    fn close(&mut self) {
        let s = self.start;
        self.end(true);
        // The next segment without a move starts at the subpath start.
        self.cur.push(Node::corner(s));
    }
    fn take(&mut self, close_all: bool) -> Vec<Contour> {
        if self.cur.len() == 1 {
            self.cur.clear();
        }
        self.end(close_all);
        std::mem::take(&mut self.contours)
    }
}

struct Ctx<'a> {
    pdf: &'a Pdf,
    /// Page height in points, to flip y.
    height: f64,
    out: Vec<(Vec<Contour>, [u8; 3], Option<String>)>,
    /// The marked-content sections that are open (`BMC` / `BDC` ... `EMC`): the layer of each, when it is one.
    marks: Vec<Option<String>>,
    /// How much may still be unpacked in all (shared by every stream of the file), and the most for one stream.
    budget: usize,
    stream_limit: usize,
    /// Something unpacked to more than it may: the file is refused.
    bomb: bool,
}

/// A PDF text string (UTF-16 with a byte order mark, or PDFDocEncoding ~ Latin-1).
fn text_string(b: &[u8]) -> String {
    if b.len() >= 2 && b[0] == 0xFE && b[1] == 0xFF {
        let u: Vec<u16> = b[2..].chunks_exact(2).map(|c| u16::from_be_bytes([c[0], c[1]])).collect();
        String::from_utf16_lossy(&u)
    } else if b.len() >= 3 && b[0] == 0xEF && b[1] == 0xBB && b[2] == 0xBF {
        String::from_utf8_lossy(&b[3..]).into_owned()
    } else {
        b.iter().map(|&c| c as char).collect()
    }
}

impl Ctx<'_> {
    /// The layer named by the operands of a `BDC`: `/OC /Name` with the group in the `Properties` of the resources, or the group inline.
    fn layer_of(&self, operands: &[Object], res: Option<&Dictionary>) -> Option<String> {
        if operands.first().and_then(|o| o.as_name().ok()) != Some(b"OC") {
            return None;
        }
        let group = match operands.get(1)? {
            Object::Name(n) => res?.get_deref(b"Properties", self.pdf).ok()?.as_dict().ok()?.get_deref(n, self.pdf).ok()?.as_dict().ok()?,
            Object::Dictionary(d) => d,
            _ => return None,
        };
        match group.get_deref(b"Name", self.pdf).ok()? {
            Object::String(b, _) => Some(text_string(b)),
            Object::Name(n) => Some(String::from_utf8_lossy(n).into_owned()),
            _ => None,
        }
    }

    fn run(&mut self, data: &[u8], res: Option<&Dictionary>, mut st: State, depth: u32) {
        let Ok(content) = Content::decode(data) else { return };
        let mut stack: Vec<State> = vec![];
        let mut path = PathBuilder::default();
        let tp = |st: &State, h: f64, x: f64, y: f64| -> Pt {
            let p = st.ctm.apply(Pt::new(x, y));
            Pt::new(p.x * PT_TO_MM, (h - p.y) * PT_TO_MM)
        };
        for op in &content.operations {
            let a = nums(&op.operands);
            let h = self.height;
            match op.operator.as_str() {
                "q" => stack.push(st.clone()),
                "Q" => {
                    if let Some(s) = stack.pop() {
                        st = s;
                    }
                }
                "cm" => st.ctm = matrix(&a).then(st.ctm),
                "m" if a.len() >= 2 => path.move_to(tp(&st, h, a[0], a[1])),
                "l" if a.len() >= 2 => path.line_to(tp(&st, h, a[0], a[1])),
                "c" if a.len() >= 6 => path.curve_to(tp(&st, h, a[0], a[1]), tp(&st, h, a[2], a[3]), tp(&st, h, a[4], a[5])),
                "v" if a.len() >= 4 => {
                    let l = path.last();
                    path.curve_to(l, tp(&st, h, a[0], a[1]), tp(&st, h, a[2], a[3]))
                }
                "y" if a.len() >= 4 => {
                    let p = tp(&st, h, a[2], a[3]);
                    path.curve_to(tp(&st, h, a[0], a[1]), p, p)
                }
                "re" if a.len() >= 4 => {
                    let (x, y, w, hh) = (a[0], a[1], a[2], a[3]);
                    path.move_to(tp(&st, h, x, y));
                    path.line_to(tp(&st, h, x + w, y));
                    path.line_to(tp(&st, h, x + w, y + hh));
                    path.line_to(tp(&st, h, x, y + hh));
                    path.close();
                    path.cur.clear();
                }
                "h" => path.close(),
                "G" => st.stroke = gray(a.first().copied().unwrap_or(0.0)),
                "g" => st.fill = gray(a.first().copied().unwrap_or(0.0)),
                "RG" => st.stroke = rgb(&a),
                "rg" => st.fill = rgb(&a),
                "K" => st.stroke = cmyk(&a),
                "k" => st.fill = cmyk(&a),
                "SC" | "SCN" => st.stroke = color_from(&a),
                "sc" | "scn" => st.fill = color_from(&a),
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" => {
                    let op_name = op.operator.as_str();
                    let strokes = matches!(op_name, "S" | "s" | "B" | "B*" | "b" | "b*");
                    let fills = matches!(op_name, "f" | "F" | "f*" | "B" | "B*" | "b" | "b*");
                    let closes = matches!(op_name, "s" | "b" | "b*") || (fills && !strokes);
                    let cs = path.take(closes);
                    if !cs.is_empty() {
                        let color = if strokes { st.stroke } else { st.fill };
                        self.out.push((cs, color, self.marks.iter().flatten().next().cloned()));
                    }
                }
                "n" => {
                    path.take(false);
                }
                "BMC" => self.marks.push(None),
                "BDC" => {
                    let l = self.layer_of(&op.operands, res);
                    self.marks.push(l);
                }
                "EMC" => {
                    self.marks.pop();
                }
                "Do" if depth < 8 => {
                    if let Some(Object::Name(n)) = op.operands.first() {
                        self.do_form(n, res, &st, depth);
                    }
                }
                _ => {}
            }
        }
    }

    fn do_form(&mut self, name: &[u8], res: Option<&Dictionary>, st: &State, depth: u32) {
        let Some(res) = res else { return };
        let Ok(xo) = res.get_deref(b"XObject", self.pdf).and_then(|o| o.as_dict()) else { return };
        let Ok(obj) = xo.get_deref(name, self.pdf) else { return };
        let Ok(stream) = obj.as_stream() else { return };
        if stream.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) != Some(b"Form") {
            return;
        }
        let m = stream.dict.get(b"Matrix").ok().and_then(|o| o.as_array().ok()).map(|a| matrix(&nums(a))).unwrap_or(Xf::IDENTITY);
        let mut inner = st.clone();
        inner.ctm = m.then(st.ctm);
        let cap = self.stream_limit.min(self.budget);
        let data = match stream.decompressed_content_with_limit(cap) {
            Ok(d) => d,
            Err(lopdf::Error::Decompress(_)) => {
                self.bomb = true;
                return;
            }
            Err(_) => return,
        };
        self.budget = self.budget.saturating_sub(data.len());
        let form_res = stream.dict.get_deref(b"Resources", self.pdf).ok().and_then(|o| o.as_dict().ok()).or(Some(res));
        self.run(&data, form_res, inner, depth + 1);
    }
}

fn media_box(pdf: &Pdf, page: ObjectId) -> Option<[f64; 4]> {
    let mut id = page;
    for _ in 0..16 {
        let d = pdf.get_dictionary(id).ok()?;
        for key in [&b"CropBox"[..], &b"MediaBox"[..]] {
            if let Ok(a) = d.get_deref(key, pdf).and_then(|o| o.as_array()) {
                let v = nums(a);
                if v.len() == 4 {
                    return Some([v[0].min(v[2]), v[1].min(v[3]), v[0].max(v[2]), v[1].max(v[3])]);
                }
            }
        }
        id = d.get(b"Parent").ok()?.as_reference().ok()?;
    }
    None
}

/// How much a file may unpack to: for one stream, and for all of them together. A small file can be made to unpack to
/// gigabytes (a decompression bomb), so both are limited.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub stream: usize,
    pub total: usize,
}

pub const LIMITS: Limits = Limits { stream: 64_000_000, total: 256_000_000 };

/// The most pages that are read.
pub const MAX_PAGES: usize = 50;
/// The space between pages that are put side by side, millimetres.
pub const PAGE_GAP_MM: f64 = 10.0;

fn load(data: &[u8], limits: &Limits) -> Result<Pdf, String> {
    if data.len() > MAX_FILE {
        return Err(format!("the file is too big ({} MB; the limit is {} MB)", data.len() / 1_000_000, MAX_FILE / 1_000_000));
    }
    // Illustrator files may carry bytes before the PDF header.
    let Some(start) = data.windows(5).take(4096).position(|w| w == b"%PDF-") else {
        let postscript = data.starts_with(b"%!PS-Adobe") || data.starts_with(b"%!");
        return Err(if postscript {
            "this is a PostScript-only Illustrator file; open it in Illustrator and save it again with \"Create PDF Compatible File\"".into()
        } else {
            "this is not a PDF or a PDF-compatible Illustrator file".into()
        });
    };
    Pdf::load_mem_with_options(&data[start..], lopdf::LoadOptions::with_max_decompressed_size(limits.stream)).map_err(|e| e.to_string())
}

/// How many pages (artboards of an Illustrator file) the file has.
pub fn page_count(data: &[u8]) -> Result<usize, String> {
    Ok(load(data, &LIMITS)?.get_pages().len())
}

/// Read one page (1 is the first) into outlines (stroke colour, else fill colour) in millimetres, the top left corner of the
/// page at (0, 0).
fn read_page(pdf: &Pdf, page: ObjectId, limits: &Limits, budget: &mut usize) -> Result<(Vec<(Vec<Contour>, [u8; 3], Option<String>)>, f64), String> {
    let mb = media_box(pdf, page).unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let bomb = || format!("the file unpacks to far more than it should (more than {} MB): it is refused", limits.stream / 1_000_000);
    let content = pdf.get_page_content_with_limit(page, limits.stream.min(*budget)).map_err(|e| match e {
        lopdf::Error::Decompress(_) => bomb(),
        other => other.to_string(),
    })?;
    *budget = budget.saturating_sub(content.len());
    let res = pdf.get_page_resources(page).ok().and_then(|(d, _)| d);
    let mut ctx = Ctx { pdf, height: mb[3] - mb[1], out: vec![], marks: vec![], budget: *budget, stream_limit: limits.stream, bomb: false };
    // Move the page origin to (0, 0): translate by the box origin before the page content's own matrices.
    let st = State { ctm: Xf::translate(-mb[0], -(mb[1])), stroke: [0, 0, 0], fill: [0, 0, 0] };
    ctx.run(&content, res, st, 0);
    if ctx.bomb {
        return Err(bomb());
    }
    *budget = ctx.budget;
    Ok((ctx.out, (mb[2] - mb[0]) * PT_TO_MM))
}

/// Read the pages of a PDF / Illustrator file into outlines. More than one page (the artboards of an Illustrator file) are put
/// side by side, in the order of the pages, with a gap between them.
pub fn parse(data: &[u8]) -> Result<Vec<crate::svg::Outline>, String> {
    parse_with(data, &LIMITS)
}

pub fn parse_with(data: &[u8], limits: &Limits) -> Result<Vec<crate::svg::Outline>, String> {
    let pdf = load(data, limits)?;
    let pages = pdf.get_pages();
    if pages.is_empty() {
        return Err("the file has no pages".into());
    }
    let mut budget = limits.total;
    let mut out: Vec<crate::svg::Outline> = vec![];
    let mut x = 0.0;
    for (_, page) in pages.iter().take(MAX_PAGES) {
        let (paths, width) = read_page(&pdf, *page, limits, &mut budget)?;
        let shift = Xf::translate(x, 0.0);
        for (contours, color, layer) in paths {
            let contours = if x == 0.0 { contours } else { contours.iter().map(|c| c.transformed(&shift)).collect() };
            out.push(crate::svg::Outline { contours, color, layer });
        }
        x += width + PAGE_GAP_MM;
    }
    if out.is_empty() {
        return Err("no vector paths found".into());
    }
    Ok(out)
}
