use super::pursuit::HeatBounds;
use super::*;

fn parsed(text: &str) -> ExtraOptions {
    ExtraOptions::parse(text).options
}

#[test]
fn the_vanilla_heat_range_runs_from_one_to_the_top_level() {
    assert_eq!(HeatBounds::default(), HeatBounds { min: 1.0, max: 10.0 });
}

#[test]
fn a_forced_heat_range_is_read_when_the_override_is_on() {
    let text = "[Pursuit]\nHeatLevelOverride = 1\nMinimumHeatLevel = 3\nMaximumHeatLevel = 7\n";
    assert_eq!(parsed(text).heat_bounds(), Some(HeatBounds { min: 3.0, max: 7.0 }));
    assert!(ExtraOptions::parse(text).unsupported.is_empty());
}

#[test]
fn the_heat_range_needs_the_override_switch() {
    assert_eq!(parsed("[Pursuit]\nMinimumHeatLevel = 3\nMaximumHeatLevel = 7\n").heat_bounds(), None);
    assert_eq!(parsed("[Pursuit]\nHeatLevelOverride = 0\nMinimumHeatLevel = 3\n").heat_bounds(), None);
}

#[test]
fn an_override_with_no_levels_keeps_the_vanilla_ends() {
    assert_eq!(parsed("[Pursuit]\nHeatLevelOverride = 1\n").heat_bounds(), Some(HeatBounds::default()));
}

#[test]
fn heat_levels_are_whole_numbers_from_one_to_the_top() {
    let low = parsed("[Pursuit]\nHeatLevelOverride = 1\nMinimumHeatLevel = 0\nMaximumHeatLevel = 99\n");
    assert_eq!(low.heat_bounds(), Some(HeatBounds { min: 1.0, max: 10.0 }));
    let rounded = parsed("[Pursuit]\nHeatLevelOverride = 1\nMinimumHeatLevel = 4.6\n");
    assert_eq!(rounded.heat_bounds().map(|b| b.min), Some(5.0));
}

#[test]
fn a_bad_heat_value_keeps_its_end_at_the_vanilla_value() {
    let bad = parsed("[Pursuit]\nHeatLevelOverride = 1\nMinimumHeatLevel = high\n");
    assert_eq!(bad.heat_bounds(), Some(HeatBounds::default()));
    assert_eq!(parsed("[Pursuit]\nHeatLevelOverride = maybe\nMinimumHeatLevel = 3\n").heat_bounds(), None);
}

#[test]
fn the_speeding_limit_is_read_in_metres_per_second() {
    let text = "[Pursuit]\nSpeedingLimit = 67\n";
    assert_eq!(parsed(text).speeding_limit(), Some(67.0));
    assert!(ExtraOptions::parse(text).unsupported.is_empty());
}

#[test]
fn the_speeding_and_reckless_limits_are_unset_by_default() {
    assert_eq!(parsed("").speeding_limit(), None);
    assert_eq!(parsed("").reckless_limit(), None);
}

#[test]
fn the_infraction_speeds_are_clamped_to_their_range() {
    assert_eq!(parsed("[Pursuit]\nSpeedingLimit = -5\n").speeding_limit(), Some(0.0));
    assert_eq!(parsed("[Pursuit]\nRecklessDrivingLimit = 5000\n").reckless_limit(), Some(1000.0));
}

#[test]
fn the_reckless_driving_limit_is_read() {
    assert_eq!(parsed("[Pursuit]\nRecklessDrivingLimit = 89.4\n").reckless_limit(), Some(89.4));
}

#[test]
fn the_excessive_speeding_limit_has_no_infraction_yet_so_it_stays_unsupported() {
    let parsed = ExtraOptions::parse("[Pursuit]\nExcessiveSpeedingLimit = 80.5\n");
    assert_eq!(parsed.unsupported, vec!["Pursuit.ExcessiveSpeedingLimit".to_owned()]);
}
