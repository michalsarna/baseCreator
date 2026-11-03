//! Printing the design on a paper printer: a PDF page with the work area (or just the objects) fitted onto it.
//! The PDF is written here, by hand (vector paths, grey images, one font); handing it to the printer is the job
//! of the program.
use crate::geom::*;
use crate::image::ImageData;
use crate::scene::{Kind, Shape};
use std::fmt::Write as _;

/// How one layer looks on the paper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrintLayer {
    pub rgb: [u8; 3],
    /// The closed shapes of the layer get a light tint of its colour.
    pub filled: bool,
    pub visible: bool,
}

/// What a program gives to be printed. A layer is `Shape::layer`, an index into `layers`.
#[derive(Clone, Debug)]
pub struct PrintSource<'a> {
    pub shapes: &'a [Shape],
    pub layers: Vec<PrintLayer>,
    /// The layers from the first drawn to the last.
    pub order: Vec<usize>,
    /// The size of the work area, or of the page of a program without a machine, in millimetres.
    pub work: (f64, f64),
    /// The name of the program, in the line under the drawing.
    pub program: &'a str,
    /// The name of the device or of the page, in the same line.
    pub device: &'a str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    /// The whole work area, to scale on the page.
    WorkArea,
    /// Only the objects, as large as the page allows.
    Objects,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paper {
    A4,
    Letter,
}

impl Paper {
    /// Portrait size in points (1/72 inch).
    pub fn size_pt(self) -> (f64, f64) {
        match self {
            Paper::A4 => (595.276, 841.89),
            Paper::Letter => (612.0, 792.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PrintOptions {
    pub scope: Scope,
    /// Only objects on visible layers; otherwise all of them.
    pub visible_only: bool,
    pub paper: Paper,
    /// Blank border on every side, millimetres.
    pub margin_mm: f64,
}

impl Default for PrintOptions {
    fn default() -> Self {
        PrintOptions { scope: Scope::WorkArea, visible_only: true, paper: Paper::A4, margin_mm: 10.0 }
    }
}

const PT_PER_MM: f64 = 72.0 / 25.4;
/// Room kept at the bottom for the line with the scale.
const FOOTER_PT: f64 = 16.0;

/// Indexes of the shapes that are printed.
pub fn included(src: &PrintSource, o: &PrintOptions) -> Vec<usize> {
    (0..src.shapes.len()).filter(|&i| !o.visible_only || src.layers.get(src.shapes[i].layer).is_some_and(|l| l.visible)).collect()
}

/// How many printed objects reach outside the work area (to warn about: on the paper they would be cut off or
/// shrink the page).
pub fn outside_area(src: &PrintSource, o: &PrintOptions) -> usize {
    let (w, h) = src.work;
    let eps = 1e-6;
    included(src, o).into_iter().filter_map(|i| src.shapes[i].bounds()).filter(|b| b.min.x < -eps || b.min.y < -eps || b.max.x > w + eps || b.max.y > h + eps).count()
}

/// The part of the design that goes on the page.
pub fn region(src: &PrintSource, o: &PrintOptions) -> Result<Rect, String> {
    let r = match o.scope {
        Scope::WorkArea => {
            let (w, h) = src.work;
            Rect { min: Pt::new(0.0, 0.0), max: Pt::new(w, h) }
        }
        Scope::Objects => included(src, o).into_iter().filter_map(|i| src.shapes[i].bounds()).reduce(Rect::union).ok_or("nothing to print")?,
    };
    let ok = |v: f64| v.is_finite() && v.abs() < 1e7;
    if !(ok(r.min.x) && ok(r.min.y) && ok(r.max.x) && ok(r.max.y)) {
        return Err("the size of the design is not usable".into());
    }
    // A single line or point still needs a page: give it a minimal size.
    let (mut r, w, h) = (r, r.max.x - r.min.x, r.max.y - r.min.y);
    if w < 0.1 {
        r.min.x -= 0.5;
        r.max.x += 0.5;
    }
    if h < 0.1 {
        r.min.y -= 0.5;
        r.max.y += 0.5;
    }
    Ok(r)
}

/// Where the region lies on the page, in points; the page turns to landscape when the region is wider than tall.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Plan {
    pub page_w: f64,
    pub page_h: f64,
    /// Points per millimetre of the design.
    pub scale: f64,
    /// Page position of the region's top-left corner (from the left and from the top).
    pub left: f64,
    pub top: f64,
    pub region: Rect,
}

pub fn layout(region: Rect, paper: Paper, margin_mm: f64) -> Option<Plan> {
    let (rw, rh) = (region.max.x - region.min.x, region.max.y - region.min.y);
    if !(rw > 0.0 && rh > 0.0) {
        return None;
    }
    let (pw, ph) = paper.size_pt();
    let (page_w, page_h) = if rw > rh { (ph, pw) } else { (pw, ph) };
    let m = margin_mm.clamp(0.0, 40.0) * PT_PER_MM;
    let (uw, uh) = (page_w - 2.0 * m, page_h - 2.0 * m - FOOTER_PT);
    if uw <= 10.0 || uh <= 10.0 {
        return None;
    }
    let scale = (uw / rw).min(uh / rh);
    Some(Plan { page_w, page_h, scale, left: m + (uw - rw * scale) / 2.0, top: m + (uh - rh * scale) / 2.0, region })
}

impl Plan {
    fn x(&self, v: f64) -> f64 {
        self.left + (v - self.region.min.x) * self.scale
    }
    /// PDF's y runs up from the bottom of the page.
    fn y(&self, v: f64) -> f64 {
        self.page_h - (self.top + (v - self.region.min.y) * self.scale)
    }
    fn pt(&self, p: Pt) -> (f64, f64) {
        (self.x(p.x), self.y(p.y))
    }
    /// Percent of the real size.
    pub fn percent(&self) -> f64 {
        self.scale / PT_PER_MM * 100.0
    }
}

fn colour(src: &PrintSource, layer: usize) -> [f64; 3] {
    let c = src.layers.get(layer).map_or([0, 0, 0], |l| l.rgb);
    [c[0] as f64 / 255.0, c[1] as f64 / 255.0, c[2] as f64 / 255.0]
}

fn pdf_text(s: &str) -> String {
    s.chars().map(|c| if c.is_ascii() && !c.is_control() { c } else { '?' }).collect::<String>().replace('\\', "\\\\").replace('(', "\\(").replace(')', "\\)")
}

/// One grey image, at most 1024 pixels on its longer side.
fn small_gray(img: &ImageData) -> Option<(usize, usize, Vec<u8>)> {
    let (w, h) = (img.px_w as usize, img.px_h as usize);
    if w == 0 || h == 0 || img.gray.len() < w * h {
        return None;
    }
    let k = w.max(h).div_ceil(1024).max(1);
    let (nw, nh) = (w.div_ceil(k), h.div_ceil(k));
    let mut out = Vec::with_capacity(nw * nh);
    for y in 0..nh {
        for x in 0..nw {
            out.push(img.gray[(y * k) * w + x * k]);
        }
    }
    Some((nw, nh, out))
}

/// The PDF of the design for the printer: one page.
pub fn pdf(src: &PrintSource, o: &PrintOptions) -> Result<Vec<u8>, String> {
    let reg = region(src, o)?;
    let plan = layout(reg, o.paper, o.margin_mm).ok_or("the page is too small for these margins")?;
    let mut content = String::new();
    let mut images: Vec<(usize, usize, Vec<u8>)> = vec![];
    // Everything is clipped to the printed region.
    let _ = writeln!(content, "q\n{:.2} {:.2} {:.2} {:.2} re W n", plan.x(reg.min.x), plan.y(reg.max.y), (reg.max.x - reg.min.x) * plan.scale, (reg.max.y - reg.min.y) * plan.scale);
    let idx = included(src, o);
    for &li in &src.order {
        let c = colour(src, li);
        let fills = src.layers.get(li).is_some_and(|l| l.filled);
        for &i in idx.iter().filter(|&&i| src.shapes[i].layer == li) {
            let s = &src.shapes[i];
            if let Kind::Image(img) = &s.kind {
                let Some(g) = small_gray(img) else { continue };
                let (p00, p10, p01) = (plan.pt(s.xf.apply(Pt::new(0.0, img.h))), plan.pt(s.xf.apply(Pt::new(img.w, img.h))), plan.pt(s.xf.apply(Pt::new(0.0, 0.0))));
                let _ = writeln!(content, "q {:.4} {:.4} {:.4} {:.4} {:.2} {:.2} cm /Im{} Do Q", p10.0 - p00.0, p10.1 - p00.1, p01.0 - p00.0, p01.1 - p00.1, p00.0, p00.1, images.len());
                images.push(g);
                continue;
            }
            let polys = s.polys();
            let mut path = String::new();
            for p in polys.iter().filter(|p| p.pts.len() >= 2) {
                for (k, q) in p.pts.iter().enumerate() {
                    let (x, y) = plan.pt(*q);
                    let _ = writeln!(path, "{x:.2} {y:.2} {}", if k == 0 { "m" } else { "l" });
                }
                if p.closed {
                    path.push_str("h\n");
                }
            }
            if path.is_empty() {
                continue;
            }
            if fills && polys.iter().any(|p| p.closed) {
                // A light tint of the layer colour; holes stay open (even-odd).
                let t = |v: f64| 1.0 - (1.0 - v) * 0.3;
                let _ = writeln!(content, "{:.3} {:.3} {:.3} rg\n{path}f*", t(c[0]), t(c[1]), t(c[2]));
            }
            let _ = writeln!(content, "{:.3} {:.3} {:.3} RG 0.7 w 1 J 1 j\n{path}S", c[0], c[1], c[2]);
        }
    }
    content.push_str("Q\n");
    // The border of the printed region (the work area).
    if o.scope == Scope::WorkArea {
        let _ = writeln!(content, "0.5 G 0.5 w [3 2] 0 d {:.2} {:.2} {:.2} {:.2} re S [] 0 d", plan.x(reg.min.x), plan.y(reg.max.y), (reg.max.x - reg.min.x) * plan.scale, (reg.max.y - reg.min.y) * plan.scale);
    }
    let what = if o.scope == Scope::WorkArea { "work area" } else { "objects" };
    let foot = format!("{} - {} - {} - {:.0}% of real size", src.program, src.device, what, plan.percent());
    let _ = writeln!(content, "BT /F1 8 Tf 0.3 g {:.2} {:.2} Td ({}) Tj ET", o.margin_mm.clamp(0.0, 40.0) * PT_PER_MM, o.margin_mm.clamp(0.0, 40.0) * PT_PER_MM * 0.6 + 4.0, pdf_text(&foot));

    // Objects: 1 catalog, 2 pages, 3 page, 4 content, 5 font, 6.. images.
    let mut out: Vec<u8> = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![];
    let mut obj = |out: &mut Vec<u8>, body: Vec<u8>| {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", offsets.len()).as_bytes());
        out.extend(body);
        out.extend_from_slice(b"\nendobj\n");
    };
    obj(&mut out, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    obj(&mut out, b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec());
    let xobjs: String = (0..images.len()).map(|k| format!("/Im{k} {} 0 R ", 6 + k)).collect();
    obj(&mut out, format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Contents 4 0 R /Resources << /Font << /F1 5 0 R >> /XObject << {xobjs}>> >> >>", plan.page_w, plan.page_h).into_bytes());
    let mut body = format!("<< /Length {} >>\nstream\n", content.len()).into_bytes();
    body.extend_from_slice(content.as_bytes());
    body.extend_from_slice(b"endstream");
    obj(&mut out, body);
    obj(&mut out, b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());
    for (w, h, g) in images {
        let mut b = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceGray /BitsPerComponent 8 /Length {} >>\nstream\n", g.len()).into_bytes();
        b.extend(g);
        b.extend_from_slice(b"\nendstream");
        obj(&mut out, b);
    }
    let xref = out.len();
    let mut x = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for off in &offsets {
        let _ = writeln!(x, "{off:010} 00000 n ");
    }
    let _ = write!(x, "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", offsets.len() + 1);
    out.extend_from_slice(x.as_bytes());
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source<'a>(shapes: &'a [Shape], device: &'a str) -> PrintSource<'a> {
        let layers = (0..4).map(|i| PrintLayer { rgb: crate::layers::color(i), filled: false, visible: true }).collect();
        PrintSource { shapes, layers, order: (0..4).collect(), work: (200.0, 100.0), program: "TestCreator", device }
    }

    fn rect(id: u64, layer: usize, x: f64, y: f64, w: f64, h: f64) -> Shape {
        Shape { id, layer, kind: Kind::Rect { w, h }, xf: Xf::translate(x, y), group: None, locked: false }
    }

    #[test]
    fn the_region_is_the_work_area_or_the_objects_and_the_page_turns_to_fit() {
        let shapes = vec![rect(1, 0, 10.0, 20.0, 30.0, 40.0), rect(2, 0, 100.0, 50.0, 20.0, 10.0)];
        let d = source(&shapes, "x");
        let wa = region(&d, &PrintOptions::default()).unwrap();
        assert_eq!((wa.min.x, wa.min.y, wa.max.x, wa.max.y), (0.0, 0.0, 200.0, 100.0));
        let ob = region(&d, &PrintOptions { scope: Scope::Objects, ..Default::default() }).unwrap();
        assert_eq!((ob.min.x, ob.min.y, ob.max.x, ob.max.y), (10.0, 20.0, 120.0, 60.0));
        // Wider than tall: landscape; taller than wide: portrait.
        let p = layout(wa, Paper::A4, 10.0).unwrap();
        assert!(p.page_w > p.page_h);
        let tall = layout(Rect { min: Pt::new(0.0, 0.0), max: Pt::new(100.0, 300.0) }, Paper::A4, 10.0).unwrap();
        assert!(tall.page_w < tall.page_h);
        // The whole region fits inside the margins and is centred.
        for pl in [p, tall] {
            let (rw, rh) = ((pl.region.max.x - pl.region.min.x) * pl.scale, (pl.region.max.y - pl.region.min.y) * pl.scale);
            let m = 10.0 * PT_PER_MM;
            assert!(pl.left >= m - 1e-9 && pl.left + rw <= pl.page_w - m + 1e-9);
            assert!(pl.top >= m - 1e-9 && pl.top + rh <= pl.page_h - m - FOOTER_PT + 1e-9);
        }
        // An empty design has nothing to print in the objects scope, but the work area is still a page.
        let empty = source(&[], "x");
        assert!(region(&empty, &PrintOptions { scope: Scope::Objects, ..Default::default() }).is_err());
        assert!(pdf(&empty, &PrintOptions::default()).is_ok());
        // Absurd margins leave no room.
        assert!(layout(wa, Paper::A4, 400.0).is_some(), "margins are limited to 40 mm");
        assert!(layout(Rect { min: Pt::new(0.0, 0.0), max: Pt::new(0.0, 5.0) }, Paper::A4, 10.0).is_none());
    }

    #[test]
    fn objects_outside_the_work_area_are_counted_and_hidden_layers_follow_the_option() {
        let shapes = vec![rect(1, 0, 10.0, 10.0, 20.0, 20.0), rect(2, 0, 190.0, 10.0, 20.0, 20.0), rect(3, 1, -5.0, 0.0, 10.0, 10.0)];
        let mut d = source(&shapes, "x");
        d.layers[1].visible = false;
        let vis = PrintOptions::default();
        assert_eq!(included(&d, &vis), vec![0, 1]);
        assert_eq!(outside_area(&d, &vis), 1, "the one reaching x = 210; the hidden one is not printed");
        let all = PrintOptions { visible_only: false, ..Default::default() };
        assert_eq!(included(&d, &all), vec![0, 1, 2]);
        assert_eq!(outside_area(&d, &all), 2);
    }

    #[test]
    fn the_pdf_is_well_formed() {
        let mut shapes = vec![rect(1, 0, 10.0, 10.0, 50.0, 30.0)];
        let img = Shape { id: 2, layer: 0, kind: Kind::Image(ImageData { name: "x".into(), px_w: 4, px_h: 2, gray: vec![0, 32, 64, 127, 127, 64, 32, 0], w: 40.0, h: 20.0, invert: false }), xf: Xf::translate(100.0, 40.0), group: None, locked: false };
        shapes.push(img);
        let mut d = source(&shapes, "Laser (test)");
        d.layers[0].filled = true;
        let bytes = pdf(&d, &PrintOptions::default()).unwrap();
        let text = String::from_utf8_lossy(&bytes).to_string();
        assert!(text.starts_with("%PDF-1.4") && text.trim_end().ends_with("%%EOF"));
        assert!(text.contains("/Subtype /Image") && text.contains("/Im0 Do") && text.contains("f*"));
        assert!(text.contains("Laser \\(test\\)"), "the name in the footer is escaped");
        // Every xref entry points at its object.
        let xref = text.rfind("\nxref\n").unwrap() + 1;
        let entries: Vec<&str> = text[xref..].lines().skip(3).take_while(|l| l.ends_with(" n ")).collect();
        assert_eq!(entries.len(), 6);
        for (k, e) in entries.iter().enumerate() {
            let off: usize = e[..10].parse().unwrap();
            assert!(text[off..].starts_with(&format!("{} 0 obj", k + 1)), "object {}", k + 1);
        }
        let start: usize = text[text.rfind("startxref\n").unwrap() + 10..].lines().next().unwrap().parse().unwrap();
        assert_eq!(start, xref);
        // The stream lengths are right.
        let c = text.find("/Length ").unwrap();
        let len: usize = text[c + 8..].split(' ').next().unwrap().parse().unwrap();
        let s = text[c..].find("stream\n").unwrap() + c + 7;
        assert_eq!(&text[s + len..s + len + 9], "endstream");
        // Non-ASCII letters in the name do not break the file.
        d.device = "Łódź";
        assert!(String::from_utf8_lossy(&pdf(&d, &PrintOptions::default()).unwrap()).contains("(TestCreator - ??d? - work area"));
    }
}
