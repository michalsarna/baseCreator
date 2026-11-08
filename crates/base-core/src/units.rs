//! Units of length and speed used to show and enter numbers. Stored geometry is always in millimetres.
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SpeedUnit {
    #[default]
    PerSecond,
    PerMinute,
}

impl SpeedUnit {
    pub const ALL: [SpeedUnit; 2] = [SpeedUnit::PerSecond, SpeedUnit::PerMinute];
    /// English label with the units of length filled in (translated in the UI).
    pub fn label(self) -> &'static str {
        match self {
            SpeedUnit::PerSecond => "Per second (mm/s, in/s)",
            SpeedUnit::PerMinute => "Per minute (mm/min, in/min)",
        }
    }
    /// Suffix for a speed, with a leading space.
    pub fn suffix(self, u: Units) -> &'static str {
        match self {
            SpeedUnit::PerSecond => u.speed_s_suffix(),
            SpeedUnit::PerMinute => u.speed_min_suffix(),
        }
    }
    /// How many of the stored units (mm per second, or mm per minute when `stored_per_min`) make one displayed unit.
    pub fn factor(self, u: Units, stored_per_min: bool) -> f64 {
        let time = match (self, stored_per_min) {
            (SpeedUnit::PerSecond, false) | (SpeedUnit::PerMinute, true) => 1.0,
            (SpeedUnit::PerSecond, true) => 60.0,
            (SpeedUnit::PerMinute, false) => 1.0 / 60.0,
        };
        u.to_mm(1.0) * time
    }
}

/// A unit of length to show and enter lengths in. All stored geometry is in millimetres. A device works in millimetres or inches
/// (`DEVICE`), the G-code it gets is in its units; the program shows lengths in any of them (`ALL`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub enum Units {
    #[default]
    Mm,
    Inch,
    Cm,
    M,
    Foot,
}

impl Units {
    /// Every unit, for showing lengths: from the smallest metric one to the largest imperial one.
    pub const ALL: [Units; 5] = [Units::Mm, Units::Cm, Units::M, Units::Inch, Units::Foot];
    /// The units a machine works in: the G-code is written in them (G21 or G20).
    pub const DEVICE: [Units; 2] = [Units::Mm, Units::Inch];
    /// Millimetres in one unit.
    pub fn mm_per_unit(self) -> f64 {
        match self {
            Units::Mm => 1.0,
            Units::Cm => 10.0,
            Units::M => 1000.0,
            Units::Inch => 25.4,
            Units::Foot => 304.8,
        }
    }
    pub fn from_mm(self, mm: f64) -> f64 {
        mm / self.mm_per_unit()
    }
    pub fn to_mm(self, v: f64) -> f64 {
        v * self.mm_per_unit()
    }
    /// Metric (mm, cm, m) or imperial (inches, feet).
    pub fn is_metric(self) -> bool {
        matches!(self, Units::Mm | Units::Cm | Units::M)
    }
    /// The unit a machine of this system works in: millimetres for a metric one, inches for an imperial one.
    pub fn for_device(self) -> Units {
        if self.is_metric() { Units::Mm } else { Units::Inch }
    }
    /// The short name of the unit.
    pub fn symbol(self) -> &'static str {
        match self {
            Units::Mm => "mm",
            Units::Cm => "cm",
            Units::M => "m",
            Units::Inch => "in",
            Units::Foot => "ft",
        }
    }
    /// Suffix for lengths, with a leading space.
    pub fn suffix(self) -> &'static str {
        match self {
            Units::Mm => " mm",
            Units::Cm => " cm",
            Units::M => " m",
            Units::Inch => " in",
            Units::Foot => " ft",
        }
    }
    pub fn speed_s_suffix(self) -> &'static str {
        match self {
            Units::Mm => " mm/s",
            Units::Cm => " cm/s",
            Units::M => " m/s",
            Units::Inch => " in/s",
            Units::Foot => " ft/s",
        }
    }
    pub fn speed_min_suffix(self) -> &'static str {
        match self {
            Units::Mm => " mm/min",
            Units::Cm => " cm/min",
            Units::M => " m/min",
            Units::Inch => " in/min",
            Units::Foot => " ft/min",
        }
    }
    /// Decimal places that make sense when entering lengths.
    pub fn decimals(self) -> usize {
        match self {
            Units::Mm => 3,
            Units::Cm => 3,
            Units::M => 4,
            Units::Inch => 4,
            Units::Foot => 4,
        }
    }
    /// English label (translated in the UI).
    pub fn label(self) -> &'static str {
        match self {
            Units::Mm => "Millimetres (mm)",
            Units::Cm => "Centimetres (cm)",
            Units::M => "Metres (m)",
            Units::Inch => "Inches (in)",
            Units::Foot => "Feet (ft)",
        }
    }
}
