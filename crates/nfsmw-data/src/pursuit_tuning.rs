//! Per-context pursuit levels; units and fields are specified in `docs/formats/pursuit-data.md`.

use super::{HEAT_LEVELS, PursuitTable, level_collection};
use blackbox_attrib::Database;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LevelTuning {
    pub front_los: f32,
    pub rear_los: f32,
    pub heli_los: f32,
    pub speed_reaction: f32,
    pub evade_timeout: f32,
    pub hidden_multiplier: f32,
    pub heli_fuel: f32,
    pub heli_gap: f32,
    pub search_heli_chance: f32,
    pub roadblock_chance: f32,
    pub spike_chance: f32,
    pub search_roadblock_radius: f32,
    pub search_roadblock_chance: f32,
    pub search_city_mph: f32,
    pub search_highway_mph: f32,
    pub bust_speed_kmh: f32,
}

impl PursuitTable {
    pub fn tuning(self, db: &Database) -> Option<Vec<LevelTuning>> {
        (1..=HEAT_LEVELS)
            .map(|heat| {
                let row = level_collection(db, self.name(), heat)?;
                Some(LevelTuning {
                    front_los: row.get_f32("frontLOSdistance")?,
                    rear_los: row.get_f32("rearLOSdistance")?,
                    heli_los: row.get_f32("heliLOSdistance")?,
                    speed_reaction: row.get_f32("SpeedReactionTime")?,
                    evade_timeout: row.get_f32("evadetimeout")?,
                    hidden_multiplier: row.get_f32("HiddenZoneTimeMultiplier")?,
                    heli_fuel: row.get_f32("HeliFuelTime")?,
                    heli_gap: row.get_f32("TimeBetweenHeliActive")?,
                    search_heli_chance: row.get_f32("SearchModeHeliSpawnChance")?,
                    roadblock_chance: row.get_f32("roadblockprobability")?,
                    spike_chance: row.get_f32("roadblockspikechance")?,
                    search_roadblock_radius: row.get_f32("SearchModeRoadblockRadius")?,
                    search_roadblock_chance: row.get_f32("SearchModeRoadblockChance")?,
                    search_city_mph: row.get_f32("SearchModeCityMPH")?,
                    search_highway_mph: row.get_f32("SearchModeHwyMPH")?,
                    bust_speed_kmh: row.get_f32("BustSpeed")?,
                })
            })
            .collect()
    }
}
