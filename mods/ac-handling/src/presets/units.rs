//! The `[units]` table of a compound preset or a tuning file: which of the optional units start on for the car, and the
//! starting levels of the aids. The player's own settings override it (settings.rs).

use serde::Deserialize;

use crate::aids::{Levels, MAX_CLICKS, MAX_ESC_LEVEL, MAX_LEVEL};

/// The unit defaults of a preset or a car file: any subset of the keys.
#[derive(Debug, Clone, Default, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnitsTable {
    /// The suspension unit starts on.
    pub suspension: Option<bool>,
    /// The aids unit (traction control, ABS, stability control, brake bias) starts on.
    pub aids: Option<bool>,
    /// The steering unit starts on.
    pub steering: Option<bool>,
    /// The thermal unit starts on.
    pub thermal: Option<bool>,
    /// Traction control level, 0 to 5.
    pub tc: Option<u32>,
    /// ABS level, 0 to 5.
    pub abs: Option<u32>,
    /// Stability control level, 0 to 4.
    pub esc: Option<u32>,
    /// Brake bias in clicks, -20 to 20.
    pub brake_bias_clicks: Option<i32>,
}

impl UnitsTable {
    /// Check the levels: inside their ranges.
    pub fn validate(&self) -> Result<(), String> {
        for (name, value, max) in
            [("tc", self.tc, MAX_LEVEL), ("abs", self.abs, MAX_LEVEL), ("esc", self.esc, MAX_ESC_LEVEL)]
        {
            if let Some(level) = value.filter(|level| *level > max) {
                return Err(format!("units.{name} = {level} is outside 0 to {max}"));
            }
        }
        if let Some(clicks) = self.brake_bias_clicks.filter(|clicks| clicks.abs() > MAX_CLICKS) {
            return Err(format!("units.brake_bias_clicks = {clicks} is outside -{MAX_CLICKS} to {MAX_CLICKS}"));
        }
        Ok(())
    }
}

/// The resolved defaults of a car: the hard-coded ones, then the preset's, then the car file's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnitDefaults {
    /// The suspension unit.
    pub suspension: bool,
    /// The aids unit.
    pub aids: bool,
    /// The steering unit.
    pub steering: bool,
    /// The thermal unit.
    pub thermal: bool,
    /// The starting levels of the aids.
    pub levels: Levels,
}

impl Default for UnitDefaults {
    /// Suspension and thermal on, aids and steering off (they change how the controls feel: the player opts in).
    fn default() -> Self {
        UnitDefaults { suspension: true, aids: false, steering: false, thermal: true, levels: Levels::default() }
    }
}

impl UnitDefaults {
    /// These defaults with the keys a table sets replaced.
    pub fn with(mut self, table: &UnitsTable) -> UnitDefaults {
        self.suspension = table.suspension.unwrap_or(self.suspension);
        self.aids = table.aids.unwrap_or(self.aids);
        self.steering = table.steering.unwrap_or(self.steering);
        self.thermal = table.thermal.unwrap_or(self.thermal);
        self.levels.tc = table.tc.unwrap_or(self.levels.tc);
        self.levels.abs = table.abs.unwrap_or(self.levels.abs);
        self.levels.esc = table.esc.unwrap_or(self.levels.esc);
        self.levels.bias_clicks = table.brake_bias_clicks.unwrap_or(self.levels.bias_clicks);
        self
    }
}
