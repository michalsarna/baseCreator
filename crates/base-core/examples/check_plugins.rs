//! Checks a folder of plugins the way a program loads them, and runs their scripts: every generator with its start
//! values (and with every option of every choice), every postprocessor on a small program, every importer on the
//! `tests/sample.<extension>` file of the plugin when there is one.
//!
//! Usage: `cargo run -p base-core --example check_plugins -- <folder with plugin folders>`
//! Exit code 0 when everything is fine, 1 otherwise. Used by the CI of the plugins repository.
use base_core::plugin_script::{self, Ctx};
use base_core::plugins::{ParamKind, Registry};

fn main() {
    let dir = std::env::args().nth(1).unwrap_or_else(|| "plugins".into());
    let reg = Registry::load(&dir);
    let mut failures = 0;
    for b in &reg.broken {
        println!("FAIL {}: {}", b.folder, b.reason);
        failures += 1;
    }
    let ctx = Ctx { app: "millcreator".into(), area_w: 300.0, area_h: 200.0 };
    for p in &reg.plugins {
        let m = &p.manifest;
        let mut fail = |what: String| {
            println!("FAIL {}: {what}", m.id);
            failures += 1;
        };
        if m.version.is_empty() {
            fail("the manifest needs a version".into());
        }
        if m.description.trim().is_empty() || m.author.trim().is_empty() || m.license.trim().is_empty() {
            fail("the manifest needs a description, an author and a license".into());
        }
        for g in &m.generators {
            let mut variants = vec![plugin_script::defaults(g)];
            for param in g.params.iter().filter(|x| x.kind == ParamKind::Choice) {
                for o in &param.options {
                    let mut v = plugin_script::defaults(g);
                    v.insert(param.id.clone(), serde_json::json!(o));
                    variants.push(v);
                }
            }
            for v in variants {
                match plugin_script::generate(p, g, &v, &ctx) {
                    Ok(s) if s.scene.shapes.is_empty() => fail(format!("generator {} made no shapes with {v:?}", g.id)),
                    Ok(_) => {}
                    Err(e) => fail(format!("generator {} failed with {v:?}: {e}", g.id)),
                }
            }
        }
        for d in &m.postprocessors {
            let lines: Vec<String> = ["G21", "G90", "G0 X1 Y1", "G1 X5 Y5 F300", "M5"].iter().map(|s| s.to_string()).collect();
            if let Err(e) = plugin_script::postprocess(p, d, &lines, &ctx) {
                fail(format!("postprocessor {} failed: {e}", d.id));
            }
        }
        for i in &m.importers {
            for ext in &i.extensions {
                let sample = p.dir.join("tests").join(format!("sample.{ext}"));
                if let Ok(bytes) = std::fs::read(&sample) {
                    match plugin_script::import(p, i, &bytes, &ctx) {
                        Ok(s) if s.scene.shapes.is_empty() => fail(format!("importer {} read no shapes from {}", i.id, sample.display())),
                        Ok(_) => {}
                        Err(e) => fail(format!("importer {} failed on {}: {e}", i.id, sample.display())),
                    }
                }
            }
        }
        println!("ok   {} {}", m.id, m.version);
    }
    if reg.plugins.is_empty() && reg.broken.is_empty() {
        println!("FAIL no plugins found in {dir}");
        failures += 1;
    }
    std::process::exit(if failures == 0 { 0 } else { 1 });
}
