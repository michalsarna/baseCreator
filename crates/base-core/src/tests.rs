use crate::*;

fn sq(x: f64, y: f64, s: f64) -> Polyline {
    Polyline::new(vec![Pt::new(x, y), Pt::new(x + s, y), Pt::new(x + s, y + s), Pt::new(x, y + s)], true)
}

fn total_area(p: &[Polyline]) -> f64 {
    // Outer rings positive, holes negative (opposite orientation).
    p.iter().map(|q| q.area()).sum::<f64>().abs()
}

fn square_with_hole() -> ImageData {
    // 40x40 white image, black 20x20 square at (10,10) with a white 6x6 hole.
    let mut g = vec![255u8; 40 * 40];
    for y in 10..30 {
        for x in 10..30 {
            g[y * 40 + x] = 0;
        }
    }
    for y in 17..23 {
        for x in 17..23 {
            g[y * 40 + x] = 255;
        }
    }
    ImageData { name: "t".into(), px_w: 40, px_h: 40, gray: g, w: 40.0, h: 40.0, invert: false }
}

fn gradient_rgb(w: usize, h: usize) -> Vec<u8> {
    (0..w * h).flat_map(|i| [(i % w * 255 / w.max(1)) as u8, (i / w * 255 / h.max(1)) as u8, 128]).collect()
}

#[test]
fn bezier_ellipse_area_and_node_insert() {
    let c = Contour::ellipse(20.0, 10.0);
    let area = c.flatten(0.01).area().abs();
    assert!((area - std::f64::consts::PI * 10.0 * 5.0).abs() < 0.5, "area {area}");
    // Inserting a node must not change the outline.
    let mut c2 = c.clone();
    let before = c2.point_at(1, 0.37);
    let at = c2.insert_node(1, 0.37);
    assert!(c2.nodes[at].p.dist(before) < 1e-9);
    let area2 = c2.flatten(0.01).area().abs();
    assert!((area - area2).abs() < 0.1);
    // Straight segment insert lands on the line.
    let mut r = Contour::rect(10.0, 10.0);
    let at = r.insert_node(0, 0.5);
    assert_eq!(r.nodes[at].p, Pt::new(5.0, 0.0));
}

#[test]
fn boolean_ops_on_squares() {
    let (a, b) = (vec![sq(0.0, 0.0, 4.0)], vec![sq(2.0, 2.0, 4.0)]);
    assert!((total_area(&ops::boolean(&a, &b, ops::BoolOp::Union)) - 28.0).abs() < 1e-6);
    assert!((total_area(&ops::boolean(&a, &b, ops::BoolOp::Intersect)) - 4.0).abs() < 1e-6);
    assert!((total_area(&ops::boolean(&a, &b, ops::BoolOp::Difference)) - 12.0).abs() < 1e-6);
    assert!((total_area(&ops::boolean(&a, &b, ops::BoolOp::Xor)) - 24.0).abs() < 1e-6);
}

#[test]
fn offset_shrinks_and_grows() {
    let a = vec![sq(0.0, 0.0, 10.0)];
    let inner = ops::offset(&a, -1.0);
    assert!((total_area(&inner) - 64.0).abs() < 0.5, "{}", total_area(&inner));
    let outer = ops::offset(&a, 1.0);
    assert!(total_area(&outer) > 120.0);
    let rings = ops::inset_rings(&a, 1.0, 100);
    assert!(rings.len() >= 4 && rings.len() <= 6, "{}", rings.len());
}

#[test]
fn trace_finds_outline_and_hole() {
    let im = square_with_hole();
    let cs = trace::trace(&im, &trace::TraceParams { smooth: false, tolerance: 0.0, ..Default::default() });
    assert_eq!(cs.len(), 2, "outline and hole");
    let areas: Vec<f64> = cs.iter().map(|c| c.flatten(0.01).area().abs()).collect();
    let (big, small) = (areas.iter().cloned().fold(0.0, f64::max), areas.iter().cloned().fold(f64::MAX, f64::min));
    assert!((big - 400.0).abs() < 1.0 && (small - 36.0).abs() < 1.0, "{areas:?}");
    // Speckle filter drops the hole.
    let cs = trace::trace(&im, &trace::TraceParams { min_area: 50.0, ..Default::default() });
    assert_eq!(cs.len(), 1);
    // Tracing the light areas finds the background frame and the hole's island.
    let cs = trace::trace(&im, &trace::TraceParams { invert: true, smooth: false, min_area: 1.0, ..Default::default() });
    assert_eq!(cs.len(), 3);
}

#[test]
fn image_adjustments() {
    let im = square_with_hole();
    let rot = im.adjusted(&Adjust { quarter_turns: 1, ..Default::default() });
    assert_eq!((rot.px_w, rot.px_h), (40, 40));
    let wide = ImageData { px_w: 4, px_h: 2, gray: vec![0, 10, 20, 30, 40, 50, 60, 70], w: 40.0, h: 20.0, ..im.clone() };
    let r = wide.adjusted(&Adjust { quarter_turns: 1, ..Default::default() });
    assert_eq!((r.px_w, r.px_h, r.w, r.h), (2, 4, 20.0, 40.0));
    // Clockwise: the top-left pixel ends up top-right.
    assert_eq!(r.gray[1], 0);
    let f = wide.adjusted(&Adjust { flip_h: true, ..Default::default() });
    assert_eq!(&f.gray[..4], &[30, 20, 10, 0]);
    let b = wide.adjusted(&Adjust { brightness: 0.5, ..Default::default() });
    assert!(b.gray[0] >= 127 && b.gray[7] > wide.gray[7]);
    let c = wide.adjusted(&Adjust { auto_levels: true, ..Default::default() });
    assert_eq!((c.gray[0], c.gray[7]), (0, 255));
    let free = im.adjusted(&Adjust { angle_deg: 45.0, ..Default::default() });
    assert!(free.px_w > 40 && free.px_w < 60);
}

#[test]
fn ser2net_configs() {
    use ser2net::{config, shell_safe, version_from_output, Settings, Version};
    let s = Settings { serial_port: "/dev/ttyUSB0".into(), baud: 115200, tcp_port: 3333, bind: None, kick_old_user: true };
    let y = config(Version::V4, &s);
    assert!(y.contains("accepter: tcp,3333\n") && y.contains("connector: serialdev,/dev/ttyUSB0,115200n81,local") && y.contains("kickolduser: true"));
    let local = Settings { bind: Some("127.0.0.1".into()), kick_old_user: false, ..s.clone() };
    assert!(config(Version::V4, &local).contains("accepter: tcp,127.0.0.1,3333"));
    let old = config(Version::V3, &s);
    assert!(old.contains("3333:raw:0:/dev/ttyUSB0:115200 8DATABITS NONE 1STOPBIT kickolduser"));
    assert!(config(Version::V3, &local).contains("127.0.0.1,3333:raw:0:"));
    assert!(shell_safe("/dev/ttyUSB0") && !shell_safe("a'b"));
    assert_eq!(version_from_output("ser2net version 4.6.1"), Some(Version::V4));
    assert_eq!(version_from_output("ser2net version 3.5.1"), Some(Version::V3));
    assert_eq!(version_from_output("nothing"), None);
    assert_eq!(Version::V4.file(), "/etc/ser2net.yaml");
}

#[test]
fn rounding_corners_of_a_rectangle() {
    let c = Contour::rect(10.0, 10.0);
    let (r, n) = fillet::round_contour(&c, 2.0, None);
    assert_eq!(n, 4);
    // Each corner loses (4 - pi) r^2 / 1 of area: area = 100 - 4 * (1 - pi/4) * r^2.
    let expect = 100.0 - 4.0 * (1.0 - std::f64::consts::FRAC_PI_4) * 4.0;
    let area = r.flatten(0.005).area().abs();
    assert!((area - expect).abs() < 0.02, "{area} vs {expect}");
    // A radius that is too large is limited to half a side: the result is a stadium-like shape, not garbage.
    let (big, n) = fillet::round_contour(&c, 50.0, None);
    assert_eq!(n, 4);
    let b = Polyline::bounds(&big.flatten(0.01)).unwrap();
    assert!((b.width() - 10.0).abs() < 1e-6 && (b.height() - 10.0).abs() < 1e-6);
    assert!(big.flatten(0.01).area().abs() < 100.0 && big.flatten(0.01).area().abs() > 70.0);
    // Only the chosen corner.
    let one: std::collections::HashSet<usize> = [0usize].into_iter().collect();
    let (_, n) = fillet::round_contour(&c, 2.0, Some(&one));
    assert_eq!(n, 1);
    // Nothing to do for a smooth ellipse.
    assert_eq!(fillet::round_contour(&Contour::ellipse(10.0, 6.0), 1.0, None).1, 0);
}

#[test]
fn filleting_two_lines() {
    // A horizontal and a vertical line that cross at (10, 10).
    let h = (Pt::new(0.0, 10.0), Pt::new(14.0, 10.0));
    let v = (Pt::new(10.0, 0.0), Pt::new(10.0, 25.0));
    let c = fillet::fillet_lines(h, v, 3.0).expect("fillet");
    // Kept: the longer parts (0,10) .. and (10,25) .. so the corner is at (10,10) turning towards +y.
    assert_eq!(c.nodes.first().unwrap().p, Pt::new(0.0, 10.0));
    assert_eq!(c.nodes.last().unwrap().p, Pt::new(10.0, 25.0));
    let tangents: Vec<Pt> = c.nodes.iter().map(|n| n.p).collect();
    assert!(tangents.contains(&Pt::new(7.0, 10.0)), "{tangents:?}");
    assert!(tangents.iter().any(|p| (p.x - 10.0).abs() < 1e-9 && (p.y - 13.0).abs() < 1e-9));
    // The arc's middle lies at distance r from its centre (7, 13) and bulges towards the corner.
    let mid = c.point_at(1, 0.5);
    assert!((mid.dist(Pt::new(7.0, 13.0)) - 3.0).abs() < 0.01, "{mid:?}");
    assert!(mid.x > 7.0 && mid.y < 13.0);
    // Parallel lines cannot be joined.
    assert!(fillet::fillet_lines((Pt::new(0.0, 0.0), Pt::new(5.0, 0.0)), (Pt::new(0.0, 3.0), Pt::new(5.0, 3.0)), 1.0).is_none());
    assert!(fillet::as_segment(&[Contour { nodes: vec![Node::corner(Pt::new(0.0, 0.0)), Node::corner(Pt::new(1.0, 1.0))], closed: false }]).is_some());
    assert!(fillet::as_segment(&[Contour::rect(1.0, 1.0)]).is_none());
}

#[test]
fn picture_adjustments_change_nothing_by_default_and_do_what_they_say() {
    let img = gradient_rgb(8, 6);
    let mut same = img.clone();
    camimg::adjust(&mut same, 8, 6, &ImageSettings::default());
    assert_eq!(same, img);
    // Brightness +100 adds 255, -100 subtracts it.
    let mut bright = img.clone();
    camimg::adjust(&mut bright, 8, 6, &ImageSettings { brightness: 20, ..Default::default() });
    assert_eq!(bright[2], 128 + 51);
    let mut dark = img.clone();
    camimg::adjust(&mut dark, 8, 6, &ImageSettings { brightness: -100, ..Default::default() });
    assert!(dark.iter().all(|v| *v == 0));
    // Contrast pushes values away from the middle: mid grey stays, lighter gets lighter, darker darker.
    let mut c = vec![128u8, 128, 128, 200, 200, 200, 60, 60, 60];
    camimg::adjust(&mut c, 3, 1, &ImageSettings { contrast: 100, ..Default::default() });
    assert_eq!(&c[0..3], &[128, 128, 128]);
    assert!(c[3] > 200 && c[6] < 60, "{c:?}");
    // White balance: a warm (2800 K) light is cooled, so blue gains on red; 6500 K is daylight and changes nothing.
    let mut warm = vec![200u8, 150, 100];
    camimg::adjust(&mut warm, 1, 1, &ImageSettings { white_balance: Some(2800), ..Default::default() });
    assert!(warm[0] < 200 && warm[2] >= 100, "{warm:?}");
    let mut day = vec![200u8, 150, 100];
    camimg::adjust(&mut day, 1, 1, &ImageSettings { white_balance: Some(6500), ..Default::default() });
    assert!(day.iter().zip([200u8, 150, 100]).all(|(a, b)| (*a as i32 - b as i32).abs() <= 4), "{day:?}");
    // Denoising smooths a lone bright pixel and leaves flat areas alone.
    let mut dot = vec![0u8; 5 * 5 * 3];
    dot[(2 * 5 + 2) * 3..(2 * 5 + 2) * 3 + 3].copy_from_slice(&[255, 255, 255]);
    camimg::adjust(&mut dot, 5, 5, &ImageSettings { denoise: 0.9, ..Default::default() });
    assert!(dot[(2 * 5 + 2) * 3] < 100 && dot[(2 * 5 + 1) * 3] > 0);
    let mut flat = vec![90u8; 6 * 6 * 3];
    camimg::adjust(&mut flat, 6, 6, &ImageSettings { denoise: 0.95, ..Default::default() });
    assert!(flat.iter().all(|v| *v == 90));
    // Values outside the ranges are clamped; garbage cannot panic.
    let wild = ImageSettings { brightness: 9999, contrast: 9999, white_balance: Some(1), denoise: f32::NAN, prefer_yuyv: false }.clamped();
    assert_eq!((wild.brightness, wild.contrast, wild.white_balance, wild.denoise), (100, 100, Some(2500), 0.0));
    let mut tiny = vec![1u8, 2, 3];
    camimg::adjust(&mut tiny, 1, 1, &ImageSettings { denoise: 0.9, brightness: 5, ..Default::default() });
    camimg::adjust(&mut tiny, 100, 100, &ImageSettings { brightness: 5, ..Default::default() }); // buffer too small: ignored
}

#[test]
fn lens_correction_with_zero_coefficients_is_the_identity_and_barrel_distortion_is_undone() {
    let img = gradient_rgb(40, 30);
    assert_eq!(camimg::undistort(&img, 40, 30, &Lens::default()), img);
    // Insane coefficients are ignored, not applied.
    assert_eq!(camimg::undistort(&img, 40, 30, &Lens { k1: f64::NAN, ..Default::default() }), img);
    assert_eq!(camimg::undistort(&img, 40, 30, &Lens { k1: 99.0, ..Default::default() }), img);
    assert_eq!(camimg::undistort(&[], 0, 0, &Lens { k1: 0.1, ..Default::default() }), Vec::<u8>::new());
    // A stretch of the picture: with k1 > 0 the corner of the result is taken from further out, so it goes black or
    // changes, while the centre keeps its colour.
    let lens = Lens { k1: 0.25, ..Default::default() };
    let out = camimg::undistort(&img, 40, 30, &lens);
    assert_eq!(out.len(), img.len());
    let centre = (15 * 40 + 20) * 3;
    assert!((out[centre] as i32 - img[centre] as i32).abs() <= 8, "the centre stays");
    assert_ne!(&out[0..3], &img[0..3], "the corner moves");
    // The mapping in the middle of a side: undistorting a picture made with the same model brings it back.
    let (w, h) = (60usize, 60usize);
    let f = w as f64;
    let src: Vec<u8> = (0..w * h).flat_map(|i| {
        let (x, y) = ((i % w) as f64, (i / w) as f64);
        // A smooth pattern, so interpolation is accurate.
        let v = (128.0 + 100.0 * ((x * 0.15).sin() * (y * 0.12).cos())) as u8;
        [v, v, v]
    }).collect();
    let l = Lens { k1: 0.05, ..Default::default() };
    let once = camimg::undistort(&src, w, h, &l);
    // A point of the result samples the source at its distorted place: check one point by hand.
    let (x, y) = (45usize, 20usize);
    let (xn, yn) = ((x as f64 + 0.5 - 30.0) / f, (y as f64 + 0.5 - 30.0) / f);
    let r2 = xn * xn + yn * yn;
    let (sx, sy) = (xn * (1.0 + l.k1 * r2) * f + 30.0 - 0.5, yn * (1.0 + l.k1 * r2) * f + 30.0 - 0.5);
    let expect = src[(sy.round() as usize * w + sx.round() as usize) * 3] as i32;
    assert!((once[(y * w + x) * 3] as i32 - expect).abs() <= 12, "{} vs {expect}", once[(y * w + x) * 3]);
}

#[test]
fn homography_from_reference_points_recovers_the_transform() {
    let mm = |u: f64, v: f64| (u * 0.5 + 10.0, v * 0.5 + 20.0); // a scale and a shift
    let pts = |list: &[(f64, f64)]| -> Vec<RefPoint> { list.iter().map(|p| RefPoint { px: *p, mm: mm(p.0, p.1) }).collect() };
    // Three points: an affine map is enough.
    let h3 = camimg::homography(&pts(&[(0.0, 0.0), (640.0, 0.0), (0.0, 480.0)])).unwrap();
    let (x, y) = camimg::apply(&h3, (320.0, 240.0)).unwrap();
    assert!((x - 170.0).abs() < 1e-6 && (y - 140.0).abs() < 1e-6, "{x},{y}");
    // Four points of a perspective view: the map sends them where they should go and recovers the others.
    let persp = |u: f64, v: f64| {
        let w = 1.0 + 0.0008 * u + 0.0005 * v;
        ((0.3 * u + 0.05 * v + 7.0) / w, (0.02 * u + 0.28 * v + 3.0) / w)
    };
    let corners = [(0.0, 0.0), (640.0, 0.0), (640.0, 480.0), (0.0, 480.0)];
    let four: Vec<RefPoint> = corners.iter().map(|p| RefPoint { px: *p, mm: persp(p.0, p.1) }).collect();
    let h = camimg::homography(&four).unwrap();
    for q in [(100.0, 100.0), (320.0, 240.0), (600.0, 50.0)] {
        let (x, y) = camimg::apply(&h, q).unwrap();
        let (ex, ey) = persp(q.0, q.1);
        assert!((x - ex).abs() < 1e-6 && (y - ey).abs() < 1e-6, "{q:?}: {x},{y} vs {ex},{ey}");
    }
    // More than four points are fitted in the least-squares sense (a point that is a little off does not break it).
    let mut five = four.clone();
    five.push(RefPoint { px: (320.0, 240.0), mm: { let p = persp(320.0, 240.0); (p.0 + 0.05, p.1 - 0.05) } });
    let h5 = camimg::homography(&five).unwrap();
    let (x, y) = camimg::apply(&h5, (100.0, 100.0)).unwrap();
    let (ex, ey) = persp(100.0, 100.0);
    assert!((x - ex).abs() < 0.2 && (y - ey).abs() < 0.2);
    // Not enough or degenerate points give nothing.
    assert!(camimg::homography(&pts(&[(0.0, 0.0), (10.0, 0.0)])).is_none());
    assert!(camimg::homography(&pts(&[(0.0, 0.0), (10.0, 10.0), (20.0, 20.0)])).is_none(), "three points on a line");
    assert!(camimg::homography(&[RefPoint { px: (f64::NAN, 0.0), mm: (0.0, 0.0) }; 4]).is_none());
    // The corners of the overlay from the map, and reference points from the corners, are each other's inverse.
    let c = camimg::corners(&h, 640.0, 480.0).unwrap();
    for (p, q) in c.iter().zip(corners.iter().map(|q| persp(q.0, q.1))) {
        assert!(p.dist(Pt::new(q.0, q.1)) < 1e-6);
    }
    let back = camimg::points_from_corners(&c, 640.0, 480.0);
    let h2 = camimg::homography(&back).unwrap();
    let (x, y) = camimg::apply(&h2, (250.0, 90.0)).unwrap();
    let (ex, ey) = persp(250.0, 90.0);
    assert!((x - ex).abs() < 1e-6 && (y - ey).abs() < 1e-6);
}


#[test]
fn xf_inverse_roundtrip() {
    let x = Xf::rotate(0.7).then(Xf::scale(2.0, 3.0)).then(Xf::translate(4.0, -1.0));
    let p = Pt::new(1.5, 2.5);
    let q = x.inverse().unwrap().apply(x.apply(p));
    assert!(p.dist(q) < 1e-9);
}

#[test]
fn crossing_selection_touches_partially_overlapped_shapes() {
    let area = Rect { min: Pt::new(0.0, 0.0), max: Pt::new(10.0, 10.0) };
    let rect_at = |x: f64, y: f64, s: f64| Shape { id: 1, layer: 0, kind: Kind::Rect { w: s, h: s }, xf: Xf::translate(x, y), group: None, locked: false };
    assert!(rect_at(5.0, 5.0, 20.0).intersects_rect(&area), "partial overlap");
    assert!(rect_at(2.0, 2.0, 3.0).intersects_rect(&area), "fully inside");
    assert!(rect_at(-50.0, -50.0, 200.0).intersects_rect(&area), "area fully inside a big shape");
    assert!(!rect_at(11.0, 11.0, 5.0).intersects_rect(&area), "outside");
    // A diagonal line whose bounding box overlaps the area but which misses it.
    let line = Shape { id: 2, layer: 0, kind: Kind::Path(vec![Polyline::new(vec![Pt::new(-1.0, 8.0), Pt::new(8.0, 20.0)], false)]), xf: Xf::IDENTITY, group: None, locked: false };
    assert!(!line.intersects_rect(&Rect { min: Pt::new(0.0, 0.0), max: Pt::new(3.0, 3.0) }));
    assert!(line.intersects_rect(&area));
    // A line crossing straight through the area with no vertex inside it.
    let through = Shape { id: 3, layer: 0, kind: Kind::Path(vec![Polyline::new(vec![Pt::new(-5.0, 5.0), Pt::new(15.0, 5.0)], false)]), xf: Xf::IDENTITY, group: None, locked: false };
    assert!(through.intersects_rect(&area));
}

#[test]
fn user_clipart_library_roundtrip() {
    let dir = temp_dir("lib");
    let mut lib = clipart::Library::open(&dir);
    assert!(lib.items.is_empty());
    let a = lib.add(CAT_SVG, "Big Cat!", "Animals", "https://example.org/cat.svg", "CC0", "Someone").unwrap();
    let b = lib.add(&CAT_SVG.replace("<path", "<path fill=\"currentColor\""), "Big Cat!", "", "file", "", "").unwrap();
    assert_eq!(a.id, "big-cat");
    assert_eq!(b.id, "big-cat-2", "same name gets a different file");
    assert_eq!(b.category, clipart::DEFAULT_CATEGORY);
    assert!(lib.svg("big-cat-2").unwrap().contains("#000000"), "currentColor is made explicit");
    lib.set_category("big-cat", "Pets").unwrap();
    lib.rename("big-cat", "Kitty").unwrap();
    // Reopen: everything is remembered.
    let lib2 = clipart::Library::open(&dir);
    assert_eq!(lib2.items.len(), 2);
    let k = lib2.items.iter().find(|c| c.id == "big-cat").unwrap();
    assert_eq!((k.name.as_str(), k.category.as_str(), k.license.as_str()), ("Kitty", "Pets", "CC0"));
    assert_eq!(lib2.categories(), vec!["My clipart".to_string(), "Pets".to_string()]);
    // Removing deletes the file and the entry; an index entry whose file vanished is dropped on open.
    let mut lib3 = clipart::Library::open(&dir);
    lib3.remove("big-cat").unwrap();
    assert!(!dir.join("big-cat.svg").exists());
    std::fs::remove_file(dir.join("big-cat-2.svg")).unwrap();
    assert!(clipart::Library::open(&dir).items.is_empty());
    // Bad input is refused.
    assert!(lib3.add("<html></html>", "x", "", "", "", "").is_err());
    assert!(lib3.add("<svg", "x", "", "", "", "").is_err());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn iconify_search_answers_are_read() {
    let json = r#"{"icons":["mdi:cat","tabler:cat","unknown:thing"],"total":3,"limit":3,"start":0,"collections":{
        "mdi":{"name":"Material Design Icons","author":{"name":"Pictogrammers","url":"https://x"},"license":{"title":"Apache 2.0","spdx":"Apache-2.0","url":"https://l"}},
        "tabler":{"name":"Tabler Icons","author":{"name":"Paweł Kuna"},"license":{"title":"MIT","spdx":"MIT"}}}}"#;
    let hits = clipart::iconify::parse_search(json).unwrap();
    assert_eq!(hits.len(), 3);
    assert_eq!(hits[0].key(), "mdi:cat");
    assert_eq!((hits[0].set_name.as_str(), hits[0].license_spdx.as_str(), hits[0].author.as_str()), ("Material Design Icons", "Apache-2.0", "Pictogrammers"));
    assert!(hits[0].needs_no_credit() && hits[1].needs_no_credit());
    assert!(hits[2].license_title.is_empty() && !hits[2].needs_no_credit());
    assert_eq!(hits[0].svg_url("https://api.iconify.design"), "https://api.iconify.design/mdi/cat.svg");
    assert_eq!(clipart::iconify::search_url("https://a", "paper plane", 20), "https://a/search?query=paper%20plane&limit=20");
    assert!(clipart::iconify::parse_search("nonsense").is_err());
    let by = clipart::iconify::Hit { prefix: "x".into(), name: "y".into(), set_name: String::new(), license_title: "CC BY 4.0".into(), license_spdx: "CC-BY-4.0".into(), license_url: String::new(), author: String::new() };
    assert!(!by.needs_no_credit(), "attribution licences are flagged");
}

fn temp_dir(tag: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("lc-test-{tag}-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

const CAT_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M2,2L22,2L22,12L2,12Z M5,15L19,15L19,22L5,22Z"/></svg>"##;

fn tiny_pdf(content: &str) -> Vec<u8> {
    use lopdf::{dictionary, Document as Pdf, Object, Stream};
    let mut pdf = Pdf::with_version("1.5");
    let pages_id = pdf.new_object_id();
    let content_id = pdf.add_object(Stream::new(dictionary! {}, content.as_bytes().to_vec()));
    let page_id = pdf.add_object(dictionary! {
        "Type" => "Page",
        "Parent" => pages_id,
        "MediaBox" => vec![0.into(), 0.into(), 200.into(), 100.into()],
        "Contents" => content_id,
    });
    pdf.objects.insert(pages_id, Object::Dictionary(dictionary! {
        "Type" => "Pages", "Kids" => vec![page_id.into()], "Count" => 1,
    }));
    let catalog = pdf.add_object(dictionary! { "Type" => "Catalog", "Pages" => pages_id });
    pdf.trailer.set("Root", catalog);
    let mut out = Vec::new();
    pdf.save_to(&mut out).unwrap();
    out
}

#[test]
fn text_produces_outlines() {
    let t = TextData { text: "Hi".into(), size: 10.0, ..TextData::default() };
    let c = text::contours(&t);
    assert!(c.len() >= 3, "{}", c.len());
    let s = Shape { id: 1, layer: 0, kind: Kind::Text(t), xf: Xf::IDENTITY, group: None, locked: false };
    let b = s.bounds().unwrap();
    assert!(b.width() > 5.0 && b.height() > 4.0 && b.height() < 12.0, "{:?}", b);
}

#[test]
fn svg_is_read_into_outlines_with_their_colours() {
    let svg = br##"<svg xmlns="http://www.w3.org/2000/svg" width="100" height="100"><rect x="10" y="10" width="40" height="20" stroke="#ff0000" fill="none"/></svg>"##;
    let out = svg::parse(svg).unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].color, [255, 0, 0]);
    let b = Polyline::bounds(&out[0].contours[0].flatten(0.01)).unwrap();
    assert!((b.width() - 40.0 * 25.4 / 96.0).abs() < 1e-3);
    assert!(svg::parse(b"nonsense").is_err());
}

#[test]
fn illustrator_pdf_paths_are_read_into_outlines() {
    // Red stroked rectangle (x 10..110, y 20..60 in PDF space, y up) and a blue curve.
    let data = tiny_pdf("1 0 0 RG 10 20 100 40 re S 0 0 1 RG 0 0 m 30 50 70 50 100 0 c S");
    let out = pdf::parse(&data).unwrap();
    assert_eq!(out.len(), 2);
    assert_eq!(out[0].color, [255, 0, 0]);
    let b = Polyline::bounds(&out[0].contours[0].flatten(0.01)).unwrap();
    assert!((b.width() - 100.0 * 25.4 / 72.0).abs() < 1e-6);
    assert!((b.min.y - (100.0 - 60.0) * 25.4 / 72.0).abs() < 1e-6);
    assert_eq!(out[1].color, [0, 0, 255]);
    assert!(out[1].contours[0].nodes.len() == 2 && out[1].contours[0].seg_is_curve(0));
    assert!(pdf::parse(b"not a pdf").is_err());
}

#[test]
fn a_scene_hands_out_ids_and_groups_and_keeps_locked_shapes_from_editing() {
    let mut s = Scene::default();
    let a = s.add(0, Kind::Rect { w: 1.0, h: 1.0 }, Xf::IDENTITY);
    let b = s.add(1, Kind::Rect { w: 1.0, h: 1.0 }, Xf::IDENTITY);
    assert_eq!((a, b), (1, 2));
    let g = s.new_group_id();
    s.shape_mut(a).unwrap().group = Some(g);
    s.shape_mut(b).unwrap().group = Some(g);
    assert_eq!(s.group_of(a), vec![a, b]);
    assert_eq!(s.new_group_id(), g + 1);
    s.shape_mut(a).unwrap().locked = true;
    assert!(s.unlocked_mut(a).is_none() && s.unlocked_mut(b).is_some());
    assert_eq!(s.unlocked_ids(&[a, b, 999]), vec![b]);
    let bounds = s.bounds_of(&[a, b]).unwrap();
    assert!((bounds.width() - 1.0).abs() < 1e-9);
    // Saved counters come back as they were; a group counter of 0 (old files) still gives a valid id.
    let (n, gr) = s.counters();
    let mut again = Scene::from_parts(s.shapes.clone(), n, gr);
    assert_eq!(again.add(0, Kind::Rect { w: 1.0, h: 1.0 }, Xf::IDENTITY), 3);
    assert_eq!(Scene::from_parts(vec![], 1, 0).new_group_id(), 1);
}

#[test]
fn programs_share_one_gallery_and_old_data_moves_in_once() {
    use crate::{clipart::Library, paths};
    let svg = |c: &str| format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'><rect width='10' height='10' fill='{c}'/></svg>");
    let root = std::env::temp_dir().join(format!("bc-paths-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let base = root.join("base");
    // Two old program folders, each with settings and a gallery; both hold a picture called "box".
    let old_a = root.join("a");
    let old_b = root.join("b");
    std::fs::create_dir_all(&old_a).unwrap();
    std::fs::create_dir_all(&old_b).unwrap();
    std::fs::write(old_a.join("app.ron"), "A").unwrap();
    std::fs::write(old_b.join("app.ron"), "B").unwrap();
    let mut la = Library::open(old_a.join("clipart"));
    la.add(&svg("red"), "box", "Mine", "file", "", "").unwrap();
    la.add(&svg("blue"), "same", "Mine", "file", "", "").unwrap();
    let mut lb = Library::open(old_b.join("clipart"));
    lb.add(&svg("green"), "box", "Mine", "file", "", "").unwrap();
    lb.add(&svg("blue"), "same", "Mine", "file", "", "").unwrap();

    let pa = paths::prepare_in(&base, "lightcreator", Some(&old_a));
    let pb = paths::prepare_in(&base, "millcreator", Some(&old_b));
    assert_eq!(std::fs::read_to_string(&pa.settings).unwrap(), "A");
    assert_eq!(std::fs::read_to_string(&pb.settings).unwrap(), "B");
    assert_eq!(pa.gallery, pb.gallery);
    let g = Library::open(&pa.gallery);
    // box (red), box (green, gets its own id), same (once: identical picture)
    assert_eq!(g.items.len(), 3, "{:?}", g.items);
    let ids: std::collections::HashSet<_> = g.items.iter().map(|c| c.id.clone()).collect();
    assert_eq!(ids.len(), 3);

    // Running it again changes nothing, even when the old folder gained a picture.
    la.add(&svg("black"), "late", "Mine", "file", "", "").unwrap();
    paths::prepare_in(&base, "lightcreator", Some(&old_a));
    assert_eq!(Library::open(&pa.gallery).items.len(), 3);
    // Existing new settings are not overwritten by old ones.
    std::fs::write(&pa.settings, "NEW").unwrap();
    std::fs::remove_file(base.join("apps/lightcreator/.migrated")).unwrap();
    paths::prepare_in(&base, "lightcreator", Some(&old_a));
    assert_eq!(std::fs::read_to_string(&pa.settings).unwrap(), "NEW");

    // Two program instances changing the gallery do not lose each other's pictures.
    let mut x = Library::open(&pa.gallery);
    let mut y = Library::open(&pa.gallery);
    x.add(&svg("pink"), "from x", "Mine", "file", "", "").unwrap();
    y.add(&svg("gray"), "from y", "Mine", "file", "", "").unwrap();
    assert!(Library::open(&pa.gallery).items.iter().any(|c| c.name == "from x"));
    assert!(Library::open(&pa.gallery).items.iter().any(|c| c.name == "from y"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn backup_restores_settings_and_adds_pictures_to_the_shared_gallery() {
    use crate::{backup, clipart::Library};
    let svg = |c: &str| format!("<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 10 10'><rect width='10' height='10' fill='{c}'/></svg>");
    let root = std::env::temp_dir().join(format!("bc-backup-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    let gallery = root.join("clipart");
    let mut lib = Library::open(&gallery);
    lib.add(&svg("red"), "old one", "Mine", "file", "", "").unwrap();
    let zip = root.join("b.zip");
    let n = backup::write(&zip, "LightCreator", "v1", &[("lang", "pl".into()), ("devices", "x\ny".into())], &gallery).unwrap();
    assert_eq!(n, 4, "settings, index, one picture, README");
    // Another program adds a picture afterwards; the restore must not lose it.
    let mut other = Library::open(&gallery);
    other.add(&svg("blue"), "newer", "Mine", "file", "", "").unwrap();
    other.remove(&other.items[0].id.clone()).unwrap();
    let r = backup::restore(&zip, "LightCreator", &gallery).unwrap();
    assert_eq!(r.settings.get("lang").map(String::as_str), Some("pl"));
    assert_eq!(r.settings.get("devices").map(String::as_str), Some("x\ny"));
    assert!(!r.settings.contains_key(backup::PROGRAM_KEY));
    assert_eq!(r.pictures, 1);
    let names: Vec<_> = Library::open(&gallery).items.iter().map(|c| c.name.clone()).collect();
    assert!(names.contains(&"old one".to_string()) && names.contains(&"newer".to_string()), "{names:?}");
    // Restoring again adds nothing.
    assert_eq!(backup::restore(&zip, "LightCreator", &gallery).unwrap().pictures, 0);
    // Another program's backup is refused, and a missing file is an error.
    assert!(backup::restore(&zip, "MillCreator", &gallery).unwrap_err().contains("lightcreator"));
    assert!(backup::restore(&root.join("none.zip"), "LightCreator", &gallery).is_err());
    assert_eq!(backup::file_name("pencreator", 1_791_676_800), "pencreator-backup-20261011.zip");
    let _ = std::fs::remove_dir_all(&root);
}
