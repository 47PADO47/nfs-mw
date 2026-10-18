//! The rap-sheet ranking thresholds of the attribute database (`GLOBAL/FE_ATTRIB.bin`): 15 thresholds per statistic,
//! lifetime and single pursuit. Spec: `docs/specs/career-rapsheet-2.md` 1.2 and 1.5, `docs/specs/career-economy-flows.md`
//! 5.3.
//!
//! The original reads each table from the `frontend` collection named after the statistic (`_in_pursuit` added for
//! a single pursuit). The shape matches the defaults of `nfsmw-career`'s `rapsheet::rank` (`LIFETIME` and `SINGLE`:
//! 10 statistics of 15 thresholds, rank 1 first), so the values can be passed to its rank rule.

use blackbox_attrib::{Database, Value};

/// Thresholds per table.
pub const THRESHOLDS: usize = 15;
/// Statistics per table, index order.
pub const STATISTICS: usize = 10;
/// The class of the ranking tables.
pub const TABLE_CLASS: &str = "frontend";
/// The field key of the array of 15 thresholds in a table (`0xF9A7D5F7`, spec 1.2). **[unconfirmed]** the field
/// name is not in the specs; the key is used as the spec gives it.
pub const THRESHOLD_FIELD: u32 = 0xF9A7_D5F7;
/// Statistic names, index order (the lifetime table name; the single pursuit table adds `_in_pursuit`).
pub const STAT_NAMES: [&str; STATISTICS] = [
    "pursuit_length",
    "cops_involved",
    "cops_damaged",
    "cops_destroyed",
    "tire_spikes_dodged",
    "roadblocks_dodged",
    "helis_involved",
    "cost_to_state",
    "total_infractions",
    "bounty",
];

/// The thresholds of every statistic. `lifetime` is the career table, `single` the single pursuit one; rank 1 first.
/// Pursuit length is in seconds.
#[derive(Debug, Clone, PartialEq)]
pub struct RankThresholds {
    pub lifetime: [[f32; THRESHOLDS]; STATISTICS],
    pub single: [[f32; THRESHOLDS]; STATISTICS],
}

/// The thresholds of every statistic, or None when any table is missing or does not hold exactly 15 numbers.
pub fn rank_thresholds(db: &Database) -> Option<RankThresholds> {
    Some(RankThresholds { lifetime: all_tables(db, true)?, single: all_tables(db, false)? })
}

/// The collection name of statistic `stat`: the bare name for `career`, `name_in_pursuit` otherwise. None past 9.
pub fn table_name(stat: usize, career: bool) -> Option<String> {
    let name = STAT_NAMES.get(stat)?;
    let suffix = match career {
        true => "",
        false => "_in_pursuit",
    };
    Some(format!("{name}{suffix}"))
}

/// The 15 thresholds held in a table's values; None unless there are exactly 15 numbers and each is a float.
pub fn thresholds_from(values: &[Value]) -> Option<[f32; THRESHOLDS]> {
    let floats: Vec<f32> = values.iter().map(Value::as_f32).collect::<Option<_>>()?;
    floats.try_into().ok()
}

/// The thresholds of every statistic for one table kind (`career` picks the lifetime tables).
fn all_tables(db: &Database, career: bool) -> Option<[[f32; THRESHOLDS]; STATISTICS]> {
    let mut tables = [[0.0; THRESHOLDS]; STATISTICS];
    for (stat, table) in tables.iter_mut().enumerate() {
        *table = table_thresholds(db, stat, career)?;
    }
    Some(tables)
}

/// The thresholds of statistic `stat` in its table of the `frontend` class; None when the table or the array is
/// missing or malformed.
fn table_thresholds(db: &Database, stat: usize, career: bool) -> Option<[f32; THRESHOLDS]> {
    let name = table_name(stat, career)?;
    let table = db.collection(TABLE_CLASS, &name)?;
    thresholds_from(table.get_by_key(THRESHOLD_FIELD)?.as_array()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_lifetime_name_is_bare_and_the_single_name_has_the_suffix() {
        assert_eq!(table_name(0, true).as_deref(), Some("pursuit_length"));
        assert_eq!(table_name(0, false).as_deref(), Some("pursuit_length_in_pursuit"));
        assert_eq!(table_name(9, false).as_deref(), Some("bounty_in_pursuit"));
        assert_eq!(table_name(7, true).as_deref(), Some("cost_to_state"));
    }

    #[test]
    fn an_index_past_nine_has_no_table() {
        assert_eq!(table_name(10, true), None);
        assert_eq!(table_name(usize::MAX, false), None);
    }

    #[test]
    fn the_statistic_names_are_distinct() {
        let mut names = STAT_NAMES.to_vec();
        names.sort_unstable();
        names.dedup();
        assert_eq!(names.len(), STATISTICS);
    }

    #[test]
    fn the_threshold_field_key_is_the_spec_key() {
        assert_eq!(THRESHOLD_FIELD, 0xF9A7_D5F7);
    }

    fn floats(count: usize) -> Vec<Value> {
        (0..count).map(|i| Value::Float(15.0 - i as f32)).collect()
    }

    #[test]
    fn fifteen_floats_are_the_thresholds_in_order() {
        let thresholds = thresholds_from(&floats(THRESHOLDS)).unwrap();
        assert_eq!(thresholds[0], 15.0);
        assert_eq!(thresholds[14], 1.0);
    }

    #[test]
    fn any_other_count_or_a_non_float_is_refused() {
        assert_eq!(thresholds_from(&floats(14)), None);
        assert_eq!(thresholds_from(&floats(16)), None);
        assert_eq!(thresholds_from(&[]), None);
        let mut mixed = floats(THRESHOLDS);
        mixed[3] = Value::Int32(4);
        assert_eq!(thresholds_from(&mixed), None);
    }

    #[test]
    fn a_double_counts_as_a_float() {
        let mut values = floats(THRESHOLDS);
        values[0] = Value::Double(2.5);
        assert_eq!(thresholds_from(&values).unwrap()[0], 2.5);
    }
}
