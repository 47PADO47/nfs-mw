//! How the converted cars drive compared with the stock car each one clones: the `[performance]` table and the
//! `[car.<id>]` tables of `config.toml`, written into each car's manifest (`[performance]` of `cars/<id>.toml`,
//! docs/modding/car-customization.md).
//!
//! Every number is a multiplier of the clone's own value. The ranges are the ones the game accepts; a number outside
//! them would make the game refuse the car, so the settings are checked here first and the mod says which key is wrong.

use std::collections::BTreeMap;

use anyhow::{Result, bail};
use serde::Deserialize;

/// What a converted car's manifest asks of the game.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(deny_unknown_fields, default)]
pub struct Performance {
    /// Engine torque.
    pub power: f32,
    /// Body mass.
    pub mass: f32,
    /// Tyre grip.
    pub grip: f32,
    /// Brake torque.
    pub brakes: f32,
    /// The car has nitrous from the factory. The 2012 cars have it unless this is turned off.
    pub nitrous: bool,
    /// Torque the nitrous adds, as a multiplier of the kit's own.
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
    /// The `[performance]` table of a car manifest.
    pub fn manifest_table(&self) -> String {
        format!(
            "\n[performance]\npower = {:?}\nmass = {:?}\ngrip = {:?}\nbrakes = {:?}\nnitrous = {}\nnitrous_power = {:?}\n",
            self.power, self.mass, self.grip, self.brakes, self.nitrous, self.nitrous_power
        )
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
