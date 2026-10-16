use super::*;
use crate::settings::speedbreaker::{Overrides, Recharge, SpeedbreakerTuning, parse, parse_over, vanilla};

const TOLERANCE: f32 = 1e-4;

fn overrides(text: &str) -> Overrides {
    ExtraOptions::parse(text).options.speedbreaker()
}

#[test]
fn the_speedbreaker_keys_parse_and_leave_nothing_unsupported() {
    let parsed =
        ExtraOptions::parse("[Gameplay]\nSBRechargeTime = 40\nSBRechargeSpeedLimit = 80\nSBMassMultiplier = 3\n");
    assert_eq!(parsed.options.sb_recharge_seconds, Some(40.0));
    assert_eq!(parsed.options.sb_recharge_mph, Some(80.0));
    assert_eq!(parsed.options.sb_mass_multiplier, Some(3.0));
    assert!(parsed.unsupported.is_empty());
}

#[test]
fn unset_speedbreaker_keys_give_no_overrides() {
    assert_eq!(overrides(""), Overrides::default());
    assert_eq!(overrides("[Gameplay]\nGameSpeed = 1\n"), Overrides::default());
}

#[test]
fn the_mods_defaults_are_the_vanilla_recharge_and_mass() {
    let set = overrides("[Gameplay]\nSBRechargeTime = 25\nSBRechargeSpeedLimit = 100\nSBMassMultiplier = 2\n");
    assert_eq!(
        set.recharge,
        Some(Recharge { min_speed: vanilla::RECHARGE_ABOVE_MPS, seconds: vanilla::RECHARGE_SECONDS })
    );
    assert_eq!(set.mass_scale, Some(vanilla::CAR_MASS_SCALE));
}

#[test]
fn a_mph_speed_limit_becomes_metres_per_second() {
    let set = overrides("[Gameplay]\nSBRechargeSpeedLimit = 60\n");
    let recharge = set.recharge.expect("the speed limit turns the recharge on");
    assert!((recharge.min_speed - 60.0 * vanilla::MPH).abs() < TOLERANCE, "{}", recharge.min_speed);
}

#[test]
fn one_recharge_key_fills_the_other_with_the_vanilla_value() {
    let time_only = overrides("[Gameplay]\nSBRechargeTime = 40\n").recharge.expect("set");
    assert_eq!((time_only.min_speed, time_only.seconds), (vanilla::RECHARGE_ABOVE_MPS, 40.0));
    let speed_only = overrides("[Gameplay]\nSBRechargeSpeedLimit = 60\n").recharge.expect("set");
    assert_eq!(speed_only.seconds, vanilla::RECHARGE_SECONDS);
}

#[test]
fn the_recharge_time_is_clamped_to_a_positive_range() {
    assert_eq!(overrides("[Gameplay]\nSBRechargeTime = 0\n").recharge.map(|r| r.seconds), Some(0.001));
    assert_eq!(overrides("[Gameplay]\nSBRechargeTime = 99999\n").recharge.map(|r| r.seconds), Some(3600.0));
}

#[test]
fn the_recharge_speed_is_clamped_to_its_range() {
    let fast = overrides("[Gameplay]\nSBRechargeSpeedLimit = 5000\n").recharge.expect("set");
    assert!((fast.min_speed - 1000.0 * vanilla::MPH).abs() < TOLERANCE);
    let negative = overrides("[Gameplay]\nSBRechargeSpeedLimit = -4\n").recharge.expect("set");
    assert_eq!(negative.min_speed, 0.0);
}

#[test]
fn the_mass_multiplier_is_clamped_to_its_range() {
    assert_eq!(overrides("[Gameplay]\nSBMassMultiplier = 500\n").mass_scale, Some(100.0));
    assert_eq!(overrides("[Gameplay]\nSBMassMultiplier = -1\n").mass_scale, Some(0.0));
}

#[test]
fn a_bad_speedbreaker_value_leaves_its_key_unset() {
    let set = overrides("[Gameplay]\nSBRechargeTime = lots\nSBMassMultiplier = NaN\n");
    assert_eq!(set, Overrides::default());
}

#[test]
fn an_empty_speedfixer_file_keeps_the_vanilla_physics_and_no_recharge() {
    let tuning = parse_over("", &Overrides::default());
    assert_eq!(tuning.physics, SpeedbreakerTuning::default().physics);
    assert_eq!(tuning.passive, None, "the mod's rule: no recharge keys, no recharge");
}

#[test]
fn an_override_stands_in_for_a_key_the_speedfixer_file_leaves_out() {
    let recharge = Recharge { min_speed: 20.0, seconds: 40.0 };
    let overrides = Overrides { recharge: Some(recharge), mass_scale: None };
    let tuning = parse_over("[Activation]\nminCarSpeed = 48.2803\n", &overrides);
    assert_eq!(tuning.passive, Some(recharge));
}

#[test]
fn the_speedfixer_file_wins_over_an_override_it_sets() {
    let recharge = Recharge { min_speed: 20.0, seconds: 40.0 };
    let overrides = Overrides { recharge: Some(recharge), mass_scale: None };
    let tuning = parse_over("[Recharging]\npassiveTime = 10.0\n", &overrides);
    let passive = tuning.passive.expect("the file turns recharge on");
    assert_eq!(passive.seconds, 10.0, "the file's time");
    assert_eq!(passive.min_speed, 20.0, "the override's speed");
}

#[test]
fn a_mass_override_stands_in_for_the_vanilla_mass_scale_only() {
    let overrides = Overrides { recharge: None, mass_scale: Some(3.0) };
    assert_eq!(parse_over("", &overrides).physics.car_mass_scale, 3.0);
    assert_eq!(parse_over("[Physics]\ncarMassScale = 1.5\n", &overrides).physics.car_mass_scale, 1.5);
}

#[test]
fn a_speedfixer_file_with_no_recharge_keys_keeps_recharge_off_without_an_override() {
    assert_eq!(parse("[Activation]\nminCarSpeed = 48.2803\n").passive, None);
}

#[test]
fn an_override_turns_recharge_on_for_a_speedfixer_file_with_no_recharge_keys() {
    let recharge = Recharge { min_speed: 20.0, seconds: 40.0 };
    let overrides = Overrides { recharge: Some(recharge), mass_scale: None };
    let tuning = parse_over("[Activation]\nminCarSpeed = 48.2803\n", &overrides);
    assert_eq!(tuning.passive, Some(recharge), "absent keys disable recharge, unless the options set it");
}
