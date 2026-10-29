//! DXF files as programs write them: R12 (POLYLINE with VERTEX and a bulge), R2000 (LWPOLYLINE, ELLIPSE, SPLINE, blocks, a layer
//! table, inches) and a drawing of separate lines. The files in `tests/dxf` are small samples written to the format description in
//! the style of each; real files of more programs are welcome there.
use base_core::dxf::parse;
use base_core::geom::{Pt, Rect};

const R12: &str = include_str!("dxf/r12_polyline.dxf");
const R2000: &str = include_str!("dxf/lwpolyline_r2000.dxf");
const LINES: &str = include_str!("dxf/lines_only.dxf");

/// The box of all the outlines, flattened.
fn bounds(d: &base_core::dxf::Dxf) -> Rect {
    let mut r: Option<Rect> = None;
    for o in &d.outlines {
        for c in &o.contours {
            let b = c.flatten(0.01).bounds();
            r = match (r, b) {
                (Some(r), Some(b)) => Some(r.union(b)),
                (None, b) => b,
                (r, None) => r,
            };
        }
    }
    r.expect("some outlines")
}

fn layers(d: &base_core::dxf::Dxf) -> Vec<String> {
    let mut v: Vec<String> = d.outlines.iter().filter_map(|o| o.layer.clone()).collect();
    v.sort();
    v.dedup();
    v
}

#[test]
fn an_r12_file_with_a_bulge_an_arc_and_a_circle_is_read_in_millimetres_on_its_layers() {
    let d = parse(R12.as_bytes(), 100.0).unwrap();
    assert_eq!(layers(&d), ["CUT", "DRILL", "ENGRAVE"]);
    // The polyline: a 40 × 20 rectangle whose right side is a half circle bulging out (radius 10): 0 … 50 wide; Y is turned over in a work area 100 high.
    let cut = d.outlines.iter().find(|o| o.layer.as_deref() == Some("CUT")).unwrap();
    assert!(cut.contours[0].closed);
    let b = cut.contours[0].flatten(0.005).bounds().unwrap();
    assert!((b.min.x - 0.0).abs() < 0.02 && (b.max.x - 50.0).abs() < 0.02, "{b:?}");
    assert!((b.min.y - 80.0).abs() < 0.02 && (b.max.y - 100.0).abs() < 0.02, "{b:?}");
    // The circle is a closed curve of 6 mm across.
    let drill = d.outlines.iter().find(|o| o.layer.as_deref() == Some("DRILL")).unwrap();
    let bb = drill.contours[0].flatten(0.005).bounds().unwrap();
    assert!(((bb.max.x - bb.min.x) - 6.0).abs() < 0.01 && ((bb.max.y - bb.min.y) - 6.0).abs() < 0.01);
    // The line and the arc (quarter, radius 8, from (28, 10) up to (20, 18)) are on one layer; the arc starts where the line ends? They only
    // meet if the ends are the same: here they are not, so there are two paths.
    assert_eq!(d.outlines.iter().filter(|o| o.layer.as_deref() == Some("ENGRAVE")).count(), 2);
    assert!(d.warnings.iter().any(|w| w.contains("TEXT")), "{:?}", d.warnings);
}

#[test]
fn an_r2000_file_reads_lwpolylines_an_ellipse_a_spline_blocks_with_a_scale_and_inches() {
    let d = parse(R2000.as_bytes(), 0.0).unwrap();
    assert_eq!(layers(&d), ["HOLES", "PARTS"]);
    // Inches: the LWPOLYLINE 1 … 3 × 1 … 2 inch is 50.8 × 25.4 mm, closed.
    let parts: Vec<_> = d.outlines.iter().filter(|o| o.layer.as_deref() == Some("PARTS")).collect();
    assert_eq!(parts.len(), 3, "the rectangle, the ellipse and the spline");
    let rect = parts.iter().find(|o| o.contours[0].nodes.len() == 4 && o.contours[0].closed).expect("the rectangle");
    let b = rect.contours[0].flatten(0.01).bounds().unwrap();
    assert!(((b.max.x - b.min.x) - 50.8).abs() < 1e-6 && ((b.max.y - b.min.y) - 25.4).abs() < 1e-6, "{b:?}");
    // The ellipse: semi-axes 1 and 0.5 inch: 50.8 × 25.4 mm, closed.
    let ell = parts.iter().find(|o| o.contours[0].nodes.len() == 4 && o.contours[0].nodes.iter().any(|n| n.has_out())).expect("the ellipse");
    let eb = ell.contours[0].flatten(0.01).bounds().unwrap();
    assert!(((eb.max.x - eb.min.x) - 50.8).abs() < 0.05 && ((eb.max.y - eb.min.y) - 25.4).abs() < 0.05, "{eb:?}");
    // The spline (a cubic Bézier with those control points) stays inside the box of its control points.
    let sp = parts.iter().find(|o| !o.contours[0].closed && o.contours[0].nodes.len() > 4).expect("the spline");
    let sb = sp.contours[0].flatten(0.01).bounds().unwrap();
    assert!(sb.min.x >= -1e-6 && sb.max.x <= 3.0 * 25.4 + 1e-6, "{sb:?}");
    // Two inserts of the block: circles of radius 0.25 and 0.5 inch.
    let holes: Vec<_> = d.outlines.iter().filter(|o| o.layer.as_deref() == Some("HOLES")).collect();
    assert_eq!(holes.len(), 2);
    let mut widths: Vec<f64> = holes.iter().map(|o| { let h = o.contours[0].flatten(0.01).bounds().unwrap(); h.max.x - h.min.x }).collect();
    widths.sort_by(|a, b| a.total_cmp(b));
    assert!((widths[0] - 12.7).abs() < 0.02 && (widths[1] - 25.4).abs() < 0.02, "{widths:?}");
    // The layer of an insert is the one the block's entities (on layer 0) take.
    assert!(d.warnings.is_empty(), "{:?}", d.warnings);
}

#[test]
fn separate_lines_are_joined_into_a_closed_path_across_a_tiny_gap_and_what_is_not_a_line_is_named() {
    let d = parse(LINES.as_bytes(), 60.0).unwrap();
    assert_eq!(d.outlines.len(), 1, "four lines, one path");
    let c = &d.outlines[0].contours[0];
    assert!(c.closed && c.nodes.len() == 4, "{c:?}");
    let b = bounds(&d);
    assert!((b.max.x - b.min.x - 100.0).abs() < 0.01 && (b.max.y - b.min.y - 60.0).abs() < 0.01);
    // No unit in the file: millimetres, and it says so; the hatch and the dimension are named.
    assert!(d.warnings.iter().any(|w| w.contains("millimetres")), "{:?}", d.warnings);
    assert!(d.warnings.iter().any(|w| w.contains("HATCH") && w.contains("DIMENSION")), "{:?}", d.warnings);
    // Y is turned over: the line along the bottom (Y = 0) is at the bottom of a work area 60 high.
    assert!(b.max.y <= 60.0 + 0.01 && b.min.y >= -0.01);
    assert!(c.nodes.iter().any(|n| n.p == Pt::new(0.0, 60.0)));
}

#[test]
fn things_that_are_not_ascii_dxf_are_refused_with_a_reason() {
    assert!(parse(b"AutoCAD Binary DXF\r\n\x1a\0", 0.0).err().unwrap().contains("binary"));
    assert!(parse(b"hello, this is text", 0.0).err().unwrap().contains("not a DXF"));
    assert!(parse(b"0\nSECTION\n2\nENTITIES\n0\nENDSEC\n0\nEOF\n", 0.0).unwrap().warnings.iter().any(|w| w.contains("no lines")));
}
