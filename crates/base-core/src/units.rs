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
        match (u, self) {
            (Units::Mm, SpeedUnit::PerSecond) => " mm/s",
            (Units::Mm, SpeedUnit::PerMinute) => " mm/min",
            (Units::Inch, SpeedUnit::PerSecond) => " in/s",
            (Units::Inch, SpeedUnit::PerMinute) => " in/min",
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

/// Unit system used to show and enter lengths in the interface. All stored geometry is in millimetres.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Units {
    #[default]
    Mm,
    Inch,
}

impl Units {
    pub const ALL: [Units; 2] = [Units::Mm, Units::Inch];
    pub fn from_mm(self, mm: f64) -> f64 {
        match self {
            Units::Mm => mm,
            Units::Inch => mm / 25.4,
        }
    }
    pub fn to_mm(self, v: f64) -> f64 {
        match self {
            Units::Mm => v,
            Units::Inch => v * 25.4,
        }
    }
    /// Suffix for lengths, with a leading space.
    pub fn suffix(self) -> &'static str {
        match self {
            Units::Mm => " mm",
            Units::Inch => " in",
        }
    }
    pub fn speed_s_suffix(self) -> &'static str {
        match self {
            Units::Mm => " mm/s",
            Units::Inch => " in/s",
        }
    }
    pub fn speed_min_suffix(self) -> &'static str {
        match self {
            Units::Mm => " mm/min",
            Units::Inch => " in/min",
        }
    }
    /// Decimal places that make sense when entering lengths.
    pub fn decimals(self) -> usize {
        match self {
            Units::Mm => 3,
            Units::Inch => 4,
        }
    }
    /// English label (translated in the UI).
    pub fn label(self) -> &'static str {
        match self {
            Units::Mm => "Millimetres (mm)",
            Units::Inch => "Inches (in)",
        }
    }
}
