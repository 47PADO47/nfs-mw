use ini_tuning::entries;

use super::*;

fn lists(text: &str) -> HeliLists {
    HeliLists::parse(&entries(text))
}

#[test]
fn without_a_file_every_list_is_the_vanilla_constants() {
    let lists = lists("");
    assert_eq!(lists.gains(5, false), Gains::default());
    assert_eq!(lists.search_radius.at(1, false), [80.0, 250.0]);
}

#[test]
fn a_default_line_sets_every_level_and_a_level_line_sets_its_own() {
    let lists = lists("[Helicopter:Leading]\ndefault = 20, 35\nheat03 = 10, 12\nrace01 = 5\n");
    assert_eq!(lists.leading.at(1, false), [20.0, 35.0]);
    assert_eq!(lists.leading.at(3, false), [10.0, 12.0]);
    assert_eq!(lists.leading.at(1, true), [5.0, 35.0], "the missing second part falls to the default line");
    assert_eq!(lists.gains(3, false).leading, [10.0, 12.0]);
}

#[test]
fn parts_are_clamped_apart_and_a_bad_part_keeps_the_vanilla_number() {
    let lists = lists("[Helicopter:ClosingSpeed]\ndefault = 5000, fast, 40\n[Helicopter:Brake]\ndefault = 2, 9\n");
    assert_eq!(lists.closing_speed.at(1, false), [1000.0, 30.0, 40.0]);
    assert_eq!(lists.brake.at(1, false), [2.0, 1.0], "the share of the speed is at most 1");
}

#[test]
fn the_turning_gain_is_used_by_its_size() {
    let lists = lists("[Helicopter:Turning]\ndefault = 1.3, -8\n");
    assert_eq!(lists.gains(1, false).turning, [1.3, 8.0]);
    assert_eq!(HeliLists::default().gains(1, false).turning, [1.3, 8.0]);
}
