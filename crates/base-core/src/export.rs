//! Writing shapes out as SVG and DXF: the shapes of a scene (all, or a selection), in millimetres, the way the work area shows them
//! (Y down for SVG, Y up for DXF).
use crate::bezier::Contour;
use crate::geom::{Polyline, Pt};
use crate::scene::{Kind, Scene, Shape};
use std::fmt::Write;

/// The contours of a shape on the work area. A rectangle, an ellipse and live text have theirs made; a picture has none.
fn contours_of(s: &Shape) -> Vec<Contour> {
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
    let _ = write!(s, "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{w}mm\" height=\"{h}mm\" viewBox=\"0 0 {w} {h}\">\n");
    let mut layers: Vec<usize> = scene.shapes.iter().filter(|x| ids.is_empty() || ids.contains(&x.id)).map(|x| x.layer).collect();
    layers.sort_unstable();
    layers.dedup();
    for l in layers {
        let name = layer_name(l).replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;");
        let c = color(l);
        let _ = writeln!(s, "<g id=\"{name}\" fill=\"none\" stroke=\"#{:02x}{:02x}{:02x}\" stroke-width=\"0.1\">", c[0], c[1], c[2]);
        for sh in scene.shapes.iter().filter(|x| x.layer == l && (ids.is_empty() || ids.contains(&x.id))) {
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

/// A DXF (R12, ASCII, millimetres) of the shapes `ids` (all when empty): one DXF layer for each layer of the project, named by
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::Xf;

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
}
