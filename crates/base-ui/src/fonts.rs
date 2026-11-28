//! The interface fonts, all taken from `assets/fonts` and embedded in the program:
//! Noto Sans (Latin, Greek, Cyrillic), Noto Sans Mono, Noto Sans SC (Chinese, also Japanese), Noto Sans KR cut to Hangul (Korean) and Noto Sans Devanagari (Hindi);
//! and the typefaces to choose for the G-code editor: JetBrains Mono, IBM Plex Mono and Fira Mono.
use egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const SANS: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf");
const MONO: &[u8] = include_bytes!("../../../assets/fonts/NotoSansMono-Regular.ttf");
const SC: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSC-Regular.otf");
const KR: &[u8] = include_bytes!("../../../assets/fonts/NotoSansKR-Hangul.otf");
const DEVANAGARI: &[u8] = include_bytes!("../../../assets/fonts/NotoSansDevanagari-Regular.ttf");
const JETBRAINS: &[u8] = include_bytes!("../../../assets/fonts/JetBrainsMono-Regular.ttf");
const PLEX: &[u8] = include_bytes!("../../../assets/fonts/IBMPlexMono-Regular.ttf");
const FIRA: &[u8] = include_bytes!("../../../assets/fonts/FiraMono-Regular.ttf");

/// The typefaces of the G-code editor: the id kept in the settings and the name shown. The first is the monospace font of the interface.
pub const EDITOR_FACES: [(&str, &str); 4] = [("noto", "Noto Sans Mono"), ("jetbrains", "JetBrains Mono"), ("plex", "IBM Plex Mono"), ("fira", "Fira Mono")];

/// The family of a typeface of the G-code editor (an id of [`EDITOR_FACES`]).
pub fn editor_family(id: &str) -> FontFamily {
    FontFamily::Name(format!("gcode-{id}").into())
}

/// Install the asset fonts as the only text fonts. egui's own symbol / emoji fonts stay at the very end of
/// the chain so that a stray symbol still shows instead of an empty box.
pub fn install(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    for (name, bytes) in [("noto-sans", SANS), ("noto-mono", MONO), ("noto-sc", SC), ("noto-kr", KR), ("noto-devanagari", DEVANAGARI), ("jetbrains-mono", JETBRAINS), ("plex-mono", PLEX), ("fira-mono", FIRA)] {
        defs.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    let symbols: Vec<String> = ["emoji-icon-font", "NotoEmoji-Regular"].iter().filter(|n| defs.font_data.contains_key(**n)).map(|n| n.to_string()).collect();
    let chain = |first: &str| -> Vec<String> {
        let mut v = vec![first.to_owned(), "noto-sans".into(), "noto-sc".into(), "noto-kr".into(), "noto-devanagari".into()];
        v.dedup();
        v.extend(symbols.iter().cloned());
        v
    };
    defs.families.insert(FontFamily::Proportional, chain("noto-sans"));
    defs.families.insert(FontFamily::Monospace, chain("noto-mono"));
    // The typefaces of the G-code editor, each with the fonts of the interface after it: a comment can name a layer in Chinese.
    for (id, font) in [("noto", "noto-mono"), ("jetbrains", "jetbrains-mono"), ("plex", "plex-mono"), ("fira", "fira-mono")] {
        let mut v = chain(font);
        v.insert(1, "noto-mono".into());
        v.dedup();
        defs.families.insert(editor_family(id), v);
    }
    ctx.set_fonts(defs);
}
