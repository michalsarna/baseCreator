//! The plugins in `examples/plugins` load, and every script in them runs.
use base_core::plugin_script::{self, Ctx};
use base_core::plugins::{PresetClass, Registry};
use base_core::scene::Kind;
use std::path::PathBuf;

fn registry() -> Registry {
    Registry::load(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../examples/plugins"))
}

fn ctx() -> Ctx {
    Ctx { app: "millcreator".into(), area_w: 300.0, area_h: 200.0 }
}

#[test]
fn the_example_plugins_load() {
    let r = registry();
    assert!(r.broken.is_empty(), "{:?}", r.broken);
    assert_eq!(r.plugins.len(), 4);
    assert_eq!(r.presets("millcreator", PresetClass::Mill).len(), 1);
    assert_eq!(r.presets("pencreator", PresetClass::Pen).len(), 1);
    assert_eq!(r.translate("pl", "Example text"), Some("Przykładowy tekst"));
    assert_eq!(r.data("millcreator", "notes").len(), 1);
}

#[test]
fn the_importer_reads_points_and_turns_them_over() {
    let r = registry();
    let (p, def) = r.importer_for("millcreator", "csv").unwrap();
    let text = "# a square\n10,10\n30,10\n30,30\n10,30\nclosed\n\n0,0\n5,5\nbroken\n";
    let out = plugin_script::import(p, def, text.as_bytes(), &ctx()).unwrap();
    assert_eq!(out.scene.shapes.len(), 2);
    let b = out.scene.shapes[0].bounds().unwrap();
    // 10..30 mm from the bottom of a 200 mm area is 170..190 from the top.
    assert!((b.min.y - 170.0).abs() < 1e-9 && (b.max.y - 190.0).abs() < 1e-9, "{b:?}");
    assert!(out.scene.shapes[0].local_polys()[0].closed);
    assert!(out.warnings.iter().any(|w| w.contains("broken")));
}

#[test]
fn the_generators_make_shapes() {
    let r = registry();
    let (p, def) = r.generators("millcreator").into_iter().find(|(_, g)| g.id == "hole-pattern").unwrap();
    let mut v = plugin_script::defaults(def);
    v.insert("count".into(), serde_json::json!(8));
    let out = plugin_script::generate(p, def, &v, &ctx()).unwrap();
    assert_eq!(out.scene.shapes.len(), 9, "eight holes and the centre one");
    assert!(out.scene.shapes.iter().all(|s| s.layer == 4 && matches!(s.kind, Kind::Ellipse { .. })));
    let (p, def) = r.generators("millcreator").into_iter().find(|(_, g)| g.id == "spur-gear").unwrap();
    let out = plugin_script::generate(p, def, &plugin_script::defaults(def), &ctx()).unwrap();
    assert_eq!(out.scene.shapes.len(), 2);
    // 18 teeth of module 2: the outer diameter is 2 * (18 + 2) = 40 mm.
    let b = out.scene.shapes[0].bounds().unwrap();
    assert!((b.width() - 40.0).abs() < 1.0, "{}", b.width());
}

#[test]
fn values_of_the_form_are_kept_in_range() {
    let r = registry();
    let (p, def) = r.generators("millcreator").into_iter().find(|(_, g)| g.id == "hole-pattern").unwrap();
    let mut v = plugin_script::defaults(def);
    v.insert("count".into(), serde_json::json!(100000));
    let out = plugin_script::generate(p, def, &v, &ctx()).unwrap();
    assert_eq!(out.scene.shapes.len(), 201, "the count is limited to 200");
}

#[test]
fn the_postprocessors_rewrite_lines() {
    let r = registry();
    let lines: Vec<String> = ["G21 (units)", "G0 X1 Y1 ; move", "", "M5"].iter().map(|s| s.to_string()).collect();
    let (p, def) = r.postprocessors("millcreator").into_iter().find(|(_, d)| d.id == "line-numbers").unwrap();
    let (out, _) = plugin_script::postprocess(p, def, &lines, &ctx()).unwrap();
    assert_eq!(out, vec!["N10 G21 (units)", "N20 G0 X1 Y1 ; move", "N30 M5"]);
    let (p, def) = r.postprocessors("millcreator").into_iter().find(|(_, d)| d.id == "no-comments").unwrap();
    let (out, _) = plugin_script::postprocess(p, def, &lines, &ctx()).unwrap();
    assert_eq!(out, vec!["G21", "G0 X1 Y1", "M5"]);
}

#[test]
fn dangerous_output_is_refused() {
    assert!(plugin_script::check_output(&["G1 X1".into()]).is_ok());
    assert!(plugin_script::check_output(&["$RST=*".into()]).is_err());
    assert!(plugin_script::check_output(&["  $H".into()]).is_err());
    assert!(plugin_script::check_output(&["!".into()]).is_err());
    assert!(plugin_script::check_output(&["G1\u{18}".into()]).is_err());
    assert!(plugin_script::check_output(&["G1 ".repeat(200)]).is_err());
}

fn scratch(name: &str, manifest: &str, script: &str) -> (std::path::PathBuf, Registry) {
    let dir = std::env::temp_dir().join(format!("bc-plugins-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let p = dir.join("bad");
    std::fs::create_dir_all(&p).unwrap();
    std::fs::write(p.join("plugin.json"), manifest).unwrap();
    std::fs::write(p.join("s.rhai"), script).unwrap();
    let r = Registry::load(&dir);
    (dir, r)
}

#[test]
fn a_script_that_never_ends_is_stopped() {
    let m = r#"{"id":"bad","name":"Bad","generators":[{"id":"g","name":"G","script":"s.rhai"}]}"#;
    let (dir, r) = scratch("loop", m, "fn generate(p, ctx) { loop { } }");
    let (p, def) = r.generators("millcreator")[0];
    let t = std::time::Instant::now();
    let e = plugin_script::generate(p, def, &Default::default(), &ctx()).unwrap_err();
    assert!(t.elapsed().as_secs() < 30, "{e}");
    assert!(!e.is_empty());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_script_cannot_reach_files_or_eval() {
    let m = r#"{"id":"bad","name":"Bad","generators":[{"id":"g","name":"G","script":"s.rhai"}]}"#;
    for body in ["fn generate(p, ctx) { eval(\"1\") }", "fn generate(p, ctx) { read_file(\"/etc/passwd\") }", "import \"x\" as y; fn generate(p, ctx) { [] }"] {
        let (dir, r) = scratch("sandbox", m, body);
        let (p, def) = r.generators("millcreator")[0];
        assert!(plugin_script::generate(p, def, &Default::default(), &ctx()).is_err(), "{body}");
        std::fs::remove_dir_all(dir).ok();
    }
}

#[test]
fn a_broken_manifest_is_listed_not_loaded() {
    let (dir, r) = scratch("broken", r#"{"id":"Bad Id","name":"x"}"#, "");
    assert!(r.plugins.is_empty());
    assert_eq!(r.broken.len(), 1);
    std::fs::remove_dir_all(dir).ok();
    let (dir, r) = scratch("reserved", r#"{"id":"bad","name":"x","importers":[{"id":"i","name":"I","extensions":["SVG"],"script":"s.rhai"}]}"#, "");
    assert!(r.plugins.is_empty() && r.broken.len() == 1, "{:?}", r.broken);
    std::fs::remove_dir_all(dir).ok();
    let (dir, r) = scratch("path", r#"{"id":"bad","name":"x","generators":[{"id":"g","name":"G","script":"../s.rhai"}]}"#, "");
    assert!(r.plugins.is_empty() && r.broken.len() == 1, "{:?}", r.broken);
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_plugin_can_be_switched_off_and_installed_from_a_zip() {
    use std::io::Write;
    let dir = std::env::temp_dir().join(format!("bc-plugins-zip-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let zip_path = dir.join("p.zip");
    {
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("my-plugin/plugin.json", o).unwrap();
        z.write_all(br#"{"id":"my-plugin","name":"Mine","postprocessors":[{"id":"p","name":"P","script":"p.rhai"}]}"#).unwrap();
        z.start_file("my-plugin/p.rhai", o).unwrap();
        z.write_all(b"fn post(lines, ctx) { lines }").unwrap();
        z.finish().unwrap();
    }
    let mut r = Registry::load(dir.join("plugins"));
    assert_eq!(r.install_zip(&zip_path).unwrap(), "my-plugin");
    assert_eq!(r.postprocessors("millcreator").len(), 1);
    r.set_enabled("my-plugin", false).unwrap();
    assert!(r.postprocessors("millcreator").is_empty());
    assert!(!Registry::load(dir.join("plugins")).plugins[0].enabled, "the choice is remembered");
    r.remove("my-plugin").unwrap();
    assert!(r.plugins.is_empty() && !dir.join("plugins/my-plugin").exists());
    // A zip that tries to leave the folder is refused.
    {
        let mut z = zip::ZipWriter::new(std::fs::File::create(&zip_path).unwrap());
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("plugin.json", o).unwrap();
        z.write_all(br#"{"id":"evil","name":"E"}"#).unwrap();
        z.start_file("../escape.txt", o).unwrap();
        z.write_all(b"x").unwrap();
        z.finish().unwrap();
    }
    assert!(r.install_zip(&zip_path).is_err());
    assert!(!dir.join("escape.txt").exists() && !dir.join("plugins/escape.txt").exists());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn clipart_of_a_pack_joins_the_gallery_once() {
    let r = registry();
    let gal = std::env::temp_dir().join(format!("bc-plugins-gal-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&gal);
    assert_eq!(r.sync_clipart("millcreator", &gal), 1);
    assert_eq!(r.sync_clipart("millcreator", &gal), 0);
    std::fs::remove_dir_all(gal).ok();
}

#[test]
fn presets_of_plugins_follow_the_built_in_ones() {
    let r = registry();
    let list = base_core::presets::with_plugins(base_core::presets::MILL, &r, "millcreator", PresetClass::Mill);
    assert_eq!(list.len(), base_core::presets::MILL.len() + 1);
    assert_eq!(list.last().unwrap().plugin.as_deref(), Some("example-pack"));
    // A plugin for another program does not show up.
    let none = base_core::presets::with_plugins(&[], &r, "lightcreator", PresetClass::Pen);
    assert_eq!(none.len(), 1, "the example pack is for all programs");
}

fn zip_of(manifest: &str) -> Vec<u8> {
    use std::io::Write;
    let mut buf = std::io::Cursor::new(Vec::new());
    {
        let mut z = zip::ZipWriter::new(&mut buf);
        let o = zip::write::SimpleFileOptions::default();
        z.start_file("plugin.json", o).unwrap();
        z.write_all(manifest.as_bytes()).unwrap();
        z.finish().unwrap();
    }
    buf.into_inner()
}

fn entry(version: &str, bytes: &[u8]) -> base_core::registry::Entry {
    use sha2::{Digest, Sha256};
    let sum: String = Sha256::digest(bytes).iter().map(|b| format!("{b:02x}")).collect();
    base_core::registry::Entry {
        id: "listed".into(),
        name: "Listed".into(),
        description: String::new(),
        author: String::new(),
        license: String::new(),
        programs: vec![],
        provides: vec![],
        version: version.into(),
        min_base: String::new(),
        url: format!("{}zips/listed-{version}.zip", base_core::registry::TRUSTED_PREFIX),
        sha256: sum,
        size: bytes.len() as u64,
        homepage: String::new(),
        changelog: String::new(),
    }
}

#[test]
fn a_plugin_of_the_official_list_is_installed_and_updated() {
    use base_core::registry::{self, Status};
    let dir = std::env::temp_dir().join(format!("bc-plugins-list-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let mut r = Registry::load(dir.join("plugins"));
    let v1 = zip_of(r#"{"id":"listed","name":"Listed","version":"1.0.0"}"#);
    let e1 = entry("1.0.0", &v1);
    assert_eq!(e1.status(&r), Status::NotInstalled);
    assert_eq!(registry::install(&mut r, &e1, &v1).unwrap(), "listed");
    assert_eq!(e1.status(&r), Status::UpToDate);
    // The list gets a newer version.
    let v2 = zip_of(r#"{"id":"listed","name":"Listed","version":"1.1.0"}"#);
    let e2 = entry("1.1.0", &v2);
    assert_eq!(e2.status(&r), Status::Update { installed: "1.0.0".into() });
    // A file that is not what the list says is refused: wrong checksum, wrong version inside.
    assert!(registry::install(&mut r, &e2, &v1).unwrap_err().contains("checksum"));
    let lie = entry("1.1.0", &v1);
    assert!(registry::install(&mut r, &lie, &v1).unwrap_err().contains("not listed 1.1.0"));
    assert_eq!(registry::install(&mut r, &e2, &v2).unwrap(), "listed");
    assert_eq!(r.find("listed").unwrap().manifest.version, "1.1.0");
    assert_eq!(e1.status(&r), Status::Newer);
    // A plugin that needs a newer program is not loaded, and the list says so.
    let mut needy = entry("2.0.0", &v2);
    needy.min_base = "99.0.0".into();
    assert!(matches!(needy.status(&r), Status::NeedsNewerProgram { .. }));
    let new_zip = zip_of(r#"{"id":"needy","name":"N","min_base":"99.0.0"}"#);
    let tmp = dir.join("n.zip");
    std::fs::write(&tmp, new_zip).unwrap();
    assert!(r.install_zip(&tmp).unwrap_err().contains("baseCreator 99.0.0"));
    // The switch for the check at the start is remembered by all programs.
    assert!(!r.auto_check());
    r.set_auto_check(true).unwrap();
    assert!(Registry::load(dir.join("plugins")).auto_check());
    std::fs::remove_dir_all(dir).ok();
}

#[test]
fn a_file_over_the_limit_is_not_read() {
    let dir = std::env::temp_dir().join(format!("bc-plugins-limit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let f = dir.join("big.bin");
    std::fs::write(&f, vec![0u8; 100]).unwrap();
    assert_eq!(base_core::plugins::read_limited(&f, 100).unwrap().len(), 100);
    assert!(base_core::plugins::read_limited(&f, 99).is_err());
    assert!(base_core::plugins::read_limited(&dir, 100).is_err(), "a folder is not a file");
    std::fs::remove_dir_all(dir).ok();
}

// ---- binary importers with a form (fixtures/plugins/binary-demo) ----

fn demo() -> Registry {
    Registry::load(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plugins"))
}

fn f32s(count: u32, values: &[f32]) -> Vec<u8> {
    let mut b = count.to_le_bytes().to_vec();
    for v in values {
        b.extend(v.to_le_bytes());
    }
    b
}

#[test]
fn a_binary_importer_gets_a_blob_and_the_values_of_its_form() {
    let r = demo();
    assert!(r.broken.is_empty(), "{:?}", r.broken);
    let (p, def) = r.importer_for("millcreator", "pts").unwrap();
    assert!(def.binary && def.interactive());
    let data = f32s(3, &[0.0, 0.0, 10.0, 0.0, 10.0, 5.0]);
    // The form made from the file: one field for each point and one of the manifest.
    let extra = plugin_script::form_for(p, def, &data, &ctx()).unwrap();
    assert_eq!(extra.len(), 4, "{extra:?}");
    assert_eq!(extra[0].id, "closed");
    assert_eq!(extra[3].label, "Keep point 3");
    // The start values: all the points, closed, scale 1.
    let mut given = plugin_script::defaults_of(&def.params.iter().chain(extra.iter()).cloned().collect::<Vec<_>>());
    let out = plugin_script::import_with(p, def, &data, &given, &extra, &ctx()).unwrap();
    assert_eq!(out.scene.shapes.len(), 1);
    let b = out.scene.shapes[0].bounds().unwrap();
    assert!((b.width() - 10.0).abs() < 1e-6 && (b.height() - 5.0).abs() < 1e-6, "{b:?}");
    assert!(out.scene.shapes[0].local_polys()[0].closed);
    // A different form: scaled by 2, open, the third point left out.
    given.insert("scale".into(), serde_json::json!(2.0));
    given.insert("closed".into(), serde_json::json!(false));
    given.insert("keep_2".into(), serde_json::json!(false));
    let out = plugin_script::import_with(p, def, &data, &given, &extra, &ctx()).unwrap();
    let b = out.scene.shapes[0].bounds().unwrap();
    assert!((b.width() - 20.0).abs() < 1e-6 && b.height() < 1e-6, "{b:?}");
    assert!(!out.scene.shapes[0].local_polys()[0].closed);
    // The plain import (no window) works as well, with the start values.
    assert_eq!(plugin_script::import(p, def, &data, &ctx()).unwrap().scene.shapes.len(), 1);
}

#[test]
fn the_fields_a_script_makes_are_checked() {
    let r = demo();
    let (p, def) = r.importer_for("millcreator", "pts").unwrap();
    // 150 points make 151 fields: too many.
    let many = f32s(150, &vec![1.0; 300]);
    let err = plugin_script::form_for(p, def, &many, &ctx()).unwrap_err();
    assert!(err.contains("more than"), "{err}");
    // An importer without `form` has no extra fields, and a file the script cannot read is an error, not a crash.
    let (p2, def2) = r.importer_for("millcreator", "tris").unwrap();
    assert!(plugin_script::form_for(p2, def2, &[], &ctx()).unwrap().is_empty());
    assert!(plugin_script::form_for(p, def, &[1, 2], &ctx()).is_err());
}

#[test]
fn many_triangles_are_merged_into_outlines_by_the_script() {
    let r = demo();
    let (p, def) = r.importer_for("pencreator", "tris").unwrap();
    // A strip of 5 000 squares (10 000 triangles) side by side is one outline.
    let mut v = vec![];
    for i in 0..5000 {
        let x = i as f32;
        v.extend([x, 0.0, x + 1.0, 0.0, x + 1.0, 1.0, x, 0.0, x + 1.0, 1.0, x, 1.0]);
    }
    let out = plugin_script::import(p, def, &f32s(10_000, &v), &Ctx { app: "pencreator".into(), area_w: 300.0, area_h: 200.0 }).unwrap();
    assert_eq!(out.scene.shapes.len(), 1);
    let b = out.scene.shapes[0].bounds().unwrap();
    assert!((b.width() - 5000.0).abs() < 1e-3 && (b.height() - 1.0).abs() < 1e-6, "{b:?}");
}

// ---- the STL plugin (fixtures/plugins/stl-projection) ----

type V3 = [f64; 3];

fn stl_of(tris: &[[V3; 3]]) -> Vec<u8> {
    let mut b = vec![0u8; 80];
    b.extend((tris.len() as u32).to_le_bytes());
    for t in tris {
        b.extend([0u8; 12]);
        for p in t {
            for c in p {
                b.extend((*c as f32).to_le_bytes());
            }
        }
        b.extend([0u8; 2]);
    }
    b
}

/// A box from (x, y, z) of size (w, d, h) as 12 triangles.
fn boxed(x: f64, y: f64, z: f64, w: f64, d: f64, h: f64) -> Vec<[V3; 3]> {
    let v = |a: f64, b: f64, c: f64| [x + a * w, y + b * d, z + c * h];
    let quad = |a: V3, b: V3, c: V3, e: V3| vec![[a, b, c], [a, c, e]];
    let mut t = vec![];
    t.extend(quad(v(0., 0., 0.), v(0., 1., 0.), v(1., 1., 0.), v(1., 0., 0.)));
    t.extend(quad(v(0., 0., 1.), v(1., 0., 1.), v(1., 1., 1.), v(0., 1., 1.)));
    t.extend(quad(v(0., 0., 0.), v(1., 0., 0.), v(1., 0., 1.), v(0., 0., 1.)));
    t.extend(quad(v(0., 1., 0.), v(0., 1., 1.), v(1., 1., 1.), v(1., 1., 0.)));
    t.extend(quad(v(0., 0., 0.), v(0., 0., 1.), v(0., 1., 1.), v(0., 1., 0.)));
    t.extend(quad(v(1., 0., 0.), v(1., 1., 0.), v(1., 1., 1.), v(1., 0., 1.)));
    t
}

fn run_stl(app: &str, data: &[u8], set: &[(&str, serde_json::Value)]) -> base_core::plugin_script::Shapes {
    let r = demo();
    let (p, def) = r.importer_for(app, "stl").unwrap();
    let c = Ctx { app: app.into(), area_w: 300.0, area_h: 200.0 };
    let extra = plugin_script::form_for(p, def, data, &c).unwrap();
    let all: Vec<_> = def.params.iter().chain(extra.iter()).cloned().collect();
    let mut given = plugin_script::defaults_of(&all);
    for (k, v) in set {
        given.insert((*k).to_string(), v.clone());
    }
    plugin_script::import_with(p, def, data, &given, &extra, &c).unwrap()
}

fn total_box(out: &base_core::plugin_script::Shapes) -> (f64, f64, f64, f64) {
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for s in &out.scene.shapes {
        let b = s.bounds().unwrap();
        x0 = x0.min(b.min.x);
        y0 = y0.min(b.min.y);
        x1 = x1.max(b.max.x);
        y1 = y1.max(b.max.y);
    }
    (x0, y0, x1, y1)
}

#[test]
fn the_stl_plugin_has_a_form_made_from_the_planes_of_the_model() {
    let r = demo();
    let (p, def) = r.importer_for("millcreator", "stl").unwrap();
    // A box on a bigger plate: planes at three heights face up (the plate top, the box top) and more.
    let mut tris = boxed(0.0, 0.0, 0.0, 30.0, 30.0, 2.0);
    tris.extend(boxed(5.0, 5.0, 2.0, 10.0, 10.0, 6.0));
    let data = stl_of(&tris);
    let fields = plugin_script::form_for(p, def, &data, &ctx()).unwrap();
    let ids: Vec<&str> = fields.iter().map(|f| f.id.as_str()).collect();
    assert_eq!(&ids[..5], ["mode", "below", "depth", "facing", "min_area"]);
    assert!(ids.iter().filter(|i| i.starts_with("plane_")).count() >= 10, "{ids:?}");
    // The router starts with the slice, the others with the shadow of the whole model.
    let mode = |app: &str| {
        let (p, def) = demo().importer_for(app, "stl").map(|(p, d)| (p.clone(), d.clone())).unwrap();
        plugin_script::form_for(&p, &def, &data, &Ctx { app: app.into(), area_w: 1.0, area_h: 1.0 }).unwrap()[0].default.clone()
    };
    assert_eq!(mode("millcreator"), serde_json::json!("Slice at a depth"));
    assert_eq!(mode("pencreator"), serde_json::json!("Shadow of the whole model"));
    // The labels of the planes show the direction, the area and the heights.
    let top = fields.iter().find(|f| f.label.contains("↑") && f.label.ends_with("z 8")).unwrap_or_else(|| panic!("{:?}", fields.iter().map(|f| &f.label).collect::<Vec<_>>()));
    assert!(top.label.contains("100 mm²"), "{}", top.label);
}

#[test]
fn the_stl_plugin_makes_the_shadow_of_the_model_the_chosen_planes_and_a_slice() {
    let mut tris = boxed(0.0, 0.0, 0.0, 30.0, 20.0, 2.0);
    tris.extend(boxed(1.0, 4.0, 2.0, 4.0, 6.0, 6.0));
    let data = stl_of(&tris);
    // The shadow of the whole model: the big plate, in the corner (the box of the model starts at 0, 0).
    let out = run_stl("pencreator", &data, &[]);
    assert_eq!(out.scene.shapes.len(), 1);
    let (x0, y0, x1, y1) = total_box(&out);
    assert!(x0.abs() < 1e-6 && y0.abs() < 1e-6 && (x1 - 30.0).abs() < 1e-6 && (y1 - 20.0).abs() < 1e-6, "{x0} {y0} {x1} {y1}");
    // Only the planes facing up that are 8 mm high: the top of the little box (4 x 6), seen from above and from below.
    let form_fields = {
        let r = demo();
        let (p, def) = r.importer_for("pencreator", "stl").unwrap();
        plugin_script::form_for(p, def, &data, &Ctx { app: "pencreator".into(), area_w: 1.0, area_h: 1.0 }).unwrap()
    };
    let top_id = form_fields.iter().find(|f| f.label.ends_with("z 8") && f.label.contains("↑")).map(|f| f.id.clone()).unwrap();
    let mut set: Vec<(&str, serde_json::Value)> = vec![("mode", serde_json::json!("Shadow of the chosen planes"))];
    let ids: Vec<String> = form_fields.iter().filter(|f| f.id.starts_with("plane_")).map(|f| f.id.clone()).collect();
    for id in &ids {
        set.push((id.as_str(), serde_json::json!(*id == top_id)));
    }
    let out = run_stl("pencreator", &data, &set);
    assert_eq!(out.scene.shapes.len(), 1);
    let (x0, y0, x1, y1) = total_box(&out);
    // The outlines keep their place in the box of the model (its top left corner is at 0, 0; Y is turned over: 4..10 of a 20 mm
    // deep model is 10..16 from the top).
    assert!((x0 - 1.0).abs() < 1e-6 && (y0 - 10.0).abs() < 1e-6 && (x1 - 5.0).abs() < 1e-6 && (y1 - 16.0).abs() < 1e-6, "{x0} {y0} {x1} {y1}");
    // From below the model is seen mirrored: 1..5 from the left of a 30 mm plate is 25..29.
    set.push(("below", serde_json::json!(true)));
    let from_below = run_stl("pencreator", &data, &set);
    let (x0, _, x1, _) = total_box(&from_below);
    assert!((x0 - 25.0).abs() < 1e-6 && (x1 - 29.0).abs() < 1e-6, "{x0} {x1}");
    // Nothing is chosen: a warning and no shapes.
    let none: Vec<(&str, serde_json::Value)> = std::iter::once(("mode", serde_json::json!("Shadow of the chosen planes"))).chain(ids.iter().map(|i| (i.as_str(), serde_json::json!(false)))).collect();
    let out = run_stl("pencreator", &data, &none);
    assert!(out.scene.shapes.is_empty() && out.warnings.iter().any(|w| w.contains("No plane")));
    // A slice 5 mm below the top (z = 3) goes through the little box only; one at 7 mm (z = 1) through the plate.
    let slice = |depth: f64| run_stl("millcreator", &data, &[("depth", serde_json::json!(depth))]);
    let (x0, y0, x1, y1) = total_box(&slice(5.0));
    assert!((x0 - 1.0).abs() < 1e-6 && (y0 - 10.0).abs() < 1e-6 && (x1 - 5.0).abs() < 1e-6 && (y1 - 16.0).abs() < 1e-6, "{x0} {y0} {x1} {y1}");
    let (_, _, x1, y1) = total_box(&slice(7.0));
    assert!((x1 - 30.0).abs() < 1e-6 && (y1 - 20.0).abs() < 1e-6);
    // Above the model there is nothing to cut: a warning.
    assert!(slice(0.0).scene.shapes.is_empty());
}

#[test]
fn the_stl_plugin_handles_a_big_mesh_in_time() {
    // A flat sheet of 20 000 triangles (a grid of 100 x 100 squares): one outline, read and merged by native code.
    let mut tris = vec![];
    for i in 0..100 {
        for j in 0..100 {
            let (x, y) = (i as f64, j as f64);
            tris.push([[x, y, 0.0], [x + 1.0, y, 0.0], [x + 1.0, y + 1.0, 0.0]]);
            tris.push([[x, y, 0.0], [x + 1.0, y + 1.0, 0.0], [x, y + 1.0, 0.0]]);
        }
    }
    let data = stl_of(&tris);
    let start = std::time::Instant::now();
    let out = run_stl("lightcreator", &data, &[]);
    assert_eq!(out.scene.shapes.len(), 1);
    assert!(start.elapsed().as_secs() < 8, "{:?}", start.elapsed());
}

/// How long a big model takes: `cargo test -p base-core --release --test plugins -- --ignored --nocapture timing`.
#[test]
#[ignore = "a measurement"]
fn timing_of_a_big_stl() {
    for side in [100usize, 300, 450] {
        let mut tris = vec![];
        for i in 0..side {
            for j in 0..side {
                let (x, y, z) = (i as f64, j as f64, ((i * j) % 7) as f64 * 0.1);
                tris.push([[x, y, z], [x + 1.0, y, z], [x + 1.0, y + 1.0, z]]);
                tris.push([[x, y, z], [x + 1.0, y + 1.0, z], [x, y + 1.0, z]]);
            }
        }
        let data = stl_of(&tris);
        for (mode, app) in [("whole", "lightcreator"), ("slice", "millcreator")] {
            let start = std::time::Instant::now();
            let set: Vec<(&str, serde_json::Value)> = if mode == "slice" { vec![("depth", serde_json::json!(0.2))] } else { vec![] };
            let out = run_stl(app, &data, &set);
            println!("TIMING {} triangles {mode}: {} shapes in {:.2}s ({} MB)", tris.len(), out.scene.shapes.len(), start.elapsed().as_secs_f64(), data.len() / 1_000_000);
        }
    }
}

#[test]
fn a_generator_picture_follows_the_choices_and_polygon_is_simple_tells_crossings() {
    let manifest = r#"{ "format": 1, "id": "figs", "name": "Figs", "version": "1.0.0", "generators": [ { "id": "g", "name": "G", "script": "g.rhai",
        "params": [ { "id": "kind", "label": "Kind", "kind": "choice", "default": "a", "options": ["a", "b"] } ],
        "figures": [ { "when": { "kind": "b" }, "file": "b.svg" }, { "file": "any.svg" } ] } ] }"#;
    let script = "fn generate(p, ctx) { let ok = polygon_is_simple([[0,0],[4,0],[4,4],[0,4]]); let bad = polygon_is_simple([[0,0],[4,4],[4,0],[0,4]]); if ok && !bad { rect(0, 0, 1, 1) } else { [] } }";
    let dir = std::env::temp_dir().join(format!("bc-plugins-figs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let pd = dir.join("figs");
    std::fs::create_dir_all(&pd).unwrap();
    std::fs::write(pd.join("plugin.json"), manifest).unwrap();
    std::fs::write(pd.join("g.rhai"), script).unwrap();
    for f in ["b.svg", "any.svg"] {
        std::fs::write(pd.join(f), "<svg xmlns='http://www.w3.org/2000/svg' width='10' height='10'/>").unwrap();
    }
    let reg = Registry::load(&dir);
    assert!(reg.broken.is_empty(), "{:?}", reg.broken);
    let (p, def) = reg.generators("millcreator").into_iter().next().unwrap();
    let mut v = plugin_script::defaults(def);
    assert_eq!(def.figure_for(&v).unwrap().file, "any.svg");
    v.insert("kind".into(), serde_json::json!("b"));
    assert_eq!(def.figure_for(&v).unwrap().file, "b.svg");
    assert_eq!(plugin_script::generate(p, def, &v, &ctx()).unwrap().scene.shapes.len(), 1);
}

#[test]
fn a_picture_that_is_not_an_svg_inside_the_plugin_is_refused() {
    let bad = |file: &str| format!(r#"{{ "format": 1, "id": "figs", "name": "Figs", "version": "1.0.0", "generators": [ {{ "id": "g", "name": "G", "script": "g.rhai", "figures": [ {{ "file": "{file}" }} ] }} ] }}"#);
    for f in ["../x.svg", "x.png", "/abs.svg"] {
        assert!(base_core::plugins::Manifest::parse(&bad(f)).is_err(), "{f}");
    }
    assert!(base_core::plugins::Manifest::parse(&bad("figures/a.svg")).is_ok());
}
