//! The pursuit tables of the attribute database: which cop cars chase at each heat level.
//! Spec: `docs/specs/ai-pursuit-heat.md` (§1, §2.1, §2.7).

use blackbox_attrib::{Database, Value, vlt_hash};

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

/// The row for heat level `heat` (1…10), outside races.
pub fn heat_row(db: &Database, heat: u32) -> Option<HeatRow> {
    let level = heat.clamp(1, HEAT_LEVELS) as usize - 1;
    let escalation = db.collections_of("pursuitescalation").find(|c| c.name() == Some("default"))?;
    let Value::Array(rows) = escalation.get("heattable")? else { return None };
    let row = db.resolve(rows.get(level)?.as_ref_spec()?)?;
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
