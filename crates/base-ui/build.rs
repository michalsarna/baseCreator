use std::fmt::Write;
use std::path::Path;

/// Write `$OUT_DIR/ui_icon_files.rs`: every SVG in `assets/ui-icons` embedded with `include_bytes!`, so a new
/// icon only needs its file added.
fn embed_ui_icons() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/ui-icons");
    let mut files: Vec<(String, String)> = vec![];
    if let Ok(items) = std::fs::read_dir(&root) {
        for it in items.flatten() {
            let p = it.path();
            if p.extension().is_some_and(|e| e == "svg") {
                let stem = p.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
                files.push((stem, p.canonicalize().unwrap_or(p).to_string_lossy().to_string()));
            }
        }
    }
    files.sort();
    let mut out = String::from("const FILES: &[(&str, &[u8])] = &[\n");
    for (n, p) in &files {
        let _ = writeln!(out, "    ({n:?}, include_bytes!({p:?})),");
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap_or_default()).join("ui_icon_files.rs");
    let _ = std::fs::write(dest, out);
    println!("cargo:rerun-if-changed=../../assets/ui-icons");
}

/// Write `$OUT_DIR/clipart_files.rs`: every SVG under `assets/clipart/<Category>/` embedded with `include_bytes!`.
fn embed_clipart() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/clipart");
    let mut files: Vec<(String, String, String)> = vec![];
    if let Ok(cats) = std::fs::read_dir(&root) {
        for cat in cats.flatten() {
            if !cat.path().is_dir() {
                continue;
            }
            let cat_name = cat.file_name().to_string_lossy().replace('-', " ");
            if let Ok(items) = std::fs::read_dir(cat.path()) {
                for it in items.flatten() {
                    let p = it.path();
                    if p.extension().is_some_and(|e| e == "svg") {
                        let stem = p.file_stem().map(|s| s.to_string_lossy().replace('-', " ")).unwrap_or_default();
                        files.push((cat_name.clone(), stem, p.canonicalize().unwrap_or(p).to_string_lossy().to_string()));
                    }
                }
            }
        }
    }
    files.sort();
    let mut out = String::from("/// (category, name, SVG bytes) of every picture shipped with the programs.\npub const BUILTIN_CLIPART: &[(&str, &str, &[u8])] = &[\n");
    for (c, n, p) in &files {
        let _ = writeln!(out, "    ({c:?}, {n:?}, include_bytes!({p:?})),");
    }
    out.push_str("];\n");
    let dest = Path::new(&std::env::var("OUT_DIR").unwrap_or_default()).join("clipart_files.rs");
    let _ = std::fs::write(dest, out);
    println!("cargo:rerun-if-changed=../../assets/clipart");
}

fn main() {
    embed_ui_icons();
    embed_clipart();
}
