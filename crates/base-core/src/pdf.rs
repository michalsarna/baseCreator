//! Import of PDF-compatible Adobe Illustrator (`.ai`) files, and plain PDFs: the vector paths and the pictures of the pages.
//! Illustrator files saved with "Create PDF Compatible File" (the default since CS) carry a PDF page; very old
//! PostScript-only files are read by `ai`. The Illustrator layers (optional content groups) are kept as the `layer` of each outline.
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
    /// The pictures drawn, each with the matrix that maps its unit square onto the page (in points).
    pics: Vec<(crate::image::ImageData, Xf)>,
    /// The marked-content sections that are open (`BMC` / `BDC` ... `EMC`): the layer of each, when it is one, and whether it is
    /// hidden.
    marks: Vec<(Option<String>, bool)>,
    /// The layers (optional content groups) that are switched off in the file: what is on them is not read.
    hidden: Vec<ObjectId>,
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
    fn layer_of(&self, operands: &[Object], res: Option<&Dictionary>) -> (Option<String>, bool) {
        if operands.first().and_then(|o| o.as_name().ok()) != Some(b"OC") {
            return (None, false);
        }
        let (group, id) = match operands.get(1) {
            Some(Object::Name(n)) => {
                let Some(props) = res.and_then(|r| r.get_deref(b"Properties", self.pdf).ok()).and_then(|o| o.as_dict().ok()) else { return (None, false) };
                let id = props.get(n).ok().and_then(|o| o.as_reference().ok());
                match props.get_deref(n, self.pdf).ok().and_then(|o| o.as_dict().ok()) {
                    Some(g) => (g, id),
                    None => return (None, false),
                }
            }
            Some(Object::Dictionary(d)) => (d, None),
            _ => return (None, false),
        };
        let hidden = id.is_some_and(|id| self.hidden.contains(&id));
        let name = match group.get_deref(b"Name", self.pdf) {
            Ok(Object::String(b, _)) => Some(text_string(b)),
            Ok(Object::Name(n)) => Some(String::from_utf8_lossy(n).into_owned()),
            _ => None,
        };
        (name, hidden)
    }

    /// Something open is on a layer that is switched off.
    fn in_hidden(&self) -> bool {
        self.marks.iter().any(|(_, h)| *h)
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
                    if !cs.is_empty() && !self.in_hidden() {
                        let color = if strokes { st.stroke } else { st.fill };
                        self.out.push((cs, color, self.marks.iter().find_map(|(l, _)| l.clone())));
                    }
                }
                "n" => {
                    path.take(false);
                }
                "BMC" => self.marks.push((None, false)),
                "BDC" => {
                    let l = self.layer_of(&op.operands, res);
                    self.marks.push(l);
                }
                "EMC" => {
                    self.marks.pop();
                }
                "Do" if depth < 8 && !self.in_hidden() => {
                    if let Some(Object::Name(n)) = op.operands.first() {
                        self.do_xobject(n, res, &st, depth);
                    }
                }
                _ => {}
            }
        }
    }

    fn do_xobject(&mut self, name: &[u8], res: Option<&Dictionary>, st: &State, depth: u32) {
        let Some(res) = res else { return };
        let Ok(xo) = res.get_deref(b"XObject", self.pdf).and_then(|o| o.as_dict()) else { return };
        let Ok(obj) = xo.get_deref(name, self.pdf) else { return };
        let Ok(stream) = obj.as_stream() else { return };
        match stream.dict.get(b"Subtype").ok().and_then(|o| o.as_name().ok()) {
            Some(b"Form") => {}
            Some(b"Image") => {
                if self.pics.len() < MAX_PICTURES {
                    let label = String::from_utf8_lossy(name).into_owned();
                    match decode_image(self.pdf, stream, &mut self.budget, self.stream_limit, &label) {
                        Ok(Some(pic)) => self.pics.push((pic, st.ctm)),
                        Ok(None) => {}
                        Err(()) => self.bomb = true,
                    }
                }
                return;
            }
            _ => return,
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

/// The most pictures read from a file, and the most pixels of one.
const MAX_PICTURES: usize = 64;
const MAX_PIXELS: u64 = 60_000_000;

/// The colour space of an image: how many samples a pixel has and how they become RGB.
enum Space {
    Gray,
    Rgb,
    Cmyk,
    /// One ink (a spot colour): 1 is full ink, so dark.
    Ink,
    /// A palette: the base space and its colours, `hival + 1` of them.
    Indexed(Box<Space>, Vec<u8>),
}

impl Space {
    fn comps(&self) -> usize {
        match self {
            Space::Gray | Space::Ink | Space::Indexed(..) => 1,
            Space::Rgb => 3,
            Space::Cmyk => 4,
        }
    }
    fn rgb(&self, s: &[u8]) -> [u8; 3] {
        match self {
            Space::Gray => [s[0]; 3],
            Space::Ink => [255 - s[0]; 3],
            Space::Rgb => [s[0], s[1], s[2]],
            Space::Cmyk => {
                let f = |v: u8| v as f64 / 255.0;
                let k = 1.0 - f(s[3]);
                [((1.0 - f(s[0])) * k * 255.0) as u8, ((1.0 - f(s[1])) * k * 255.0) as u8, ((1.0 - f(s[2])) * k * 255.0) as u8]
            }
            Space::Indexed(base, lookup) => {
                let n = base.comps();
                let i = s[0] as usize * n;
                match lookup.get(i..i + n) {
                    Some(c) => base.rgb(c),
                    None => [0, 0, 0],
                }
            }
        }
    }
}

fn space_of(pdf: &Pdf, o: &Object, depth: u32) -> Option<Space> {
    if depth > 4 {
        return None;
    }
    let o = match o {
        Object::Reference(id) => pdf.get_object(*id).ok()?,
        o => o,
    };
    match o {
        Object::Name(n) => match n.as_slice() {
            b"DeviceGray" | b"CalGray" | b"G" => Some(Space::Gray),
            b"DeviceRGB" | b"CalRGB" | b"RGB" => Some(Space::Rgb),
            b"DeviceCMYK" | b"CMYK" => Some(Space::Cmyk),
            _ => None,
        },
        Object::Array(a) => {
            let kind = a.first()?.as_name().ok()?;
            match kind {
                b"ICCBased" => {
                    let s = pdf.get_object(a.get(1)?.as_reference().ok()?).ok()?.as_stream().ok()?;
                    if let Ok(alt) = s.dict.get(b"Alternate") {
                        if let Some(sp) = space_of(pdf, alt, depth + 1) {
                            return Some(sp);
                        }
                    }
                    match s.dict.get(b"N").ok().map(num).unwrap_or(3.0) as i64 {
                        1 => Some(Space::Gray),
                        4 => Some(Space::Cmyk),
                        _ => Some(Space::Rgb),
                    }
                }
                b"CalGray" => Some(Space::Gray),
                b"CalRGB" | b"Lab" => Some(Space::Rgb),
                b"Separation" => Some(Space::Ink),
                b"DeviceN" => None,
                b"Indexed" | b"I" => {
                    let base = space_of(pdf, a.get(1)?, depth + 1)?;
                    let hival = num(a.get(2)?).max(0.0) as usize;
                    let lookup = match a.get(3)? {
                        Object::String(b, _) => b.clone(),
                        Object::Reference(id) => match pdf.get_object(*id).ok()? {
                            Object::String(b, _) => b.clone(),
                            Object::Stream(s) => s.decompressed_content().unwrap_or_else(|_| s.content.clone()),
                            _ => return None,
                        },
                        Object::Stream(s) => s.decompressed_content().unwrap_or_else(|_| s.content.clone()),
                        _ => return None,
                    };
                    let need = (hival + 1) * base.comps();
                    (lookup.len() >= need.min(lookup.len()) && !lookup.is_empty()).then_some(Space::Indexed(Box::new(base), lookup))
                }
                _ => None,
            }
        }
        _ => None,
    }
}

/// The last filter of a stream.
fn last_filter(dict: &Dictionary) -> Option<Vec<u8>> {
    match dict.get(b"Filter").ok()? {
        Object::Name(n) => Some(n.clone()),
        Object::Array(a) => a.last()?.as_name().ok().map(|n| n.to_vec()),
        _ => None,
    }
}

/// The samples of a stream that is not a JPEG: unpacked, within the budget. `Err` when it unpacks to more than it may.
fn samples(stream: &lopdf::Stream, budget: &mut usize, limit: usize) -> Result<Option<Vec<u8>>, ()> {
    if stream.dict.get(b"Filter").is_err() {
        return Ok(Some(stream.content.clone()));
    }
    match stream.decompressed_content_with_limit(limit.min(*budget)) {
        Ok(d) => {
            *budget = budget.saturating_sub(d.len());
            Ok(Some(d))
        }
        Err(lopdf::Error::Decompress(_)) => Err(()),
        Err(_) => Ok(None),
    }
}

/// The gray samples (0..255) of a soft mask: the transparency of a picture.
fn soft_mask(pdf: &Pdf, dict: &Dictionary, w: u32, h: u32, budget: &mut usize, limit: usize) -> Result<Option<Vec<u8>>, ()> {
    let Ok(id) = dict.get(b"SMask").and_then(|o| o.as_reference()) else { return Ok(None) };
    let Ok(m) = pdf.get_object(id).and_then(|o| o.as_stream()) else { return Ok(None) };
    let mw = m.dict.get(b"Width").map(num).unwrap_or(0.0) as u32;
    let mh = m.dict.get(b"Height").map(num).unwrap_or(0.0) as u32;
    let bits = m.dict.get(b"BitsPerComponent").map(num).unwrap_or(8.0) as u32;
    if mw != w || mh != h || bits != 8 || last_filter(&m.dict).as_deref() == Some(b"DCTDecode") {
        return Ok(None);
    }
    Ok(samples(m, budget, limit)?.filter(|d| d.len() >= (w * h) as usize))
}

/// Decode an image XObject into a picture (its size is set later from where it lies). `Ok(None)`: a kind that is not read (a mask, a
/// JPEG 2000, an unknown colour space); `Err`: it unpacks to more than it may.
fn decode_image(pdf: &Pdf, stream: &lopdf::Stream, budget: &mut usize, limit: usize, name: &str) -> Result<Option<crate::image::ImageData>, ()> {
    let d = &stream.dict;
    if d.get(b"ImageMask").ok().and_then(|o| o.as_bool().ok()) == Some(true) {
        return Ok(None);
    }
    let w = d.get(b"Width").map(num).unwrap_or(0.0) as u32;
    let h = d.get(b"Height").map(num).unwrap_or(0.0) as u32;
    if w == 0 || h == 0 || w as u64 * h as u64 > MAX_PIXELS {
        return Ok(None);
    }
    let alpha = soft_mask(pdf, d, w, h, budget, limit)?;
    let mut rgba: Vec<u8>;
    match last_filter(d).as_deref() {
        Some(b"DCTDecode") => {
            // A JPEG: the stream holds the file (after any filters before it).
            let bytes = if matches!(d.get(b"Filter"), Ok(Object::Array(a)) if a.len() > 1) {
                match samples(stream, budget, limit)? {
                    Some(b) => b,
                    None => return Ok(None),
                }
            } else {
                stream.content.clone()
            };
            let Ok(img) = image::load_from_memory_with_format(&bytes, image::ImageFormat::Jpeg) else { return Ok(None) };
            let img = img.to_rgba8();
            if img.width() != w || img.height() != h {
                return Ok(None);
            }
            // The JPEG decoder already turns the inverted CMYK of Adobe (what Illustrator writes with its Decode of [1 0 ...]) into
            // the right colours, so the Decode array is not applied again.
            rgba = img.into_raw();
        }
        Some(b"JPXDecode") | Some(b"JBIG2Decode") | Some(b"CCITTFaxDecode") => return Ok(None),
        _ => {
            let Some(space) = d.get(b"ColorSpace").ok().and_then(|o| space_of(pdf, o, 0)) else { return Ok(None) };
            let bits = d.get(b"BitsPerComponent").map(num).unwrap_or(8.0) as u32;
            if !matches!(bits, 1 | 2 | 4 | 8) {
                return Ok(None);
            }
            let Some(data) = samples(stream, budget, limit)? else { return Ok(None) };
            let n = space.comps();
            let row = (w as usize * n * bits as usize).div_ceil(8);
            if data.len() < row * h as usize {
                return Ok(None);
            }
            // Decode [1 0] swaps the samples of a gray or ink picture; an indexed one keeps its indices.
            let invert = !matches!(space, Space::Indexed(..))
                && d.get(b"Decode").ok().and_then(|o| o.as_array().ok()).is_some_and(|a| a.len() >= 2 && num(&a[0]) > num(&a[1]));
            let max = (1u32 << bits) - 1;
            rgba = Vec::with_capacity((w * h * 4) as usize);
            let mut px = vec![0u8; n];
            for y in 0..h as usize {
                let line = &data[y * row..(y + 1) * row];
                for x in 0..w as usize {
                    for (c, v) in px.iter_mut().enumerate() {
                        let i = (x * n + c) * bits as usize;
                        let raw = if bits == 8 { line[i / 8] as u32 } else { (line[i / 8] as u32 >> (8 - bits - (i % 8) as u32)) & max };
                        let raw = if invert { max - raw } else { raw };
                        *v = if matches!(space, Space::Indexed(..)) { raw as u8 } else { (raw * 255 / max) as u8 };
                    }
                    let [r, g, b] = space.rgb(&px);
                    rgba.extend_from_slice(&[r, g, b, 255]);
                }
            }
        }
    }
    if let Some(a) = alpha {
        for (p, a) in rgba.chunks_exact_mut(4).zip(a) {
            p[3] = a;
        }
    }
    Ok(crate::image::ImageData::from_rgba(w, h, &rgba, name, w.max(h) as f64).ok())
}

/// The layers switched off in the default view of the file (`/OCProperties /D /OFF`).
fn hidden_layers(pdf: &Pdf) -> Vec<ObjectId> {
    let off = || -> Option<Vec<ObjectId>> {
        let cat = pdf.catalog().ok()?;
        let d = cat.get_deref(b"OCProperties", pdf).ok()?.as_dict().ok()?.get_deref(b"D", pdf).ok()?.as_dict().ok()?;
        Some(d.get_deref(b"OFF", pdf).ok()?.as_array().ok()?.iter().filter_map(|o| o.as_reference().ok()).collect())
    };
    off().unwrap_or_default()
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
type Paths = Vec<(Vec<Contour>, [u8; 3], Option<String>)>;
type Pictures = Vec<(crate::image::ImageData, Xf)>;

fn read_page(pdf: &Pdf, page: ObjectId, limits: &Limits, budget: &mut usize) -> Result<(Paths, Pictures, f64), String> {
    let mb = media_box(pdf, page).unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let bomb = || format!("the file unpacks to far more than it should (more than {} MB): it is refused", limits.stream / 1_000_000);
    let content = pdf.get_page_content_with_limit(page, limits.stream.min(*budget)).map_err(|e| match e {
        lopdf::Error::Decompress(_) => bomb(),
        other => other.to_string(),
    })?;
    *budget = budget.saturating_sub(content.len());
    let res = pdf.get_page_resources(page).ok().and_then(|(d, _)| d);
    let mut ctx = Ctx { pdf, height: mb[3] - mb[1], out: vec![], pics: vec![], marks: vec![], hidden: hidden_layers(pdf), budget: *budget, stream_limit: limits.stream, bomb: false };
    // Move the page origin to (0, 0): translate by the box origin before the page content's own matrices.
    let st = State { ctm: Xf::translate(-mb[0], -(mb[1])), stroke: [0, 0, 0], fill: [0, 0, 0] };
    ctx.run(&content, res, st, 0);
    if ctx.bomb {
        return Err(bomb());
    }
    *budget = ctx.budget;
    // A picture lies on its unit square, row 0 at the top: its millimetres (y down) → the unit square → the page → millimetres.
    let k = PT_TO_MM;
    let page_mm = Xf { a: k, b: 0.0, c: 0.0, d: -k, e: 0.0, f: ctx.height * k };
    let pics = ctx
        .pics
        .into_iter()
        .filter_map(|(mut pic, ctm)| {
            let (w, h) = (ctm.a.hypot(ctm.b) * k, ctm.c.hypot(ctm.d) * k);
            if w < 1e-6 || h < 1e-6 {
                return None;
            }
            pic.w = w;
            pic.h = h;
            let xf = Xf { a: 1.0 / w, b: 0.0, c: 0.0, d: -1.0 / h, e: 0.0, f: 1.0 }.then(ctm).then(page_mm);
            Some((pic, xf))
        })
        .collect();
    Ok((ctx.out, pics, (mb[2] - mb[0]) * PT_TO_MM))
}

/// Whether the bytes are a PDF or an Illustrator file (of either kind), an EPS or a VectorCraft drawing: not an SVG.
pub fn is_pdf_or_ai(data: &[u8]) -> bool {
    data.windows(5).take(1024).any(|w| w == b"%PDF-") || crate::ai::is_postscript_ai(data) || crate::eps::is_eps(data) || crate::vectorcraft::is_vectorcraft(data)
}

/// Read the pages of a PDF / Illustrator file into outlines. More than one page (the artboards of an Illustrator file) are put
/// side by side, in the order of the pages, with a gap between them.
pub fn parse(data: &[u8]) -> Result<Vec<crate::svg::Outline>, String> {
    parse_with(data, &LIMITS)
}

pub fn parse_with(data: &[u8], limits: &Limits) -> Result<Vec<crate::svg::Outline>, String> {
    Ok(read_all(data, limits)?.0)
}

/// The pictures of a PDF / Illustrator file (an embedded photo, for example), each with the transform from the picture (0..w, 0..h
/// millimetres) to the drawing, in the millimetres of `parse`. None when the file cannot be read.
pub fn images(data: &[u8]) -> Vec<(crate::image::ImageData, Xf)> {
    if crate::vectorcraft::is_vectorcraft(data) || crate::eps::is_eps(data) || crate::ai::is_postscript_ai(data) {
        return vec![];
    }
    read_all(data, &LIMITS).map(|(_, p)| p).unwrap_or_default()
}

fn read_all(data: &[u8], limits: &Limits) -> Result<(Vec<crate::svg::Outline>, Pictures), String> {
    // A VectorCraft drawing and an EPS have readers of their own.
    if crate::vectorcraft::is_vectorcraft(data) {
        return Ok((crate::vectorcraft::parse(data)?, vec![]));
    }
    if crate::eps::is_eps(data) {
        return Ok((crate::eps::parse(data)?, vec![]));
    }
    // An Illustrator file of the old PostScript kind (LightBurn writes those) has no PDF page.
    if crate::ai::is_postscript_ai(data) {
        return Ok((crate::ai::parse(data)?, vec![]));
    }
    let pdf = load(data, limits)?;
    let pages = pdf.get_pages();
    if pages.is_empty() {
        return Err("the file has no pages".into());
    }
    let mut budget = limits.total;
    let mut out: Vec<crate::svg::Outline> = vec![];
    let mut pictures: Pictures = vec![];
    let mut x = 0.0;
    for (_, page) in pages.iter().take(MAX_PAGES) {
        let (paths, pics, width) = read_page(&pdf, *page, limits, &mut budget)?;
        let shift = Xf::translate(x, 0.0);
        pictures.extend(pics.into_iter().map(|(p, xf)| (p, xf.then(shift))));
        for (contours, color, layer) in paths {
            let contours = if x == 0.0 { contours } else { contours.iter().map(|c| c.transformed(&shift)).collect() };
            out.push(crate::svg::Outline { contours, color, layer });
        }
        x += width + PAGE_GAP_MM;
    }
    if out.is_empty() && pictures.is_empty() {
        return Err("no vector paths found".into());
    }
    Ok((out, pictures))
}
