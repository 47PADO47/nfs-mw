use super::*;
use crate::message::{ServerName, Settings};

#[test]
fn drag_cops_and_mod_modes_are_refused_with_a_reason() {
    assert_eq!(support(Mode::Drag), Err(Unsupported::Drag));
    assert_eq!(support(Mode::CopsVsRacers), Err(Unsupported::CopsVsRacers));
    assert_eq!(support(Mode::Custom(200)), Err(Unsupported::ModMode(200)));
    let codes: Vec<u8> = [Unsupported::Drag, Unsupported::CopsVsRacers, Unsupported::ModMode(130)]
        .into_iter()
        .map(Unsupported::code)
        .collect();
    assert_eq!(codes, [1, 2, 3]);
    for mode in [Mode::Drag, Mode::CopsVsRacers, Mode::Custom(128)] {
        let setup = ModeSetup::new(mode, rules(2), vec![p(1), p(2)], START);
        assert!(matches!(AnyMode::build(&setup), Err(RuleError::Unsupported(_))), "{mode:?}");
    }
}

#[test]
fn every_built_mode_is_supported() {
    let built =
        [Mode::FreeRoam, Mode::Circuit, Mode::Sprint, Mode::Speedtrap, Mode::Knockout, Mode::PursuitCoop, Mode::Tag];
    for mode in built {
        assert_eq!(support(mode), Ok(()), "{mode:?}");
        let setup = ModeSetup::new(mode, rules(2), vec![p(2), p(1)], START);
        assert!(AnyMode::build(&setup).is_ok(), "{mode:?}");
    }
}

#[test]
fn ranges_are_checked() {
    let base = rules(3);
    let cases = [
        (CustomRules { laps: 11, ..base }, RuleError::TooManyLaps),
        (CustomRules { traffic_density: 101, ..base }, RuleError::TrafficDensity),
        (CustomRules { cop_level_cap: 6, ..base }, RuleError::CopLevel),
        (CustomRules { time_of_day: MINUTES_PER_DAY, ..base }, RuleError::TimeOfDay),
        (CustomRules { cop_level_cap: 0, ..base }, RuleError::PursuitsWithoutPolice),
    ];
    for (rules, error) in cases {
        assert_eq!(rules.validate(Mode::Circuit), Err(error));
    }
    let edge =
        CustomRules { laps: 10, traffic_density: 100, cop_level_cap: 5, time_of_day: MINUTES_PER_DAY - 1, ..base };
    assert_eq!(edge.validate(Mode::Circuit), Ok(edge));
}

#[test]
fn laps_are_required_where_a_mode_has_laps_and_dropped_elsewhere() {
    assert_eq!(rules(0).validate(Mode::Circuit), Err(RuleError::NoLaps));
    assert_eq!(rules(0).validate(Mode::Knockout), Err(RuleError::NoLaps));
    assert_eq!(rules(4).validate(Mode::Sprint).map(|rules| rules.laps), Ok(0));
    assert_eq!(rules(4).validate(Mode::Tag).map(|rules| rules.laps), Ok(0));
}

#[test]
fn pursuit_coop_needs_pursuits_and_police_may_be_off_elsewhere() {
    let off = CustomRules { pursuits: false, cop_level_cap: 0, ..rules(0) };
    assert_eq!(off.validate(Mode::PursuitCoop), Err(RuleError::CoopNeedsPursuits));
    assert_eq!(off.validate(Mode::FreeRoam), Ok(off));
    let setup = ModeSetup::new(Mode::Sprint, off, vec![p(1)], START);
    assert!(!AnyMode::build(&setup).expect("valid").police());
}

#[test]
fn a_player_listed_twice_is_refused() {
    let setup = ModeSetup::new(Mode::Sprint, rules(0), vec![p(1), p(1)], START);
    assert_eq!(AnyMode::build(&setup), Err(RuleError::DuplicatePlayer));
}

#[test]
fn rules_come_from_the_session_settings() {
    let mut settings = Settings::lan(ServerName::new("LAN").expect("a name"));
    settings.laps = 4;
    settings.nitrous = false;
    let rules = CustomRules::from_settings(&settings, 1_200);
    assert_eq!(rules.laps, 4);
    assert!(!rules.nitrous && rules.pursuits);
    assert_eq!(rules.time_of_day, 1_200);
    assert_eq!(rules.validate(Mode::Circuit), Ok(rules));
}
