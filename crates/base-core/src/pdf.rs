//! Import of PDF-compatible Adobe Illustrator (`.ai`) files, and plain PDFs: vector paths of the first page.
//! Illustrator files saved with "Create PDF Compatible File" (the default since CS) carry a PDF page; very old
//! PostScript-only files are not supported.
use crate::bezier::{Contour, Node};
use crate::geom::*;
use lopdf::content::Content;
use lopdf::{Dictionary, Document as Pdf, Object, ObjectId};

const PT_TO_MM: f64 = 25.4 / 72.0;

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
    out: Vec<(Vec<Contour>, [u8; 3])>,
}

impl Ctx<'_> {
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
                        self.out.push((cs, color));
                    }
                }
                "n" => {
                    path.take(false);
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
        let Ok(data) = stream.decompressed_content() else { return };
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

/// Read the first page of a PDF / Illustrator file into outlines (stroke colour, else fill colour).
pub fn parse(data: &[u8]) -> Result<Vec<crate::svg::Outline>, String> {
    // Illustrator files may carry bytes before the PDF header.
    let start = data.windows(5).position(|w| w == b"%PDF-").unwrap_or(0);
    let pdf = Pdf::load_mem(&data[start..]).map_err(|e| e.to_string())?;
    let pages = pdf.get_pages();
    let (_, page) = pages.iter().next().ok_or("the file has no pages")?;
    let mb = media_box(&pdf, *page).unwrap_or([0.0, 0.0, 612.0, 792.0]);
    let content = pdf.get_page_content(*page);
    let res = pdf.get_page_resources(*page).ok().and_then(|(d, _)| d);
    let mut ctx = Ctx { pdf: &pdf, height: mb[3], out: vec![] };
    // Move the page origin to (0, 0): translate by the box origin before the page content's own matrices.
    let st = State { ctm: Xf::translate(-mb[0], -(mb[1])), stroke: [0, 0, 0], fill: [0, 0, 0] };
    ctx.height = mb[3] - mb[1];
    ctx.run(&content, res, st, 0);
    if ctx.out.is_empty() {
        return Err("no vector paths found".into());
    }
    Ok(ctx.out.into_iter().map(|(contours, color)| crate::svg::Outline { contours, color }).collect())
}
