//! The opening exchange of a chase and the parameters of its lines.

use glam::Vec3;

use super::super::*;
use super::support::*;
use crate::infraction::Infraction;

#[test]
fn a_chase_opens_with_the_vehicle_stop_dispatch_and_the_vehicle_report() {
    let mut chase = Chase::new(1);
    chase.run(12.0);
    let played = chase.radio.played_lines();
    assert_eq!(
        &played[..3],
        &[Line::AttemptVehicleStop, Line::GoAhead, Line::VehicleReport],
        "the opening exchange: {played:?}"
    );
    let stop = chase.radio.first(Line::AttemptVehicleStop).unwrap();
    assert!(layout().primary.contains(&stop.speaker), "the leader speaks with a primary voice: {}", stop.speaker);
    assert_eq!(chase.radio.first(Line::GoAhead).unwrap().speaker, DISPATCH);
}

#[test]
fn the_vehicle_stop_names_the_unit_the_suspects_and_the_offence() {
    let mut chase = Chase::new(2);
    chase.infraction(Infraction::RecklessDriving);
    chase.run(6.0);
    let params = chase.radio.first(Line::AttemptVehicleStop).unwrap().params;
    assert_eq!(params.pursuit_type, Some(PursuitType::Reckless));
    assert_eq!(params.suspects, Some(Suspects::One));
    assert_eq!(params.battalion, Some(CITY));
    assert!((1..=20).contains(&params.call_sign.unwrap()), "{:?}", params.call_sign);
}

#[test]
fn the_wording_of_the_offence_follows_the_last_infraction() {
    let cases = [
        (Infraction::Speeding, PursuitType::GenericSpeeder),
        (Infraction::Racing, PursuitType::GenericSpeeder),
        (Infraction::OffRoad, PursuitType::Reckless),
        (Infraction::Assault, PursuitType::UnitRammed),
        (Infraction::HitAndRun, PursuitType::HitAndRun),
        (Infraction::DamageToProperty, PursuitType::HitAndRun),
        (Infraction::ResistingArrest, PursuitType::PossibleWanted),
    ];
    for (infraction, wanted) in cases {
        let mut chase = Chase::new(3);
        chase.infraction(infraction);
        chase.run(6.0);
        let params = chase.radio.first(Line::AttemptVehicleStop).unwrap().params;
        assert_eq!(params.pursuit_type, Some(wanted), "{infraction:?}");
    }
}

#[test]
fn a_hot_chase_calls_speeding_a_possible_wanted() {
    let mut chase = Chase::new(4);
    chase.snap.pursuit.heat = 2.0;
    chase.infraction(Infraction::Speeding);
    chase.run(6.0);
    let params = chase.radio.first(Line::AttemptVehicleStop).unwrap().params;
    assert_eq!(params.pursuit_type, Some(PursuitType::PossibleWanted));
}

#[test]
fn the_vehicle_report_describes_the_car_and_the_speed() {
    let mut chase = Chase::new(5);
    chase.snap.player.speed_mph = 150.0;
    chase.run(8.0);
    let report = chase.radio.first(Line::VehicleReport).unwrap().params;
    assert_eq!(report.car, Some(Car { kind: 2, colour: 1 }));
    assert_eq!(report.speed, Some(150.0));
    assert!(matches!(report.measurement, Some(Measurement::Imperial | Measurement::Generic)));
}

#[test]
fn a_slow_player_is_over_the_speed_limit_in_generic_units() {
    let mut chase = Chase::new(6);
    chase.snap.player.speed_mph = 70.0;
    chase.run(8.0);
    let report = chase.radio.first(Line::VehicleReport).unwrap().params;
    assert_eq!(report.measurement, Some(Measurement::Generic));
    assert!(report.speed.unwrap() < 100.0);
}

#[test]
fn a_metric_speedometer_reports_kilometres() {
    let mut chase = Chase::new(7);
    chase.snap.player.speed_mph = 150.0;
    chase.snap.player.metric = true;
    chase.run(8.0);
    let report = chase.radio.first(Line::VehicleReport).unwrap().params;
    assert!((report.speed.unwrap() - 241.4).abs() < 0.5, "{:?}", report.speed);
}

#[test]
fn no_car_description_means_no_vehicle_report() {
    let mut chase = Chase::new(8);
    chase.snap.player.car = None;
    chase.run(8.0);
    assert_eq!(chase.radio.count(Line::VehicleReport), 0);
    assert!(chase.radio.count(Line::AttemptVehicleStop) > 0, "the stop is still said");
}

#[test]
fn the_opening_waits_for_the_initial_delay() {
    let mut chase = Chase::new(9);
    chase.run(1.5);
    assert!(
        chase.radio.lines().is_empty(),
        "nothing before the pursuit has run two seconds: {:?}",
        chase.radio.lines()
    );
}

#[test]
fn the_leader_reports_the_heading_once_it_has_held_for_three_seconds() {
    let mut chase = Chase::new(10);
    chase.snap.player.velocity_dir = Vec3::NEG_X;
    chase.run(14.0);
    let report = chase.radio.first(Line::LocationReport).unwrap().params;
    assert_eq!(report.direction, Some(Direction::West));
    assert_eq!(report.road, Some(5));
    assert_eq!(report.encounter, Some(Encounter::First));
}

#[test]
fn no_location_is_reported_on_a_road_the_cops_cannot_name() {
    let mut chase = Chase::new(11);
    chase.snap.player.road = Some(0);
    chase.run(14.0);
    assert_eq!(chase.radio.count(Line::LocationReport), 0);
}

#[test]
fn a_hit_on_a_cop_restarts_the_opening_as_unit_rammed() {
    let mut chase = Chase::new(12);
    chase.run(2.6);
    chase.notices.push(Notice::PlayerHitCop { cop: 10, impact: Impact::RearEnd, intensity: 0.3, deliberate: true });
    chase.run(6.0);
    let stops = chase.radio.all(Line::AttemptVehicleStop);
    assert_eq!(stops.len(), 2, "the opening is said again after the hit");
    assert_eq!(stops.last().unwrap().params.pursuit_type, Some(PursuitType::UnitRammed));
    assert_eq!(
        chase.radio.played_lines().iter().filter(|l| **l == Line::VehicleReport).count(),
        0,
        "the first report was cut with the queue and there is none after an assault"
    );
}

#[test]
fn a_cop_the_player_blew_past_opens_with_a_spotter() {
    let mut chase = Chase::new(13);
    // The cop is slow, far behind the player's speed, and close.
    chase.snap.cops[0].speed_mph = 10.0;
    chase.snap.cops[0].distance = 30.0;
    chase.snap.player.speed_mph = 100.0;
    chase.run(8.0);
    let played = chase.radio.played_lines();
    assert!(played.contains(&Line::Spotter) || played.contains(&Line::SpotterWanted), "{played:?}");
    assert!(!played.contains(&Line::AttemptVehicleStop));
}

#[test]
fn a_stopped_player_cuts_the_opening_short() {
    let mut chase = Chase::new(14);
    chase.run(2.6);
    chase.snap.player.speed_mph = 0.0;
    chase.run(5.0);
    assert!(chase.radio.count(Line::Interrupt) > 0, "{:?}", chase.radio.lines());
}

#[test]
fn only_one_cop_speaks_for_each_voice() {
    let mut chase = Chase::new(15);
    chase.snap.cops = vec![cop(10, 40.0), cop(11, 60.0), cop(12, 80.0), cop(13, 90.0)];
    chase.run(5.0);
    let mut voices: Vec<u8> = chase.machine.cops().iter().map(|a| a.voice).collect();
    voices.sort_unstable();
    voices.dedup();
    assert_eq!(voices.len(), chase.machine.cops().len(), "{voices:?}");
}

#[test]
fn call_signs_are_not_repeated_within_a_battalion() {
    let mut chase = Chase::new(16);
    chase.snap.cops = (0..6).map(|i| cop(10 + i, 40.0 + 10.0 * i as f32)).collect();
    chase.run(5.0);
    let mut signs: Vec<(u16, u8)> = chase.machine.cops().iter().map(|a| (a.battalion.0, a.call_sign)).collect();
    let all = signs.len();
    signs.sort_unstable();
    signs.dedup();
    assert_eq!(signs.len(), all);
}

#[test]
fn a_cop_that_leaves_the_world_frees_its_voice_for_the_next() {
    let mut chase = Chase::new(17);
    chase.snap.cops = (0..6).map(|i| cop(10 + i, 40.0 + 10.0 * i as f32)).collect();
    chase.run(3.0);
    assert_eq!(chase.machine.cops().len(), 6, "six voices besides the cross car's, six cops");
    // Three drive away for good and three more arrive farther out than the rest.
    chase.snap.cops.drain(0..3);
    chase.run(1.0);
    assert_eq!(chase.machine.cops().len(), 3);
    chase.snap.cops.extend((0..3).map(|i| cop(30 + i, 200.0 + 10.0 * i as f32)));
    chase.run(1.0);
    let mut voices: Vec<u8> = chase.machine.cops().iter().map(|a| a.voice).collect();
    voices.sort_unstable();
    voices.dedup();
    assert_eq!(voices.len(), 6, "every cop has a voice of its own again: {voices:?}");
}

#[test]
fn a_nearer_cop_takes_the_place_of_a_farther_one_when_the_voices_run_out() {
    let mut chase = Chase::new(18);
    chase.snap.cops = (0..6).map(|i| cop(10 + i, 100.0 + 10.0 * i as f32)).collect();
    chase.run(3.0);
    let farthest = chase.machine.cops().iter().max_by(|a, b| a.distance.total_cmp(&b.distance)).copied().unwrap();
    chase.snap.cops.push(cop(50, 20.0));
    chase.run(1.0);
    assert_eq!(chase.machine.cops().len(), 6, "no seventh voice");
    let newcomer = chase.machine.cops().iter().find(|a| a.id == 50).expect("the nearer cop has an actor");
    assert!(newcomer.distance < farthest.distance);
}
