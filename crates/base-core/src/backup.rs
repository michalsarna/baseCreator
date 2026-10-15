//! Backup of one program: a zip with its settings (`app.ron`, the format of the settings file) and the shared
//! clipart gallery. Restoring replaces the settings and *adds* the pictures to the gallery, so pictures that
//! another program saved in the meantime stay.
use crate::paths::merge_gallery;
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Entry of the settings in the zip.
pub const SETTINGS: &str = "app.ron";
/// Settings key that tells which program made the backup.
pub const PROGRAM_KEY: &str = "_program";

/// `yyyymmdd` of a Unix time, for the suggested file name.
pub fn ymd(secs: u64) -> String {
    // Days since 1970-01-01 to a calendar date (Howard Hinnant's algorithm).
    let z = (secs / 86400) as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    format!("{:04}{:02}{:02}", if m <= 2 { y + 1 } else { y }, m, d)
}

/// The suggested file name, `<program>-backup-yyyymmdd.zip`.
pub fn file_name(program: &str, secs: u64) -> String {
    format!("{program}-backup-{}.zip", ymd(secs))
}

fn files_in(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let p = e.path();
        if p.is_dir() {
            files_in(&p, out);
        } else if p.is_file() {
            out.push(p);
        }
    }
}

/// Write the zip: the settings, the whole `clipart` folder of `gallery` and a short README. `title` is the program's
/// name as shown (`LightCreator`); its lower case is the folder name inside the data folder. Returns the number of files.
pub fn write(dest: &Path, title: &str, version: &str, settings: &[(&str, String)], gallery: &Path) -> Result<usize, String> {
    let program = title.to_lowercase();
    let file = std::fs::File::create(dest).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipWriter::new(file);
    let opts = zip::write::SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let mut count = 0usize;
    let mut add = |zip: &mut zip::ZipWriter<std::fs::File>, name: &str, data: &[u8]| -> Result<(), String> {
        zip.start_file(name, opts).map_err(|e| e.to_string())?;
        zip.write_all(data).map_err(|e| e.to_string())?;
        count += 1;
        Ok(())
    };
    let mut map: BTreeMap<&str, String> = settings.iter().map(|(k, v)| (*k, v.clone())).collect();
    map.insert(PROGRAM_KEY, program.clone());
    let ron_text = ron::ser::to_string_pretty(&map, ron::ser::PrettyConfig::default()).map_err(|e| e.to_string())?;
    add(&mut zip, SETTINGS, ron_text.as_bytes())?;
    let mut files = vec![];
    files_in(gallery, &mut files);
    for f in &files {
        let rel = f.strip_prefix(gallery).unwrap_or(f).to_string_lossy().replace('\\', "/");
        let data = std::fs::read(f).map_err(|e| format!("{}: {}", f.display(), e))?;
        add(&mut zip, &format!("clipart/{rel}"), &data)?;
    }
    let readme = format!(
        "{title} {version} backup\n\n\
         app.ron   the settings of {title}: device profiles, language, colour scheme, recent files and so on\n\
         clipart/  the picture gallery shared by LightCreator, MillCreator and PenCreator, with index.json\n\n\
         To restore, use Help > Restore from backup in {title}. The settings are replaced; the pictures are added to the gallery.\n\
         By hand: close {title}, copy app.ron into apps/{program} and the clipart folder into clipart, both inside the\n\
         baseCreator data folder (macOS: ~/Library/Application Support/baseCreator, Windows: %APPDATA%\\baseCreator,\n\
         Linux: ~/.local/share/baseCreator).\n"
    );
    add(&mut zip, "README.txt", readme.as_bytes())?;
    zip.finish().map_err(|e| e.to_string())?;
    Ok(count)
}

/// What a restore found in the zip.
#[derive(Debug)]
pub struct Restored {
    /// The settings, as key / value strings, to be applied by the program.
    pub settings: BTreeMap<String, String>,
    /// Pictures added to the gallery (pictures that are already there are not added again).
    pub pictures: usize,
}

/// Limits of what a backup may hold (a zip can claim anything).
const MAX_SETTINGS: u64 = 8_000_000;
const MAX_PICTURE: u64 = 20_000_000;
const MAX_TOTAL: u64 = 500_000_000;
const MAX_ENTRIES: usize = 50_000;

/// Read the settings of the backup in `src` and add its pictures to `gallery`. A backup made by another program
/// is refused; one without a program mark (made by LightCreator before it was shared) is accepted.
pub fn restore(src: &Path, title: &str, gallery: &Path) -> Result<Restored, String> {
    let program = title.to_lowercase();
    let file = std::fs::File::open(src).map_err(|e| e.to_string())?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;
    // Settings first: a file without them is not one of ours.
    let mut settings: BTreeMap<String, String> = {
        let f = zip.by_name(SETTINGS).map_err(|_| format!("{SETTINGS} not found in the file"))?;
        let mut text = String::new();
        f.take(MAX_SETTINGS + 1).read_to_string(&mut text).map_err(|e| e.to_string())?;
        if text.len() as u64 > MAX_SETTINGS {
            return Err("the settings in the backup are too big".into());
        }
        ron::from_str(&text).map_err(|e| e.to_string())?
    };
    if let Some(p) = settings.remove(PROGRAM_KEY) {
        if p != program {
            return Err(format!("this backup was made by {p}, not {program}"));
        }
    }
    // The pictures go to a temporary folder first and are merged from there.
    let tmp = std::env::temp_dir().join(format!("baseCreator-restore-{}-{}", std::process::id(), std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos()).unwrap_or(0)));
    let result: Result<usize, String> = (|| {
        if zip.len() > MAX_ENTRIES {
            return Err("the backup has too many files".into());
        }
        // The sizes in a zip can lie: count what is really written.
        let mut written = 0u64;
        for i in 0..zip.len() {
            let mut f = zip.by_index(i).map_err(|e| e.to_string())?;
            if f.is_dir() {
                continue;
            }
            // Only files below `clipart/`, and never outside the folder.
            let Some(rel) = f.enclosed_name().and_then(|p| p.strip_prefix("clipart").ok().map(Path::to_path_buf)) else { continue };
            if rel.as_os_str().is_empty() {
                continue;
            }
            let target = tmp.join(rel);
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut out = std::fs::File::create(&target).map_err(|e| format!("{}: {}", target.display(), e))?;
            let n = std::io::copy(&mut std::io::Read::take(&mut f, MAX_PICTURE + 1), &mut out).map_err(|e| e.to_string())?;
            written += n;
            if n > MAX_PICTURE || written > MAX_TOTAL {
                return Err("the pictures in the backup are too big".into());
            }
        }
        Ok(merge_gallery(&tmp, gallery))
    })();
    let _ = std::fs::remove_dir_all(&tmp);
    Ok(Restored { settings, pictures: result? })
}
