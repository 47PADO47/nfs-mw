//! Original frontend performance ratings, independent of physical driving and AI planning.
//! Spec: `docs/specs/garage-performance-bars.md`.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Result, ensure};
use blackbox_attrib::{Database, vlt_hash};
use game_install::GameDir;
use glam::Vec3;

use super::{PerformanceCatalog, PerformancePackage, bounds::CarBounds, car_physics};
use crate::shared;

mod estimate;

const EPSILON: f32 = 1e-6;
/// Rows: tires, brakes, suspension, transmission, engine, induction, nitrous.
/// Columns: top speed, handling, acceleration.
const WEIGHTS: [[f32; 3]; 7] = [
    [0.25, 1.5, 0.25],
    [0.0, 0.5, 0.0],
    [0.25, 1.0, 0.2],
    [1.0, 0.0, 0.75],
    [0.5, 0.0, 1.0],
    [0.25, 0.0, 1.25],
    [0.25, 0.0, 1.5],
];

/// Normalized frontend scores of a car's original equipment and maximum ordinary packages.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RatingEndpoints {
    pub stock: [f32; 3],
    pub maximum: [f32; 3],
    pub max_levels: [u8; 7],
}

impl RatingEndpoints {
    /// `[top speed, handling, acceleration]`, including the native Junkman endpoint/position adjustment.
    pub fn rating(self, package: PerformancePackage) -> [f32; 3] {
        let fractions = fractions(package, self.max_levels);
        std::array::from_fn(|axis| {
            let total: f32 = WEIGHTS.iter().map(|weights| weights[axis]).sum();
            let value: f32 = WEIGHTS.iter().zip(fractions).map(|(weights, value)| weights[axis] * value).sum();
            let junkman: f32 = WEIGHTS
                .iter()
                .enumerate()
                .filter(|(i, _)| package.junkman & (1 << i) != 0)
                .map(|(_, weights)| weights[axis])
                .sum();
            if total <= EPSILON {
                return self.stock[axis];
            }
            let mask = junkman / total;
            let high = self.maximum[axis] + (1.0 - self.maximum[axis]) * mask;
            let low = self.stock[axis] + (high - self.stock[axis]) * mask * 0.33;
            let position = ((1.0 + mask * 0.33) * value / total).min(1.0);
            (low + (high - low) * position).clamp(0.0, 1.0)
        })
    }
}

/// The native combined stock/max population, calculated once for the installed attribute database.
pub struct RatingBook {
    cars: HashMap<u32, RatingEndpoints>,
}

#[derive(Clone, Copy, Debug)]
struct Raw {
    top_speed: f32,
    handling: f32,
    time_to_100: f32,
}

struct Entry {
    key: u32,
    stock: Raw,
    maximum: Option<Raw>,
    max_levels: [u8; 7],
}

impl RatingBook {
    pub fn load(dir: &GameDir) -> Result<Arc<Self>> {
        let db = shared::attributes(dir)?;
        crate::memo(dir, &[shared::ATTRIBUTES_FILE], || Self::read(&db))
    }

    pub fn for_car(&self, car: &str) -> Option<RatingEndpoints> {
        self.cars.get(&vlt_hash(&car.to_ascii_lowercase())).copied()
    }

    pub fn len(&self) -> usize {
        self.cars.len()
    }
    pub fn is_empty(&self) -> bool {
        self.cars.is_empty()
    }

    fn read(db: &Database) -> Result<Self> {
        let mut entries = Vec::new();
        for vehicle in db.collections_of("pvehicle") {
            let Some(name) = vehicle.name() else { continue };
            let Some(model) = vehicle.get_string_key("MODEL") else { continue };
            if model.hash32 == 0 {
                continue;
            }
            let Some(low) = vehicle.get_at("HandlingRating", 0).and_then(|value| value.as_f32()) else { continue };
            let Some(high) = vehicle.get_at("HandlingRating", 1).and_then(|value| value.as_f32()) else { continue };
            if !low.is_finite() || !high.is_finite() || high <= low || high <= 0.0 {
                continue;
            }
            // The frontend estimate never reads collision dimensions, transforms or contacts.
            let bounds = CarBounds { half_dimensions: Vec3::ONE, pivot: Vec3::ZERO };
            let Ok(stock_physics) = car_physics(db, name, bounds) else { continue };
            let Some(stock) = estimate::estimate(&stock_physics.spec).map(|stats| Raw {
                top_speed: stats.top_speed,
                time_to_100: stats.time_to_100,
                handling: low,
            }) else {
                continue;
            };
            let Ok(catalog) = PerformanceCatalog::read(db, name) else { continue };
            let package = catalog.maximum();
            let mut maximum_physics = stock_physics;
            let maximum = super::performance::apply(db, name, package, &mut maximum_physics)
                .ok()
                .and_then(|_| estimate::estimate(&maximum_physics.spec))
                .map(|stats| Raw {
                    top_speed: stats.top_speed,
                    time_to_100: stats.time_to_100,
                    handling: low + (high - low) * handling_fraction(package, package.levels),
                });
            entries.push(Entry { key: vehicle.key(), stock, maximum, max_levels: package.levels });
        }
        ensure!(!entries.is_empty(), "No installed vehicles have valid frontend performance ratings");
        Ok(Self::from_entries(&entries))
    }

    fn from_entries(entries: &[Entry]) -> Self {
        let all: Vec<_> = entries.iter().flat_map(|entry| std::iter::once(entry.stock).chain(entry.maximum)).collect();
        let ranges = [range(&all, |s| s.top_speed), range(&all, |s| s.handling), range(&all, |s| s.time_to_100)];
        let normalize = |raw: Raw| {
            [
                normalized(raw.top_speed, ranges[0]),
                normalized(raw.handling, ranges[1]),
                1.0 - normalized(raw.time_to_100, ranges[2]),
            ]
        };
        let cars = entries
            .iter()
            .map(|entry| {
                (
                    entry.key,
                    RatingEndpoints {
                        stock: normalize(entry.stock),
                        maximum: normalize(entry.maximum.unwrap_or(entry.stock)),
                        max_levels: entry.max_levels,
                    },
                )
            })
            .collect();
        Self { cars }
    }
}

fn fractions(package: PerformancePackage, maximum: [u8; 7]) -> [f32; 7] {
    std::array::from_fn(|i| match maximum[i] {
        0 => 0.0,
        max => (f32::from(package.levels[i]) / f32::from(max)).clamp(0.0, 1.0),
    })
}

fn handling_fraction(package: PerformancePackage, maximum: [u8; 7]) -> f32 {
    let fractions = fractions(package, maximum);
    let total: f32 = WEIGHTS.iter().map(|w| w[1]).sum();
    WEIGHTS.iter().zip(fractions).map(|(w, p)| w[1] * p).sum::<f32>() / total
}

fn range(values: &[Raw], select: impl Fn(Raw) -> f32) -> (f32, f32) {
    values
        .iter()
        .map(|s| select(*s))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(low, high), value| (low.min(value), high.max(value)))
}

fn normalized(value: f32, (low, high): (f32, f32)) -> f32 {
    if high - low <= EPSILON {
        return 0.0;
    }
    ((value - low) / (high - low)).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests;
