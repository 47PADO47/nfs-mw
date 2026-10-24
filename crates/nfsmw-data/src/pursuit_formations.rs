//! Installed formation selection and execution parameters; `docs/formats/pursuit-data.md` sections 2.1 and 2.2.

use super::{HEAT_LEVELS, PursuitTable, level_collection};
use blackbox_attrib::{Database, Value};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FormationRecord {
    pub kind: u32,
    pub duration: f32,
    pub frequency: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FormationRow {
    pub records: Vec<FormationRecord>,
    pub stagger_time: f32,
    pub boxin_tightness: f32,
    pub boxin_duration: f32,
    pub rolling_block_tightness: f32,
    pub rolling_block_duration: f32,
    pub collapse_aggression: f32,
    /// km/h, as stored in `CollapseSpeed`.
    pub collapse_speed: f32,
    pub collapse_inner: f32,
    pub collapse_outer: f32,
    pub max_cops_collapsing: usize,
}

impl PursuitTable {
    pub fn formations(self, db: &Database) -> Option<Vec<FormationRow>> {
        (1..=HEAT_LEVELS)
            .map(|heat| {
                let row = level_collection(db, self.name(), heat)?;
                let records = row.get("CopFormations")?.as_array()?.iter().map(record).collect::<Option<_>>()?;
                Some(FormationRow {
                    records,
                    stagger_time: row.get_f32("StaggerFormationTime")?,
                    boxin_tightness: row.get_f32("BoxinTightness")?,
                    boxin_duration: row.get_f32("BoxinDuration")?,
                    rolling_block_tightness: row.get_f32("RollingBlockTightness")?,
                    rolling_block_duration: row.get_f32("RollingBlockDuration")?,
                    collapse_aggression: row.get_f32("CollapseAggression")?,
                    collapse_speed: row.get_f32("CollapseSpeed")?,
                    collapse_inner: row.get_i32("CollapseInnerRadius")? as f32,
                    collapse_outer: row.get_f32("CollapseOuterRadius")?,
                    max_cops_collapsing: row.get_i32("MaxCopsCollapsing")?.max(0) as usize,
                })
            })
            .collect()
    }
}

fn record(value: &Value) -> Option<FormationRecord> {
    let Value::Raw { bytes, .. } = value else { return None };
    decode_record(bytes)
}

fn decode_record(bytes: &[u8]) -> Option<FormationRecord> {
    let bytes = bytes.get(..12)?;
    Some(FormationRecord {
        kind: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
        duration: f32::from_le_bytes(bytes[4..8].try_into().ok()?),
        frequency: f32::from_le_bytes(bytes[8..12].try_into().ok()?),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formation_record_keeps_kind_duration_and_frequency_distinct() {
        let bytes: Vec<u8> = [3_u32.to_le_bytes(), 20_f32.to_le_bytes(), 40_f32.to_le_bytes()].concat();
        assert_eq!(decode_record(&bytes), Some(FormationRecord { kind: 3, duration: 20.0, frequency: 40.0 }));
        assert_eq!(decode_record(&bytes[..11]), None);
    }

    #[test]
    fn incomplete_tables_do_not_shift_heat_indices() {
        let db = Database::default();
        for table in [PursuitTable::OpenWorld, PursuitTable::Race] {
            assert!(table.rows(&db).is_none());
            assert!(table.formations(&db).is_none());
        }
    }
}
