//! The pursuit tables of the attribute database: which cop cars chase at each heat level, and the heat values the
//! career uses (factors and growth per career bin).
//! Spec: `docs/specs/ai-pursuit-heat.md` (§1, §2.1, §2.2, §2.3, §2.7, §3.2), `docs/specs/career-safehouse.md` heat.

use blackbox_attrib::{CollectionRef, Database, Value, vlt_hash};

/// Heat levels with their own row.
pub const HEAT_LEVELS: u32 = 10;

/// One kind of cop car in a wave.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CopRecord {
    /// The `pvehicle` collection name (`copmidsize`, `copheli`...).
    pub name: String,
    /// How many of them the wave has.
    pub count: u32,
    /// Spawn weight, percent.
    pub chance: u32,
}

/// The values of one row of `pursuitlevels` that the cop manager reads.
#[derive(Debug, Clone, PartialEq)]
pub struct HeatRow {
    pub cops: Vec<CopRecord>,
    pub patrol_cars: u32,
    /// Cops that have to be evaded to end the wave.
    pub full_engagement_cops: u32,
    pub time_between_first_four_spawns: f32,
    pub time_between_cop_spawns: f32,
}

/// The `cops` record is 24 bytes: a string key (the name hash sits at offset 8), a count and a weight.
const COP_RECORD_LEN: usize = 24;

/// The helicopter record of a wave is not a car.
pub const HELICOPTER: &str = "copheli";

/// The row for heat level `heat` (1…10), outside races (`heattable`).
pub fn heat_row(db: &Database, heat: u32) -> Option<HeatRow> {
    let mut row = heat_row_in(db, "heattable", heat)?;
    // The cop rows that mods add to this level (car/mods.rs); none without mods.
    crate::car::mods::add_cop_rows(crate::car::mods::sealed(), heat.clamp(1, HEAT_LEVELS), &mut row);
    Some(row)
}

/// The `pursuitlevels` collection of heat level `heat` (1…10, clamped) in the escalation table `table`
/// (`heattable` or `racetable`), from the `default` collection of `pursuitescalation`.
fn level_collection<'a>(db: &'a Database, table: &str, heat: u32) -> Option<CollectionRef<'a>> {
    let level = heat.clamp(1, HEAT_LEVELS) as usize - 1;
    let escalation = db.collections_of("pursuitescalation").find(|c| c.name() == Some("default"))?;
    let Value::Array(rows) = escalation.get(table)? else { return None };
    db.resolve(rows.get(level)?.as_ref_spec()?)
}

/// The row for heat level `heat` (1…10) while racing a race event (`racetable`).
pub fn race_heat_row(db: &Database, heat: u32) -> Option<HeatRow> {
    heat_row_in(db, "racetable", heat)
}

/// The rows of every heat level that has one, in level order: the row of level `n` is at index `n - 1`.
pub fn heat_rows(db: &Database) -> Vec<HeatRow> {
    (1..=HEAT_LEVELS).filter_map(|heat| heat_row(db, heat)).collect()
}

/// The car records of a wave: the wave without the helicopter, which is not a car.
pub fn ground_cops(row: &HeatRow) -> impl Iterator<Item = &CopRecord> {
    row.cops.iter().filter(|c| c.name != HELICOPTER)
}

/// Every car type the rows can put in a chase, without the helicopter, once each, in first-seen order.
pub fn wave_names(rows: &[HeatRow]) -> Vec<String> {
    let mut names: Vec<String> = Vec::new();
    for cop in rows.iter().flat_map(ground_cops) {
        if !names.contains(&cop.name) {
            names.push(cop.name.clone());
        }
    }
    names
}

/// The row of `heat` in the escalation table `table` (`heattable` or `racetable`).
fn heat_row_in(db: &Database, table: &str, heat: u32) -> Option<HeatRow> {
    let row = level_collection(db, table, heat)?;
    let names: Vec<(u32, String)> = db
        .collections_of("pvehicle")
        .filter_map(|c| c.name().filter(|n| n.starts_with("cop")).map(|n| (vlt_hash(n), n.to_owned())))
        .collect();
    let cops = match row.get("cops")? {
        Value::Array(records) => records
            .iter()
            .filter_map(|r| match r {
                Value::Raw { bytes, .. } if bytes.len() >= COP_RECORD_LEN => {
                    let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap());
                    let name = names.iter().find(|(hash, _)| *hash == word(8))?.1.clone();
                    Some(CopRecord { name, count: word(16), chance: word(20) })
                }
                _ => None,
            })
            .collect(),
        _ => Vec::new(),
    };
    let int = |field: &str| row.get_i32(field).unwrap_or(0).max(0) as u32;
    Some(HeatRow {
        cops,
        patrol_cars: int("NumPatrolCars"),
        full_engagement_cops: int("FullEngagementCopCount"),
        time_between_first_four_spawns: row.get_f32("TimeBetweenFirstFourSpawn").unwrap_or(10.0),
        time_between_cop_spawns: row.get_f32("TimeBetweenCopSpawn").unwrap_or(5.0),
    })
}

/// Patrol cruise speed of a cop car on city lanes, when the `default` row has no `SearchModeCityMPH`.
pub const FALLBACK_PATROL_CITY_MPH: f32 = 50.0;
/// Patrol cruise speed of a cop car on highway lanes, when the `default` row has no `SearchModeHwyMPH`.
pub const FALLBACK_PATROL_HIGHWAY_MPH: f32 = 71.0;

/// The speeds, in mph, at which a cop car patrols (no pursuit running) on city and on highway lanes:
/// `SearchModeCityMPH` and `SearchModeHwyMPH` of the `default` collection of `pursuitlevels`.
pub fn patrol_speeds_mph(db: &Database) -> (f32, f32) {
    let Some(row) = db.collection("pursuitlevels", "default") else {
        return (FALLBACK_PATROL_CITY_MPH, FALLBACK_PATROL_HIGHWAY_MPH);
    };
    (
        row.get_f32("SearchModeCityMPH").unwrap_or(FALLBACK_PATROL_CITY_MPH),
        row.get_f32("SearchModeHwyMPH").unwrap_or(FALLBACK_PATROL_HIGHWAY_MPH),
    )
}

/// The `aivehicle` numbers of a car that limit how fast an AI drives it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AiVehicle {
    /// `MAXIMUM_AI_SPEED`, km/h.
    pub max_speed_kmh: f32,
    pub acceleration_multiplier: f32,
    pub top_speed_multiplier: f32,
}

impl Default for AiVehicle {
    fn default() -> Self {
        Self { max_speed_kmh: 200.0, acceleration_multiplier: 1.0, top_speed_multiplier: 1.0 }
    }
}

/// The `aivehicle` record of the `pvehicle` named `car` (lower case); the defaults when it has none.
pub fn ai_vehicle(db: &Database, car: &str) -> AiVehicle {
    let defaults = AiVehicle::default();
    let Some(record) = db.collection("pvehicle", &car.to_ascii_lowercase()).and_then(|p| p.follow("aivehicle")) else {
        return defaults;
    };
    AiVehicle {
        max_speed_kmh: record.get_f32("MAXIMUM_AI_SPEED").unwrap_or(defaults.max_speed_kmh),
        acceleration_multiplier: record.get_f32("AccelerationMultiplier").unwrap_or(1.0),
        top_speed_multiplier: record.get_f32("TopSpeedMultiplier").unwrap_or(1.0),
    }
}

/// Career bins of the heat growth (`ScaleEscalationPerBucket` has one value per bin).
pub const CAREER_BINS: usize = 15;
/// The largest bin index: the bin of a challenge race, and the cap of `min(current_bin, 14)`.
pub const LAST_CAREER_BIN: u32 = 14;

/// The heat growth of one heat level: the base seconds to climb a level and the multiplier of each career bin.
/// Spec: `docs/specs/ai-pursuit-heat.md` 2.3 and 3.2.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeatGrowth {
    /// `TimePerHeatLevel`, seconds.
    pub time_per_heat_level: f32,
    /// `ScaleEscalationPerBucket`, index = career bin 0 to 14.
    pub scale_per_bucket: [f32; CAREER_BINS],
}

impl HeatGrowth {
    /// Seconds to climb one heat level in career bin `bin`: `TimePerHeatLevel * ScaleEscalationPerBucket[bin]`.
    /// Bins past 14 use 14 (`min(current_bin, 14)`); a challenge race passes 14.
    pub fn seconds_for_bin(&self, bin: u32) -> f32 {
        self.time_per_heat_level * self.scale_per_bucket[bin.min(LAST_CAREER_BIN) as usize]
    }
}

/// The `pursuitlevels` values the career heat uses (`docs/specs/ai-pursuit-heat.md` 2.3 and 3, and
/// `docs/specs/career-safehouse.md` heat section).
#[derive(Debug, Clone, PartialEq)]
pub struct CareerHeat {
    /// `EvadeSuccessHeatAdjust` of `default` (0.95): every car of the stable, on an evade.
    pub evade_factor: Option<f32>,
    /// `EventWinHeatAdjust` of `default` (0.94): every car of the stable, on a career race win.
    pub race_win_factor: Option<f32>,
    /// `MilestoneCompleteHeatAdjust` of `default` (0.93): every car of the stable, on a milestone.
    /// **[unconfirmed]** the pairing of this factor with the milestone moment (spec 3.3).
    pub milestone_factor: Option<f32>,
    /// The growth of heat level `n` at index `n - 1`; None for a level whose row lacks a value.
    pub growth: [Option<HeatGrowth>; HEAT_LEVELS as usize],
}

/// The career heat values, or None when `pursuitlevels/default` is not in the database.
pub fn career_heat(db: &Database) -> Option<CareerHeat> {
    let defaults = db.collection("pursuitlevels", "default")?;
    Some(CareerHeat {
        evade_factor: defaults.get_f32("EvadeSuccessHeatAdjust"),
        race_win_factor: defaults.get_f32("EventWinHeatAdjust"),
        milestone_factor: defaults.get_f32("MilestoneCompleteHeatAdjust"),
        growth: std::array::from_fn(|index| heat_growth(db, index as u32 + 1)),
    })
}

/// The growth of heat level `heat` (1…10) from its `heattable` row; None when the row lacks a value or the bin
/// array does not hold 15 numbers.
fn heat_growth(db: &Database, heat: u32) -> Option<HeatGrowth> {
    let row = level_collection(db, "heattable", heat)?;
    let time_per_heat_level = row.get_f32("TimePerHeatLevel")?;
    let values = row.get("ScaleEscalationPerBucket")?.as_array()?;
    let scale: Vec<f32> = values.iter().map(Value::as_f32).collect::<Option<_>>()?;
    Some(HeatGrowth { time_per_heat_level, scale_per_bucket: scale.try_into().ok()? })
}

/// The rep rates of one heat level: the `pursuitlevels` row's `RepPointsPerMinute` and `DestroyCopBonusTime`
/// (`docs/specs/ai-pursuit-heat.md` sections 2.4 and 5.1).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct RepRates {
    /// `RepPointsPerMinute`: the normal rep points of each 10 s step of chase while the cops see the player. The name
    /// says minute; the code steps every 10 s, so the spec takes the field as the points per step.
    pub per_ten_seconds: u32,
    /// `DestroyCopBonusTime`, seconds: the window in which a further cop kill raises the destruction multiplier.
    pub destroy_bonus_secs: f32,
}

/// The rep rates and destruction points a chase reads, from the attribute database ([`rep_table`]).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RepTable {
    /// The rates of heat level `n` at index `n - 1`.
    pub rates: Vec<RepRates>,
    /// Each cop car type (lower case name) with the destruction points of heat level `n` at index `n - 1`
    /// (`RepPointsForDestroying` of its `aivehicle` record).
    pub destroy: Vec<(String, Vec<u32>)>,
}

impl RepTable {
    /// The rates of heat level `level` (clamped to 1…10); the zero rates when the table lacks the level.
    pub fn rate(&self, level: u32) -> RepRates {
        let index = level.clamp(1, HEAT_LEVELS) as usize - 1;
        self.rates.get(index).copied().unwrap_or_default()
    }

    /// The destruction points of cop car `car` wrecked at heat level `level` (clamped); 0 for a car the table lacks.
    pub fn destroy_points(&self, car: &str, level: u32) -> u32 {
        let index = level.clamp(1, HEAT_LEVELS) as usize - 1;
        let row = self.destroy.iter().find(|(name, _)| name.eq_ignore_ascii_case(car));
        row.and_then(|(_, points)| points.get(index)).copied().unwrap_or(0)
    }
}

/// The rep rates of heat level `heat` (1…10) from its `pursuitlevels` row. None when the row is not in the database.
pub fn rep_rates(db: &Database, heat: u32) -> Option<RepRates> {
    let row = level_collection(db, "heattable", heat)?;
    Some(RepRates {
        per_ten_seconds: row.get_i32("RepPointsPerMinute").unwrap_or(0).max(0) as u32,
        destroy_bonus_secs: row.get_f32("DestroyCopBonusTime").unwrap_or(0.0),
    })
}

/// The destruction points of cop car type `car` wrecked at heat `heat` (1…10): `RepPointsForDestroying` of its
/// `aivehicle` record, at index `heat - 1`. None when the record or the value is missing.
pub fn destroy_rep_points(db: &Database, car: &str, heat: u32) -> Option<u32> {
    let record = db.collection("pvehicle", &car.to_ascii_lowercase())?.follow("aivehicle")?;
    let index = heat.clamp(1, HEAT_LEVELS) as usize - 1;
    let points = record.get("RepPointsForDestroying")?.as_array()?.get(index)?.as_f32()?;
    Some(points.max(0.0) as u32)
}

/// The rep tables of the install: the rates of every heat level and the destruction points of every cop car type.
pub fn rep_table(db: &Database) -> RepTable {
    let rates = (1..=HEAT_LEVELS).map(|heat| rep_rates(db, heat).unwrap_or_default()).collect();
    let destroy = cop_car_names(db)
        .into_iter()
        .map(|name| {
            let points = (1..=HEAT_LEVELS).map(|heat| destroy_rep_points(db, &name, heat).unwrap_or(0)).collect();
            (name, points)
        })
        .collect();
    RepTable { rates, destroy }
}

/// The names of the cop car types: the `pvehicle` collections whose name starts with `cop`.
fn cop_car_names(db: &Database) -> Vec<String> {
    db.collections_of("pvehicle").filter_map(|c| c.name().filter(|n| n.starts_with("cop")).map(str::to_owned)).collect()
}

#[cfg(test)]
#[path = "pursuit_rep_tests.rs"]
mod rep_tests;

#[cfg(test)]
mod tests {
    use super::*;

    /// The level 1 bins of the install: 120 s for bins 0 to 10, then 180, 240, 360 and 600 s (bins 11 to 14).
    const LEVEL_ONE_SCALE: [f32; CAREER_BINS] =
        [120., 120., 120., 120., 120., 120., 120., 120., 120., 120., 120., 180., 240., 360., 600.];

    #[test]
    fn a_level_one_climb_takes_600_seconds_in_the_last_two_bins() {
        let growth = HeatGrowth { time_per_heat_level: 1.0, scale_per_bucket: LEVEL_ONE_SCALE };
        assert_eq!(growth.seconds_for_bin(14), 600.0);
        assert_eq!(growth.seconds_for_bin(13), 360.0);
        assert_eq!(growth.seconds_for_bin(12), 240.0);
        assert_eq!(growth.seconds_for_bin(11), 180.0);
        assert_eq!(growth.seconds_for_bin(10), 120.0);
        assert_eq!(growth.seconds_for_bin(0), 120.0);
    }

    #[test]
    fn a_bin_past_fourteen_uses_fourteen() {
        let growth = HeatGrowth { time_per_heat_level: 2.0, scale_per_bucket: LEVEL_ONE_SCALE };
        assert_eq!(growth.seconds_for_bin(15), growth.seconds_for_bin(14));
        assert_eq!(growth.seconds_for_bin(u32::MAX), 1200.0);
    }

    fn cop(name: &str, count: u32) -> CopRecord {
        CopRecord { name: name.to_owned(), count, chance: 100 }
    }

    fn row(cops: Vec<CopRecord>) -> HeatRow {
        HeatRow {
            cops,
            patrol_cars: 0,
            full_engagement_cops: 0,
            time_between_first_four_spawns: 0.0,
            time_between_cop_spawns: 0.0,
        }
    }

    #[test]
    fn the_ground_wave_leaves_the_helicopter_out() {
        let heat_four = row(vec![cop("copgtoghost", 8), cop(HELICOPTER, 1)]);
        let names: Vec<&str> = ground_cops(&heat_four).map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["copgtoghost"]);
    }

    #[test]
    fn the_wave_names_cover_every_level_once_and_no_helicopter() {
        let rows = vec![
            row(vec![cop("copmidsize", 4)]),
            row(vec![cop("copgtoghost", 8), cop(HELICOPTER, 1)]),
            row(vec![cop("copmidsize", 4), cop("copsuvpatrol", 2)]),
        ];
        assert_eq!(wave_names(&rows), vec!["copmidsize", "copgtoghost", "copsuvpatrol"]);
    }
}
