//! Shared installed prices for checkout and the original garage's financial fields.

mod items;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use anyhow::{Context, Result, ensure};
use game_install::GameDir;
use nfsmw_career::economy::shop::Item;
use nfsmw_data::car::{CarTables, customization::Category, shop::Prices};

use super::{Car, Selection, menu_rows::Row};

#[derive(Clone)]
pub struct Pricing {
    tables: Arc<CarTables>,
    prices: Arc<Prices>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Totals {
    pub cost: u64,
    pub trade_in: u64,
    pub net: i64,
    pub markers: usize,
}

impl Pricing {
    pub fn load(dir: &GameDir) -> Result<Self> {
        Ok(Self { tables: Arc::new(CarTables::load(dir)?), prices: Prices::load(dir)? })
    }

    /// Selected gross cost and installed credit. This does not validate cash or spend anything.
    pub(crate) fn item(
        &self,
        row: Row,
        selected: &Selection,
        installed: &Selection,
        car: &Car,
        backroom: bool,
    ) -> Result<Item> {
        let (entity, level, cost, trade_in, marker) = match row {
            Row::Part(category) => return self.visual(category, selected, installed, car, backroom),
            Row::Color(0) => return self.visual(Category::Paint, selected, installed, car, backroom),
            Row::Color(1) => return self.visual(Category::RimPaint, selected, installed, car, backroom),
            Row::Performance(category) => {
                let level = nfsmw_data::car::shop::global_level(
                    category,
                    car.performance.categories[category as usize].max_level,
                    selected.performance.get(category),
                )?;
                let entity = nfsmw_data::car::shop::performance_entity(category);
                (
                    entity,
                    level,
                    self.prices.equipment_cost(entity, level)?,
                    0,
                    (backroom && level > 0).then_some((items::performance_marker(category), 0)),
                )
            }
            Row::Junkman(category) => (
                nfsmw_data::car::shop::performance_entity(category),
                0,
                0,
                0,
                backroom.then_some((items::performance_marker(category), 0)),
            ),
            Row::Decal(zone, position) => {
                let level = self.level(selected.parts.decals[zone][position]);
                let entity = match zone {
                    0 | 1 => 44,
                    2 | 3 => 46,
                    _ => 48,
                };
                (
                    entity,
                    level,
                    self.prices.equipment_cost(entity, level)?,
                    0,
                    (backroom && level > 0).then_some((15, 0)),
                )
            }
            Row::Number(_) => {
                let level = self.level(selected.parts.decals[3][6]);
                ensure!(!backroom, "Door numbers have no backroom marker");
                (43, level, self.prices.equipment_cost(43, level)?, 0, None)
            }
            Row::Gauge => {
                let level = self.gauge_level(selected.gauges.skin)?;
                (17, level, self.prices.equipment_cost(17, level)?, 0, (backroom && level > 0).then_some((13, 0)))
            }
            _ => (0, 0, 0, 0, None),
        };
        Ok(Item {
            entity,
            level,
            cost: match backroom {
                true => 0,
                false => cost,
            },
            trade_in,
            marker,
        })
    }

    fn level(&self, hash: Option<u32>) -> u32 {
        hash.and_then(|hash| self.tables.parts.by_name_hash(hash)).map_or(0, |part| u32::from(part.upgrade_level))
    }

    fn visual(
        &self,
        category: Category,
        selected: &Selection,
        installed: &Selection,
        car: &Car,
        backroom: bool,
    ) -> Result<Item> {
        let catalog = car.parts.as_ref().context("Part catalog is unavailable")?;
        let hash = selected.parts.parts[category as usize].or(catalog.stock[category as usize]);
        let level = self.level(hash);
        let entity = nfsmw_data::car::shop::visual_entity(category);
        let marker = match backroom && level > 0 {
            true => Some((items::visual_marker(category).context("This category has no backroom marker")?, 0)),
            false => None,
        };
        let cost = match backroom || hash.is_none() || (Category::MODELS.contains(&category) && level == 0) {
            true => 0,
            false => self.prices.cost(entity, level)?,
        };
        let trade_in = match !backroom && Category::MODELS.contains(&category) {
            true => {
                installed.parts.parts[category as usize]
                    .and_then(|hash| self.tables.parts.by_name_hash(hash))
                    .map(|part| self.prices.cost(entity, u32::from(part.upgrade_level)))
                    .transpose()?
                    .unwrap_or(0)
                    / 4
            }
            false => 0,
        };
        Ok(Item { entity, level, cost, trade_in, marker })
    }

    fn gauge_level(&self, skin: u8) -> Result<u32> {
        if skin == 0 {
            return Ok(0);
        }
        self.tables
            .parts
            .parts()
            .iter()
            .find(|part| {
                part.part_id == 82
                    && self
                        .tables
                        .parts
                        .attributes(part)
                        .any(|(key, value)| key == 0x3111_51f8 && value == u32::from(skin))
            })
            .map(|part| u32::from(part.upgrade_level))
            .context("Installed gauge price tier is unavailable")
    }
}

impl Totals {
    pub fn of(items: &[Item]) -> Self {
        let cost = items.iter().map(|item| u64::from(item.cost)).sum();
        let trade_in = items.iter().map(|item| u64::from(item.trade_in)).sum();
        Self {
            cost,
            trade_in,
            net: cost as i64 - trade_in as i64,
            markers: items.iter().filter(|item| item.marker.is_some()).count(),
        }
    }
}
