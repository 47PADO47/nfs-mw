//! Tests of the helicopter's tunables (`helicopter.rs`): the vanilla table, the file's overrides and the clamping.

use super::*;

#[test]
fn line_of_sight_is_251_to_heat_7_and_301_from_heat_8() {
    let s = HeliSettings::default();
    assert_eq!(s.tuning(7, false).line_of_sight, 251.0);
    assert_eq!(s.tuning(8, false).line_of_sight, 301.0);
    assert_eq!(s.tuning(10, false).line_of_sight, 301.0);
}

#[test]
fn fuel_is_the_install_value_for_each_heat_level() {
    let s = HeliSettings::default();
    let fuel: Vec<f32> = (4..=10).map(|level| s.tuning(level, false).fuel_time).collect();
    assert_eq!(fuel, vec![60.0, 90.0, 180.0, 10.0, 75.0, 120.0, 400.0]);
}

#[test]
fn the_race_table_has_its_own_fuel_and_sight() {
    let s = HeliSettings::default();
    assert_eq!(s.tuning(1, true).fuel_time, 10.0);
    assert_eq!(s.tuning(10, true).fuel_time, 120.0);
    assert_eq!(s.tuning(7, true).fuel_time, 220.0);
    assert_eq!(s.tuning(8, true).line_of_sight, 301.0);
}

#[test]
fn the_wait_before_a_new_helicopter_is_per_heat_level() {
    let s = HeliSettings::default();
    assert_eq!(s.respawn_secs(3, false), 120.0);
    assert_eq!(s.respawn_secs(10, false), 5.0);
    assert_eq!(s.respawn_secs(0, false), 0.0, "levels below 1 are clamped to 1");
    assert_eq!(s.respawn_secs(10, true), 120.0);
}

#[test]
fn the_levels_are_clamped_to_one_to_ten() {
    let s = HeliSettings::default();
    assert_eq!(s.tuning(42, false).fuel_time, 400.0);
}

#[test]
fn a_file_overrides_the_default_and_a_heat_line_overrides_both() {
    let text = "[Helicopter:LineOfSight]\ndefault = 260\nheat09 = 350\n";
    let s = parse(text);
    assert_eq!(s.tuning(7, false).line_of_sight, 260.0, "the default sets the levels without a line");
    assert_eq!(s.tuning(8, false).line_of_sight, 260.0);
    assert_eq!(s.tuning(9, false).line_of_sight, 350.0, "its own line wins");
    assert_eq!(s.tuning(9, true).line_of_sight, 260.0, "the default reaches the race table too, but not a heat line");
}

#[test]
fn a_race_line_sets_only_its_race_level() {
    let s = parse("[Helicopter:FuelTime]\nrace02 = 35\n");
    assert_eq!(s.tuning(2, true).fuel_time, 35.0);
    assert_eq!(s.tuning(2, false).fuel_time, 10.0, "the heat table keeps its vanilla value");
}

#[test]
fn out_of_range_values_are_clamped_and_malformed_ones_are_vanilla() {
    let s = parse("[Helicopter:FuelTime]\nheat04 = 99999\nheat05 = soon\n");
    assert_eq!(s.tuning(4, false).fuel_time, 3600.0);
    assert_eq!(s.tuning(5, false).fuel_time, 90.0);
}

#[test]
fn the_switches_read_as_words_and_numbers_and_fall_back_when_wrong() {
    let s = parse("[Helicopter:FrontSightOnly]\ndefault = 0\n[Helicopter:HidingSpots]\ndefault = maybe\n");
    assert!(!s.front_sight_only);
    assert!(s.hiding_spots, "not a switch: vanilla on");
    assert!(s.heli_sheet, "absent: vanilla on");
}

#[test]
fn the_flight_keys_reach_the_model() {
    let s = parse("[Helicopter:AttackDuration]\ndefault = 12\n[Helicopter:AllAroundSight]\ndefault = 120\n");
    let t = s.tuning(5, false);
    assert_eq!(t.attack_duration, 12.0);
    assert_eq!(t.all_around_sight, 120.0);
    assert_eq!(t.search_speed, 70.0);
}

#[test]
fn a_missing_file_is_the_vanilla_tuning() {
    let path = std::env::temp_dir().join("nfsmw-no-such-helicopter-settings.ini");
    assert_eq!(read(&path), HeliSettings::default());
}
