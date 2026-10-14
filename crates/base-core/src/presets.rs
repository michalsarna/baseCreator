//! Starting points for a new device: the usual sizes of hobby machines and of sheets. Nothing here is a promise about a
//! particular machine; read the machine from its controller (`$$`) for the real travel.

#[derive(Clone, Copy, Debug)]
pub struct Preset {
    pub name: &'static str,
    /// Travel of X, Y and Z, mm (Z is 0 for a plotter).
    pub area: [f64; 3],
    /// A speed of moves that such machines manage, mm/min.
    pub rapid: f64,
    /// A spindle speed range top, S units (0 for a plotter).
    pub spindle_max: f64,
}

/// Router and mill classes by their usual table size.
pub const MILL: &[Preset] = &[
    Preset { name: "3018 class (300 × 180)", area: [300.0, 180.0, 45.0], rapid: 1500.0, spindle_max: 10_000.0 },
    Preset { name: "3040 class (400 × 300)", area: [400.0, 300.0, 55.0], rapid: 2000.0, spindle_max: 12_000.0 },
    Preset { name: "6040 class (600 × 400)", area: [600.0, 400.0, 75.0], rapid: 3000.0, spindle_max: 24_000.0 },
    Preset { name: "1010 class (1000 × 1000)", area: [1000.0, 1000.0, 100.0], rapid: 5000.0, spindle_max: 24_000.0 },
    Preset { name: "1325 class (1300 × 2500)", area: [2500.0, 1300.0, 150.0], rapid: 8000.0, spindle_max: 24_000.0 },
];

/// Sheets of paper, landscape.
pub const PEN: &[Preset] = &[
    Preset { name: "A5 (210 × 148)", area: [210.0, 148.0, 0.0], rapid: 6000.0, spindle_max: 0.0 },
    Preset { name: "A4 (297 × 210)", area: [297.0, 210.0, 0.0], rapid: 6000.0, spindle_max: 0.0 },
    Preset { name: "Letter (279 × 216)", area: [279.0, 216.0, 0.0], rapid: 6000.0, spindle_max: 0.0 },
    Preset { name: "A3 (420 × 297)", area: [420.0, 297.0, 0.0], rapid: 6000.0, spindle_max: 0.0 },
    Preset { name: "A2 (594 × 420)", area: [594.0, 420.0, 0.0], rapid: 6000.0, spindle_max: 0.0 },
];

/// A preset to show in a list: the ones of the program and the ones plugins bring.
#[derive(Clone, Debug, PartialEq)]
pub struct PresetItem {
    pub name: String,
    pub area: [f64; 3],
    pub rapid: f64,
    pub spindle_max: f64,
    /// From a plugin (its id), or `None` for the program's own.
    pub plugin: Option<String>,
}

/// The built-in presets of a class followed by those of the plugins that are on and for this program.
pub fn with_plugins(builtin: &[Preset], reg: &crate::plugins::Registry, app: &str, class: crate::plugins::PresetClass) -> Vec<PresetItem> {
    let mut v: Vec<PresetItem> = builtin.iter().map(|p| PresetItem { name: p.name.to_string(), area: p.area, rapid: p.rapid, spindle_max: p.spindle_max, plugin: None }).collect();
    for p in reg.plugins.iter().filter(|p| p.enabled && p.manifest.is_for(app)) {
        for d in p.manifest.presets.iter().filter(|d| d.class == class) {
            v.push(PresetItem { name: d.name.clone(), area: d.area, rapid: d.rapid, spindle_max: d.spindle_max, plugin: Some(p.manifest.id.clone()) });
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sizes_are_sane() {
        for p in MILL.iter().chain(PEN) {
            assert!(p.area[0] >= p.area[1] && p.area[1] >= 100.0 && p.rapid > 0.0, "{}", p.name);
        }
        assert!(MILL.iter().all(|p| p.area[2] > 0.0 && p.spindle_max > 0.0));
        assert!(PEN.iter().all(|p| p.area[2] == 0.0));
    }
}
