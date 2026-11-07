//! The same drawings saved by LightBurn 2.2 (DXF, DXF legacy = R12, Illustrator, LBRN, LBRN2, SVG) and MillMage 0.9 (MAGE), in
//! `tests/fixtures/interchange` (the thumbnails taken out of the project files): every one is read to the same shapes, and what
//! the programs write in each format is read back to the same shapes. The work area is 400 × 400 mm.
use base_core::geom::Pt;
use base_core::lightburn::{self, CutKind, Flavor, LayerInfo};
use base_core::scene::{Kind, Scene};
use base_core::svg::Outline;

const AREA: (f64, f64) = (400.0, 400.0);
const SAMPLES: [&str; 11] = ["elipsa", "elipsy-3", "kolka-3", "kolko", "ksztalty", "kwadrat", "kwadraty-3", "star", "stars-3", "trojkat", "trojkaty-3"];

fn file(name: &str) -> Vec<u8> {
    std::fs::read(format!("{}/tests/fixtures/interchange/{name}", env!("CARGO_MANIFEST_DIR"))).expect(name)
}

/// (left, top, right, bottom) in millimetres.
type Bounds = (f64, f64, f64, f64);

fn of_outlines(o: &[Outline]) -> Bounds {
    let mut b = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in o.iter().flat_map(|o| o.contours.iter()).flat_map(|c| c.flatten(0.005).pts) {
        b = (b.0.min(p.x), b.1.min(p.y), b.2.max(p.x), b.3.max(p.y));
    }
    b
}

fn of_scene(s: &Scene) -> Bounds {
    let ids: Vec<u64> = s.shapes.iter().map(|x| x.id).collect();
    let r = s.bounds_of(&ids).expect("the scene has shapes");
    (r.min.x, r.min.y, r.max.x, r.max.y)
}

fn size(b: Bounds) -> (f64, f64) {
    (b.2 - b.0, b.3 - b.1)
}

fn close(a: Bounds, b: Bounds, tol: f64, what: &str) {
    let d = [(a.0 - b.0).abs(), (a.1 - b.1).abs(), (a.2 - b.2).abs(), (a.3 - b.3).abs()];
    assert!(d.iter().all(|d| *d < tol), "{what}: {a:?} and {b:?}");
}

fn close_size(a: Bounds, b: Bounds, tol: f64, what: &str) {
    let (sa, sb) = (size(a), size(b));
    assert!((sa.0 - sb.0).abs() < tol && (sa.1 - sb.1).abs() < tol, "{what}: {sa:?} and {sb:?}");
}

/// The flattened contours, moved so their box starts at (0, 0): two drawings that only lie elsewhere compare equal.
type Lines = Vec<(Vec<Pt>, bool)>;

fn lines_of(cs: &[base_core::bezier::Contour]) -> Lines {
    let ps: Vec<_> = cs.iter().map(|c| c.flatten(0.01)).collect();
    let (mut x0, mut y0) = (f64::MAX, f64::MAX);
    for q in ps.iter().flat_map(|p| p.pts.iter()) {
        x0 = x0.min(q.x);
        y0 = y0.min(q.y);
    }
    ps.into_iter().map(|p| (p.pts.iter().map(|q| Pt::new(q.x - x0, q.y - y0)).collect(), p.closed)).collect()
}

fn outline_lines(o: &[Outline]) -> Lines {
    lines_of(&o.iter().flat_map(|o| o.contours.iter().cloned()).collect::<Vec<_>>())
}

fn scene_lines(s: &Scene) -> Lines {
    lines_of(&s.shapes.iter().flat_map(base_core::export::contours_of).collect::<Vec<_>>())
}

fn dist_to(p: Pt, l: &Lines) -> f64 {
    let mut best = f64::MAX;
    for (pts, closed) in l {
        let n = pts.len();
        let segs = if *closed { n } else { n.saturating_sub(1) };
        for i in 0..segs {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            let len2 = dx * dx + dy * dy;
            let t = if len2 > 0.0 { (((p.x - a.x) * dx + (p.y - a.y) * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
            best = best.min((p.x - a.x - t * dx).hypot(p.y - a.y - t * dy));
        }
    }
    best
}

/// Every point of one drawing lies on the other and back, within `tol`: a mirrored or turned drawing (the same box) fails.
fn same_shape(a: &Lines, b: &Lines, tol: f64, what: &str) {
    for (x, y, dir) in [(a, b, "→"), (b, a, "←")] {
        for p in x.iter().flat_map(|(pts, _)| pts.iter()) {
            let d = dist_to(*p, y);
            assert!(d < tol, "{what} {dir}: {p:?} is {d} mm away");
        }
    }
}

fn contours_count(o: &[Outline]) -> usize {
    o.iter().map(|o| o.contours.len()).sum()
}

fn names(l: usize) -> String {
    format!("C{l:02}")
}

fn colors(_: usize) -> [u8; 3] {
    [0, 0, 0]
}

#[test]
fn every_format_of_lightburn_and_millmage_is_read_to_the_same_shapes() {
    for name in SAMPLES {
        let dxf = base_core::dxf::parse(&file(&format!("{name}.dxf")), AREA.1).unwrap();
        let reference = of_outlines(&dxf.outlines);
        let shape = outline_lines(&dxf.outlines);
        let n = contours_count(&dxf.outlines);
        let legacy = base_core::dxf::parse(&file(&format!("{name}-legacy.dxf")), AREA.1).unwrap();
        close(of_outlines(&legacy.outlines), reference, 0.05, &format!("{name}: DXF legacy"));
        same_shape(&outline_lines(&legacy.outlines), &shape, 0.05, &format!("{name}: DXF legacy"));
        assert_eq!(contours_count(&legacy.outlines), n, "{name}: DXF legacy");
        for ext in ["lbrn", "lbrn2"] {
            let imp = lightburn::import(&String::from_utf8(file(&format!("{name}.{ext}"))).unwrap(), AREA.1).unwrap();
            close(of_scene(&imp.scene), reference, 0.05, &format!("{name}.{ext}"));
            same_shape(&scene_lines(&imp.scene), &shape, 0.05, &format!("{name}.{ext}"));
            assert_eq!(imp.scene.shapes.len(), n, "{name}.{ext}");
            assert!(imp.warnings.is_empty(), "{name}.{ext}: {:?}", imp.warnings);
        }
        // MillMage: the same shapes, placed elsewhere on its own work area.
        let mage = lightburn::import(&String::from_utf8(file(&format!("{name}.mage"))).unwrap(), AREA.1).unwrap();
        close_size(of_scene(&mage.scene), reference, 0.05, &format!("{name}.mage"));
        same_shape(&scene_lines(&mage.scene), &shape, 0.05, &format!("{name}.mage"));
        assert_eq!(mage.scene.shapes.len(), n, "{name}.mage");
        assert!(!mage.cuts.is_empty(), "{name}.mage: its layers");
        // Illustrator and SVG put the drawing in the top left corner.
        let ai = base_core::svg::read_drawing(&file(&format!("{name}.ai"))).unwrap();
        if name == "stars-3" {
            // LightBurn wrote only one of the three stars (the one of 61.5 × 57.5 radii) into this file.
            assert_eq!(contours_count(&ai), 1);
            let s = size(of_outlines(&ai));
            assert!((s.1 - 57.5 * (1.0 + (std::f64::consts::PI / 5.0).cos())).abs() < 0.05, "{s:?}");
        } else {
            close_size(of_outlines(&ai), reference, 0.05, &format!("{name}.ai"));
            same_shape(&outline_lines(&ai), &shape, 0.05, &format!("{name}.ai"));
            assert_eq!(contours_count(&ai), n, "{name}.ai");
        }
        assert_eq!(ai[0].layer.as_deref(), Some("Layer 1"));
        let svg = base_core::svg::read_drawing(&file(&format!("{name}.svg"))).unwrap();
        close_size(of_outlines(&svg), reference, 0.05, &format!("{name}.svg"));
        same_shape(&outline_lines(&svg), &shape, 0.05, &format!("{name}.svg"));
    }
}

#[test]
fn a_picture_comes_from_lightburn_millmage_and_svg_with_its_size() {
    let want = (257.49561, 171.78131);
    for ext in ["lbrn", "lbrn2", "mage"] {
        let imp = lightburn::import(&String::from_utf8(file(&format!("zdjecie.{ext}"))).unwrap(), AREA.1).unwrap();
        assert_eq!(imp.scene.shapes.len(), 1, "zdjecie.{ext}");
        assert!(imp.scene.shapes[0].is_image());
        let s = size(of_scene(&imp.scene));
        assert!((s.0 - want.0).abs() < 0.01 && (s.1 - want.1).abs() < 0.01, "zdjecie.{ext}: {s:?}");
    }
    let pics = base_core::svg::images(&file("zdjecie.svg"));
    assert_eq!(pics.len(), 1);
    let (pic, xf) = &pics[0];
    let (a, b) = (xf.apply(base_core::geom::Pt::new(0.0, 0.0)), xf.apply(base_core::geom::Pt::new(pic.w, pic.h)));
    assert!(((b.x - a.x).abs() - want.0).abs() < 0.01 && ((b.y - a.y).abs() - want.1).abs() < 0.01, "{a:?} {b:?}");
    // The picture stands upright: its top row is the top row of the one LightBurn has.
    let lb = lightburn::import(&String::from_utf8(file("zdjecie.lbrn2")).unwrap(), AREA.1).unwrap();
    let Kind::Image(lb_pic) = &lb.scene.shapes[0].kind else { panic!() };
    assert_eq!((pic.px_w, pic.px_h), (lb_pic.px_w, lb_pic.px_h));
    assert_eq!(&pic.gray[..64], &lb_pic.gray[..64]);
    // No vectors in the other files, and they say so rather than panic.
    assert!(base_core::svg::read_drawing(&file("zdjecie.ai")).is_err());
    assert!(base_core::dxf::parse(&file("zdjecie.dxf"), AREA.1).unwrap().outlines.is_empty());
}

#[test]
fn what_is_written_in_every_format_is_read_back_to_the_same_shapes() {
    let no_cut = |_: usize| None;
    let info = LayerInfo { name: &names, color: &colors, cut: &no_cut };
    for name in SAMPLES.into_iter().chain(["zdjecie"]) {
        let imp = lightburn::import(&String::from_utf8(file(&format!("{name}.lbrn2"))).unwrap(), AREA.1).unwrap();
        let scene = imp.scene;
        let reference = of_scene(&scene);
        let shape = scene_lines(&scene);
        let pictures = scene.shapes.iter().filter(|s| s.is_image()).count();
        for flavor in [Flavor::Lbrn, Flavor::Lbrn2, Flavor::Mage] {
            let text = lightburn::write(&scene, &[], AREA, &info, flavor);
            let back = lightburn::import(&text, AREA.1).unwrap_or_else(|e| panic!("{name} {flavor:?}: {e}"));
            assert_eq!(back.scene.shapes.len(), scene.shapes.len(), "{name} {flavor:?}");
            close(of_scene(&back.scene), reference, 0.02, &format!("{name} {flavor:?}"));
            same_shape(&scene_lines(&back.scene), &shape, 0.02, &format!("{name} {flavor:?}"));
            // A picture comes back where it was, upright: its corner (0, 0) lands on the same spot and its top row is the same.
            for (a, b) in scene.shapes.iter().zip(&back.scene.shapes).filter(|(a, _)| a.is_image()) {
                let (Kind::Image(pa), Kind::Image(pb)) = (&a.kind, &b.kind) else { panic!("{name} {flavor:?}: not a picture") };
                for corner in [Pt::new(0.0, 0.0), Pt::new(pa.w, 0.0), Pt::new(0.0, pa.h)] {
                    let k = Pt::new(corner.x / pa.w * pb.w, corner.y / pa.h * pb.h);
                    let (p, q) = (a.xf.apply(corner), b.xf.apply(k));
                    assert!(p.dist(q) < 0.02, "{name} {flavor:?}: the corner {corner:?} went from {p:?} to {q:?}");
                }
                assert_eq!((pa.px_w, pa.px_h), (pb.px_w, pb.px_h));
                assert_eq!(&pa.gray[..pa.px_w as usize], &pb.gray[..pb.px_w as usize], "{name} {flavor:?}: the top row");
            }
            assert!(back.warnings.is_empty(), "{name} {flavor:?}: {:?}", back.warnings);
            assert_eq!(back.scene.shapes.iter().filter(|s| s.is_image()).count(), pictures);
        }
        if pictures > 0 {
            // A picture keeps its place in the SVG too.
            let svg = base_core::export::svg(&scene, &[], AREA.0, AREA.1, &names, &colors);
            let pics = base_core::svg::images(svg.as_bytes());
            assert_eq!(pics.len(), 1);
            let (pic, xf) = &pics[0];
            let a = xf.apply(base_core::geom::Pt::new(0.0, 0.0));
            let b = xf.apply(base_core::geom::Pt::new(pic.w, pic.h));
            close((a.x.min(b.x), a.y.min(b.y), a.x.max(b.x), a.y.max(b.y)), reference, 0.05, "zdjecie: SVG");
            continue;
        }
        let modern = base_core::export::dxf_modern(&scene, &[], AREA.1, &names);
        assert!(modern.contains("AC1021") && modern.contains("LWPOLYLINE"));
        let m = base_core::dxf::parse(modern.as_bytes(), AREA.1).unwrap().outlines;
        close(of_outlines(&m), reference, 0.02, &format!("{name}: DXF"));
        same_shape(&outline_lines(&m), &shape, 0.02, &format!("{name}: DXF"));
        let legacy = base_core::export::dxf(&scene, &[], AREA.1, &names);
        assert!(legacy.contains("AC1009") && !legacy.contains("LWPOLYLINE"));
        let l = base_core::dxf::parse(legacy.as_bytes(), AREA.1).unwrap().outlines;
        close(of_outlines(&l), reference, 0.02, &format!("{name}: DXF legacy"));
        same_shape(&outline_lines(&l), &shape, 0.02, &format!("{name}: DXF legacy"));
        let ai = base_core::ai::write(&scene, &[], AREA.1, &names, &colors);
        let back = base_core::svg::read_drawing(ai.as_bytes()).unwrap();
        close_size(of_outlines(&back), reference, 0.02, &format!("{name}: Illustrator"));
        same_shape(&outline_lines(&back), &shape, 0.02, &format!("{name}: Illustrator"));
        assert_eq!(back[0].layer.as_deref(), Some("C00"));
        let svg = base_core::export::svg(&scene, &[], AREA.0, AREA.1, &names, &colors);
        let v = base_core::svg::parse(svg.as_bytes()).unwrap();
        close(of_outlines(&v), reference, 0.02, &format!("{name}: SVG"));
        same_shape(&outline_lines(&v), &shape, 0.02, &format!("{name}: SVG"));
    }
}

#[test]
fn an_illustrator_file_with_curves_layers_and_a_compound_path_is_read() {
    let ai = b"%!PS-Adobe-3.0\n%%Creator: test\n%AI5_FileFormat 2.0\n%%EndSetup\n%AI5_BeginLayer\n(Outer) Ln\n0 0 0 1 K\n0 0 m\n72 0 L\n72 72 l\n0 72 L\ns\n%AI5_EndLayer--\n%AI5_BeginLayer\n(Inner \\(cut\\)) Ln\n*u\n0 0 m\n36 72 72 72 72 0 C\nf\n10 10 m\n20 10 L\n20 20 L\nh\n*U\n0 0 m 10 10 L N\n%AI5_EndLayer--\n%%Trailer\n";
    let o = base_core::pdf::parse(ai).unwrap();
    assert_eq!(o.len(), 2, "the square and the compound path; the unpainted line is left out");
    assert_eq!(o[0].layer.as_deref(), Some("Outer"));
    assert_eq!(o[1].layer.as_deref(), Some("Inner (cut)"));
    assert!(o[0].contours[0].closed && o[0].contours[0].nodes.len() == 4);
    assert_eq!(o[1].contours.len(), 1, "an `h` path is not painted on its own");
    let b = of_outlines(&o);
    close(b, (0.0, 0.0, 25.4, 25.4), 1e-6, "one inch");
    // Broken input: errors, no panics.
    for bad in [&b"%!PS-Adobe-3.0\n%AI5_\n%%EndSetup\n1 2 C\nm m L (unclosed\n"[..], b"%!PS-Adobe-3.0\n%AI\n", b"%!PS-Adobe-3.0 %AI 1e400 1e400 m 5 5 L S"] {
        let _ = base_core::ai::parse(bad);
    }
}

#[test]
fn a_mirrored_drawing_is_not_the_same_shape() {
    let imp = lightburn::import(&String::from_utf8(file("trojkat.lbrn2")).unwrap(), AREA.1).unwrap();
    let mut flipped = imp.scene.clone();
    for sh in &mut flipped.shapes {
        sh.xf = sh.xf.then(base_core::geom::Xf { a: 1.0, b: 0.0, c: 0.0, d: -1.0, e: 0.0, f: 0.0 });
    }
    let r = std::panic::catch_unwind(|| same_shape(&scene_lines(&imp.scene), &scene_lines(&flipped), 0.05, "flipped"));
    assert!(r.is_err(), "a triangle upside down has the same box but not the same shape");
}

#[test]
fn the_settings_of_the_layers_go_into_a_lightburn_project() {
    let imp = lightburn::import(&String::from_utf8(file("ksztalty.lbrn2")).unwrap(), AREA.1).unwrap();
    let cut = |l: usize| {
        Some(lightburn::Cut { index: l, name: String::new(), kind: if l == 0 { CutKind::Fill } else { CutKind::Line }, power_max: 55.0, power_min: 5.0, speed_mm_s: 42.0, passes: 3, interval_mm: Some(0.1) })
    };
    let info = LayerInfo { name: &names, color: &colors, cut: &cut };
    let back = lightburn::import(&lightburn::write(&imp.scene, &[], AREA, &info, Flavor::Lbrn2), AREA.1).unwrap();
    let c = &back.cuts[0];
    assert_eq!((c.kind, c.power_max, c.power_min, c.speed_mm_s, c.passes, c.interval_mm), (CutKind::Fill, 55.0, 5.0, 42.0, 3, Some(0.1)));
    assert_eq!(c.name, "C00");
}
