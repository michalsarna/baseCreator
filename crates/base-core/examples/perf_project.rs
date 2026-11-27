//! Makes the performance test project of the programs (`testdata/perf/perf-block.cre`): a block of 300 × 400 × 500 mm with on each
//! of its six faces 10 texts and 100 objects (squares, circles, hearts, stars, polylines and a few dwarfs of a real project) on ten
//! layers, and 5000 objects beside the work areas (some of them big dwarfs), which no preview may show.
//!
//! `cargo run --release -p base-core --example perf_project -- <out.cre> <dwarfs.cre>`: the dwarfs are taken from a project of
//! LightCreator (the curves of its biggest layer). Always the same file for the same input (a fixed seed).
use base_core::faces::{Faces, Solid};
use base_core::geom::{Polyline, Pt, Xf};
use base_core::project::Project;
use base_core::scene::{Kind, Scene, Shape};
use base_core::text::TextData;

/// A small fixed generator: the same numbers every time.
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        ((self.0 >> 11) as f64) / ((1u64 << 53) as f64)
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

fn heart(s: f64) -> Kind {
    let pts = (0..96)
        .map(|i| {
            let t = i as f64 / 96.0 * std::f64::consts::TAU;
            let x = 16.0 * t.sin().powi(3);
            let y = 13.0 * t.cos() - 5.0 * (2.0 * t).cos() - 2.0 * (3.0 * t).cos() - (4.0 * t).cos();
            Pt::new((x + 17.0) * s / 34.0, (17.0 - y) * s / 34.0)
        })
        .collect();
    Kind::Path(vec![Polyline::new(pts, true)])
}

fn star(s: f64, n: usize) -> Kind {
    let pts = (0..2 * n)
        .map(|i| {
            let a = i as f64 / (2 * n) as f64 * std::f64::consts::TAU;
            let r = if i % 2 == 0 { s / 2.0 } else { s / 4.5 };
            Pt::new(s / 2.0 + r * a.sin(), s / 2.0 - r * a.cos())
        })
        .collect();
    Kind::Path(vec![Polyline::new(pts, true)])
}

fn zigzag(rng: &mut Rng, s: f64) -> Kind {
    let n = 12;
    let pts = (0..=n).map(|i| Pt::new(i as f64 * s / n as f64, rng.range(0.0, s / 2.0))).collect();
    Kind::Path(vec![Polyline::new(pts, false)])
}

const WORDS: [&str; 10] = ["Wydajność", "Performance", "Krasnale 2024", "ŻÓŁĆ gęślą jaźń", "LightCreator", "MillCreator", "PenCreator", "BaseCreator", "0123456789", "Test of speed"];

/// A shape of `kind` `size` mm big at a random place of the box (x0, y0)–(x1, y1).
fn place(rng: &mut Rng, x0: f64, y0: f64, x1: f64, y1: f64, size: f64) -> Xf {
    Xf::translate(rng.range(x0, (x1 - size).max(x0)), rng.range(y0, (y1 - size).max(y0)))
}

/// A dwarf moved to (x, y) and made `size` mm tall.
fn dwarf(d: &Shape, x: f64, y: f64, size: f64) -> Option<(Kind, Xf)> {
    let b = d.bounds()?;
    let k = size / (b.max.y - b.min.y).max(1e-6);
    Some((d.kind.clone(), d.xf.then(Xf::translate(-b.min.x, -b.min.y)).then(Xf::scale(k, k)).then(Xf::translate(x, y))))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let out = args.get(1).expect("the file to write");
    let dwarfs: Vec<Shape> = match args.get(2) {
        Some(p) => {
            let text = std::fs::read_to_string(p).expect("the project with the dwarfs");
            let pr = Project::from_json(&text).expect("a project");
            // The curves of the layer with the most of them, the middle-sized ones (not the biggest, not the dots).
            let mut by_layer = std::collections::BTreeMap::<usize, usize>::new();
            for s in &pr.scene.shapes {
                *by_layer.entry(s.layer).or_default() += 1;
            }
            let layer = by_layer.iter().max_by_key(|(_, n)| **n).map(|(l, _)| *l).unwrap_or(0);
            let mut v: Vec<(usize, Shape)> = pr.scene.shapes.iter().filter(|s| s.layer == layer && matches!(s.kind, Kind::Bezier(_))).map(|s| (serde_json::to_string(&s.kind).map_or(0, |j| j.len()), s.clone())).collect();
            v.sort_by_key(|(n, _)| *n);
            let mid = v.len() / 2;
            v.into_iter().skip(mid.saturating_sub(10)).take(20).map(|(_, s)| s).collect()
        }
        None => vec![],
    };
    let solid = Solid::block(300.0, 400.0, 500.0);
    let mut rng = Rng(2024);
    let mut faces = Faces::new(solid);
    let outside_each = 5000 / solid.count() + 1;
    let mut outside_left = 5000usize;
    for i in 0..solid.count() {
        let f = solid.face(i);
        let mut s = Scene::default();
        let (x0, y0, x1, y1) = (10.0, 10.0, f.w - 10.0, f.h - 10.0);
        // 10 texts.
        for (k, w) in WORDS.iter().enumerate() {
            let t = TextData { text: format!("{w} {}", "ABCDEF".as_bytes()[i] as char), size: rng.range(6.0, 18.0), bold: k % 3 == 0, ..TextData::default() };
            let xf = place(&mut rng, x0, y0, x1, y1, 60.0);
            s.add(10 + k % 3, Kind::Text(t), xf);
        }
        // 100 objects on ten layers; a few of them dwarfs.
        for k in 0..100 {
            let layer = k % 10;
            let size = rng.range(5.0, 40.0);
            let kind = match k % 7 {
                0 => Kind::Rect { w: size, h: size },
                1 => Kind::Ellipse { w: size, h: size },
                2 => heart(size),
                3 => star(size, 5 + k % 4),
                4 => zigzag(&mut rng, size),
                5 => Kind::Rect { w: size, h: size / 2.0 },
                _ => Kind::Ellipse { w: size, h: size * 0.6 },
            };
            let xf = place(&mut rng, x0, y0, x1, y1, size);
            if k % 25 == 24 && !dwarfs.is_empty() {
                let d = &dwarfs[(i * 4 + k / 25) % dwarfs.len()];
                let (x, y) = (rng.range(x0, x1 - 60.0), rng.range(y0, y1 - 60.0));
                if let Some((kind, xf)) = dwarf(d, x, y, 50.0) {
                    s.add(2, kind, xf);
                    continue;
                }
            }
            s.add(layer, kind, xf);
        }
        // Beside every work area (no machine reaches 5 m): plain shapes, and big dwarfs as in the project that hung the preview.
        let n = outside_each.min(outside_left);
        outside_left -= n;
        for k in 0..n {
            let size = rng.range(10.0, 80.0);
            let (x, y) = (rng.range(5000.0, 9000.0), rng.range(-2000.0, 4000.0));
            if k % 50 == 0 && !dwarfs.is_empty() {
                if let Some((kind, xf)) = dwarf(&dwarfs[k / 50 % dwarfs.len()], x, y, 300.0) {
                    s.add(2, kind, xf);
                    continue;
                }
            }
            let kind = match k % 4 {
                0 => Kind::Rect { w: size, h: size },
                1 => Kind::Ellipse { w: size, h: size },
                2 => heart(size),
                _ => star(size, 6),
            };
            s.add(k % 10, kind, Xf::translate(x, y));
        }
        faces.scenes[i] = s;
    }
    let mut p = Project::new(Scene::default());
    p.description = "Performance test project of LightCreator, MillCreator, PenCreator and BaseCreator: a block of 300 × 400 × 500 mm, on each face 10 texts and 100 objects on ten layers, and 5000 objects beside the work areas. Made by crates/base-core/examples/perf_project.rs; do not change it by hand.".into();
    faces.write(&mut p);
    std::fs::write(out, p.to_json()).expect("written");
    let total: usize = faces.scenes.iter().map(|s| s.shapes.len()).sum();
    eprintln!("{out}: {total} shapes on {} faces", faces.scenes.len());
}
