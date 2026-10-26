//! The user's multipliers on the imported stats: the `[performance]` table and the `[car.<id>]` tables of `config.toml`.
//!
//! Each car gets its base stats from its own 2012 bundle ([`crate::stats`]). These multipliers are optional and change the
//! stats before they are written into the car's manifest (`[stats]` of `cars/<id>.toml`), so a car whose settings are left
//! alone is exactly the 2012 car, and `power = 1.15` is 15 percent more torque than the 2012 car has at every engine
//! speed. The game never sees the multipliers: they are applied here, at conversion time. The ranges keep the results
//! inside what the game's `[stats]` accepts, and the mod says which key is wrong.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::Deserialize;

use crate::stats::ImportedStats;

/// The multipliers on a converted car's stats.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Performance {
    /// Engine torque at every engine speed.
    pub power: f32,
    /// Body mass.
    pub mass: f32,
    /// Tyre grip.
    pub grip: f32,
    /// Brake torque.
    pub brakes: f32,
    /// The car keeps the nitrous its 2012 data gives it. Off removes it.
    pub nitrous: bool,
    /// Multiplier of the extra torque the nitrous adds.
    pub nitrous_power: f32,
}

impl Default for Performance {
    fn default() -> Self {
        Self { power: 1.0, mass: 1.0, grip: 1.0, brakes: 1.0, nitrous: true, nitrous_power: 1.0 }
    }
}

/// What a `[car.<id>]` table changes over the `[performance]` table, for the cars it names.
#[derive(Debug, Clone, Copy, PartialEq, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Change {
    power: Option<f32>,
    mass: Option<f32>,
    grip: Option<f32>,
    brakes: Option<f32>,
    nitrous: Option<bool>,
    nitrous_power: Option<f32>,
}

impl Change {
    fn over(&self, base: Performance) -> Performance {
        Performance {
            power: self.power.unwrap_or(base.power),
            mass: self.mass.unwrap_or(base.mass),
            grip: self.grip.unwrap_or(base.grip),
            brakes: self.brakes.unwrap_or(base.brakes),
            nitrous: self.nitrous.unwrap_or(base.nitrous),
            nitrous_power: self.nitrous_power.unwrap_or(base.nitrous_power),
        }
    }
}

/// The settings for every car: the general table, and the cars that differ from it.
#[derive(Debug, Clone, PartialEq)]
pub struct Tuning<'a> {
    pub general: Performance,
    pub cars: &'a BTreeMap<String, Change>,
}

impl Performance {
    /// Apply the multipliers to the car's imported stats. The nitrous the 2012 car has stays unless `nitrous` is off or
    /// `nitrous_power` is 0; `nitrous_power` scales its extra torque.
    pub fn apply(&self, stats: &mut ImportedStats) {
        stats.scale_torque(self.power);
        stats.scale_mass(self.mass);
        stats.scale_grip(self.grip);
        stats.scale_brakes(self.brakes);
        stats.nitrous_boost = match self.nitrous {
            true => (stats.nitrous_boost * self.nitrous_power).clamp(0.0, 3.0),
            false => 0.0,
        };
    }

    /// Refuse a number the game would refuse, naming the key.
    pub fn check(&self, table: &str) -> Result<()> {
        let ranges = [
            ("power", self.power, 0.25, 4.0),
            ("mass", self.mass, 0.25, 4.0),
            ("grip", self.grip, 0.25, 3.0),
            ("brakes", self.brakes, 0.25, 4.0),
            ("nitrous_power", self.nitrous_power, 0.0, 4.0),
        ];
        for (key, value, low, high) in ranges {
            if !value.is_finite() || value < low || value > high {
                bail!("{table}: {key} = {value} must be a number from {low} to {high}");
            }
        }
        Ok(())
    }
}

impl Tuning<'_> {
    /// The settings of the car with `id` and display `name`: the general table, then every `[car.<pattern>]` whose
    /// pattern matches the car (in the order of the patterns), each changing only the keys it names.
    pub fn for_car(&self, id: &str, name: &str) -> Performance {
        let matching = self.cars.iter().filter(|(pattern, _)| crate::config::matches(pattern, id, name));
        matching.fold(self.general, |performance, (_, change)| change.over(performance))
    }

    /// Check the general table and every `[car.<pattern>]` merged over it.
    pub fn check(&self) -> Result<()> {
        self.general.check("[performance]")?;
        for (pattern, change) in self.cars {
            change.over(self.general).check(&format!("[car.{pattern}]"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
