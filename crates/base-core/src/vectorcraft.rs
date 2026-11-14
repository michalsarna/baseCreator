//! VectorCraft drawings (`.vectorcraft`, written by VectorCraft 0.8 and later): a JSON document (`"format": "vectorcraft"`), and
//! after it, when the file holds pictures or a large PDF copy, a block of raw data that starts with `\0VCBLOBS\0` (the JSON names
//! parts of it by `"blob": [offset, length]`). The same document is kept, zlib-packed, in an EPS that VectorCraft writes (see `eps`).
//!
//! What is read: the paths of every layer and group (their anchors with the two handles, in points with Y down from the top left
//! corner of the artboard), each with its stroke colour (or its fill when it has no stroke), and the pictures with where they lie.
//! The layers of the file are kept as the `layer` of each outline when there are several of them. The file is untrusted input:
//! sizes are limited and nothing panics.
use crate::bezier::{Contour, Node};
use crate::geom::{Pt, Xf};
use crate::svg::Outline;
use serde_json::Value;

const PT_TO_MM: f64 = 25.4 / 72.0;
/// The biggest file read.
const MAX_BYTES: usize = 256 * 1024 * 1024;
/// The most anchors taken from one file.
const MAX_POINTS: usize = 2_000_000;
/// How deep groups may be nested.
const MAX_DEPTH: usize = 64;
/// The mark before the raw data.
const BLOBS: &[u8] = b"\0VCBLOBS\0";

/// Whether the bytes are a VectorCraft drawing.
pub fn is_vectorcraft(data: &[u8]) -> bool {
    let head = &data[..data.len().min(512)];
    let text = String::from_utf8_lossy(head);
    let t = text.trim_start();
    t.starts_with('{') && t.contains("\"format\"") && t.contains("\"vectorcraft\"")
}

/// The JSON document of the file and its raw data (empty when it has none).
fn split(data: &[u8]) -> Result<(Value, &[u8]), String> {
    if data.len() > MAX_BYTES {
        return Err("the VectorCraft file is too big".into());
    }
    let (json, blobs) = match data.windows(BLOBS.len()).position(|w| w == BLOBS) {
        Some(i) => (&data[..i], &data[i + BLOBS.len()..]),
        None => (data, &data[data.len()..]),
    };
    let v: Value = serde_json::from_slice(json).map_err(|e| format!("this is not a VectorCraft file ({e})"))?;
    if v.get("format").and_then(Value::as_str) != Some("vectorcraft") {
        return Err("this is not a VectorCraft file".into());
    }
    if v.get("document").is_none() {
        return Err("the VectorCraft file has no document".into());
    }
    Ok((v, blobs))
}

/// From the points of the document to the millimetres of the drawing: from the top left corner of the (first) artboard.
fn page_xf(doc: &Value) -> Xf {
    let rect = doc.pointer("/artboards/0/rect");
    let at = |k: &str| rect.and_then(|r| r.get(k)).and_then(Value::as_f64).unwrap_or(0.0);
    let (x0, y0) = (at("x0").min(at("x1")), at("y0").min(at("y1")));
    Xf::translate(-x0, -y0).then(Xf::scale(PT_TO_MM, PT_TO_MM))
}

/// A transform of the file: `[a, b, c, d, e, f]`.
fn xf_of(v: Option<&Value>) -> Option<Xf> {
    let a = v?.as_array()?;
    if a.len() != 6 {
        return None;
    }
    let n: Vec<f64> = a.iter().map(|x| x.as_f64().filter(|f| f.is_finite())).collect::<Option<Vec<_>>>()?;
    Some(Xf { a: n[0], b: n[1], c: n[2], d: n[3], e: n[4], f: n[5] })
}

fn pt_of(v: Option<&Value>) -> Option<Pt> {
    let a = v?.as_array()?;
    let (x, y) = (a.first()?.as_f64()?, a.get(1)?.as_f64()?);
    (x.is_finite() && y.is_finite()).then(|| Pt::new(x, y))
}

/// The colour of a paint: RGB, grey or CMYK, 0..1.
fn colour_of(paint: &Value) -> Option<[u8; 3]> {
    if paint.get("type").and_then(Value::as_str) != Some("solid") {
        return None;
    }
    let c = paint.get("color")?;
    let f = |k: &str| c.get(k).and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0);
    let byte = |v: f64| (v * 255.0).round() as u8;
    match c.get("model").and_then(Value::as_str).unwrap_or("rgb") {
        "cmyk" => {
            let k = f("k");
            let ch = |v: f64| byte((1.0 - v) * (1.0 - k));
            Some([ch(f("c")), ch(f("m")), ch(f("y"))])
        }
        "gray" | "grey" => {
            let g = byte(c.get("g").or_else(|| c.get("gray")).and_then(Value::as_f64).unwrap_or(0.0).clamp(0.0, 1.0));
            Some([g, g, g])
        }
        _ => Some([byte(f("r")), byte(f("g")), byte(f("b"))]),
    }
}

/// The stroke colour of an object, or its fill when it has no stroke; black when it has neither.
fn appearance_colour(node: &Value) -> [u8; 3] {
    let items = node.pointer("/appearance/items").and_then(Value::as_array);
    let pick = |kind: &str| items.and_then(|it| it.iter().filter(|i| i.get("kind").and_then(Value::as_str) == Some(kind)).find_map(|i| i.get("paint").and_then(colour_of)));
    pick("stroke").or_else(|| pick("fill")).unwrap_or([0, 0, 0])
}

/// The contours of a path (`{"subpaths": [{"anchors": [...], "closed": ...}]}`) through `xf`.
fn contours_of(path: &Value, xf: &Xf, points: &mut usize) -> Vec<Contour> {
    let mut out = vec![];
    for sp in path.get("subpaths").and_then(Value::as_array).into_iter().flatten() {
        let mut nodes = vec![];
        for a in sp.get("anchors").and_then(Value::as_array).into_iter().flatten() {
            let Some(p) = pt_of(a.get("p")) else { continue };
            let hin = pt_of(a.get("in")).unwrap_or(p);
            let hout = pt_of(a.get("out")).unwrap_or(p);
            let smooth = matches!(a.get("kind").and_then(Value::as_str), Some("Smooth" | "Symmetric" | "smooth"));
            nodes.push(Node { p: xf.apply(p), hin: xf.apply(hin), hout: xf.apply(hout), smooth });
            *points += 1;
        }
        if !nodes.is_empty() {
            out.push(Contour { nodes, closed: sp.get("closed").and_then(Value::as_bool).unwrap_or(false) });
        }
    }
    out
}

/// What is read from the tree of objects.
struct Walk<'a> {
    outlines: Vec<Outline>,
    pictures: Vec<(crate::image::ImageData, Xf)>,
    points: usize,
    images: Option<&'a Value>,
    blobs: &'a [u8],
    want_pictures: bool,
}

impl Walk<'_> {
    fn node(&mut self, n: &Value, xf: Xf, layer: &Option<String>, depth: usize) -> Result<(), String> {
        if depth > MAX_DEPTH {
            return Ok(());
        }
        if self.points > MAX_POINTS {
            return Err("the VectorCraft file has too many points".into());
        }
        let Some(kind) = n.get("kind") else { return Ok(()) };
        // A transform of a group or an object goes before the one of what holds it.
        let xf = xf_of(kind.get("xf").or_else(|| n.get("xf"))).map(|own| own.then(xf)).unwrap_or(xf);
        let ty = kind.get("type").and_then(Value::as_str).unwrap_or("");
        if ty == "image" {
            if self.want_pictures {
                if let Some(p) = self.picture(kind, &xf) {
                    self.pictures.push(p);
                }
            }
            return Ok(());
        }
        // A path, or the paths of a compound path.
        let mut contours = vec![];
        if let Some(p) = kind.get("path") {
            contours.extend(contours_of(p, &xf, &mut self.points));
        }
        for p in kind.get("paths").and_then(Value::as_array).into_iter().flatten() {
            contours.extend(contours_of(p.get("path").unwrap_or(p), &xf, &mut self.points));
        }
        if !contours.is_empty() {
            self.outlines.push(Outline { contours, color: appearance_colour(n), layer: layer.clone() });
        }
        for c in kind.get("children").and_then(Value::as_array).into_iter().flatten() {
            self.node(c, xf, layer, depth + 1)?;
        }
        Ok(())
    }

    /// A picture: its bytes (in the raw data, or as base64 in the JSON), placed by its transform from its own units (0..width,
    /// 0..height along its rows; VectorCraft keeps the rows bottom first and turns them over with the transform) to the document.
    fn picture(&self, kind: &Value, xf: &Xf) -> Option<(crate::image::ImageData, Xf)> {
        use base64::Engine;
        let key = kind.get("key").and_then(Value::as_str)?;
        let entry = self.images?.get(key)?;
        let bytes: Vec<u8> = if let Some(b) = entry.get("blob").and_then(Value::as_array) {
            let (off, len) = (b.first()?.as_u64()? as usize, b.get(1)?.as_u64()? as usize);
            self.blobs.get(off..off.checked_add(len)?)?.to_vec()
        } else {
            let s = entry.get("data").and_then(Value::as_str)?;
            base64::engine::general_purpose::STANDARD.decode(s.trim()).ok()?
        };
        let (w, h) = (kind.get("width").and_then(Value::as_f64)?, kind.get("height").and_then(Value::as_f64)?);
        if !(w > 0.0 && h > 0.0) {
            return None;
        }
        let pic = crate::image::ImageData::from_bytes(&bytes, key, 100.0).ok()?;
        // The picture's millimetres → its own units → the document → the drawing; the transform carries the scale.
        let to_units = Xf::scale(w / pic.w, h / pic.h);
        Some((pic, to_units.then(*xf)))
    }
}

/// The outlines and (with `want_pictures`) the pictures of a file.
type Read = (Vec<Outline>, Vec<(crate::image::ImageData, Xf)>);

fn walk(data: &[u8], want_pictures: bool) -> Result<Read, String> {
    let (v, blobs) = split(data)?;
    let doc = &v["document"];
    let page = page_xf(doc);
    let mut w = Walk { outlines: vec![], pictures: vec![], points: 0, images: v.get("images"), blobs, want_pictures };
    let layers: Vec<&Value> = doc.get("layers").and_then(Value::as_array).map(|l| l.iter().collect()).unwrap_or_default();
    let named = layers.len() > 1;
    for l in layers {
        let name = if named { l.get("name").and_then(Value::as_str).map(str::to_string) } else { None };
        w.node(l, page, &name, 0)?;
    }
    Ok((w.outlines, w.pictures))
}

/// Read the paths of a VectorCraft drawing into outlines, in millimetres from the top left corner of its artboard (none for a
/// drawing of pictures only).
pub fn parse(data: &[u8]) -> Result<Vec<Outline>, String> {
    Ok(walk(data, false)?.0)
}

/// The pictures of a VectorCraft drawing, each with the transform from the picture (0..w, 0..h millimetres) to the drawing.
pub fn images(data: &[u8]) -> Vec<(crate::image::ImageData, Xf)> {
    walk(data, true).map(|r| r.1).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;

    fn png() -> String {
        let img = image::RgbaImage::from_pixel(4, 2, image::Rgba([0, 0, 0, 255]));
        let mut out = std::io::Cursor::new(vec![]);
        img.write_to(&mut out, image::ImageFormat::Png).unwrap();
        base64::engine::general_purpose::STANDARD.encode(out.into_inner())
    }

    fn doc() -> String {
        let anchor = |x: f64, y: f64| format!("{{\"p\": [{x}, {y}]}}");
        let path = |pts: &[(f64, f64)], colour: &str| {
            format!(
                "{{\"id\": 5, \"appearance\": {{\"items\": [{{\"kind\": \"fill\", \"paint\": {{\"type\": \"solid\", \"color\": {colour}}}}}]}}, \"kind\": {{\"type\": \"path\", \"path\": {{\"subpaths\": [{{\"anchors\": [{}], \"closed\": true}}]}}}}}}",
                pts.iter().map(|(x, y)| anchor(*x, *y)).collect::<Vec<_>>().join(",")
            )
        };
        let square = path(&[(0.0, 0.0), (72.0, 0.0), (72.0, 72.0), (0.0, 72.0)], "{\"model\": \"cmyk\", \"c\": 0, \"m\": 1, \"y\": 1, \"k\": 0}");
        let moved = format!("{{\"id\": 4, \"kind\": {{\"type\": \"group\", \"xf\": [1, 0, 0, 1, 72, 36], \"children\": [{square}]}}}}");
        let picture = "{\"id\": 6, \"kind\": {\"type\": \"image\", \"key\": \"img-1\", \"width\": 4, \"height\": 2, \"xf\": [18, 0, 0, -18, 0, 36]}}";
        format!(
            "{{\"format\": \"vectorcraft\", \"version\": 4, \"document\": {{\"artboards\": [{{\"rect\": {{\"x0\": -72, \"y0\": 0, \"x1\": 400, \"y1\": 400}}}}],
             \"layers\": [{{\"name\": \"Cut\", \"kind\": {{\"type\": \"layer\", \"children\": [{moved}]}}}}, {{\"name\": \"Photo\", \"kind\": {{\"type\": \"layer\", \"children\": [{picture}]}}}}]}},
             \"images\": {{\"img-1\": {{\"mime\": \"image/png\", \"data\": \"{}\"}}}}}}",
            png()
        )
    }

    #[test]
    fn groups_with_their_transforms_layers_colours_and_pictures_are_read() {
        let data = doc().into_bytes();
        assert!(is_vectorcraft(&data));
        let out = parse(&data).unwrap();
        assert_eq!(out.len(), 1);
        // A square of an inch, moved an inch right and half down, from an artboard that starts an inch left.
        let c = &out[0].contours[0];
        let inch = 25.4;
        assert!(c.closed && c.nodes.len() == 4);
        assert!((c.nodes[0].p.x - 2.0 * inch).abs() < 1e-9 && (c.nodes[0].p.y - 0.5 * inch).abs() < 1e-9, "{:?}", c.nodes[0].p);
        assert!((c.nodes[2].p.x - 3.0 * inch).abs() < 1e-9 && (c.nodes[2].p.y - 1.5 * inch).abs() < 1e-9);
        assert_eq!(out[0].color, [255, 0, 0]);
        assert_eq!(out[0].layer.as_deref(), Some("Cut"));
        // The picture: 4 × 2 units of 18 points, its first row at the bottom (the transform turns it over).
        let pics = images(&data);
        assert_eq!(pics.len(), 1);
        let (im, xf) = &pics[0];
        assert_eq!((im.px_w, im.px_h), (4, 2));
        let (a, b) = (xf.apply(Pt::new(0.0, 0.0)), xf.apply(Pt::new(im.w, im.h)));
        assert!((a.x - inch).abs() < 1e-9 && (a.y - 0.5 * inch).abs() < 1e-9, "{a:?}");
        assert!((b.x - 2.0 * inch).abs() < 1e-9 && b.y.abs() < 1e-9, "{b:?}");
        assert!(parse(b"{\"format\": \"other\"}").is_err());
        assert!(parse(b"{\"format\": \"vectorcraft\", \"document\": {").is_err());
    }
}
