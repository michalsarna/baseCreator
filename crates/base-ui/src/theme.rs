//! Colour schemes: flat chrome modelled on VectorCraft with a red-orange accent.
//! Four neutral schemes (Dark, Light Dark, Medium Light, Light) and, for each colour, a dark and a light scheme
//! tinted with it (the same palettes with the greys shifted towards the colour and the accent in that colour).
//! Around a holiday (see `seasonal`) a dark and a light scheme of that holiday join the list; they are never offered
//! the rest of the year.
use egui::{self, Color32, CornerRadius, Stroke};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::OnceLock;

use crate::seasonal::Holiday;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Scheme {
    Dark,
    LightDark,
    MediumLight,
    Light,
    BlueDark,
    BlueLight,
    GreenDark,
    GreenLight,
    PurpleDark,
    PurpleLight,
    BrownDark,
    BrownLight,
    OrangeDark,
    OrangeLight,
    ChristmasDark,
    ChristmasLight,
    NewYearDark,
    NewYearLight,
    HalloweenDark,
    HalloweenLight,
    EasterDark,
    EasterLight,
}

impl Scheme {
    pub const ALL: [Scheme; 14] = [Scheme::Dark, Scheme::LightDark, Scheme::MediumLight, Scheme::Light, Scheme::BlueDark, Scheme::BlueLight, Scheme::GreenDark, Scheme::GreenLight, Scheme::PurpleDark, Scheme::PurpleLight, Scheme::BrownDark, Scheme::BrownLight, Scheme::OrangeDark, Scheme::OrangeLight];

    /// The schemes of the holidays: two for each, offered only around that holiday.
    pub const FESTIVE: [Scheme; 8] = [Scheme::ChristmasDark, Scheme::ChristmasLight, Scheme::NewYearDark, Scheme::NewYearLight, Scheme::HalloweenDark, Scheme::HalloweenLight, Scheme::EasterDark, Scheme::EasterLight];
    /// All of them; the position is the index of the palette.
    const EVERY: [Scheme; 22] = [
        Scheme::Dark, Scheme::LightDark, Scheme::MediumLight, Scheme::Light, Scheme::BlueDark, Scheme::BlueLight, Scheme::GreenDark, Scheme::GreenLight, Scheme::PurpleDark, Scheme::PurpleLight, Scheme::BrownDark, Scheme::BrownLight, Scheme::OrangeDark, Scheme::OrangeLight,
        Scheme::ChristmasDark, Scheme::ChristmasLight, Scheme::NewYearDark, Scheme::NewYearLight, Scheme::HalloweenDark, Scheme::HalloweenLight, Scheme::EasterDark, Scheme::EasterLight,
    ];

    /// The holiday of a festive scheme.
    pub fn holiday(self) -> Option<Holiday> {
        match self {
            Scheme::ChristmasDark | Scheme::ChristmasLight => Some(Holiday::Christmas),
            Scheme::NewYearDark | Scheme::NewYearLight => Some(Holiday::NewYearsEve),
            Scheme::HalloweenDark | Scheme::HalloweenLight => Some(Holiday::Halloween),
            Scheme::EasterDark | Scheme::EasterLight => Some(Holiday::Easter),
            _ => None,
        }
    }

    /// The schemes to offer on a date: the usual ones and, around a holiday, the two of that holiday.
    pub fn available_on(date: (i32, u32, u32)) -> Vec<Scheme> {
        let holiday = crate::seasonal::holiday_on(date.0, date.1, date.2);
        let festive = Scheme::FESTIVE.into_iter().filter(|s| holiday.is_some() && s.holiday() == holiday);
        Scheme::ALL.into_iter().chain(festive).collect()
    }

    /// The schemes to offer today.
    pub fn available() -> Vec<Scheme> {
        Scheme::available_on(crate::seasonal::today())
    }

    /// The usual scheme a festive one gives way to when its holiday is over.
    fn fallback(self) -> Scheme {
        if self.is_dark() { Scheme::LightDark } else { Scheme::Light }
    }

    /// The scheme to start with: the saved one, or the one of `BASECREATOR_SCHEME` (any scheme, to look at it out of
    /// season). A festive scheme saved at the holiday turns into the usual one of the same brightness afterwards.
    pub fn from_saved(saved: Option<&str>) -> Scheme {
        Scheme::from_saved_on(saved, std::env::var("BASECREATOR_SCHEME").ok().as_deref(), crate::seasonal::today())
    }

    fn from_saved_on(saved: Option<&str>, forced: Option<&str>, date: (i32, u32, u32)) -> Scheme {
        if let Some(s) = forced.and_then(Scheme::parse) {
            return s;
        }
        match saved.and_then(Scheme::parse) {
            Some(s) if s.holiday().is_some() && !Scheme::available_on(date).contains(&s) => s.fallback(),
            Some(s) => s,
            None => Scheme::LightDark,
        }
    }

    /// English label (translated through `i18n::tr`).
    pub fn label(self) -> &'static str {
        match self {
            Scheme::Dark => "Dark",
            Scheme::LightDark => "Light Dark",
            Scheme::MediumLight => "Medium Light",
            Scheme::Light => "Light",
            Scheme::OrangeDark => "Dark Orange",
            Scheme::OrangeLight => "Light Orange",
            Scheme::BrownDark => "Dark Brown",
            Scheme::BrownLight => "Light Brown",
            Scheme::PurpleDark => "Dark Purple",
            Scheme::PurpleLight => "Light Purple",
            Scheme::GreenDark => "Dark Green",
            Scheme::GreenLight => "Light Green",
            Scheme::BlueDark => "Dark Blue",
            Scheme::BlueLight => "Light Blue",
            Scheme::ChristmasDark => "Christmas (Dark)",
            Scheme::ChristmasLight => "Christmas (Light)",
            Scheme::NewYearDark => "New Year's Eve (Dark)",
            Scheme::NewYearLight => "New Year's Eve (Light)",
            Scheme::HalloweenDark => "Halloween (Dark)",
            Scheme::HalloweenLight => "Halloween (Light)",
            Scheme::EasterDark => "Easter (Dark)",
            Scheme::EasterLight => "Easter (Light)",
        }
    }
    pub fn id(self) -> &'static str {
        match self {
            Scheme::Dark => "dark",
            Scheme::LightDark => "light-dark",
            Scheme::MediumLight => "medium-light",
            Scheme::Light => "light",
            Scheme::OrangeDark => "orange-dark",
            Scheme::OrangeLight => "orange-light",
            Scheme::BrownDark => "brown-dark",
            Scheme::BrownLight => "brown-light",
            Scheme::PurpleDark => "purple-dark",
            Scheme::PurpleLight => "purple-light",
            Scheme::GreenDark => "green-dark",
            Scheme::GreenLight => "green-light",
            Scheme::BlueDark => "blue-dark",
            Scheme::BlueLight => "blue-light",
            Scheme::ChristmasDark => "christmas-dark",
            Scheme::ChristmasLight => "christmas-light",
            Scheme::NewYearDark => "new-year-dark",
            Scheme::NewYearLight => "new-year-light",
            Scheme::HalloweenDark => "halloween-dark",
            Scheme::HalloweenLight => "halloween-light",
            Scheme::EasterDark => "easter-dark",
            Scheme::EasterLight => "easter-light",
        }
    }
    /// The scheme with this id, unless it is a festive one whose holiday is not near.
    pub fn from_id(s: &str) -> Option<Scheme> {
        Scheme::parse(s).filter(|x| Scheme::available().contains(x))
    }
    fn parse(s: &str) -> Option<Scheme> {
        Scheme::EVERY.into_iter().find(|x| x.id() == s)
    }
    pub fn is_dark(self) -> bool {
        matches!(self, Scheme::Dark | Scheme::LightDark | Scheme::OrangeDark | Scheme::BrownDark | Scheme::PurpleDark | Scheme::GreenDark | Scheme::BlueDark | Scheme::ChristmasDark | Scheme::NewYearDark | Scheme::HalloweenDark | Scheme::EasterDark)
    }
    fn index(self) -> u8 {
        Scheme::EVERY.iter().position(|x| *x == self).unwrap_or(3) as u8
    }
    /// The palette of this scheme.
    fn palette(self) -> Palette {
        match self {
            Scheme::Dark => DARK,
            Scheme::LightDark => LIGHT_DARK,
            Scheme::MediumLight => MEDIUM_LIGHT,
            Scheme::Light => LIGHT,
            Scheme::OrangeDark => tinted(DARK, ORANGE, false),
            Scheme::OrangeLight => tinted(LIGHT, ORANGE, true),
            Scheme::BrownDark => tinted(DARK, BROWN, false),
            Scheme::BrownLight => tinted(LIGHT, BROWN, true),
            Scheme::PurpleDark => tinted(DARK, PURPLE, false),
            Scheme::PurpleLight => tinted(LIGHT, PURPLE, true),
            Scheme::GreenDark => tinted(DARK, GREEN, false),
            Scheme::GreenLight => tinted(LIGHT, GREEN, true),
            Scheme::BlueDark => tinted(DARK, BLUE, false),
            Scheme::BlueLight => tinted(LIGHT, BLUE, true),
            Scheme::ChristmasDark => CHRISTMAS_DARK,
            Scheme::ChristmasLight => CHRISTMAS_LIGHT,
            Scheme::NewYearDark => NEW_YEAR_DARK,
            Scheme::NewYearLight => NEW_YEAR_LIGHT,
            Scheme::HalloweenDark => HALLOWEEN_DARK,
            Scheme::HalloweenLight => HALLOWEEN_LIGHT,
            Scheme::EasterDark => EASTER_DARK,
            Scheme::EasterLight => EASTER_LIGHT,
        }
    }
}

#[derive(Clone, Copy)]
struct Palette {
    accent: Color32,
    panel: Color32,
    panel_dark: Color32,
    border: Color32,
    workspace: Color32,
    text: Color32,
    text_dim: Color32,
    widget: Color32,
    widget_weak: Color32,
    widget_hover: Color32,
    widget_hover_weak: Color32,
    input: Color32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color32 {
    Color32::from_rgb(r, g, b)
}

const DARK: Palette = Palette {
    accent: rgb(0xe8, 0x57, 0x3f),
    panel: rgb(0x26, 0x27, 0x2b),
    panel_dark: rgb(0x1e, 0x1f, 0x22),
    border: rgb(0x3a, 0x3b, 0x42),
    workspace: rgb(0x16, 0x17, 0x19),
    text: rgb(0xe6, 0xe6, 0xea),
    text_dim: rgb(0x93, 0x93, 0x9e),
    widget: rgb(0x34, 0x35, 0x3b),
    widget_weak: rgb(0x2f, 0x30, 0x35),
    widget_hover: rgb(0x44, 0x45, 0x4d),
    widget_hover_weak: rgb(0x3d, 0x3e, 0x45),
    input: rgb(0x1a, 0x1b, 0x1e),
};
const LIGHT_DARK: Palette = Palette {
    accent: rgb(0xe8, 0x57, 0x3f),
    panel: rgb(0x3c, 0x3d, 0x43),
    panel_dark: rgb(0x33, 0x34, 0x39),
    border: rgb(0x52, 0x53, 0x5b),
    workspace: rgb(0x2b, 0x2c, 0x30),
    text: rgb(0xec, 0xec, 0xf0),
    text_dim: rgb(0xa6, 0xa6, 0xb0),
    widget: rgb(0x4c, 0x4d, 0x55),
    widget_weak: rgb(0x46, 0x47, 0x4e),
    widget_hover: rgb(0x5a, 0x5b, 0x64),
    widget_hover_weak: rgb(0x54, 0x55, 0x5e),
    input: rgb(0x2f, 0x30, 0x34),
};
const MEDIUM_LIGHT: Palette = Palette {
    accent: rgb(0xe8, 0x57, 0x3f),
    panel: rgb(0xdd, 0xdd, 0xe1),
    panel_dark: rgb(0xcf, 0xcf, 0xd4),
    border: rgb(0xb4, 0xb4, 0xbb),
    workspace: rgb(0xb8, 0xb8, 0xbf),
    text: rgb(0x22, 0x22, 0x27),
    text_dim: rgb(0x5c, 0x5c, 0x66),
    widget: rgb(0xcb, 0xcb, 0xd1),
    widget_weak: rgb(0xd2, 0xd2, 0xd8),
    widget_hover: rgb(0xbd, 0xbd, 0xc5),
    widget_hover_weak: rgb(0xc4, 0xc4, 0xcc),
    input: rgb(0xf0, 0xf0, 0xf2),
};
const LIGHT: Palette = Palette {
    accent: rgb(0xe8, 0x57, 0x3f),
    panel: rgb(0xf4, 0xf4, 0xf5),
    panel_dark: rgb(0xe9, 0xe9, 0xeb),
    border: rgb(0xd2, 0xd2, 0xd6),
    workspace: rgb(0xdc, 0xdc, 0xe0),
    text: rgb(0x2b, 0x2b, 0x30),
    text_dim: rgb(0x80, 0x80, 0x88),
    widget: rgb(0xe4, 0xe4, 0xe8),
    widget_weak: rgb(0xea, 0xea, 0xee),
    widget_hover: rgb(0xd8, 0xd8, 0xde),
    widget_hover_weak: rgb(0xdc, 0xdc, 0xe2),
    input: Color32::WHITE,
};

// The schemes of the holidays. The accents are chosen so that the text stays readable on them (light text on the dark
// schemes, dark text on the light ones).
const CHRISTMAS_DARK: Palette = Palette {
    accent: rgb(0xd0, 0x38, 0x42),
    panel: rgb(0x1f, 0x2a, 0x24),
    panel_dark: rgb(0x18, 0x21, 0x1c),
    border: rgb(0x34, 0x49, 0x3c),
    workspace: rgb(0x12, 0x19, 0x15),
    text: rgb(0xea, 0xf2, 0xec),
    text_dim: rgb(0x95, 0xa8, 0x9b),
    widget: rgb(0x2d, 0x3d, 0x33),
    widget_weak: rgb(0x28, 0x35, 0x2d),
    widget_hover: rgb(0x3b, 0x4f, 0x42),
    widget_hover_weak: rgb(0x35, 0x47, 0x3c),
    input: rgb(0x16, 0x1e, 0x19),
};
const CHRISTMAS_LIGHT: Palette = Palette {
    accent: rgb(0xd9, 0x5a, 0x55),
    panel: rgb(0xf6, 0xf1, 0xea),
    panel_dark: rgb(0xeb, 0xe3, 0xd8),
    border: rgb(0xd4, 0xc8, 0xb8),
    workspace: rgb(0xdf, 0xd6, 0xc8),
    text: rgb(0x2a, 0x23, 0x23),
    text_dim: rgb(0x7c, 0x6e, 0x6e),
    widget: rgb(0xe8, 0xdf, 0xd2),
    widget_weak: rgb(0xee, 0xe6, 0xda),
    widget_hover: rgb(0xdc, 0xd0, 0xbf),
    widget_hover_weak: rgb(0xe1, 0xd6, 0xc6),
    input: Color32::WHITE,
};
const NEW_YEAR_DARK: Palette = Palette {
    accent: rgb(0xd4, 0xa2, 0x2f),
    panel: rgb(0x1b, 0x20, 0x36),
    panel_dark: rgb(0x14, 0x18, 0x30),
    border: rgb(0x30, 0x38, 0x5c),
    workspace: rgb(0x0f, 0x12, 0x26),
    text: rgb(0xee, 0xf0, 0xfa),
    text_dim: rgb(0x9a, 0xa0, 0xc4),
    widget: rgb(0x29, 0x30, 0x50),
    widget_weak: rgb(0x24, 0x2b, 0x48),
    widget_hover: rgb(0x37, 0x3f, 0x66),
    widget_hover_weak: rgb(0x31, 0x39, 0x5e),
    input: rgb(0x12, 0x16, 0x2b),
};
const NEW_YEAR_LIGHT: Palette = Palette {
    accent: rgb(0xd9, 0xa5, 0x2b),
    panel: rgb(0xf2, 0xf4, 0xfa),
    panel_dark: rgb(0xe5, 0xe9, 0xf3),
    border: rgb(0xc9, 0xd0, 0xe3),
    workspace: rgb(0xd5, 0xdb, 0xea),
    text: rgb(0x1d, 0x22, 0x38),
    text_dim: rgb(0x6c, 0x73, 0x92),
    widget: rgb(0xe1, 0xe6, 0xf2),
    widget_weak: rgb(0xe8, 0xec, 0xf6),
    widget_hover: rgb(0xd3, 0xda, 0xe9),
    widget_hover_weak: rgb(0xd9, 0xdf, 0xed),
    input: Color32::WHITE,
};
const HALLOWEEN_DARK: Palette = Palette {
    accent: rgb(0xf0, 0x7a, 0x14),
    panel: rgb(0x1c, 0x16, 0x20),
    panel_dark: rgb(0x15, 0x0f, 0x19),
    border: rgb(0x3a, 0x2c, 0x42),
    workspace: rgb(0x0e, 0x0a, 0x11),
    text: rgb(0xf0, 0xe8, 0xf2),
    text_dim: rgb(0xa5, 0x95, 0xad),
    widget: rgb(0x2c, 0x22, 0x33),
    widget_weak: rgb(0x26, 0x1d, 0x2c),
    widget_hover: rgb(0x3b, 0x2e, 0x44),
    widget_hover_weak: rgb(0x35, 0x29, 0x3d),
    input: rgb(0x12, 0x0d, 0x16),
};
const HALLOWEEN_LIGHT: Palette = Palette {
    accent: rgb(0xd9, 0x69, 0x0f),
    panel: rgb(0xef, 0xe7, 0xf3),
    panel_dark: rgb(0xe3, 0xd8, 0xea),
    border: rgb(0xcd, 0xbd, 0xd9),
    workspace: rgb(0xd9, 0xcd, 0xe3),
    text: rgb(0x2a, 0x1c, 0x33),
    text_dim: rgb(0x76, 0x65, 0x8a),
    widget: rgb(0xe5, 0xdb, 0xed),
    widget_weak: rgb(0xeb, 0xe2, 0xf1),
    widget_hover: rgb(0xd9, 0xcc, 0xe5),
    widget_hover_weak: rgb(0xdf, 0xd3, 0xea),
    input: rgb(0xff, 0xfc, 0xff),
};
const EASTER_DARK: Palette = Palette {
    accent: rgb(0x5f, 0xc0, 0x8a),
    panel: rgb(0x2a, 0x28, 0x33),
    panel_dark: rgb(0x22, 0x1f, 0x2b),
    border: rgb(0x43, 0x3f, 0x55),
    workspace: rgb(0x1a, 0x18, 0x22),
    text: rgb(0xf1, 0xee, 0xf7),
    text_dim: rgb(0xa7, 0xa0, 0xbb),
    widget: rgb(0x37, 0x34, 0x4a),
    widget_weak: rgb(0x31, 0x2e, 0x42),
    widget_hover: rgb(0x46, 0x42, 0x5e),
    widget_hover_weak: rgb(0x40, 0x3c, 0x57),
    input: rgb(0x1d, 0x1b, 0x26),
};
const EASTER_LIGHT: Palette = Palette {
    accent: rgb(0xe0, 0x67, 0x9a),
    panel: rgb(0xee, 0xf6, 0xee),
    panel_dark: rgb(0xe0, 0xee, 0xe1),
    border: rgb(0xc3, 0xd9, 0xc6),
    workspace: rgb(0xd0, 0xe3, 0xd3),
    text: rgb(0x22, 0x30, 0x2a),
    text_dim: rgb(0x62, 0x78, 0x6b),
    widget: rgb(0xdf, 0xee, 0xe1),
    widget_weak: rgb(0xe6, 0xf2, 0xe7),
    widget_hover: rgb(0xd1, 0xe4, 0xd4),
    widget_hover_weak: rgb(0xd7, 0xe9, 0xda),
    input: Color32::WHITE,
};

/// A colour for the tinted schemes: the colour that tints the greys and the accent used with it.
#[derive(Clone, Copy)]
struct Hue {
    tint: [f32; 3],
    accent: Color32,
}

/// `base` with every grey moved towards the colour of `hue` (keeping its brightness): `k` is how far for the
/// surfaces, the text moves a third of that. The accent is the colour's own.
fn tinted(base: Palette, hue: Hue, light: bool) -> Palette {
    let mean = (hue.tint[0] + hue.tint[1] + hue.tint[2]) / 3.0;
    let k = if light { 0.28 } else { 0.40 };
    let shift = |c: Color32, k: f32| {
        let f = |v: u8, t: f32| (v as f32 * (1.0 + k * (t / mean - 1.0))).round().clamp(0.0, 255.0) as u8;
        Color32::from_rgb(f(c.r(), hue.tint[0]), f(c.g(), hue.tint[1]), f(c.b(), hue.tint[2]))
    };
    Palette {
        accent: if light { Color32::from_rgb((hue.accent.r() as f32 * 0.82) as u8, (hue.accent.g() as f32 * 0.82) as u8, (hue.accent.b() as f32 * 0.82) as u8) } else { hue.accent },
        panel: shift(base.panel, k),
        panel_dark: shift(base.panel_dark, k),
        border: shift(base.border, k),
        workspace: shift(base.workspace, k),
        text: shift(base.text, k / 3.0),
        text_dim: shift(base.text_dim, k / 2.0),
        widget: shift(base.widget, k),
        widget_weak: shift(base.widget_weak, k),
        widget_hover: shift(base.widget_hover, k),
        widget_hover_weak: shift(base.widget_hover_weak, k),
        input: shift(base.input, k / 2.0),
    }
}

const BLUE: Hue = Hue { tint: [0.25, 0.45, 0.95], accent: rgb(0x4a, 0x8f, 0xf0) };

const GREEN: Hue = Hue { tint: [0.3, 0.8, 0.4], accent: rgb(0x3f, 0xb8, 0x60) };

const PURPLE: Hue = Hue { tint: [0.6, 0.4, 0.9], accent: rgb(0x9b, 0x6b, 0xe8) };

const BROWN: Hue = Hue { tint: [0.65, 0.45, 0.3], accent: rgb(0xc0, 0x8a, 0x5a) };

const ORANGE: Hue = Hue { tint: [0.95, 0.6, 0.2], accent: rgb(0xf0, 0x8c, 0x1e) };

static CURRENT: AtomicU8 = AtomicU8::new(3);
static PALETTES: OnceLock<Vec<Palette>> = OnceLock::new();

fn pal() -> &'static Palette {
    let all = PALETTES.get_or_init(|| Scheme::EVERY.iter().map(|s| s.palette()).collect());
    &all[(CURRENT.load(Ordering::Relaxed) as usize).min(all.len() - 1)]
}

pub fn accent() -> Color32 {
    pal().accent
}
pub fn panel() -> Color32 {
    pal().panel
}
pub fn panel_dark() -> Color32 {
    pal().panel_dark
}
pub fn border() -> Color32 {
    pal().border
}
pub fn workspace() -> Color32 {
    pal().workspace
}
pub fn text() -> Color32 {
    pal().text
}
/// Background of the console output: near black in the dark schemes, a warm paper tone in the light ones.
pub fn console_bg() -> Color32 {
    if Scheme::EVERY[(CURRENT.load(Ordering::Relaxed) as usize).min(Scheme::EVERY.len() - 1)].is_dark() {
        rgb(0x0f, 0x13, 0x1b)
    } else {
        rgb(0xfb, 0xf7, 0xe9)
    }
}
pub fn text_dim() -> Color32 {
    pal().text_dim
}

/// The colour schemes for the web page: id, name and the colours it needs, as `#rrggbb`.
pub fn web_themes() -> serde_json::Value {
    let hex = |c: Color32| format!("#{:02x}{:02x}{:02x}", c.r(), c.g(), c.b());
    Scheme::available()
        .iter()
        .map(|s| {
            let p = s.palette();
            serde_json::json!({
                "id": s.id(), "label": s.label(), "dark": s.is_dark(),
                "panel": hex(p.panel), "panel_dark": hex(p.panel_dark), "border": hex(p.border), "workspace": hex(p.workspace),
                "text": hex(p.text), "text_dim": hex(p.text_dim), "widget": hex(p.widget), "widget_hover": hex(p.widget_hover),
                "input": hex(p.input), "accent": hex(p.accent),
            })
        })
        .collect()
}

pub fn apply(ctx: &egui::Context, scheme: Scheme) {
    CURRENT.store(scheme.index(), Ordering::Relaxed);
    let p = pal();
    let accent = p.accent;
    let mut v = if scheme.is_dark() { egui::Visuals::dark() } else { egui::Visuals::light() };
    v.panel_fill = p.panel;
    v.window_fill = p.panel;
    v.extreme_bg_color = p.input;
    v.faint_bg_color = p.panel_dark;
    v.override_text_color = Some(p.text);
    v.window_stroke = Stroke::new(1.0, p.border);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.selection.bg_fill = accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, accent);
    v.hyperlink_color = accent;
    let r = CornerRadius::same(4);
    for w in [&mut v.widgets.noninteractive, &mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.corner_radius = r;
    }
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.inactive.bg_fill = p.widget;
    v.widgets.inactive.weak_bg_fill = p.widget_weak;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, p.border);
    v.widgets.hovered.bg_fill = p.widget_hover;
    v.widgets.hovered.weak_bg_fill = p.widget_hover_weak;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, accent.gamma_multiply(0.6));
    v.widgets.active.bg_fill = accent;
    v.widgets.active.weak_bg_fill = accent.gamma_multiply(0.8);
    v.widgets.active.bg_stroke = Stroke::new(1.0, accent);
    ctx.set_visuals(v);
    ctx.global_style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(6.0, 5.0);
        s.spacing.button_padding = egui::vec2(8.0, 3.0);
        s.spacing.interact_size.y = 22.0;
        // Wide enough that menu entries are not cut short with an ellipsis.
        s.spacing.menu_width = 280.0;
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scheme_has_its_own_id_label_and_palette() {
        for (i, a) in Scheme::EVERY.iter().enumerate() {
            assert_eq!(Scheme::parse(a.id()), Some(*a));
            assert_eq!(a.index() as usize, i);
            for b in &Scheme::EVERY[i + 1..] {
                assert!(a.id() != b.id() && a.label() != b.label());
            }
        }
    }

    #[test]
    fn the_web_page_gets_every_scheme_with_valid_colours() {
        let v = web_themes();
        let list = v.as_array().unwrap();
        assert_eq!(list.len(), Scheme::available().len());
        for t in list {
            for k in ["panel", "panel_dark", "border", "workspace", "text", "text_dim", "widget", "widget_hover", "input", "accent"] {
                let c = t[k].as_str().unwrap();
                assert!(c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|d| d.is_ascii_hexdigit()), "{k}: {c}");
            }
            assert!(Scheme::from_id(t["id"].as_str().unwrap()).is_some());
        }
    }

    #[test]
    fn text_is_readable_on_every_panel() {
        let lum = |c: Color32| 0.2126 * c.r() as f32 + 0.7152 * c.g() as f32 + 0.0722 * c.b() as f32;
        for s in Scheme::EVERY {
            let p = s.palette();
            assert!((lum(p.text) - lum(p.accent)).abs() > 55.0, "{}: text on the accent", s.id());
            assert!((lum(p.text) - lum(p.panel)).abs() > 110.0, "{}: text on the panel", s.id());
            assert!((lum(p.text) - lum(p.input)).abs() > 110.0, "{}: text in an input", s.id());
            assert_eq!(lum(p.panel) < 128.0, s.is_dark(), "{}: dark or light", s.id());
        }
    }

    #[test]
    fn the_festive_schemes_are_offered_only_around_their_holiday() {
        let usual = Scheme::ALL.to_vec();
        assert_eq!(Scheme::available_on((2026, 7, 1)), usual);
        let on = |d| Scheme::available_on(d).into_iter().filter(|s| s.holiday().is_some()).collect::<Vec<_>>();
        assert_eq!(on((2026, 12, 24)), vec![Scheme::ChristmasDark, Scheme::ChristmasLight]);
        assert_eq!(on((2026, 12, 30)), vec![Scheme::NewYearDark, Scheme::NewYearLight]);
        assert_eq!(on((2026, 10, 28)), vec![Scheme::HalloweenDark, Scheme::HalloweenLight]);
        assert_eq!(on((2026, 4, 4)), vec![Scheme::EasterDark, Scheme::EasterLight]);
        assert_eq!(Scheme::available_on((2026, 12, 24)).len(), Scheme::ALL.len() + 2);
    }

    #[test]
    fn a_festive_scheme_gives_way_to_a_usual_one_after_the_holiday() {
        let (xmas, summer) = ((2026, 12, 24), (2026, 7, 1));
        assert_eq!(Scheme::from_saved_on(Some("christmas-dark"), None, xmas), Scheme::ChristmasDark);
        assert_eq!(Scheme::from_saved_on(Some("christmas-dark"), None, summer), Scheme::LightDark);
        assert_eq!(Scheme::from_saved_on(Some("easter-light"), None, summer), Scheme::Light);
        assert_eq!(Scheme::from_saved_on(Some("blue-dark"), None, summer), Scheme::BlueDark);
        assert_eq!(Scheme::from_saved_on(Some("nonsense"), None, summer), Scheme::LightDark);
        assert_eq!(Scheme::from_saved_on(None, None, summer), Scheme::LightDark);
        assert_eq!(Scheme::from_saved_on(Some("blue-dark"), Some("halloween-light"), summer), Scheme::HalloweenLight);
    }
}
