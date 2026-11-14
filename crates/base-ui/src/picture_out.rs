//! Pictures of what the interface draws, made without a window: the shapes of egui are tessellated and filled into an image by a
//! small rasteriser (triangles with the colours of their corners and the texture of the fonts), so a picture has the same lines and
//! letters as the screen (Ø and the letters of every language included). Saved as PNG or JPEG, or as a PDF page with the picture.
use egui::{Color32, Pos2, Rect, Vec2};
use std::collections::HashMap;

/// The picture of `size` points drawn by `paint`, at `ppp` pixels for a point, on white.
pub fn render(size: Vec2, ppp: f32, mut paint: impl FnMut(&mut egui::Ui)) -> image::RgbaImage {
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    ctx.set_pixels_per_point(ppp);
    let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), ..Default::default() };
    let mut textures: HashMap<egui::TextureId, egui::ColorImage> = HashMap::new();
    let mut out = ctx.run_ui(input(), |ui| paint(ui));
    // The fonts are known from the first pass: draw once more so every letter is in the texture.
    apply(&mut textures, &out.textures_delta);
    out.textures_delta.clear();
    out = ctx.run_ui(input(), |ui| paint(ui));
    apply(&mut textures, &out.textures_delta);
    out.textures_delta.clear();
    let prims = ctx.tessellate(out.shapes, out.pixels_per_point);
    let (w, h) = ((size.x * ppp).round() as u32, (size.y * ppp).round() as u32);
    let mut px = vec![[1.0f32, 1.0, 1.0, 1.0]; (w * h) as usize];
    for p in prims {
        let egui::epaint::Primitive::Mesh(mesh) = p.primitive else { continue };
        let tex = textures.get(&mesh.texture_id);
        let clip = Rect::from_min_max((p.clip_rect.min.to_vec2() * ppp).to_pos2(), (p.clip_rect.max.to_vec2() * ppp).to_pos2());
        for t in mesh.indices.chunks_exact(3) {
            let v = [mesh.vertices[t[0] as usize], mesh.vertices[t[1] as usize], mesh.vertices[t[2] as usize]];
            triangle(&mut px, w, h, clip, ppp, &v, tex);
        }
    }
    let mut img = image::RgbaImage::new(w, h);
    for (i, c) in px.iter().enumerate() {
        let q = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
        img.put_pixel(i as u32 % w, i as u32 / w, image::Rgba([q(c[0]), q(c[1]), q(c[2]), 255]));
    }
    img
}

/// The lines and the words that `paint` draws on a sheet of `size` points, as a drawing in millimetres (`mm` for a point, Y up) for
/// a DXF (`base_core::export::dxf_drawing`): what is filled alone (the paper, a background) is left out; strokes thicker than
/// `bold` points go on the layer OUTLINE, the others on THIN, the words on TEXT.
pub fn drawing_of(size: Vec2, mm: f64, bold: f32, mut paint: impl FnMut(&mut egui::Ui)) -> base_core::export::Drawing {
    use base_core::export::{Drawing, DrawingLine, DrawingText};
    use base_core::geom::Pt;
    use egui::epaint::Shape;
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    let input = || egui::RawInput { screen_rect: Some(Rect::from_min_size(Pos2::ZERO, size)), ..Default::default() };
    let mut first = ctx.run_ui(input(), |ui| paint(ui));
    first.textures_delta.clear();
    let mut out = ctx.run_ui(input(), |ui| paint(ui));
    out.textures_delta.clear();
    let to = |p: Pos2| Pt::new(p.x as f64 * mm, (size.y - p.y) as f64 * mm);
    let mut d = Drawing::default();
    fn walk(s: &Shape, d: &mut Drawing, to: &dyn Fn(Pos2) -> Pt, mm: f64, bold: f32) {
        let layer = |w: f32| if w > bold { "OUTLINE" } else { "THIN" };
        let mut line = |pts: Vec<Pos2>, closed: bool, w: f32| {
            if pts.len() >= 2 {
                d.lines.push(DrawingLine { layer: layer(w), pts: pts.into_iter().map(to).collect(), closed });
            }
        };
        match s {
            Shape::Vec(v) => v.iter().for_each(|s| walk(s, d, to, mm, bold)),
            Shape::LineSegment { points, stroke } if stroke.width > 0.0 => line(points.to_vec(), false, stroke.width),
            Shape::Path(p) => {
                // An arrowhead or another filled outline is drawn by its outline.
                if p.stroke.width > 0.0 || (p.closed && p.fill != egui::Color32::TRANSPARENT) {
                    line(p.points.clone(), p.closed, p.stroke.width);
                }
            }
            Shape::Rect(r) if r.stroke.width > 0.0 => {
                let q = r.rect;
                line(vec![q.left_top(), q.right_top(), q.right_bottom(), q.left_bottom()], true, r.stroke.width);
            }
            Shape::Circle(c) if c.stroke.width > 0.0 || c.fill != egui::Color32::TRANSPARENT => {
                let n = 48;
                line((0..n).map(|i| c.center + Vec2::angled(i as f32 / n as f32 * std::f32::consts::TAU) * c.radius).collect(), true, c.stroke.width);
            }
            Shape::Ellipse(e) if e.stroke.width > 0.0 => {
                let n = 48;
                line((0..n).map(|i| { let a = i as f32 / n as f32 * std::f32::consts::TAU; e.center + Vec2::new(a.cos() * e.radius.x, a.sin() * e.radius.y) }).collect(), true, e.stroke.width);
            }
            Shape::QuadraticBezier(b) if b.stroke.width > 0.0 => line(b.flatten(Some(0.2)), b.closed, b.stroke.width),
            Shape::CubicBezier(b) if b.stroke.width > 0.0 => line(b.flatten(Some(0.2)), b.closed, b.stroke.width),
            Shape::Text(t) => {
                // Each row of the words on its own, from the left end of its base line, turned with the text.
                let (sin, cos) = t.angle.sin_cos();
                let turn = |v: Vec2| Vec2::new(v.x * cos - v.y * sin, v.x * sin + v.y * cos);
                for row in &t.galley.rows {
                    let text: String = row.row.glyphs.iter().map(|g| g.chr).collect();
                    if text.trim().is_empty() {
                        continue;
                    }
                    let Some(g) = row.row.glyphs.first() else { continue };
                    // The glyph is placed by its base line; the capitals are about 0.6 of the height of the line of the font.
                    let base = row.pos + g.pos.to_vec2();
                    let at = t.pos + turn(base.to_vec2());
                    d.texts.push(DrawingText { layer: "TEXT", at: to(at), height: (g.font_height * 0.6) as f64 * mm, angle: -(t.angle as f64).to_degrees(), text: text.trim_end().to_string() });
                }
            }
            _ => {}
        }
    }
    for c in &out.shapes {
        walk(&c.shape, &mut d, &to, mm, bold);
    }
    d
}

fn apply(textures: &mut HashMap<egui::TextureId, egui::ColorImage>, delta: &egui::TexturesDelta) {
    for (id, ds) in &delta.set {
        for d in ds.iter() {
        let egui::ImageData::Color(img) = &d.image;
        match d.pos {
            None => {
                textures.insert(*id, (**img).clone());
            }
            Some([x0, y0]) => {
                if let Some(t) = textures.get_mut(id) {
                    for y in 0..img.size[1] {
                        for x in 0..img.size[0] {
                            if x0 + x < t.size[0] && y0 + y < t.size[1] {
                                t.pixels[(y0 + y) * t.size[0] + x0 + x] = img.pixels[y * img.size[0] + x];
                            }
                        }
                    }
                }
            }
        }
    }
    }
}

/// Fill one triangle into the pixels (premultiplied colours, painted over what is there).
fn triangle(px: &mut [[f32; 4]], w: u32, h: u32, clip: Rect, ppp: f32, v: &[egui::epaint::Vertex; 3], tex: Option<&egui::ColorImage>) {
    let p: Vec<Pos2> = v.iter().map(|q| (q.pos.to_vec2() * ppp).to_pos2()).collect();
    let area = (p[1].x - p[0].x) * (p[2].y - p[0].y) - (p[2].x - p[0].x) * (p[1].y - p[0].y);
    if area.abs() < 1e-6 {
        return;
    }
    let lo = |a: f32, b: f32, c: f32, min: f32| a.min(b).min(c).max(min).floor() as i64;
    let hi = |a: f32, b: f32, c: f32, max: f32| a.max(b).max(c).min(max).ceil() as i64;
    let (x0, x1) = (lo(p[0].x, p[1].x, p[2].x, clip.min.x.max(0.0)), hi(p[0].x, p[1].x, p[2].x, clip.max.x.min(w as f32)));
    let (y0, y1) = (lo(p[0].y, p[1].y, p[2].y, clip.min.y.max(0.0)), hi(p[0].y, p[1].y, p[2].y, clip.max.y.min(h as f32)));
    let col = |c: Color32| [c.r() as f32 / 255.0, c.g() as f32 / 255.0, c.b() as f32 / 255.0, c.a() as f32 / 255.0];
    let cs = [col(v[0].color), col(v[1].color), col(v[2].color)];
    for y in y0..y1 {
        for x in x0..x1 {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let e = |a: Pos2, b: Pos2| (b.x - a.x) * (fy - a.y) - (fx - a.x) * (b.y - a.y);
            let (w0, w1, w2) = (e(p[1], p[2]) / area, e(p[2], p[0]) / area, e(p[0], p[1]) / area);
            if w0 < -1e-4 || w1 < -1e-4 || w2 < -1e-4 {
                continue;
            }
            let mut c = [0.0f32; 4];
            for k in 0..4 {
                c[k] = cs[0][k] * w0 + cs[1][k] * w1 + cs[2][k] * w2;
            }
            if let Some(t) = tex {
                let u = v[0].uv.x * w0 + v[1].uv.x * w1 + v[2].uv.x * w2;
                let vv = v[0].uv.y * w0 + v[1].uv.y * w1 + v[2].uv.y * w2;
                let tx = ((u * t.size[0] as f32) as usize).min(t.size[0] - 1);
                let ty = ((vv * t.size[1] as f32) as usize).min(t.size[1] - 1);
                let tc = col(t.pixels[ty * t.size[0] + tx]);
                for k in 0..4 {
                    c[k] *= tc[k];
                }
            }
            let d = &mut px[(y as u32 * w + x as u32) as usize];
            for k in 0..3 {
                d[k] = c[k] + d[k] * (1.0 - c[3]);
            }
        }
    }
}

/// A PDF of one page `page_pt` (points, 1/72 inch) filled by a JPEG picture of `w` × `h` pixels.
pub fn pdf_of_jpeg(jpeg: &[u8], w: u32, h: u32, page_pt: (f64, f64)) -> Vec<u8> {
    let mut out: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = vec![];
    let content = format!("q {:.2} 0 0 {:.2} 0 0 cm /Im0 Do Q", page_pt.0, page_pt.1);
    let objects: Vec<Vec<u8>> = vec![
        b"<< /Type /Catalog /Pages 2 0 R >>".to_vec(),
        b"<< /Type /Pages /Kids [3 0 R] /Count 1 >>".to_vec(),
        format!("<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {:.2} {:.2}] /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>", page_pt.0, page_pt.1).into_bytes(),
        {
            let mut o = format!("<< /Type /XObject /Subtype /Image /Width {w} /Height {h} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /DCTDecode /Length {} >>\nstream\n", jpeg.len()).into_bytes();
            o.extend_from_slice(jpeg);
            o.extend_from_slice(b"\nendstream");
            o
        },
        format!("<< /Length {} >>\nstream\n{content}\nendstream", content.len()).into_bytes(),
    ];
    for (i, o) in objects.iter().enumerate() {
        offsets.push(out.len());
        out.extend_from_slice(format!("{} 0 obj\n", i + 1).as_bytes());
        out.extend_from_slice(o);
        out.extend_from_slice(b"\nendobj\n");
    }
    let xref = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n0000000000 65535 f \n", objects.len() + 1).as_bytes());
    for o in offsets {
        out.extend_from_slice(format!("{o:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(format!("trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n", objects.len() + 1).as_bytes());
    out
}

/// Save a picture by the extension of the path: `.png`, `.jpg` / `.jpeg`, or `.pdf` (an A4 page lying down, or standing when the
/// picture is higher than wide).
pub fn save(img: &image::RgbaImage, path: &std::path::Path) -> Result<(), String> {
    let ext = path.extension().map(|e| e.to_string_lossy().to_lowercase()).unwrap_or_default();
    let rgb = image::DynamicImage::ImageRgba8(img.clone()).to_rgb8();
    match ext.as_str() {
        "png" => img.save_with_format(path, image::ImageFormat::Png).map_err(|e| e.to_string()),
        "jpg" | "jpeg" => rgb.save_with_format(path, image::ImageFormat::Jpeg).map_err(|e| e.to_string()),
        "pdf" => {
            let mut jpeg = std::io::Cursor::new(Vec::new());
            image::codecs::jpeg::JpegEncoder::new_with_quality(&mut jpeg, 92).encode_image(&rgb).map_err(|e| e.to_string())?;
            let a4 = (841.89, 595.28);
            let page = if img.height() > img.width() { (a4.1, a4.0) } else { a4 };
            std::fs::write(path, pdf_of_jpeg(jpeg.get_ref(), img.width(), img.height(), page)).map_err(|e| e.to_string())
        }
        _ => Err(format!("unknown kind of picture: .{ext}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_drawing_is_rendered_with_its_lines_and_letters_and_saved_as_png_jpg_and_pdf() {
        let img = render(Vec2::new(200.0, 100.0), 2.0, |ui| {
            let p = ui.painter();
            p.line_segment([Pos2::new(10.0, 50.0), Pos2::new(190.0, 50.0)], egui::Stroke::new(2.0, Color32::BLACK));
            p.text(Pos2::new(100.0, 20.0), egui::Align2::CENTER_CENTER, "Ø 12", egui::FontId::proportional(14.0), Color32::BLACK);
        });
        assert_eq!((img.width(), img.height()), (400, 200));
        let dark = |x: u32, y: u32| img.get_pixel(x, y).0[0] < 100;
        assert!(dark(200, 100), "the line");
        assert!(!dark(200, 160), "white under it");
        let letters = (160..240).flat_map(|x| (25..55).map(move |y| (x, y))).filter(|(x, y)| dark(*x, *y)).count();
        assert!(letters > 20, "the letters are drawn: {letters} dark pixels");
        let dir = std::env::temp_dir().join(format!("picture-out-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for ext in ["png", "jpg", "pdf"] {
            let p = dir.join(format!("a.{ext}"));
            save(&img, &p).unwrap();
            let bytes = std::fs::read(&p).unwrap();
            assert!(bytes.len() > 100, "{ext}");
            if ext == "pdf" {
                assert!(bytes.starts_with(b"%PDF-1.4") && bytes.ends_with(b"%%EOF\n"));
            }
        }
        assert!(save(&img, &dir.join("a.gif")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod drawing_tests {
    use super::*;

    #[test]
    fn what_is_drawn_becomes_lines_and_words_in_millimetres_and_the_paper_is_left_out() {
        let size = Vec2::new(200.0, 100.0);
        let d = drawing_of(size, 0.5, 1.2, |ui| {
            let p = ui.painter();
            p.rect_filled(Rect::from_min_size(Pos2::ZERO, size), 0.0, egui::Color32::WHITE);
            p.rect_stroke(Rect::from_min_max(Pos2::new(10.0, 10.0), Pos2::new(110.0, 60.0)), 0.0, egui::Stroke::new(1.5, egui::Color32::BLACK), egui::StrokeKind::Middle);
            p.line_segment([Pos2::new(0.0, 90.0), Pos2::new(40.0, 90.0)], egui::Stroke::new(0.8, egui::Color32::BLACK));
            p.text(Pos2::new(20.0, 70.0), egui::Align2::LEFT_TOP, "Front view", egui::FontId::proportional(12.0), egui::Color32::BLACK);
            // A number turned a quarter turn as a vertical dimension is: it reads upwards.
            let galley = ui.fonts_mut(|f| f.layout_no_wrap("25".into(), egui::FontId::proportional(12.0), egui::Color32::BLACK));
            p.add(egui::epaint::TextShape::new(Pos2::new(150.0, 80.0), galley, egui::Color32::BLACK).with_angle(-std::f32::consts::FRAC_PI_2));
        });
        let outline = d.lines.iter().find(|l| l.layer == "OUTLINE").expect("the box");
        assert!(outline.closed && outline.pts.len() == 4);
        // 100 × 50 points at half a millimetre each, Y up from the bottom of the sheet.
        let xs: Vec<f64> = outline.pts.iter().map(|q| q.x).collect();
        let ys: Vec<f64> = outline.pts.iter().map(|q| q.y).collect();
        assert!((xs.iter().cloned().fold(f64::MIN, f64::max) - 55.0).abs() < 1e-3 && (ys.iter().cloned().fold(f64::MIN, f64::max) - 45.0).abs() < 1e-3, "{xs:?} {ys:?}");
        let thin = d.lines.iter().find(|l| l.layer == "THIN").expect("the thin line");
        assert!((thin.pts[0].y - 5.0).abs() < 1e-3);
        assert_eq!(d.lines.len(), 2, "the white paper is not a line");
        let up = d.texts.iter().find(|t| t.text == "25").expect("the turned number");
        assert!((up.angle - 90.0).abs() < 1e-4, "reads upwards: {up:?}");
        // Its base line starts right of the point it was drawn from (the top of the letters is towards the left) and goes up from it.
        assert!(up.at.x > 75.0 && up.at.x < 82.0 && (up.at.y - 10.0).abs() < 1.0, "{up:?}");
        let t = &d.texts[0];
        assert_eq!(t.text, "Front view");
        assert!((t.at.x - 10.0).abs() < 0.5 && t.at.y < 15.0 && t.at.y > 5.0 && t.height > 2.0 && t.height < 6.0, "{t:?}");
    }
}
