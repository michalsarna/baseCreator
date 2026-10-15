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
