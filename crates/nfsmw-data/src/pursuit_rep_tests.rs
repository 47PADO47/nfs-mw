use super::*;

fn table() -> RepTable {
    RepTable {
        rates: (1..=HEAT_LEVELS)
            .map(|level| RepRates { per_ten_seconds: level * 100, destroy_bonus_secs: 10.0 })
            .collect(),
        destroy: vec![
            ("copgto".to_owned(), vec![2500; 10]),
            ("copsport".to_owned(), [vec![20_000; 9], vec![100_000]].concat()),
        ],
    }
}

#[test]
fn a_level_reads_its_own_rate_and_a_level_out_of_range_is_clamped() {
    let table = table();
    assert_eq!(table.rate(3).per_ten_seconds, 300);
    assert_eq!(table.rate(0).per_ten_seconds, 100, "level 0 clamps to 1");
    assert_eq!(table.rate(99).per_ten_seconds, 1000, "a level past 10 clamps to 10");
}

#[test]
fn a_table_without_rates_gives_the_zero_rates() {
    let empty = RepTable::default();
    assert_eq!(empty.rate(5), RepRates::default());
    assert_eq!(empty.rate(5).destroy_bonus_secs, 0.0);
}

#[test]
fn destroy_points_match_the_car_name_without_case() {
    let table = table();
    assert_eq!(table.destroy_points("COPGTO", 4), 2500);
    assert_eq!(table.destroy_points("copgto", 4), 2500);
}

#[test]
fn an_unknown_car_or_a_missing_level_scores_nothing() {
    let table = table();
    assert_eq!(table.destroy_points("copmidsize", 4), 0, "not in the table");
    assert_eq!(table.destroy_points("copsport", 10), 0, "heat 10 runs off the 10-entry array");
}

#[test]
fn heat_n_reads_entry_n_as_the_original_does() {
    let table = table();
    assert_eq!(table.destroy_points("copsport", 8), 20_000);
    assert_eq!(table.destroy_points("copsport", 9), 100_000, "heat 9 reads the last entry");
}

#[test]
fn destroy_points_read_the_int32_values_of_the_install_and_a_float() {
    assert_eq!(points_of(&Value::Int32(250)), Some(250), "the install stores the points as Int32");
    assert_eq!(points_of(&Value::Int32(100_000)), Some(100_000));
    assert_eq!(points_of(&Value::Float(2500.0)), Some(2500), "an edited database may hold a float");
    assert_eq!(points_of(&Value::Int32(-5)), Some(0), "negative points score nothing");
    assert_eq!(points_of(&Value::Bool(true)), None);
}
