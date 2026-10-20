//! The run-time parameters of a race and the opponent characters. Spec: `docs/formats/gameplay-races.md` sections 3.2,
//! 3.3 and 4; `docs/specs/race-mode.md`.

use blackbox_attrib::{CollectionRef, Database};

use super::{Placement, collection, inherits, instances, key_array, key_field, name_of, placement, text};

/// `CashValue` (`Float`, hash `0xD8BAA07B`): the cash a career win pays (`docs/specs/career-economy-flows.md` section 7).
/// Read by its hash, as the career crate documents it.
const CASH_VALUE_FIELD: u32 = 0xD8BA_A07B;

/// What a race asks of the run: the start, the clock, the opponents, the traffic and the rank rule.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceParams {
    /// `DoCountdown`: the 3, 2, 1 board with the cars held.
    pub do_countdown: bool,
    /// `RollingStart`: no countdown; every racer starts moving.
    pub rolling_start: bool,
    /// `InitialPlayerSpeed`: the start speed of a rolling start (assumed m/s).
    pub initial_speed: f32,
    /// `IsLoopingRace`: the laps run through the same checkpoints.
    pub looping: bool,
    /// `KnockoutsPerLap`: racers dropped at the end of each lap (knockout races).
    pub knockouts_per_lap: i32,
    /// `TimeLimit` in seconds, 0 for none.
    pub time_limit: f32,
    /// `OvertimePenaltyPerSec`: points lost per second over the limit (speed-trap races).
    pub overtime_penalty: f32,
    /// `DifficultyLevel`: 0..100; below 34 easy, 34 to 66 medium, 67 and above hard.
    pub difficulty: i32,
    /// `TrafficLevel`: 0..100 percent.
    pub traffic_level: i32,
    /// `ForceTrafficDensity` (hash `0x5E161BBA`): when above 0 the race sets the traffic density to it at the start
    /// (the executable's `StartRace` reads it; a drag race sets 0 instead).
    pub force_traffic_density: i32,
    /// `ForceHeatLevel` (hash `0xE4211F4F`): the heat the racers' cars start at, when above the current heat.
    pub force_heat_level: i32,
    /// `CopsInRace`.
    pub cops_in_race: bool,
    /// `PursuitRace`: the challenge races.
    pub pursuit_race: bool,
    /// `CatchUp`: the rubber band switch.
    pub catch_up: bool,
    /// `RankPlayersByPoints`: the speed-trap races rank by points.
    pub rank_by_points: bool,
    /// `TrafficPattern`, for example `drag`.
    pub traffic_pattern: Option<String>,
    /// `Opponents`: the keys of the opponent characters, in order.
    pub opponents: Vec<u32>,
    /// `SpeedTrapList`: the keys of the speed-trap triggers, in order.
    pub speed_traps: Vec<u32>,
    /// `Shortcuts`: the keys of the shortcut markers.
    pub shortcuts: Vec<u32>,
    /// `CashValue` (field `0xD8BAA07B`): the career win's cash.
    pub cash_value: f32,
}

impl Default for RaceParams {
    /// The values of the `basicrace` template.
    fn default() -> Self {
        Self {
            do_countdown: true,
            rolling_start: false,
            initial_speed: 0.0,
            looping: false,
            knockouts_per_lap: 0,
            time_limit: 0.0,
            overtime_penalty: 0.0,
            difficulty: 100,
            traffic_level: 100,
            force_traffic_density: 0,
            force_heat_level: 0,
            cops_in_race: false,
            pursuit_race: false,
            catch_up: true,
            rank_by_points: false,
            traffic_pattern: None,
            opponents: Vec::new(),
            speed_traps: Vec::new(),
            shortcuts: Vec::new(),
            cash_value: 0.0,
        }
    }
}

impl RaceParams {
    pub(super) fn read(c: CollectionRef<'_>) -> Self {
        let d = Self::default();
        Self {
            do_countdown: c.get_bool("DoCountdown").unwrap_or(d.do_countdown),
            rolling_start: c.get_bool("RollingStart").unwrap_or(d.rolling_start),
            initial_speed: c.get_f32("InitialPlayerSpeed").unwrap_or(d.initial_speed),
            looping: c.get_bool("IsLoopingRace").unwrap_or(d.looping),
            knockouts_per_lap: c.get_i32("KnockoutsPerLap").unwrap_or(d.knockouts_per_lap),
            time_limit: c.get_f32("TimeLimit").unwrap_or(d.time_limit),
            overtime_penalty: c.get_f32("OvertimePenaltyPerSec").unwrap_or(d.overtime_penalty),
            difficulty: c.get_i32("DifficultyLevel").unwrap_or(d.difficulty),
            traffic_level: c.get_i32("TrafficLevel").unwrap_or(d.traffic_level),
            force_traffic_density: c.get_i32("ForceTrafficDensity").unwrap_or(0),
            force_heat_level: c.get_i32("ForceHeatLevel").unwrap_or(0),
            cops_in_race: c.get_bool("CopsInRace").unwrap_or(d.cops_in_race),
            pursuit_race: c.get_bool("PursuitRace").unwrap_or(d.pursuit_race),
            catch_up: c.get_bool("CatchUp").unwrap_or(d.catch_up),
            rank_by_points: c.get_bool("RankPlayersByPoints").unwrap_or(d.rank_by_points),
            traffic_pattern: text(c, "TrafficPattern"),
            opponents: key_array(c, "Opponents"),
            speed_traps: key_array(c, "SpeedTrapList"),
            shortcuts: key_array(c, "Shortcuts"),
            cash_value: c.get_by_key(CASH_VALUE_FIELD).and_then(|v| v.as_f32()).unwrap_or(d.cash_value),
        }
    }

    /// The difficulty bucket the game derives from `DifficultyLevel`: 0 easy, 1 medium, 2 hard.
    pub fn difficulty_class(&self) -> usize {
        match self.difficulty {
            ..=33 => 0,
            34..=66 => 1,
            _ => 2,
        }
    }
}

/// One opponent driver (a `character`, `boss` or `character_smart` collection).
#[derive(Debug, Clone, PartialEq)]
pub struct Character {
    /// The collection's key (the hash of its name); a race's `Opponents` refer to it.
    pub key: u32,
    /// For example `race_bin_10/character1`.
    pub name: String,
    /// `SkillLevel`: 0..100.
    pub skill_level: i32,
    /// `MinimumAIPerformance`: 0..1.
    pub min_performance: f32,
    /// `PresetRide`: a front-end preset car name.
    pub preset_ride: Option<String>,
    /// `CarType`: a `pvehicle` collection name.
    pub car_type: Option<String>,
    /// `IsBoss`.
    pub is_boss: bool,
    /// `ForceStartPosition`: the marker this racer starts at instead of the grid.
    pub force_start: Option<Placement>,
}

/// Every opponent character, in database order.
pub(crate) fn characters(db: &Database) -> Vec<Character> {
    instances(db).filter(|c| inherits(*c, "character")).map(|c| character(db, c)).collect()
}

fn character(db: &Database, c: CollectionRef<'_>) -> Character {
    Character {
        key: c.key(),
        name: name_of(c),
        skill_level: c.get_i32("SkillLevel").unwrap_or(100),
        min_performance: c.get_f32("MinimumAIPerformance").unwrap_or(1.0),
        preset_ride: text(c, "PresetRide"),
        car_type: text(c, "CarType"),
        is_boss: c.get_bool("IsBoss").unwrap_or(false),
        force_start: key_field(c, "ForceStartPosition").and_then(|k| collection(db, k)).map(placement),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_defaults_are_the_basic_race_template() {
        let p = RaceParams::default();
        assert!(p.do_countdown && !p.rolling_start && p.catch_up);
        assert_eq!((p.difficulty, p.traffic_level), (100, 100));
        assert_eq!(p.time_limit, 0.0);
    }

    #[test]
    fn difficulty_buckets_follow_the_game_thresholds() {
        let class = |difficulty| RaceParams { difficulty, ..Default::default() }.difficulty_class();
        assert_eq!([class(0), class(33), class(34), class(66), class(67), class(100)], [0, 0, 1, 1, 2, 2]);
    }
}
