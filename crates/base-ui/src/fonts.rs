//! The interface fonts, all taken from `assets/fonts` and embedded in the program:
//! Noto Sans (Latin, Greek, Cyrillic), Noto Sans Mono, Noto Sans SC (Chinese) and Noto Sans Devanagari (Hindi).
use egui::{self, FontData, FontDefinitions, FontFamily};
use std::sync::Arc;

const SANS: &[u8] = include_bytes!("../../../assets/fonts/NotoSans-Regular.ttf");
const MONO: &[u8] = include_bytes!("../../../assets/fonts/NotoSansMono-Regular.ttf");
const SC: &[u8] = include_bytes!("../../../assets/fonts/NotoSansSC-Regular.otf");
const DEVANAGARI: &[u8] = include_bytes!("../../../assets/fonts/NotoSansDevanagari-Regular.ttf");

/// Install the asset fonts as the only text fonts. egui's own symbol / emoji fonts stay at the very end of
/// the chain so that a stray symbol still shows instead of an empty box.
pub fn install(ctx: &egui::Context) {
    let mut defs = FontDefinitions::default();
    for (name, bytes) in [("noto-sans", SANS), ("noto-mono", MONO), ("noto-sc", SC), ("noto-devanagari", DEVANAGARI)] {
        defs.font_data.insert(name.to_owned(), Arc::new(FontData::from_static(bytes)));
    }
    let symbols: Vec<String> = ["emoji-icon-font", "NotoEmoji-Regular"].iter().filter(|n| defs.font_data.contains_key(**n)).map(|n| n.to_string()).collect();
    let chain = |first: &str| -> Vec<String> {
        let mut v = vec![first.to_owned(), "noto-sans".into(), "noto-sc".into(), "noto-devanagari".into()];
        v.dedup();
        v.extend(symbols.iter().cloned());
        v
    };
    defs.families.insert(FontFamily::Proportional, chain("noto-sans"));
    defs.families.insert(FontFamily::Monospace, chain("noto-mono"));
    ctx.set_fonts(defs);
}
