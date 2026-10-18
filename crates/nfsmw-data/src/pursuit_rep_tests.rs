use super::*;

fn table() -> RepTable {
    RepTable {
        rates: (1..=HEAT_LEVELS)
            .map(|level| RepRates { per_ten_seconds: level * 100, destroy_bonus_secs: 10.0 })
            .collect(),
        destroy: vec![("copgto".to_owned(), vec![2500; 10]), ("copsport".to_owned(), vec![20_000; 9])],
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
    assert_eq!(table.destroy_points("copsport", 10), 0, "the row stops at level 9");
    assert_eq!(table.destroy_points("copsport", 9), 20_000);
}
