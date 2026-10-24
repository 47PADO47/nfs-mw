//! Selected installed rows reach execution; non-stock mod values retain precedence.

use super::Police;
use crate::scenes::world::ai::support;
use crate::settings::helicopter::HeliSettings;
use blackbox_pursuit::helicopter::Tuning;
use blackbox_pursuit::roadblock::request::Odds;
use blackbox_pursuit::support::{Option_, Strategy};
use nfsmw_data::pursuit::{PursuitTable, SupportRecord};

fn strategy(raw: u32) -> Option<Strategy> {
    Some(match raw {
        1 => Strategy::EBrake,
        2 => Strategy::CoordinatedEBrake,
        3 => Strategy::Ram,
        4 => Strategy::HeavyRoadblock,
        5 => Strategy::CrossFollow,
        6 => Strategy::CrossBrake,
        7 => Strategy::CrossPlusVBlock,
        _ => return None,
    })
}

fn option(record: &SupportRecord) -> Option<Option_> {
    Some(Option_ { strategy: strategy(record.strategy)?, chance: record.chance, duration: record.duration })
}

impl Police {
    pub(in crate::scenes::world::ai) fn support_minimum_delay(&self, level: u32) -> f32 {
        self.support_row(level).map_or_else(|| support::minimum_delay(level), |row| row.minimum_delay)
    }

    pub(super) fn heavy_options(&self, level: u32) -> Vec<Option_> {
        self.support_row(level)
            .map_or_else(|| support::heavy_options(level), |row| row.heavy.iter().filter_map(option).collect())
    }

    pub(super) fn leader_options(&self, level: u32) -> Vec<(Option_, f32, f32)> {
        self.support_row(level).map_or_else(
            || support::leader_options(level),
            |row| {
                row.leaders
                    .iter()
                    .filter_map(|record| Some((option(record)?, record.priority_chance, record.priority_time)))
                    .collect()
            },
        )
    }

    pub(in crate::scenes::world::ai) fn big_suv_chance(&self, level: u32, kind: Strategy) -> f32 {
        self.support_row(level).map_or_else(
            || support::big_suv_chance(level, kind),
            |row| {
                row.heavy
                    .iter()
                    .find(|record| strategy(record.strategy) == Some(kind))
                    .map_or(0.0, |record| record.big_suv_chance)
            },
        )
    }

    pub(in crate::scenes::world::ai) fn roadblock_odds(&self, level: u32) -> Odds {
        self.level_tuning(level).map_or_else(
            || blackbox_pursuit::roadblock::request::odds(level),
            |row| Odds {
                probability: row.roadblock_chance,
                spike_chance: row.spike_chance,
                search_chance: row.search_roadblock_chance,
                search_radius: row.search_roadblock_radius,
            },
        )
    }

    pub(in crate::scenes::world::ai) fn pursuit_out_of_range(&self, level: u32, distance: f32, facing: f32) -> bool {
        self.level_tuning(level).map_or_else(
            || blackbox_pursuit::sight::out_of_range(level, distance, facing),
            |row| {
                distance >= row.front_los
                    || (distance >= row.rear_los && facing <= blackbox_pursuit::sight::REAR_CONE_DOT)
            },
        )
    }

    pub(super) fn heli_runtime_tuning(&self, level: u32) -> Tuning {
        let racing = self.pursuit_context().table() == PursuitTable::Race;
        let mut tuning = self.heli_settings.tuning(level, racing);
        let stock = HeliSettings::default().tuning(level, racing);
        if let Some(row) = self.level_tuning(level) {
            if tuning.fuel_time == stock.fuel_time {
                tuning.fuel_time = row.heli_fuel;
            }
            if tuning.line_of_sight == stock.line_of_sight {
                tuning.line_of_sight = row.heli_los;
            }
            tuning.search_radius = row.search_roadblock_radius;
        }
        tuning.skid_hit = self
            .support_row(level)
            .map_or_else(|| [5, 6, 7, 9, 10].contains(&level), |row| row.air.iter().any(|record| record.strategy == 2));
        tuning
    }

    pub(super) fn heli_respawn_seconds(&self, level: u32) -> f32 {
        let racing = self.pursuit_context().table() == PursuitTable::Race;
        let configured = self.heli_settings.respawn_secs(level, racing);
        let stock = HeliSettings::default().respawn_secs(level, racing);
        match configured == stock {
            true => self.level_tuning(level).map_or(configured, |row| row.heli_gap),
            false => configured,
        }
    }
}
