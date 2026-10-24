use super::super::tests::{sensed, view};
use super::*;

#[test]
fn an_airborne_helicopter_tracks_current_heat_tuning_without_refilling() {
    use nfsmw_data::pursuit::LevelTuning;
    let row = LevelTuning {
        front_los: 151.0,
        rear_los: 151.0,
        heli_los: 333.0,
        speed_reaction: 0.1,
        evade_timeout: 120.0,
        hidden_multiplier: 4.0,
        heli_fuel: 999.0,
        heli_gap: 180.0,
        search_heli_chance: 4.0,
        roadblock_chance: 40.0,
        spike_chance: 80.0,
        search_roadblock_radius: 4321.0,
        search_roadblock_chance: 20.0,
        search_city_mph: 60.0,
        search_highway_mph: 100.0,
        bust_speed_kmh: 35.0,
    };
    let mut police = chase(4);
    police.set_level_tuning(vec![row; 10]);
    police.heli = Some(Helicopter::spawn(Vec3::new(-100.0, 40.0, 0.0), 0.0, Default::default()));
    police.heli.as_mut().unwrap().fuel = 4.0;
    police.heat.as_mut().unwrap().raise_to(5.0);
    police.think_helicopter(crate::scenes::world::drive::STEP, true, &view(10.0), sensed(true), &|_, _| true);
    let heli = police.heli.as_ref().unwrap();
    assert_eq!(heli.tuning.line_of_sight, 333.0);
    assert_eq!(heli.tuning.search_radius, 4321.0);
    assert!(heli.tuning.skid_hit);
    assert!(heli.fuel <= 4.0);
    assert_eq!(heli.tuning.fuel_time, 60.0, "the original tank capacity remains the spawn capacity");
}

#[test]
fn drawing_interpolates_poses_without_changing_physics_telemetry() {
    let mut police = chase(5);
    police.heli = Some(Helicopter::spawn(Vec3::X * 10.0, 0.0, Default::default()));
    police.heli_plan.previous = Some((Vec3::ZERO, Quat::IDENTITY));
    for (alpha, expected) in [(0.0, 0.0), (0.25, 2.5), (0.5, 5.0), (1.0, 10.0)] {
        police.set_heli_alpha(alpha);
        assert!((police.helicopter_pose().unwrap().0.x - expected).abs() < 1e-5);
        assert_eq!(police.helicopter_motion().unwrap().0.x, 10.0);
    }
}

fn chase(level: u32) -> Police {
    let mut police = Police::default();
    police.begin(level);
    police
}

/// A chase that has lasted long enough, with the first wait spent.
fn ready(level: u32) -> Police {
    let mut police = chase(level);
    police.heli_wait = 0.0;
    police.pursuit_time = 1000.0;
    police
}

#[test]
fn the_helicopter_waits_for_its_timer_and_the_minimum_support_delay() {
    let mut police = chase(5);
    assert_eq!(police.heli_ask(5, 0.9), Ask::No, "the 10 s timer has not run out");
    police.heli_wait = 0.0;
    assert_eq!(police.heli_ask(5, 0.9), Ask::No, "heat 5 asks 35 s of chase first");
    police.pursuit_time = 36.0;
    assert_eq!(police.heli_ask(5, 0.9), Ask::Allowed);
}

#[test]
fn one_helicopter_at_a_time_and_none_while_one_is_on_its_way() {
    let mut police = ready(5);
    police.queue_helicopter(Spawn { position: Vec3::ZERO, heading: 0.0 });
    assert_eq!(police.heli_ask(5, 0.9), Ask::No);
    police.try_join(&PlayerView {
        nos_input: false,
        nitrous_boost: 0.0,
        velocity: (Vec3::X) * (0.0),
        position: Vec3::ZERO,
        forward: Vec3::X,
        speed: 0.0,
        nitrous: 0.0,
        throttle: 0.0,
    });
    assert!(police.helicopter_present());
    assert_eq!(police.heli_ask(5, 0.9), Ask::No);
}

#[test]
fn a_busted_player_is_not_chased_by_a_new_helicopter() {
    let mut police = ready(5);
    police.start_bust(0.0);
    assert_eq!(police.heli_ask(5, 0.9), Ask::No);
}

#[test]
fn losing_the_cops_arms_the_search_roll_once_and_a_low_roll_forces_the_helicopter() {
    let mut police = ready(5);
    police.update(1.0, true, &view(10.0), sensed(true));
    for _ in 0..240 {
        police.update(0.05, true, &view(10.0), sensed(false));
    }
    assert_eq!(police.cooldown_status(), CooldownStatus::CoolDown);
    assert!(police.heli_plan.search_armed, "the meter appeared");
    assert_eq!(police.heli_ask(5, 0.03), Ask::Forced, "3 percent is under the 4 of heat 5");
    assert!(police.heli_search_call(), "the quadrant call is due");
    assert!(!police.heli_plan.search_armed, "the roll is spent");
}

#[test]
fn the_chase_coming_back_disarms_the_search_roll() {
    let mut police = ready(5);
    police.update(1.0, true, &view(10.0), sensed(true));
    for _ in 0..240 {
        police.update(0.05, true, &view(10.0), sensed(false));
    }
    for _ in 0..80 {
        police.update(0.05, true, &view(10.0), sensed(true));
    }
    assert_eq!(police.cooldown_status(), CooldownStatus::InitialChase);
    assert!(!police.heli_plan.search_armed);
}

#[test]
fn a_high_roll_leaves_the_helicopter_to_the_wave() {
    let mut police = ready(5);
    police.heli_plan.search_armed = true;
    assert_eq!(police.heli_ask(5, 0.5), Ask::Allowed);
    assert!(!police.heli_search_call());
    assert_eq!(police.heli_ask(5, 0.0), Ask::Allowed, "the roll is made only once per loss of contact");
}

#[test]
fn heat_one_to_three_have_no_search_spawn() {
    let mut police = ready(3);
    police.heli_plan.search_armed = true;
    assert_eq!(police.heli_ask(3, 0.0), Ask::Allowed, "a zero chance never forces it");
}

#[test]
fn a_breaker_destroys_the_helicopter_and_the_next_one_waits() {
    let mut police = ready(5);
    police.queue_helicopter(Spawn { position: Vec3::ZERO, heading: 0.0 });
    police.try_join(&PlayerView {
        nos_input: false,
        nitrous_boost: 0.0,
        velocity: (Vec3::X) * (0.0),
        position: Vec3::ZERO,
        forward: Vec3::X,
        speed: 0.0,
        nitrous: 0.0,
        throttle: 0.0,
    });
    police.helicopter_destroyed();
    assert!(!police.helicopter_present());
    assert_eq!(police.heli_wait, 180.0, "heat 5 waits 180 s");
    assert_eq!(police.heli_ask(5, 0.9), Ask::No);
}

#[test]
fn the_status_tells_fuel_sight_and_mode_for_the_speech() {
    let mut police = ready(5);
    assert!(police.helicopter_status().is_none());
    police.queue_helicopter(Spawn { position: Vec3::ZERO, heading: 0.0 });
    police.try_join(&PlayerView {
        nos_input: false,
        nitrous_boost: 0.0,
        velocity: (Vec3::X) * (0.0),
        position: Vec3::ZERO,
        forward: Vec3::X,
        speed: 0.0,
        nitrous: 0.0,
        throttle: 0.0,
    });
    let status = police.helicopter_status().expect("flying");
    assert_eq!(status.mode, HeliMode::Following);
    assert_eq!(status.fuel_secs, 90.0, "heat 5's tank");
    assert_eq!(status.fuel_fraction, 1.0);
    assert_eq!(status.health, 1.0);
}
