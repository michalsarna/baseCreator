//! LightBurn projects made by LightBurn 2.1.04 (`tests/fixtures/lightburn`), read with the work area 400 mm high.
//! The expected numbers are what LightBurn itself showed for each project (its X/Y position, width and height fields
//! and the picture of the work area), turned over: scene Y = 400 - LightBurn Y.
use base_core::geom::{Pt, Rect};
use base_core::lightburn::{import, CutKind, Import};
use base_core::scene::Kind;

const H: f64 = 400.0;

fn load(name: &str) -> Import {
    let path = format!("{}/tests/fixtures/lightburn/{name}.lbrn2", env!("CARGO_MANIFEST_DIR"));
    import(&std::fs::read_to_string(path).unwrap(), H).unwrap()
}

fn close(a: f64, b: f64, tol: f64, what: &str) {
    assert!((a - b).abs() <= tol, "{what}: {a} is not {b}");
}

/// The bounds of shape `i` as LightBurn shows them: X and Y of the left and the bottom, Y counting up.
fn lb_bounds(im: &Import, i: usize) -> (f64, f64, f64, f64) {
    let b: Rect = im.scene.shapes[i].bounds().unwrap();
    (b.min.x, H - b.max.y, b.max.x, H - b.min.y)
}

#[test]
fn a_rectangle_keeps_its_place_and_size() {
    let im = load("kwadrat");
    assert_eq!(im.scene.shapes.len(), 1);
    assert!(matches!(im.scene.shapes[0].kind, Kind::Rect { w, h } if w == 80.0 && h == 74.0));
    let (x0, y0, x1, y1) = lb_bounds(&im, 0);
    // LightBurn: XPos 63, YPos 94 (the centre), width 80, height 74.
    for (got, want, what) in [(x0, 23.0, "left"), (y0, 57.0, "bottom"), (x1, 103.0, "right"), (y1, 131.0, "top")] {
        close(got, want, 1e-9, what);
    }
    assert_eq!(im.device_name, "No Machine");
    assert!(im.warnings.is_empty(), "{:?}", im.warnings);
}

#[test]
fn several_shapes_come_out_in_file_order() {
    let im = load("kwadraty3");
    assert_eq!(im.scene.shapes.len(), 3);
    let (x0, _, x1, _) = lb_bounds(&im, 2);
    close(x1 - x0, 56.0, 1e-9, "third width");
    let im = load("elipsy4");
    assert_eq!(im.scene.shapes.len(), 4);
    assert!(im.scene.shapes.iter().all(|s| matches!(s.kind, Kind::Ellipse { .. })));
}

#[test]
fn an_ellipse_is_centred_on_its_position() {
    let im = load("elipsa");
    let (x0, y0, x1, y1) = lb_bounds(&im, 0);
    for (got, want) in [(x0, 39.0), (y0, 161.0), (x1, 97.0), (y1, 251.0)] {
        close(got, want, 0.05, "ellipse");
    }
}

#[test]
fn a_triangle_points_up_as_it_does_in_lightburn() {
    let im = load("trojkat");
    let (x0, y0, x1, y1) = lb_bounds(&im, 0);
    // Centre (101.5, 203.5), Rx 49.5, Ry 46.5: the tip 46.5 above the centre, the base 23.25 below it.
    close(y1, 250.0, 1e-6, "tip");
    close(y0, 203.5 - 23.25, 1e-6, "base");
    close(x1 - x0, 2.0 * 49.5 * (std::f64::consts::TAU / 3.0).sin().abs(), 1e-6, "width");
}

#[test]
fn turned_and_mirrored_triangles_land_where_lightburn_drew_them() {
    let im = load("trojkaty3");
    assert_eq!(im.scene.shapes.len(), 3);
    // The picture: the third one (the last drawn) has its centre at (134.7, 233.3) with a 58.6 x 58.1 box.
    let (x0, y0, x1, y1) = lb_bounds(&im, 2);
    close((x0 + x1) / 2.0, 133.227, 0.05, "x centre");
    close((y0 + y1) / 2.0, 239.899, 0.05, "y centre");
    close(x1 - x0, 58.640, 0.05, "width");
    close(y1 - y0, 58.106, 0.05, "height");
}

#[test]
fn a_star_has_five_tips_and_the_inner_ratio() {
    let im = load("gwiazda");
    let Kind::Bezier(cs) = &im.scene.shapes[0].kind else { panic!("a star becomes contours") };
    assert_eq!((cs.len(), cs[0].nodes.len(), cs[0].closed), (1, 10, true));
    let (_, y0, _, y1) = lb_bounds(&im, 0);
    close(y1, 240.5 + 59.5, 1e-6, "the tip points up");
    close(y1 - y0, 107.637, 0.01, "LightBurn shows the height 107.637");
    // The inner corner after the first tip (36 degrees round): LightBurn's picture has it near (128.8, 259.9).
    let b = &cs[0].nodes[1];
    let p = Pt::new(b.p.x, H - b.p.y);
    close(p.x, 128.3, 0.1, "inner x");
    close(p.y, 259.8, 0.1, "inner y");
}

#[test]
fn a_path_with_curves_and_straight_segments_is_read() {
    let im = load("linie");
    assert_eq!(im.scene.shapes.len(), 2);
    let Kind::Bezier(a) = &im.scene.shapes[0].kind else { panic!() };
    assert_eq!((a.len(), a[0].nodes.len(), a[0].closed), (1, 2, false));
    // The curve sags below its ends: LightBurn's picture shows the lowest point near Y 224.
    let b = im.scene.shapes[0].bounds().unwrap();
    close(H - b.max.y, 224.0, 3.0, "the lowest point of the curve");
    // The second path leaves out its segment list and uses the first one's.
    let Kind::Bezier(c) = &im.scene.shapes[1].kind else { panic!() };
    assert_eq!(c[0].nodes.len(), 2);
}

#[test]
fn a_rounded_path_is_one_closed_contour_with_the_right_extent() {
    let im = load("zaokraglone-rogi-5-10-15");
    let Kind::Bezier(cs) = &im.scene.shapes[0].kind else { panic!() };
    assert_eq!((cs.len(), cs[0].closed), (1, true));
    let (x0, y0, x1, y1) = lb_bounds(&im, 0);
    for (got, want) in [(x0, 130.0 - 57.0), (y0, 248.5 - 49.5), (x1, 130.0 + 57.0), (y1, 248.5 + 49.5)] {
        close(got, want, 0.05, "extent");
    }
}

#[test]
fn text_comes_as_outlines_where_it_was() {
    let im = load("text");
    assert!(im.scene.shapes.len() >= 2, "{}", im.scene.shapes.len());
    let Kind::Bezier(cs) = &im.scene.shapes[0].kind else { panic!("the text is outlines") };
    assert!(cs.len() >= 8, "one contour per letter part: {}", cs.len());
    let (x0, y0, x1, y1) = lb_bounds(&im, 0);
    // "Arial 25" is 25 mm high text: a line some 18 mm tall, left of its position's centre.
    assert!(y1 - y0 > 15.0 && y1 - y0 < 30.0, "{}", y1 - y0);
    assert!(x0 > 60.0 && x1 < 200.0 && y0 > 240.0 && y1 < 300.0, "{x0} {y0} {x1} {y1}");
}

#[test]
fn the_layers_come_with_their_modes_speeds_and_powers() {
    let im = load("layers");
    let kinds: Vec<CutKind> = im.cuts.iter().map(|c| c.kind).collect();
    assert_eq!(kinds, [CutKind::Line, CutKind::Fill, CutKind::Offset, CutKind::Fill, CutKind::Line, CutKind::Offset, CutKind::Line]);
    let powers: Vec<f64> = im.cuts.iter().map(|c| c.power_max).collect();
    assert_eq!(powers, [100.0, 50.0, 20.0, 50.0, 75.0, 100.0, 30.0]);
    close(im.cuts[0].speed_mm_s, 8.33333, 1e-9, "speed of C00 (500 mm/min)");
    close(im.cuts[1].speed_mm_s, 83.3333, 1e-9, "speed of C01");
    assert_eq!(im.cuts[3].name, "C03");
    // Each shape is on the layer the file names.
    let layers: Vec<usize> = im.scene.shapes.iter().map(|s| s.layer).collect();
    assert_eq!(layers, [0, 1, 2, 6, 5, 4, 3]);
}

/// Where the top left corner of the picture (its first pixel row) lands, in LightBurn's coordinates.
fn picture(name: &str) -> (Pt, Pt, (f64, f64)) {
    let im = load(name);
    let s = &im.scene.shapes[0];
    let Kind::Image(img) = &s.kind else { panic!("a bitmap") };
    let top_left = s.xf.apply(Pt::new(0.0, 0.0));
    let top_right = s.xf.apply(Pt::new(img.w, 0.0));
    let flip = |p: Pt| Pt::new(p.x, H - p.y);
    (flip(top_left), flip(top_right), (img.w, img.h))
}

#[test]
fn a_bitmap_has_its_size_in_millimetres_and_stands_upright() {
    let (tl, tr, (w, h)) = picture("bitmap-normal");
    close(w, 40.0, 1e-9, "width");
    close(h, 60.0, 1e-9, "height");
    // Centre (120, 190): the first row is at the top, the left edge at the left.
    close(tl.x, 100.0, 1e-9, "left");
    close(tl.y, 220.0, 1e-9, "top");
    close(tr.x, 140.0, 1e-9, "right");
}

#[test]
fn mirrored_bitmaps_are_mirrored() {
    let (tl, tr, _) = picture("bitmap-mirror-horizontal");
    close(tl.x, 140.0, 1e-9, "the first column is now at the right");
    close(tr.x, 100.0, 1e-9, "and the last at the left");
    close(tl.y, 220.0, 1e-9, "the top stays");
    let (tl, _, _) = picture("bitmap-mirror-vertical");
    close(tl.y, 160.0, 1e-9, "the first row is now at the bottom");
    close(tl.x, 100.0, 1e-9, "the left stays");
}

#[test]
fn a_turned_bitmap_keeps_its_size() {
    let (tl, tr, (w, h)) = picture("bitmap-rotated");
    close(w, 40.0, 1e-9, "width");
    close(h, 60.0, 1e-9, "height");
    // Turned a quarter turn: the first row runs along the Y axis.
    close((tr.x - tl.x).abs(), 0.0, 1e-9, "the top edge is now vertical");
    close((tr.y - tl.y).abs(), 40.0, 1e-9, "and as long as the width");
}

#[test]
fn broken_files_are_errors_and_not_panics() {
    for text in ["", "not xml", "<a/>", "<LightBurnProject><Shape Type=\"Path\" CutIndex=\"0\"><VertList>V1</VertList></Shape></LightBurnProject>"] {
        let r = import(text, H);
        if let Ok(im) = r {
            assert!(im.scene.shapes.is_empty(), "{text}");
        }
    }
    // Nothing numeric is trusted: missing sizes and absurd counts are left out with a warning.
    let im = import("<LightBurnProject><Shape Type=\"Rect\" CutIndex=\"0\"><XForm>1 0 0 1 0 0</XForm></Shape><Shape Type=\"Polygon\" CutIndex=\"0\" Rx=\"5\" Ry=\"5\" N=\"99999999\"><XForm>1 0 0 1 0 0</XForm></Shape><Shape Type=\"Mystery\" CutIndex=\"0\"/></LightBurnProject>", H).unwrap();
    assert!(im.scene.shapes.is_empty());
    assert!(im.warnings.iter().any(|w| w.contains("Mystery")), "{:?}", im.warnings);
}

#[test]
fn groups_keep_their_children_together() {
    let text = r#"<LightBurnProject><Shape Type="Group" CutIndex="0"><XForm>1 0 0 1 10 0</XForm><Children>
        <Shape Type="Rect" CutIndex="0" W="4" H="2" Cr="0"><XForm>1 0 0 1 2 1</XForm></Shape>
        <Shape Type="Ellipse" CutIndex="1" Rx="1" Ry="1"><XForm>1 0 0 1 8 1</XForm></Shape></Children></Shape></LightBurnProject>"#;
    let im = import(text, H).unwrap();
    assert_eq!(im.scene.shapes.len(), 2);
    assert!(im.scene.shapes[0].group.is_some() && im.scene.shapes[0].group == im.scene.shapes[1].group);
    let (x0, _, _, _) = lb_bounds(&im, 0);
    close(x0, 10.0, 1e-9, "the group's move applies to the child");
}
