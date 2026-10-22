//! The run-time parameters of a race and the opponent characters. Spec: `docs/formats/gameplay-races.md` sections 3.2,
//! 3.3 and 4; `docs/specs/race-mode.md`.

use blackbox_attrib::{CollectionRef, Database, Value};

use super::{Placement, collection, inherits, instances, key_array, key_field, name_of, placement, text};

/// `CashValue` (`Float`, hash `0xD8BAA07B`): the cash a career win pays (`docs/specs/career-economy-flows.md` section 7).
/// Read by its hash, as the career crate documents it.
const CASH_VALUE_FIELD: u32 = 0xD8BA_A07B;
/// The three race flags the exe reads by hash (names unknown): the first and the second open the race in a quick race from the
/// start of a career, the third keeps it out of the quick race lists (`docs/provenance/custom-race.md`).
const QUICK_OPEN_A_FIELD: u32 = 0xB39E_D8C3;
const QUICK_OPEN_B_FIELD: u32 = 0xEA85_5EAF;
const NEVER_IN_QUICK_RACE_FIELD: u32 = 0xA4E6_FCFD;

/// What a race asks of the run: the start, the clock, the opponents, the traffic and the rank rule.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceParams {
    /// `DoCountdown`: the 3, 2, 1 board with the cars held.
    pub do_countdown: bool,
    /// `RollingStart`: no countdown; every racer starts moving.
    pub rolling_start: bool,
    /// `InitialPlayerSpeed`: the start speed of a rolling start (m/s, see docs/provenance/race-mode.md section 9).
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
    /// `MilestoneName`: the statistic a pursuit challenge must reach (`pursuit_length`, `cops_hit`, ...); empty for a
    /// race that is not one.
    pub milestone: Option<String>,
    /// `ThreshholdValue` (the field's spelling): the value the statistic must reach.
    pub threshold: f32,
    /// `PostRaceActivity`: the key of the race the career runs after this one (prologue and boss races chain this way).
    pub post_race_activity: Option<u32>,
    /// `InitiallyUnlocked`: the race is open from the start of a career (the side bins' ladders start from these).
    pub initially_unlocked: bool,
    /// The quick race is open from the start (flag `0xB39ED8C3` or `0xEA855EAF` of the exe's score setup).
    pub quick_race_open: bool,
    /// The race is left out of the quick race lists (flag `0xA4E6FCFD`).
    pub never_in_quick_race: bool,
    /// `UnlockRaces`: the keys of the races a win opens (the challenge ladders).
    pub unlock_races: Vec<u32>,
    /// `LocalizationTag`: the number the challenge series' texts are named by (`CHALLENGE_SERIES_DESC_<tag>`); 0 for
    /// a race with none (`docs/specs/challenge-series.md`).
    pub localization_tag: i32,
    /// `ChallengeSeriesRace`: the event belongs to the challenge series (its money is not paid in a career).
    pub challenge_series: bool,
    /// `RaceLength` (hash `0x7C11C52E`): the route in metres as the editor stored it, 0 when it has none. The Quick Play
    /// brief's `Length:` line reads it (`docs/specs/quick-play.md` section 5).
    pub route_length: f32,
    /// `FinishCamera` (hash `0x62DFC259`): the name of the replay camera (an ICE track of `InGameB.bun`'s replay pack)
    /// that frames the win photo, for example `DefaultFinish` or `PHoldFin_17`
    /// (`docs/specs/race-finish-effects.md` section 3).
    pub finish_camera: Option<String>,
    /// The win photo's backdrop texture (field `0x038A3B53`, name unknown): `PHOTOFINISH_SPRINT`, `PHOTOFINISH_DRAG` or
    /// `PHOTOFINISH_TOOBOOTH` in the install.
    pub photo_texture: Option<String>,
}

/// The race field that names the win photo's backdrop texture; its name is not known, the key is the executable's.
const PHOTO_TEXTURE_FIELD: u32 = 0x038A_3B53;

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
            milestone: None,
            threshold: 0.0,
            post_race_activity: None,
            initially_unlocked: false,
            quick_race_open: false,
            never_in_quick_race: false,
            unlock_races: Vec::new(),
            localization_tag: 0,
            challenge_series: false,
            route_length: 0.0,
            finish_camera: None,
            photo_texture: None,
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
            milestone: text(c, "MilestoneName").filter(|name| !name.is_empty()),
            threshold: c.get_f32("ThreshholdValue").unwrap_or(d.threshold),
            post_race_activity: key_field(c, "PostRaceActivity"),
            initially_unlocked: c.get_bool("InitiallyUnlocked").unwrap_or(false),
            quick_race_open: [QUICK_OPEN_A_FIELD, QUICK_OPEN_B_FIELD]
                .iter()
                .any(|key| c.get_by_key(*key).and_then(Value::as_bool).unwrap_or(false)),
            never_in_quick_race: c.get_by_key(NEVER_IN_QUICK_RACE_FIELD).and_then(Value::as_bool).unwrap_or(false),
            unlock_races: key_array(c, "UnlockRaces"),
            localization_tag: c.get_i32("LocalizationTag").unwrap_or(0),
            challenge_series: c.get_bool("ChallengeSeriesRace").unwrap_or(false),
            route_length: c.get_f32("RaceLength").unwrap_or(d.route_length),
            finish_camera: text(c, "FinishCamera"),
            photo_texture: c
                .get_by_key(PHOTO_TEXTURE_FIELD)
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_owned),
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
    /// `RacerName`: the language label of a name this driver always has in a career race, for example
    /// `BLACKLIST_RIVAL_03_LEADERBOARD`. Unset for the street rivals and the Blacklist bosses (a boss race names its
    /// racer from the bin instead); the prologue's characters set it (`docs/specs/racer-names.md`).
    pub racer_name: Option<String>,
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
        racer_name: text(c, "RacerName"),
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
