//! Writing shapes out as SVG and DXF: the shapes of a scene (all, or a selection), in millimetres, the way the work area shows them
//! (Y down for SVG, Y up for DXF).
use crate::bezier::Contour;
use crate::geom::{Polyline, Pt};
use crate::scene::{Kind, Scene, Shape};
use std::fmt::Write;

/// The contours of a shape on the work area. A rectangle, an ellipse and live text have theirs made; a picture has none.
pub fn contours_of(s: &Shape) -> Vec<Contour> {
    match &s.kind {
        Kind::Rect { w, h } => vec![Contour::rect(*w, *h).transformed(&s.xf)],
        Kind::Ellipse { w, h } => vec![Contour::ellipse(*w, *h).transformed(&s.xf)],
        Kind::Bezier(cs) => cs.iter().map(|c| c.transformed(&s.xf)).collect(),
        Kind::Path(ps) => ps.iter().map(|p| Contour::from_polyline(&p.transformed(&s.xf))).collect(),
        Kind::Text(_) => s.polys().iter().map(Contour::from_polyline).collect(),
        Kind::Image(_) => vec![],
    }
}

fn num(v: f64) -> String {
    let s = format!("{v:.4}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s.is_empty() || s == "-0" { "0".into() } else { s.to_string() }
}

/// The `d` of an SVG path for the contours (curves are kept).
fn path_data(cs: &[Contour]) -> String {
    let mut d = String::new();
    for c in cs {
        let n = c.nodes.len();
        if n < 2 {
            continue;
        }
        let _ = write!(d, "M{} {}", num(c.nodes[0].p.x), num(c.nodes[0].p.y));
        let segs = if c.closed { n } else { n - 1 };
        for i in 0..segs {
            let (a, b) = (c.nodes[i], c.nodes[(i + 1) % n]);
            let curve = a.has_out() || b.has_in();
            // The straight piece back to the start of a closed path is what Z does.
            if !curve && c.closed && i == n - 1 {
                continue;
            }
            if curve {
                let _ = write!(d, "C{} {} {} {} {} {}", num(a.hout.x), num(a.hout.y), num(b.hin.x), num(b.hin.y), num(b.p.x), num(b.p.y));
            } else {
                let _ = write!(d, "L{} {}", num(b.p.x), num(b.p.y));
            }
        }
        if c.closed {
            d.push('Z');
        }
    }
    d
}

/// An SVG of the shapes `ids` (all the shapes when `ids` is empty), `w` × `h` millimetres, one path per shape in the colour of its
/// layer (`color(layer)`), with the layer in a group named by `layer_name`.
pub fn svg(scene: &Scene, ids: &[u64], w: f64, h: f64, layer_name: &dyn Fn(usize) -> String, color: &dyn Fn(usize) -> [u8; 3]) -> String {
    let mut s = String::new();
    let _ = write!(s, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">\n");
    let mut layers: Vec<usize> = scene.shapes.iter().filter(|x| ids.is_empty() || ids.contains(&x.id)).map(|x| x.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    for l in layers {
        let name = layer_name(l).replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;");
        let c = color(l);
        let _ = writeln!(s, "<g id=\"{name}\" fill=\"none\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"0.1\">", c[0], c[1], c[2]);
        for sh in scene.shapes.iter().filter(|x| x.layer == l && (ids.is_empty() || ids.contains(&x.id))) {
            if let Kind::Image(im) = &sh.kind {
                if let Some(png) = crate::lightburn::png_of(im) {
                    use base64::Engine;
                    let x = sh.xf;
                    let data = base64::engine::general_purpose::STANDARD.encode(png);
                    let _ = writeln!(s, "<image transform=\"matrix({},{},{},{},{},{})\" x=\"0\" y=\"0\" width=\"{}\" height=\"{}\" preserveAspectRatio=\"none\" xlink:href=\"data:image/png;base64,{data}\"/>", num(x.a), num(x.b), num(x.c), num(x.d), num(x.e), num(x.f), num(im.w), num(im.h));
                }
                continue;
            }
            let d = path_data(&contours_of(sh));
            if !d.is_empty() {
                let _ = writeln!(s, "<path d=\"{d}\"/>");
            }
        }
        s.push_str("</g>\n");
    }
    s.push_str("</svg>\n");
    s
}

/// A DXF of the old kind (R12, `AC1009`, ASCII, millimetres; *legacy* in LightBurn) of the shapes `ids` (all when empty): one DXF layer for each layer of the project, named by
/// `layer_name`; the curves are flattened to within 0.01 mm. `area_h` is the height of the work area: the scene counts Y down, DXF up.
pub fn dxf(scene: &Scene, ids: &[u64], area_h: f64, layer_name: &dyn Fn(usize) -> String) -> String {
    let clean = |n: String| -> String { n.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect() };
    let mut layers: Vec<usize> = scene.shapes.iter().filter(|x| ids.is_empty() || ids.contains(&x.id)).map(|x| x.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    let mut s = String::new();
    s.push_str("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    let _ = write!(s, "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLAYER\n70\n{}\n", layers.len());
    for l in &layers {
        let _ = write!(s, "0\nLAYER\n2\n{}\n70\n0\n62\n7\n6\nCONTINUOUS\n", clean(layer_name(*l)));
    }
    s.push_str("0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n");
    for l in &layers {
        let name = clean(layer_name(*l));
        for sh in scene.shapes.iter().filter(|x| x.layer == *l && (ids.is_empty() || ids.contains(&x.id))) {
            for c in contours_of(sh) {
                let p: Polyline = c.flatten(0.01);
                if p.pts.len() < 2 {
                    continue;
                }
                let _ = write!(s, "0\nPOLYLINE\n8\n{}\n66\n1\n70\n{}\n", name, if p.closed { 1 } else { 0 });
                for q in &p.pts {
                    let q = Pt::new(q.x, area_h - q.y);
                    let _ = write!(s, "0\nVERTEX\n8\n{}\n10\n{:.4}\n20\n{:.4}\n", name, q.x, q.y);
                }
                let _ = write!(s, "0\nSEQEND\n8\n{}\n", name);
            }
        }
    }
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

/// A DXF of the current kind (AutoCAD 2007, `AC1021`, ASCII, millimetres) of the shapes `ids` (all when empty), laid out as LightBurn
/// writes it: the tables a strict reader wants (handles, block records, model and paper space), one DXF layer for each layer of the
/// project, every contour an `LWPOLYLINE` with the curves flattened to within 0.01 mm. `dxf` writes the older R12 kind.
pub fn dxf_modern(scene: &Scene, ids: &[u64], area_h: f64, layer_name: &dyn Fn(usize) -> String) -> String {
    let clean = |n: String| -> String { n.chars().map(|c| if c.is_alphanumeric() || c == '_' || c == '-' { c } else { '_' }).collect() };
    let mut layers: Vec<usize> = scene.shapes.iter().filter(|x| ids.is_empty() || ids.contains(&x.id)).map(|x| x.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    let names: Vec<String> = layers.iter().map(|l| clean(layer_name(*l))).collect();
    // Handles below 0x30 are the fixed ones of the tables; the layers and entities count up from 0x30.
    let mut next = 0x30u32;
    let mut handle = || {
        next += 1;
        format!("{:X}", next)
    };
    let mut s = String::new();
    s.push_str("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1021\n9\n$DWGCODEPAGE\n3\nANSI_1252\n9\n$HANDSEED\n5\n20000\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    s.push_str("0\nSECTION\n2\nTABLES\n");
    s.push_str("0\nTABLE\n2\nVPORT\n5\n8\n330\n0\n100\nAcDbSymbolTable\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nLTYPE\n5\n5\n330\n0\n100\nAcDbSymbolTable\n70\n3\n");
    for (h, name, text) in [("14", "ByBlock", ""), ("15", "ByLayer", ""), ("16", "Continuous", "Solid line")] {
        let _ = write!(s, "0\nLTYPE\n5\n{h}\n330\n5\n100\nAcDbSymbolTableRecord\n100\nAcDbLinetypeTableRecord\n2\n{name}\n70\n0\n3\n{text}\n72\n65\n73\n0\n40\n0\n");
    }
    s.push_str("0\nENDTAB\n");
    let _ = write!(s, "0\nTABLE\n2\nLAYER\n5\n2\n330\n0\n100\nAcDbSymbolTable\n70\n{}\n", names.len() + 1);
    s.push_str("0\nLAYER\n5\n10\n330\n2\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n0\n70\n0\n62\n7\n6\nCONTINUOUS\n370\n-3\n390\nF\n");
    for n in &names {
        let _ = write!(s, "0\nLAYER\n5\n{}\n330\n2\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n{n}\n70\n0\n62\n7\n6\nCONTINUOUS\n370\n-3\n390\nF\n", handle());
    }
    s.push_str("0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nSTYLE\n5\n3\n330\n0\n100\nAcDbSymbolTable\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nVIEW\n5\n6\n330\n0\n100\nAcDbSymbolTable\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nUCS\n5\n7\n330\n0\n100\nAcDbSymbolTable\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nAPPID\n5\n9\n330\n0\n100\nAcDbSymbolTable\n70\n1\n0\nAPPID\n5\n12\n330\n9\n100\nAcDbSymbolTableRecord\n100\nAcDbRegAppTableRecord\n2\nACAD\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nDIMSTYLE\n5\nA\n330\n0\n100\nAcDbSymbolTable\n70\n0\n100\nAcDbDimStyleTable\n71\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n330\n0\n100\nAcDbSymbolTable\n70\n2\n");
    s.push_str("0\nBLOCK_RECORD\n5\n1F\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Model_Space\n70\n0\n280\n1\n281\n0\n");
    s.push_str("0\nBLOCK_RECORD\n5\n1B\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Paper_Space\n70\n0\n280\n1\n281\n0\n");
    s.push_str("0\nENDTAB\n0\nENDSEC\n");
    s.push_str("0\nSECTION\n2\nBLOCKS\n");
    s.push_str("0\nBLOCK\n5\n20\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*Model_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Model_Space\n1\n\n");
    s.push_str("0\nENDBLK\n5\n21\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n");
    s.push_str("0\nBLOCK\n5\n1C\n330\n1B\n100\nAcDbEntity\n67\n1\n8\n0\n100\nAcDbBlockBegin\n2\n*Paper_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Paper_Space\n1\n\n");
    s.push_str("0\nENDBLK\n5\n1D\n330\n1B\n100\nAcDbEntity\n67\n1\n8\n0\n100\nAcDbBlockEnd\n");
    s.push_str("0\nENDSEC\n0\nSECTION\n2\nENTITIES\n");
    for (l, name) in layers.iter().zip(&names) {
        for sh in scene.shapes.iter().filter(|x| x.layer == *l && (ids.is_empty() || ids.contains(&x.id))) {
            for c in contours_of(sh) {
                let p: Polyline = c.flatten(0.01);
                if p.pts.len() < 2 {
                    continue;
                }
                let _ = write!(s, "0\nLWPOLYLINE\n5\n{}\n330\n1F\n100\nAcDbEntity\n8\n{name}\n100\nAcDbPolyline\n90\n{}\n70\n{}\n", handle(), p.pts.len(), if p.closed { 1 } else { 0 });
                for q in &p.pts {
                    let _ = write!(s, "10\n{}\n20\n{}\n", num(q.x), num(area_h - q.y));
                }
            }
        }
    }
    s.push_str("0\nENDSEC\n0\nSECTION\n2\nOBJECTS\n");
    s.push_str("0\nDICTIONARY\n5\nC\n330\n0\n100\nAcDbDictionary\n281\n1\n3\nACAD_GROUP\n350\nD\n");
    s.push_str("0\nDICTIONARY\n5\nD\n330\nC\n100\nAcDbDictionary\n281\n1\n");
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

/// A drawing made of lines and words (a technical drawing), in millimetres with Y up, for [`dxf_drawing`].
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawing {
    pub lines: Vec<DrawingLine>,
    pub texts: Vec<DrawingText>,
}

/// A line of a [`Drawing`]: its points, closed or not, on a DXF layer.
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingLine {
    pub layer: &'static str,
    pub pts: Vec<Pt>,
    pub closed: bool,
}

/// Words of a [`Drawing`]: the left end of their base line, the height of the capitals, the turn (degrees, anticlockwise).
#[derive(Clone, Debug, PartialEq)]
pub struct DrawingText {
    pub layer: &'static str,
    pub at: Pt,
    pub height: f64,
    pub angle: f64,
    pub text: String,
}

impl Drawing {
    /// The DXF layers it uses, in the order they first come.
    fn layers(&self) -> Vec<&'static str> {
        let mut out: Vec<&'static str> = vec![];
        for l in self.lines.iter().map(|l| l.layer).chain(self.texts.iter().map(|t| t.layer)) {
            if !out.contains(&l) {
                out.push(l);
            }
        }
        out
    }
}

/// Words for a DXF of the old kind: the letters outside ASCII written as `\U+XXXX`, as AutoCAD reads them.
fn r12_text(t: &str) -> String {
    t.chars().map(|c| if c.is_ascii() && !c.is_ascii_control() { c.to_string() } else if c.is_control() { " ".to_string() } else { format!("\\U+{:04X}", c as u32) }).collect()
}

/// A drawing as a DXF: of the old kind (R12, `AC1009`) or of the current one (AutoCAD 2007, `AC1021`, UTF-8), in millimetres. The
/// lines are polylines and the words TEXT entities, on the layers the drawing names.
pub fn dxf_drawing(d: &Drawing, modern: bool) -> String {
    let layers = d.layers();
    let mut s = String::new();
    if !modern {
        s.push_str("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1009\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
        let _ = write!(s, "0\nSECTION\n2\nTABLES\n0\nTABLE\n2\nLTYPE\n70\n1\n0\nLTYPE\n2\nCONTINUOUS\n70\n0\n3\nSolid line\n72\n65\n73\n0\n40\n0.0\n0\nENDTAB\n0\nTABLE\n2\nLAYER\n70\n{}\n", layers.len());
        for l in &layers {
            let _ = write!(s, "0\nLAYER\n2\n{l}\n70\n0\n62\n7\n6\nCONTINUOUS\n");
        }
        s.push_str("0\nENDTAB\n0\nTABLE\n2\nSTYLE\n70\n1\n0\nSTYLE\n2\nSTANDARD\n70\n0\n40\n0.0\n41\n1.0\n50\n0.0\n71\n0\n42\n2.5\n3\ntxt\n4\n\n0\nENDTAB\n0\nENDSEC\n0\nSECTION\n2\nENTITIES\n");
        for l in &d.lines {
            if l.pts.len() < 2 {
                continue;
            }
            let _ = write!(s, "0\nPOLYLINE\n8\n{}\n66\n1\n70\n{}\n", l.layer, if l.closed { 1 } else { 0 });
            for q in &l.pts {
                let _ = write!(s, "0\nVERTEX\n8\n{}\n10\n{:.4}\n20\n{:.4}\n", l.layer, q.x, q.y);
            }
            let _ = write!(s, "0\nSEQEND\n8\n{}\n", l.layer);
        }
        for t in &d.texts {
            let _ = write!(s, "0\nTEXT\n8\n{}\n10\n{:.4}\n20\n{:.4}\n30\n0.0\n40\n{:.4}\n1\n{}\n50\n{:.4}\n7\nSTANDARD\n", t.layer, t.at.x, t.at.y, t.height, r12_text(&t.text), t.angle);
        }
        s.push_str("0\nENDSEC\n0\nEOF\n");
        return s;
    }
    let mut next = 0x30u32;
    let mut handle = || {
        next += 1;
        format!("{:X}", next)
    };
    s.push_str("0\nSECTION\n2\nHEADER\n9\n$ACADVER\n1\nAC1021\n9\n$DWGCODEPAGE\n3\nANSI_1252\n9\n$HANDSEED\n5\n20000\n9\n$INSUNITS\n70\n4\n0\nENDSEC\n");
    s.push_str("0\nSECTION\n2\nTABLES\n");
    s.push_str("0\nTABLE\n2\nVPORT\n5\n8\n330\n0\n100\nAcDbSymbolTable\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nLTYPE\n5\n5\n330\n0\n100\nAcDbSymbolTable\n70\n3\n");
    for (h, name, text) in [("14", "ByBlock", ""), ("15", "ByLayer", ""), ("16", "Continuous", "Solid line")] {
        let _ = write!(s, "0\nLTYPE\n5\n{h}\n330\n5\n100\nAcDbSymbolTableRecord\n100\nAcDbLinetypeTableRecord\n2\n{name}\n70\n0\n3\n{text}\n72\n65\n73\n0\n40\n0\n");
    }
    s.push_str("0\nENDTAB\n");
    let _ = write!(s, "0\nTABLE\n2\nLAYER\n5\n2\n330\n0\n100\nAcDbSymbolTable\n70\n{}\n", layers.len() + 1);
    s.push_str("0\nLAYER\n5\n10\n330\n2\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n0\n70\n0\n62\n7\n6\nCONTINUOUS\n370\n-3\n390\nF\n");
    for n in &layers {
        let _ = write!(s, "0\nLAYER\n5\n{}\n330\n2\n100\nAcDbSymbolTableRecord\n100\nAcDbLayerTableRecord\n2\n{n}\n70\n0\n62\n7\n6\nCONTINUOUS\n370\n-3\n390\nF\n", handle());
    }
    s.push_str("0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nSTYLE\n5\n3\n330\n0\n100\nAcDbSymbolTable\n70\n1\n0\nSTYLE\n5\n11\n330\n3\n100\nAcDbSymbolTableRecord\n100\nAcDbTextStyleTableRecord\n2\nStandard\n70\n0\n40\n0\n41\n1\n50\n0\n71\n0\n42\n2.5\n3\narial.ttf\n4\n\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nVIEW\n5\n6\n330\n0\n100\nAcDbSymbolTable\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nUCS\n5\n7\n330\n0\n100\nAcDbSymbolTable\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nAPPID\n5\n9\n330\n0\n100\nAcDbSymbolTable\n70\n1\n0\nAPPID\n5\n12\n330\n9\n100\nAcDbSymbolTableRecord\n100\nAcDbRegAppTableRecord\n2\nACAD\n70\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nDIMSTYLE\n5\nA\n330\n0\n100\nAcDbSymbolTable\n70\n0\n100\nAcDbDimStyleTable\n71\n0\n0\nENDTAB\n");
    s.push_str("0\nTABLE\n2\nBLOCK_RECORD\n5\n1\n330\n0\n100\nAcDbSymbolTable\n70\n2\n");
    s.push_str("0\nBLOCK_RECORD\n5\n1F\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Model_Space\n70\n0\n280\n1\n281\n0\n");
    s.push_str("0\nBLOCK_RECORD\n5\n1B\n330\n1\n100\nAcDbSymbolTableRecord\n100\nAcDbBlockTableRecord\n2\n*Paper_Space\n70\n0\n280\n1\n281\n0\n");
    s.push_str("0\nENDTAB\n0\nENDSEC\n");
    s.push_str("0\nSECTION\n2\nBLOCKS\n");
    s.push_str("0\nBLOCK\n5\n20\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockBegin\n2\n*Model_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Model_Space\n1\n\n");
    s.push_str("0\nENDBLK\n5\n21\n330\n1F\n100\nAcDbEntity\n8\n0\n100\nAcDbBlockEnd\n");
    s.push_str("0\nBLOCK\n5\n1C\n330\n1B\n100\nAcDbEntity\n67\n1\n8\n0\n100\nAcDbBlockBegin\n2\n*Paper_Space\n70\n0\n10\n0\n20\n0\n30\n0\n3\n*Paper_Space\n1\n\n");
    s.push_str("0\nENDBLK\n5\n1D\n330\n1B\n100\nAcDbEntity\n67\n1\n8\n0\n100\nAcDbBlockEnd\n");
    s.push_str("0\nENDSEC\n0\nSECTION\n2\nENTITIES\n");
    for l in &d.lines {
        if l.pts.len() < 2 {
            continue;
        }
        let _ = write!(s, "0\nLWPOLYLINE\n5\n{}\n330\n1F\n100\nAcDbEntity\n8\n{}\n100\nAcDbPolyline\n90\n{}\n70\n{}\n", handle(), l.layer, l.pts.len(), if l.closed { 1 } else { 0 });
        for q in &l.pts {
            let _ = write!(s, "10\n{}\n20\n{}\n", num(q.x), num(q.y));
        }
    }
    for t in &d.texts {
        let text: String = t.text.chars().map(|c| if c.is_control() { ' ' } else { c }).collect();
        let _ = write!(s, "0\nTEXT\n5\n{}\n330\n1F\n100\nAcDbEntity\n8\n{}\n100\nAcDbText\n10\n{}\n20\n{}\n30\n0\n40\n{}\n1\n{text}\n50\n{}\n7\nStandard\n100\nAcDbText\n", handle(), t.layer, num(t.at.x), num(t.at.y), num(t.height), num(t.angle));
    }
    s.push_str("0\nENDSEC\n0\nSECTION\n2\nOBJECTS\n");
    s.push_str("0\nDICTIONARY\n5\nC\n330\n0\n100\nAcDbDictionary\n281\n1\n3\nACAD_GROUP\n350\nD\n");
    s.push_str("0\nDICTIONARY\n5\nD\n330\nC\n100\nAcDbDictionary\n281\n1\n");
    s.push_str("0\nENDSEC\n0\nEOF\n");
    s
}

/// How many of the shapes `ids` (all when empty) are pictures: they have no outlines, so no file of outlines holds them.
pub fn pictures_in(scene: &Scene, ids: &[u64]) -> usize {
    scene.shapes.iter().filter(|x| x.is_image() && (ids.is_empty() || ids.contains(&x.id))).count()
}

/// HPGL (`.plt`) of the shapes `ids` (all when empty): 40 plotter units per millimetre, one pen per layer (pen = layer + 1); the curves
/// are flattened to within 0.01 mm. `area_h` is the height of the work area: the scene counts Y down, HPGL up.
pub fn hpgl(scene: &Scene, ids: &[u64], area_h: f64) -> String {
    let unit = |v: f64| (v * 40.0).round() as i64;
    let mut layers: Vec<usize> = scene.shapes.iter().filter(|x| ids.is_empty() || ids.contains(&x.id)).map(|x| x.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    let mut s = String::from("IN;\n");
    for l in layers {
        let _ = writeln!(s, "SP{};", l + 1);
        for sh in scene.shapes.iter().filter(|x| x.layer == l && (ids.is_empty() || ids.contains(&x.id))) {
            for c in contours_of(sh) {
                let p: Polyline = c.flatten(0.01);
                if p.pts.len() < 2 {
                    continue;
                }
                let at = |q: &Pt| format!("{},{}", unit(q.x), unit(area_h - q.y));
                let _ = writeln!(s, "PU{};", at(&p.pts[0]));
                let mut down: Vec<String> = p.pts[1..].iter().map(at).collect();
                if p.closed {
                    down.push(at(&p.pts[0]));
                }
                let _ = writeln!(s, "PD{};", down.join(","));
            }
        }
    }
    s.push_str("PU;\nSP0;\n");
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Xf;

    #[test]
    fn hpgl_has_one_pen_per_layer_counts_y_up_and_closes_what_is_closed() {
        let (sc, ids) = scene();
        let all = hpgl(&sc, &[], 50.0);
        assert!(all.starts_with("IN;\nSP1;\n") && all.contains("SP2;") && all.ends_with("PU;\nSP0;\n"), "{all}");
        // The rectangle from (10, 10) to (50, 30) on an area 50 high: it starts at y = 40 mm, 1600 units, and comes back there.
        assert!(all.contains("PU400,1600;\nPD2000,1600,2000,800,400,800,400,1600;\n"), "{all}");
        let one = hpgl(&sc, &ids[..1], 50.0);
        assert!(one.contains("SP1;") && !one.contains("SP2;"), "{one}");
    }

    fn scene() -> (Scene, Vec<u64>) {
        let mut sc = Scene::default();
        let a = sc.add(0, Kind::Rect { w: 40.0, h: 20.0 }, Xf::translate(10.0, 10.0));
        let b = sc.add(1, Kind::Ellipse { w: 10.0, h: 10.0 }, Xf::translate(60.0, 15.0));
        (sc, vec![a, b])
    }

    #[test]
    fn a_selection_is_written_as_svg_with_curves_and_one_group_per_layer() {
        let (sc, ids) = scene();
        let all = svg(&sc, &[], 100.0, 50.0, &|l| format!("L{l}"), &|_| [255, 0, 0]);
        assert!(all.contains("viewBox=\"0 0 100 50\"") && all.contains("id=\"L0\"") && all.contains("id=\"L1\""), "{all}");
        assert!(all.contains("M10 10L50 10L50 30L10 30Z"), "{all}");
        assert!(all.contains('C'), "the ellipse keeps its curves");
        let one = svg(&sc, &ids[..1], 100.0, 50.0, &|l| format!("L{l}"), &|_| [0, 0, 0]);
        assert!(one.contains("id=\"L0\"") && !one.contains("id=\"L1\""));
    }

    #[test]
    fn a_selection_written_as_dxf_is_read_back_as_the_same_shapes() {
        let (sc, ids) = scene();
        let text = dxf(&sc, &ids, 50.0, &|l| format!("Layer {l}"));
        assert!(text.contains("Layer_0") && text.contains("AC1009"));
        let back = crate::dxf::parse(text.as_bytes(), 50.0).unwrap();
        assert_eq!(back.outlines.len(), 2);
        let b = back.outlines.iter().filter_map(|o| o.contours[0].flatten(0.01).bounds()).fold(None, |a: Option<crate::geom::Rect>, b| Some(a.map_or(b, |a| a.union(b)))).unwrap();
        assert!((b.min.x - 10.0).abs() < 0.01 && (b.max.x - 70.0).abs() < 0.01, "{b:?}");
        assert!((b.min.y - 10.0).abs() < 0.01 && (b.max.y - 30.0).abs() < 0.01, "{b:?}");
        // Only the selected one.
        assert_eq!(crate::dxf::parse(dxf(&sc, &ids[1..], 50.0, &|l| format!("L{l}")).as_bytes(), 50.0).unwrap().outlines.len(), 1);
    }
    #[test]
    fn a_drawing_goes_into_both_kinds_of_dxf_with_its_lines_and_words() {
        let d = Drawing {
            lines: vec![DrawingLine { layer: "OUTLINE", pts: vec![Pt::new(0.0, 0.0), Pt::new(100.0, 0.0), Pt::new(100.0, 50.0)], closed: true }],
            texts: vec![DrawingText { layer: "TEXT", at: Pt::new(10.0, 60.0), height: 3.5, angle: 90.0, text: "Widok z przodu Ø 12".into() }],
        };
        let old = dxf_drawing(&d, false);
        assert!(old.contains("AC1009") && old.contains("\nPOLYLINE\n8\nOUTLINE\n") && old.contains("\\U+00D8") && old.contains("\nTEXT\n8\nTEXT\n"));
        let new = dxf_drawing(&d, true);
        assert!(new.contains("AC1021") && new.contains("LWPOLYLINE") && new.contains("Widok z przodu Ø 12") && new.contains("\n50\n90\n"));
        // Read back: the outline is the polyline of 100 × 50 mm.
        for text in [old, new] {
            let read = crate::dxf::parse(text.as_bytes(), 100.0).unwrap();
            let o = read.outlines.iter().find(|o| o.layer.as_deref() == Some("OUTLINE")).unwrap();
            let b = o.contours[0].flatten(0.01).bounds().unwrap();
            assert!((b.max.x - b.min.x - 100.0).abs() < 1e-3 && (b.max.y - b.min.y - 50.0).abs() < 1e-3, "{b:?}");
        }
    }

}
