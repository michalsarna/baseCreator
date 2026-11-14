//! SVG reading (via usvg): the outlines of a picture as editable Bezier contours with their colours.
use crate::bezier::{Contour, Node};
use crate::geom::*;
use usvg::tiny_skia_path::PathSegment;

const PX_TO_MM: f64 = 25.4 / 96.0;

/// One path of a picture: its contours in millimetres and its stroke (or fill) colour, black when it has none.
pub struct Outline {
    pub contours: Vec<Contour>,
    pub color: [u8; 3],
    /// The Illustrator layer (a PDF optional content group) it is on: the outermost one, `None` for SVG and for content outside of layers.
    pub layer: Option<String>,
}

/// The names of the layers of some outlines, each once, in the order they first appear.
pub fn layer_names(outlines: &[Outline]) -> Vec<String> {
    let mut names: Vec<String> = vec![];
    for o in outlines {
        if let Some(l) = &o.layer {
            if !names.contains(l) {
                names.push(l.clone());
            }
        }
    }
    names
}

/// A scene of read outlines, one shape each: on the layer nearest to their colour, or with `by_layers` on the layer with the number of
/// their Illustrator layer in the file (the first on layer 1, ...; the rest on the last one; outlines on no layer still go by colour).
pub fn into_scene(outlines: Vec<Outline>, by_layers: bool) -> crate::scene::Scene {
    let names = layer_names(&outlines);
    let mut scene = crate::scene::Scene::default();
    for o in outlines {
        let named = if by_layers { o.layer.as_ref().and_then(|n| names.iter().position(|x| x == n)) } else { None };
        let layer = named.map(|k| k.min(crate::layers::LAYER_COUNT - 1)).unwrap_or_else(|| crate::layers::nearest(o.color));
        scene.add(layer, crate::scene::Kind::Bezier(o.contours), Xf::IDENTITY);
    }
    scene
}

/// Read a drawing into outlines: a PDF, Illustrator, EPS or VectorCraft file (known by its content), or else an SVG.
pub fn read_drawing(data: &[u8]) -> Result<Vec<Outline>, String> {
    if crate::pdf::is_pdf_or_ai(data) {
        crate::pdf::parse(data)
    } else {
        parse(data)
    }
}

/// Read a drawing (SVG, PDF or Illustrator, by its content) into a new scene, as *File → Open…* does: every outline one shape on layer
/// 0, and the pictures of an SVG on layer 0 too.
pub fn read_scene(data: &[u8]) -> Result<crate::scene::Scene, String> {
    let outlines = read_drawing(data)?;
    let pictures = pictures(data);
    let mut scene = crate::scene::Scene::default();
    for o in outlines {
        scene.add(0, crate::scene::Kind::Bezier(o.contours), Xf::IDENTITY);
    }
    for (im, xf) in pictures {
        scene.add(0, crate::scene::Kind::Image(im), xf);
    }
    Ok(scene)
}

/// The pictures of a drawing read by [`read_drawing`]: those of an SVG, of a VectorCraft drawing or of the VectorCraft document in an
/// EPS (a PDF and an Illustrator file give none).
pub fn pictures(data: &[u8]) -> Vec<(crate::image::ImageData, Xf)> {
    if crate::vectorcraft::is_vectorcraft(data) {
        crate::vectorcraft::images(data)
    } else if crate::eps::is_eps(data) {
        crate::eps::images(data)
    } else if crate::pdf::is_pdf_or_ai(data) {
        vec![]
    } else {
        images(data)
    }
}

/// Read an SVG into outlines, in document order.
pub fn parse(data: &[u8]) -> Result<Vec<Outline>, String> {
    let tree = usvg::Tree::from_data(data, &usvg::Options::default()).map_err(|e| e.to_string())?;
    let mut out = vec![];
    walk(tree.root(), &mut out);
    Ok(out)
}

/// The pictures of an SVG (`<image>` with a PNG, JPEG, GIF or WebP in it, as LightBurn writes them), each with where it lies: the
/// transform from the picture (0..w, 0..h millimetres) to the drawing, in the millimetres of `parse`.
pub fn images(data: &[u8]) -> Vec<(crate::image::ImageData, Xf)> {
    let Ok(tree) = usvg::Tree::from_data(data, &usvg::Options::default()) else { return vec![] };
    let mut out = vec![];
    walk_images(tree.root(), &mut out);
    out
}

fn walk_images(g: &usvg::Group, out: &mut Vec<(crate::image::ImageData, Xf)>) {
    for node in g.children() {
        match node {
            usvg::Node::Group(g) => walk_images(g, out),
            usvg::Node::Image(im) if im.is_visible() => {
                let bytes = match im.kind() {
                    usvg::ImageKind::PNG(b) | usvg::ImageKind::JPEG(b) | usvg::ImageKind::GIF(b) | usvg::ImageKind::WEBP(b) => b.clone(),
                    usvg::ImageKind::SVG(_) => continue,
                };
                let size = im.size();
                let (w, h) = (size.width() as f64 * PX_TO_MM, size.height() as f64 * PX_TO_MM);
                if w <= 0.0 || h <= 0.0 {
                    continue;
                }
                let name = if im.id().is_empty() { "picture" } else { im.id() };
                let Ok(mut pic) = crate::image::ImageData::from_bytes(&bytes, name, w.max(h)) else { continue };
                pic.w = w;
                pic.h = h;
                let t = im.abs_transform();
                // The picture's millimetres → its SVG units → the drawing → millimetres.
                let xf = Xf::scale(1.0 / PX_TO_MM, 1.0 / PX_TO_MM)
                    .then(Xf { a: t.sx as f64, b: t.ky as f64, c: t.kx as f64, d: t.sy as f64, e: t.tx as f64, f: t.ty as f64 })
                    .then(Xf::scale(PX_TO_MM, PX_TO_MM));
                out.push((pic, xf));
            }
            _ => {}
        }
    }
}

fn walk(g: &usvg::Group, out: &mut Vec<Outline>) {
    for node in g.children() {
        match node {
            usvg::Node::Group(g) => walk(g, out),
            usvg::Node::Text(t) => walk(t.flattened(), out),
            usvg::Node::Path(p) => {
                let t = p.abs_transform();
                let xf = Xf { a: t.sx as f64, b: t.ky as f64, c: t.kx as f64, d: t.sy as f64, e: t.tx as f64, f: t.ty as f64 }
                    .then(Xf::scale(PX_TO_MM, PX_TO_MM));
                let polys = contours(p.data(), &xf);
                if polys.is_empty() {
                    continue;
                }
                let color = [p.stroke().map(|s| s.paint()), p.fill().map(|f| f.paint())]
                    .into_iter()
                    .flatten()
                    .find_map(|paint| match paint {
                        usvg::Paint::Color(c) => Some([c.red, c.green, c.blue]),
                        _ => None,
                    })
                    .unwrap_or([0, 0, 0]);
                out.push(Outline { contours: polys, color, layer: None });
            }
            _ => {}
        }
    }
}

/// Convert an SVG path into editable Bézier contours (curves are kept, not flattened).
fn contours(path: &usvg::tiny_skia_path::Path, xf: &Xf) -> Vec<Contour> {
    let tp = |p: usvg::tiny_skia_path::Point| xf.apply(Pt::new(p.x as f64, p.y as f64));
    let mut out: Vec<Contour> = vec![];
    let mut cur: Vec<Node> = vec![];
    let finish = |cur: &mut Vec<Node>, closed: bool, out: &mut Vec<Contour>| {
        let mut nodes = std::mem::take(cur);
        if closed && nodes.len() > 2 && nodes[0].p.dist(nodes[nodes.len() - 1].p) < 1e-6 {
            let last = nodes.pop().unwrap_or(nodes[0]);
            nodes[0].hin = last.hin;
        }
        if nodes.len() >= 2 {
            for n in &mut nodes {
                n.smooth = n.has_in() && n.has_out() && {
                    let (a, b) = (Pt::new(n.p.x - n.hin.x, n.p.y - n.hin.y), Pt::new(n.hout.x - n.p.x, n.hout.y - n.p.y));
                    let cross = a.x * b.y - a.y * b.x;
                    let dot = a.x * b.x + a.y * b.y;
                    dot > 0.0 && cross.abs() <= 0.02 * (dot.abs() + 1e-12)
                };
            }
            out.push(Contour { nodes, closed });
        }
    };
    let mut last = Pt::default();
    for seg in path.segments() {
        match seg {
            PathSegment::MoveTo(p) => {
                finish(&mut cur, false, &mut out);
                last = tp(p);
                cur.push(Node::corner(last));
            }
            PathSegment::LineTo(p) => {
                last = tp(p);
                cur.push(Node::corner(last));
            }
            PathSegment::QuadTo(c, p) => {
                let (c, p) = (tp(c), tp(p));
                let c1 = Pt::new(last.x + 2.0 / 3.0 * (c.x - last.x), last.y + 2.0 / 3.0 * (c.y - last.y));
                let c2 = Pt::new(p.x + 2.0 / 3.0 * (c.x - p.x), p.y + 2.0 / 3.0 * (c.y - p.y));
                if let Some(n) = cur.last_mut() {
                    n.hout = c1;
                }
                cur.push(Node { p, hin: c2, hout: p, smooth: false });
                last = p;
            }
            PathSegment::CubicTo(c1, c2, p) => {
                let (c1, c2, p) = (tp(c1), tp(c2), tp(p));
                if let Some(n) = cur.last_mut() {
                    n.hout = c1;
                }
                cur.push(Node { p, hin: c2, hout: p, smooth: false });
                last = p;
            }
            PathSegment::Close => {
                let start = cur.first().map(|n| n.p);
                finish(&mut cur, true, &mut out);
                if let Some(s) = start {
                    last = s;
                    cur.push(Node::corner(s));
                }
            }
        }
    }
    finish(&mut cur, false, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn outline(color: [u8; 3], layer: Option<&str>) -> Outline {
        Outline { contours: vec![Contour::rect(10.0, 10.0)], color, layer: layer.map(String::from) }
    }

    #[test]
    fn outlines_go_on_layers_by_colour_or_by_the_layers_of_the_file() {
        let red = crate::layers::color(1);
        let green = crate::layers::color(2);
        let by_colour = into_scene(vec![outline(red, Some("A")), outline(green, Some("B"))], false);
        assert_eq!(by_colour.shapes.iter().map(|s| s.layer).collect::<Vec<_>>(), [1, 2]);
        let by_layers = into_scene(vec![outline(green, Some("B")), outline(red, Some("A")), outline(red, None)], true);
        assert_eq!(by_layers.shapes.iter().map(|s| s.layer).collect::<Vec<_>>(), [0, 1, 1], "first layer of the file on the first layer; no layer: by colour");
    }

    #[test]
    fn a_drawing_is_known_by_its_content() {
        let svg = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10mm" height="10mm"><rect width="5" height="5" fill="red"/></svg>"#;
        assert_eq!(read_drawing(svg).unwrap().len(), 1);
        assert!(read_drawing(b"%PDF-1.4 not really").is_err());
    }
}
