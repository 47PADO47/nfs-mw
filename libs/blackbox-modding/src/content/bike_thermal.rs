//! The `[bike.front.thermal]` and `[bike.rear.thermal]` tables: the temperature and wear of a tyre
//! (`docs/specs/ride6-tyre-thermal.md`), in the units of `blackbox_vehicle::bike::TyreThermal`.

use blackbox_vehicle::bike::{TyreSection, TyreThermal};
use serde::Deserialize;

use super::ContentError;
use super::bike_check::{all_ranged, field, ranged};

fn pairs(points: &[[f32; 2]]) -> Vec<(f32, f32)> {
    points.iter().map(|&[x, y]| (x, y)).collect()
}

/// One strip of the tread (`[[bike.front.thermal.sections]]`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ThermalSection {
    /// The lowest efficiency the strip may give.
    pub efficiency_limit: f32,
    pub static_coef: f32,
    pub dynamic_coef: f32,
    pub lat_slip_coef: f32,
    pub long_slip_coef: f32,
    pub rolling_coef: f32,
    /// `[deg C, wear rate]`, `[wear, efficiency]` and `[deg C, efficiency]`; a missing curve counts as 1.
    #[serde(default)]
    pub temperature_to_wear: Vec<[f32; 2]>,
    #[serde(default)]
    pub wear_to_efficiency: Vec<[f32; 2]>,
    #[serde(default)]
    pub temperature_to_efficiency: Vec<[f32; 2]>,
}

/// The temperature and wear of a tyre.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Thermal {
    /// The strips of the tread, left to right.
    pub sections: Vec<ThermalSection>,
    pub sections_dependency: f32,
    pub sections_angle_rad: f32,
    /// The heat transfer between neighbouring strips; 0 (the default) for none.
    #[serde(default)]
    pub heat_transfer: f32,
    pub initial_thickness: f32,
    pub min_temperature_c: f32,
    pub max_temperature_c: f32,
    pub asphalt_importance: f32,
    pub wear_length_m: f32,
    pub uniform: bool,
    pub static_time_scale: f32,
    pub dynamic_time_scale: f32,
    pub decay_distance_factor: f32,
    pub wear_intensity: f32,
}

impl Thermal {
    pub(super) fn to_spec(&self) -> TyreThermal {
        TyreThermal {
            sections: self
                .sections
                .iter()
                .map(|s| TyreSection {
                    efficiency_limit: s.efficiency_limit,
                    static_coef: s.static_coef,
                    dynamic_coef: s.dynamic_coef,
                    lat_slip_coef: s.lat_slip_coef,
                    long_slip_coef: s.long_slip_coef,
                    rolling_coef: s.rolling_coef,
                    temperature_to_wear: pairs(&s.temperature_to_wear),
                    wear_to_efficiency: pairs(&s.wear_to_efficiency),
                    temperature_to_efficiency: pairs(&s.temperature_to_efficiency),
                })
                .collect(),
            sections_dependency: self.sections_dependency,
            sections_angle: self.sections_angle_rad,
            heat_transfer: self.heat_transfer,
            initial_thickness: self.initial_thickness,
            min_temperature: self.min_temperature_c,
            max_temperature: self.max_temperature_c,
            asphalt_importance: self.asphalt_importance,
            wear_length: self.wear_length_m,
            uniform: self.uniform,
            static_time_scale: self.static_time_scale,
            dynamic_time_scale: self.dynamic_time_scale,
            decay_distance_factor: self.decay_distance_factor,
            wear_intensity: self.wear_intensity,
        }
    }

    /// The ranges of the numbers; `name` is the key of the table.
    pub(super) fn check(&self, name: &'static str) -> Result<(), ContentError> {
        if self.sections.is_empty() || self.sections.len() > 9 {
            return Err(field(name, "a tyre has 1 to 9 strips"));
        }
        for section in &self.sections {
            ranged(section.efficiency_limit, name, 0.0, 1.0)?;
            all_ranged(&[section.static_coef, section.dynamic_coef], name, 0.0, 10.0)?;
            all_ranged(&[section.lat_slip_coef, section.long_slip_coef, section.rolling_coef], name, 0.0, 1.0)?;
            for curve in [&section.temperature_to_wear, &section.wear_to_efficiency, &section.temperature_to_efficiency]
            {
                if curve.len() > 32 || curve.windows(2).any(|pair| pair[1][0] <= pair[0][0]) {
                    return Err(field(name, "a curve has up to 32 points with the first number rising"));
                }
                all_ranged(&curve.concat(), name, -1_000.0, 1_000.0)?;
            }
        }
        all_ranged(&[self.sections_dependency, self.heat_transfer, self.wear_intensity], name, 0.0, 100.0)?;
        ranged(self.sections_angle_rad, name, 0.0, 3.2)?;
        ranged(self.initial_thickness, name, 0.0, 100.0)?;
        all_ranged(&[self.min_temperature_c, self.max_temperature_c], name, -100.0, 1_000.0)?;
        if self.max_temperature_c <= self.min_temperature_c {
            return Err(field(name, "the ceiling temperature is above the floor"));
        }
        ranged(self.asphalt_importance, name, 0.0, 1.0)?;
        ranged(self.wear_length_m, name, 0.0, 1.0e7)?;
        all_ranged(&[self.static_time_scale, self.dynamic_time_scale, self.decay_distance_factor], name, 0.0, 1.0e3)
    }
}
