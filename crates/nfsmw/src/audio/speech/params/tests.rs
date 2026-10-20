use super::signatures::SIGNATURES;
use super::values::{car_color, car_type, direction, measurement, pursuit_type, speed};
use super::*;

#[test]
fn every_parameter_has_a_name_that_finds_it_again() {
    for param in Param::ALL {
        assert_eq!(Param::from_name(param.name()), Some(param));
        assert_eq!(Param::from_name(&param.name().to_ascii_uppercase()), Some(param));
    }
    assert_eq!(Param::from_name("not_a_parameter"), None);
    let names: std::collections::HashSet<&str> = Param::ALL.iter().map(|p| p.name()).collect();
    assert_eq!(names.len(), Param::ALL.len());
}

#[test]
fn the_interface_list_covers_the_events_and_no_event_repeats_a_parameter() {
    assert!(SIGNATURES.len() >= 130);
    let mut seen = std::collections::HashSet::new();
    for (name, _, fields) in SIGNATURES {
        assert!(seen.insert(*name), "{name} twice");
        assert!(fields.len() <= CAPACITY, "{name}");
        let distinct: std::collections::HashSet<_> = fields.iter().collect();
        assert_eq!(distinct.len(), fields.len(), "{name} repeats a parameter");
    }
    assert_eq!(
        signature("setup_vehiclereport").unwrap().1,
        [Param::CarColor, Param::CarType, Param::Speed, Param::Measurement]
    );
    assert_eq!(signature("VehicleReport"), signature("setup_vehiclereport"), "case does not matter");
    assert_eq!(signature("cellcall"), Some((false, &[Param::CellCallBucket, Param::CellCallNumber][..])));
    assert_eq!(signature("nonsense"), None);
}

#[test]
fn parameters_are_numbered_after_the_speaker() {
    let params = SpeechParams::new()
        .with(Param::PursuitType, pursuit_type::HIT_AND_RUN)
        .with(Param::NumSuspects, 2)
        .with(Param::SpeakerBattalion, 4)
        .with(Param::SpeakerCallSignId, call_sign(7));
    let numbered = lib_params("setup_attmptvehstp", 4, &params);
    assert_eq!([1, 2, 3, 4, 5, 6].map(|n| numbered.get(n)), [Some(4), Some(4), Some(2), Some(4), Some(64), None]);
    // Events without a speaker start at 1; the speaker is dropped.
    let cell = lib_params("cellcall", 3, &SpeechParams::new().with(Param::CellCallBucket, 2));
    assert_eq!((cell.get(1), cell.get(2)), (Some(2), None));
}

#[test]
fn a_request_without_parameters_leaves_every_variant_open() {
    let numbered = lib_params("setup_vehiclereport", 0, &SpeechParams::new());
    assert!((1..=5).all(|n| numbered.get(n).is_none()));
    // Parameters the event does not take are ignored.
    let stray =
        lib_params("setup_vehiclereport", 2, &SpeechParams::new().with(Param::Direction, direction::NORTHBOUND));
    assert_eq!((stray.get(1), stray.get(2)), (Some(2), None));
    // An event the list does not know still gets its voice first.
    assert_eq!(lib_params("nonsense", 5, &SpeechParams::new()).get(1), Some(5));
}

#[test]
fn a_parameter_set_twice_keeps_the_last_value_and_the_set_has_a_limit() {
    let mut params = SpeechParams::new().with(Param::CarColor, car_color::RED);
    params.set(Param::CarColor, car_color::BLUE);
    assert_eq!(params.get(Param::CarColor), Some(car_color::BLUE));
    assert_eq!(params.iter().count(), 1);
    assert!(!params.is_empty());
    assert!(SpeechParams::new().is_empty());
    let mut full = SpeechParams::new();
    for param in Param::ALL.iter().take(CAPACITY + 3) {
        full.set(*param, 1);
    }
    assert_eq!(full.iter().count(), CAPACITY);
}

#[test]
fn values_are_the_bits_of_the_games_interface() {
    assert_eq!((car_type::MCLAREN, car_type::CADILLAC), (1, 1 << 30));
    assert_eq!(measurement::IMPERIAL_ONLY, 2);
    assert_eq!(location(31), 1 << 30);
    assert_eq!(call_sign(20), 1 << 19);
    assert_eq!(heat_level(2), 1);
    assert_eq!(heat_level(10), 256);
}

#[test]
fn speeds_step_by_twenty_from_one_hundred() {
    assert_eq!(speed_for(0.0), speed::OVER_SPEED_LIMIT);
    assert_eq!(speed_for(99.9), speed::OVER_SPEED_LIMIT);
    assert_eq!(speed_for(100.0), speed::SPEEDS_IN_EXCESS_OF_100);
    assert_eq!(speed_for(119.0), speed::SPEEDS_IN_EXCESS_OF_100);
    assert_eq!(speed_for(120.0), speed::SPEEDS_IN_EXCESS_OF_120);
    assert_eq!(speed_for(299.0), speed::SPEEDS_IN_EXCESS_OF_280);
    assert_eq!(speed_for(300.0), speed::SPEEDS_IN_EXCESS_OF_300);
    assert_eq!(speed_for(900.0), speed::SPEEDS_IN_EXCESS_OF_300);
}
