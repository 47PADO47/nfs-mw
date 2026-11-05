//! Equipment named by the installed `presetride` collections.
use anyhow::{Result, ensure};
use blackbox_attrib::{Database, vlt_hash};
use blackbox_carparts::PresetRide;

use super::{PerformanceCatalog, PerformanceCategory, PerformancePackage};

/// A preset can share another car's geometry while naming a distinct vehicle record.
pub fn vehicle_name<'a>(db: &'a Database, ride: &'a PresetRide) -> &'a str {
    db.collection_by_key(vlt_hash("pvehicle"), ride.vehicle_key as u32)
        .and_then(|vehicle| vehicle.name())
        .unwrap_or(&ride.car_type_name)
}

/// Ordinary installed levels; these records do not specify Junkman parts.
pub fn read(db: &Database, car: &str, name: &str) -> Result<Option<PerformancePackage>> {
    let Some(record) = db.collection("presetride", &name.to_ascii_lowercase()) else { return Ok(None) };
    let mut package = PerformancePackage::default();
    let catalog = PerformanceCatalog::read(db, car)?;
    for category in PerformanceCategory::ALL {
        let level = record.get_i32(category.field()).unwrap_or(0);
        ensure!((0..=u8::MAX as i32).contains(&level), "invalid preset {} level {level}", category.field());
        package.levels[category as usize] = (level as u8).min(catalog.categories[category as usize].max_level);
    }
    Ok(Some(package))
}
