use anyhow::{Context, Result, ensure};
use nfsmw_career::economy::shop::Item;
use nfsmw_data::car::{customization::Category, physics::PerformanceCategory};

use super::{Car, Pricing, Row, Selection};
use crate::garage::options::{Store, VehicleOptions};
use crate::garage::rider_gear;

pub(super) fn performance_marker(category: PerformanceCategory) -> u32 {
    match category {
        PerformanceCategory::Brakes => 1,
        PerformanceCategory::Engine => 2,
        PerformanceCategory::Nitrous => 3,
        PerformanceCategory::Induction => 4,
        PerformanceCategory::Chassis => 5,
        PerformanceCategory::Tires => 6,
        PerformanceCategory::Transmission => 7,
    }
}

pub(super) fn visual_marker(category: Category) -> Option<u32> {
    match category {
        Category::Body => Some(8),
        Category::Hood => Some(9),
        Category::Spoiler => Some(10),
        Category::Wheels => Some(11),
        Category::Roof => Some(12),
        Category::Vinyl => Some(14),
        Category::Paint => Some(16),
        _ => None,
    }
}

pub(super) fn visual_changed(category: Category, old: &Selection, new: &Selection) -> bool {
    if old.parts.parts[category as usize] != new.parts.parts[category as usize] {
        return true;
    }
    match category {
        Category::Paint => old.color != new.color,
        Category::RimPaint => old.parts.rim_color != new.parts.rim_color,
        Category::Vinyl => {
            old.parts.vinyl_colors != new.parts.vinyl_colors
                || old.parts.vinyl_color_parts != new.parts.vinyl_color_parts
        }
        _ => false,
    }
}

impl Pricing {
    pub(crate) fn items(
        &self,
        car: &Car,
        installed: &Selection,
        selected: &Selection,
        backroom: bool,
        free_junkman: bool,
    ) -> Result<Vec<Item>> {
        ensure!(selected.car == installed.car, "The shop cannot change the current career car");
        let mut result = Vec::new();
        for category in Category::ALL {
            if visual_changed(category, installed, selected) {
                result.push(self.item(Row::Part(category), selected, installed, car, backroom)?);
            }
        }
        for category in PerformanceCategory::ALL {
            if installed.performance.get(category) != selected.performance.get(category) {
                result.push(self.item(Row::Performance(category), selected, installed, car, backroom)?);
            }
            if selected.performance.has_junkman(category) && !installed.performance.has_junkman(category) {
                ensure!(
                    car.performance.junkman_available(category, selected.performance),
                    "This equipment cannot accept a Junkman package"
                );
                ensure!(
                    backroom || free_junkman,
                    "Junkman requires a backroom marker or a previously completed profile"
                );
                result.push(self.item(Row::Junkman(category), selected, installed, car, backroom)?);
            }
        }
        for zone in 0..6 {
            let limit = match zone {
                2 | 3 => 6,
                _ => 8,
            };
            for position in 0..limit {
                if installed.parts.decals[zone][position] != selected.parts.decals[zone][position] {
                    result.push(self.item(Row::Decal(zone, position), selected, installed, car, backroom)?);
                }
            }
        }
        if installed.parts.decals[3][6..8] != selected.parts.decals[3][6..8] {
            result.push(self.item(Row::Number(0), selected, installed, car, backroom)?);
        }
        if installed.gauges != selected.gauges {
            result.push(self.item(Row::Gauge, selected, installed, car, backroom)?);
        }
        self.component_items(car, installed, selected, backroom, &mut result)?;
        Ok(result)
    }
}

impl Pricing {
    /// The items only a vehicle's components and the rider's gear make: a component slot the garage has no category for, and each
    /// part of the gear that differs. Neither is sold in the backroom (a marker is a model category's or a level's).
    fn component_items(
        &self,
        car: &Car,
        installed: &Selection,
        selected: &Selection,
        backroom: bool,
        result: &mut Vec<Item>,
    ) -> Result<()> {
        if backroom {
            ensure!(installed.gear == selected.gear, "The backroom does not sell rider gear");
            ensure!(installed.parts.slots == selected.parts.slots, "The backroom does not sell component parts");
            return Ok(());
        }
        let before = VehicleOptions::of_selection(car, installed);
        let after = VehicleOptions::of_selection(car, selected);
        if let (Some(before), Some(after)) = (&before, &after) {
            for (at, slot) in after.slots.iter().enumerate().filter(|(_, slot)| slot.store == Store::Session) {
                if before.slots.get(at).is_some_and(|old| old.current == slot.current) {
                    continue;
                }
                let at = u8::try_from(at).context("A vehicle has too many component slots")?;
                result.push(self.item(Row::Slot(at), selected, installed, car, backroom)?);
            }
        }
        for (part, _) in rider_gear::price::changes(&installed.gear, &selected.gear, rider_gear::owns) {
            result.push(self.item(Row::Gear(part), selected, installed, car, backroom)?);
        }
        Ok(())
    }
}
