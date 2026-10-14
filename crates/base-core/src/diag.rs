//! The diagnostics file: one zip with what a person helping with a machine problem asks for first, the report of the
//! program, the settings of the controller, the console and the program that was loaded.
use std::io::Write;
use std::path::Path;

/// One file of the zip: a name and its text.
pub struct Part<'a> {
    pub name: &'a str,
    pub text: &'a str,
}

/// Write the zip. Names are plain file names; empty parts are left out. Returns the number of files.
pub fn write(dest: &Path, parts: &[Part]) -> Result<usize, String> {
    let file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut n = 0;
    for p in parts {
        if p.text.is_empty() || p.name.contains(['/', '\\']) {
            continue;
        }
        zip.start_file(p.name, opts).map_err(|e| e.to_string())?;
        zip.write_all(p.text.as_bytes()).map_err(|e| e.to_string())?;
        n += 1;
    }
    zip.finish().map_err(|e| e.to_string())?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Read;

    #[test]
    fn the_parts_go_into_the_zip_and_empty_or_odd_ones_do_not() {
        let dir = std::env::temp_dir().join(format!("bc-diag-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let z = dir.join("d.zip");
        let n = write(&z, &[Part { name: "report.txt", text: "hello" }, Part { name: "empty.txt", text: "" }, Part { name: "../evil.txt", text: "x" }, Part { name: "console.txt", text: "> $$" }]).unwrap();
        assert_eq!(n, 2);
        let mut a = zip::ZipArchive::new(std::fs::File::open(&z).unwrap()).unwrap();
        let mut s = String::new();
        a.by_name("console.txt").unwrap().read_to_string(&mut s).unwrap();
        assert_eq!(s, "> $$");
        assert_eq!(a.len(), 2);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
