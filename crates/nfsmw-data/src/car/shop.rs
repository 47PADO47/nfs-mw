//! Installed garage prices. Behavioral contract: `docs/specs/garage-shop.md`.
use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use blackbox_attrib::{Database, Value};
use game_install::GameDir;

use super::{customization::Category, physics::PerformanceCategory};

const PART_NAME: u32 = 0x3b87_4697;
const PART_INFO: u32 = 0x1002_ea65;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Price {
    pub level: u32,
    pub cost: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Prices {
    entries: HashMap<u32, Vec<Price>>,
}

impl Prices {
    pub fn load(dir: &GameDir) -> Result<Arc<Self>> {
        crate::memo(dir, &[crate::shared::ATTRIBUTES_FILE, crate::career_cars::FE_ATTRIB_FILE], || {
            let attributes = crate::shared::attributes(dir)?;
            let mut db = Database::clone(&attributes);
            db.load(&crate::read_unwrapped(dir, crate::career_cars::FE_ATTRIB_FILE)?)?;
            Self::read(&db)
        })
    }

    pub fn read(db: &Database) -> Result<Self> {
        let mut entries = HashMap::new();
        for collection in db.collections_of("frontend") {
            if collection.parent().is_none_or(|parent| parent.key() != blackbox_attrib::vlt_hash("carparts")) {
                continue;
            }
            let Some(entity) = collection.get_by_key(PART_NAME).and_then(entity_id) else { continue };
            let Some(values) = collection.get_by_key(PART_INFO) else { continue };
            let values = values.as_array().unwrap_or(std::slice::from_ref(values));
            let prices = values.iter().map(decode_price).collect::<Result<Vec<_>>>()?;
            if prices.is_empty() {
                continue;
            }
            // A category is represented by one child of carparts, not by every inherited collection.
            if collection.data().own(PART_INFO).is_none() {
                continue;
            }
            ensure!(entries.insert(entity, prices).is_none(), "Duplicate installed garage price category {entity}");
        }
        ensure!(!entries.is_empty(), "Installed garage prices are unavailable");
        Ok(Self { entries })
    }

    pub fn cost(&self, entity: u32, level: u32) -> Result<u32> {
        self.entries
            .get(&entity)
            .and_then(|prices| prices.iter().find(|p| p.level == level))
            .map(|p| p.cost)
            .with_context(|| format!("Installed price is unavailable for category {entity}, level {level}"))
    }

    /// Original equipment restoration has no purchase charge.
    pub fn equipment_cost(&self, entity: u32, level: u32) -> Result<u32> {
        if level == 0 {
            return Ok(0);
        }
        self.cost(entity, level)
    }
}

fn decode_price(value: &Value) -> Result<Price> {
    let Value::Raw { bytes, .. } = value else { anyhow::bail!("Invalid installed garage price type") };
    ensure!(bytes.len() == 12, "Invalid installed garage price record length");
    let level = i32::from_le_bytes(bytes[..4].try_into().unwrap());
    let cost = f32::from_le_bytes(bytes[8..12].try_into().unwrap());
    ensure!(
        level >= 0 && cost.is_finite() && cost >= 0.0 && f64::from(cost) <= f64::from(u32::MAX),
        "Invalid installed garage price"
    );
    Ok(Price { level: level as u32, cost: cost as u32 })
}

fn entity_id(value: &Value) -> Option<u32> {
    if let Some(id) = value.as_u32() {
        return Some(id);
    }
    let Value::Raw { bytes, .. } = value else { return None };
    Some(u32::from_le_bytes(bytes.as_slice().try_into().ok()?))
}

pub fn visual_entity(category: Category) -> u32 {
    match category {
        Category::Body => 11,
        Category::Spoiler => 12,
        Category::Wheels => 13,
        Category::Hood => 14,
        Category::Roof => 15,
        Category::WindowTint => 18,
        Category::Paint | Category::RimPaint => 23,
        Category::Vinyl => 40,
    }
}

pub fn performance_entity(category: PerformanceCategory) -> u32 {
    4 + category as u32
}

pub fn global_level(category: PerformanceCategory, car_maximum: u8, selected: u8) -> Result<u32> {
    if selected == 0 {
        return Ok(0);
    }
    let maximum: u8 = match category {
        PerformanceCategory::Engine | PerformanceCategory::Transmission | PerformanceCategory::Brakes => 4,
        _ => 3,
    };
    ensure!(selected <= car_maximum && car_maximum <= maximum, "Performance stage has no installed shop tier");
    Ok(u32::from(maximum - car_maximum + selected))
}

#[cfg(test)]
#[path = "shop_tests.rs"]
mod tests;
