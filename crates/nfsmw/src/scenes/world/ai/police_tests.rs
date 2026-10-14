//! Tests of the police state (`police.rs`).

use super::*;
use glam::Quat;

fn view(speed: f32) -> PlayerView {
    PlayerView { position: Vec3::ZERO, forward: Vec3::X, speed, nitrous: 0.0, throttle: 0.0 }
}

fn sensed(seen: bool) -> Sensed {
    Sensed {
        seen,
        unit_random: 0.5,
        on_road: true,
        nearest_cop: 10.0,
        hit_traffic: false,
        hidden: false,
        hidden_from_heli: false,
        in_tunnel: false,
        formation_random: [0.5, 0.5],
        formation_gap: None,
        support_random: 0.5,
    }
}

#[test]
fn a_chase_starts_at_its_level_and_clears_the_log() {
    let mut police = Police::default();
    police.infractions.record(Infraction::Racing, &FINES);
    police.begin(4);
    assert_eq!(police.level(), Some(4));
    assert_eq!(police.infractions().count(), 0);
}

#[test]
fn only_a_chase_counts_the_cops_it_wrecks_and_each_chase_starts_at_zero() {
    let mut police = Police::default();
    police.record_wreck();
    assert_eq!(police.cops_destroyed(), 0, "a wreck outside a chase is not counted");
    police.begin(3);
    police.record_wreck();
    police.record_wreck();
    assert_eq!(police.cops_destroyed(), 2);
    police.end(EndReason::None);
    assert_eq!(police.cops_destroyed(), 2, "the count stays after the chase, for the summary");
    police.begin(5);
    assert_eq!(police.cops_destroyed(), 0);
}

#[test]
fn speeding_while_chased_is_recorded_once() {
    let mut police = Police::default();
    police.begin(1);
    police.update(0.1, true, &view(40.0), sensed(true));
    police.update(0.1, true, &view(40.0), sensed(true));
    assert!(police.infractions().contains(Infraction::Speeding));
    assert_eq!(police.infractions().count(), 1);
}

#[test]
fn losing_the_cops_is_resisting_arrest() {
    let mut police = Police::default();
    police.begin(1);
    police.update(0.1, true, &view(5.0), sensed(true));
    police.update(0.1, true, &view(5.0), sensed(false));
    assert!(police.infractions().contains(Infraction::ResistingArrest));
}

#[test]
fn heat_grows_while_chased_and_not_otherwise() {
    let mut police = Police::default();
    police.begin(1);
    let start = police.heat.map(|h| h.value());
    for _ in 0..600 {
        police.update(1.0, true, &view(5.0), sensed(true));
    }
    assert!(police.heat.map(|h| h.value()) > start, "heat rose while chased");
    let before = police.heat.map(|h| h.value());
    police.update(1000.0, false, &view(5.0), sensed(true));
    assert_eq!(police.heat.map(|h| h.value()), before, "no growth when not chased");
}

#[test]
fn a_helicopter_joins_at_a_helicopter_level_after_its_first_wait() {
    let mut police = Police::default();
    police.begin(4);
    for _ in 0..200 {
        police.update(0.1, true, &view(5.0), sensed(true));
    }
    assert!(police.heli.is_some(), "joined after the 10 s first wait");
}

#[test]
fn no_helicopter_at_a_level_without_one() {
    let mut police = Police::default();
    police.begin(3);
    for _ in 0..200 {
        police.update(0.1, true, &view(5.0), sensed(true));
    }
    assert!(police.heli.is_none());
}

#[test]
fn the_helicopter_spawns_behind_the_player_in_physics_space() {
    let mut police = Police::default();
    police.begin(5);
    police.heli_wait = 0.0;
    police.try_join(&view(5.0));
    let heli = police.heli.expect("joined");
    assert!(heli.body.position.z < -200.0, "250 m behind: physics z is forward");
    assert!(heli.body.position.y > 19.0, "20 m up");
}

#[test]
fn the_blip_is_where_the_helicopter_is_in_render_space() {
    let mut police = Police::default();
    police.begin(5);
    police.heli_wait = 0.0;
    police.try_join(&view(5.0));
    let heli = police.heli.expect("joined");
    let ([x, y], _) = police.helicopter_blip().expect("flying");
    let physics = space::to_physics(Vec3::new(x, y, 0.0));
    assert!((physics[2] - heli.body.position.z).abs() < 1e-3, "round trip through render space");
}

#[test]
fn speed_effects_follow_the_speed() {
    let mut police = Police::default();
    police.begin(5);
    police.update(0.1, true, &view(90.0), sensed(true));
    assert!(police.effects().streaks > 0.0);
}

#[test]
fn a_hit_with_the_throttle_held_and_a_cop_close_is_hit_and_run() {
    let watched = Sensed { hit_traffic: true, nearest_cop: 10.0, ..sensed(true) };
    let mut accident = Police::default();
    accident.begin(1);
    accident.update(0.1, true, &PlayerView { speed: 20.0, throttle: 0.0, ..view(20.0) }, watched);
    assert!(!accident.infractions().contains(Infraction::HitAndRun), "no throttle: an accident");
    let mut deliberate = Police::default();
    deliberate.begin(1);
    deliberate.update(0.1, true, &PlayerView { speed: 20.0, throttle: 1.0, ..view(20.0) }, watched);
    assert!(deliberate.infractions().contains(Infraction::HitAndRun));
}

#[test]
fn a_support_roll_asks_for_a_roadblock_after_the_delay_while_seen() {
    let mut police = Police::default();
    police.begin(3);
    // Heat 3: the roadblock row (5 percent) comes first, so a low roll picks it. Its delay is 60 s.
    let roll = Sensed { support_random: 0.01, ..sensed(true) };
    for _ in 0..500 {
        police.update(0.1, true, &view(5.0), roll);
        if police.take_roadblock_request() {
            panic!("asked before the 60 s delay");
        }
    }
    let mut asked = false;
    for _ in 0..200 {
        police.update(0.1, true, &view(5.0), roll);
        asked |= police.take_roadblock_request();
    }
    assert!(asked, "after the delay a low roll asks for the roadblock");
}

#[test]
fn no_support_is_asked_while_the_cops_cannot_see_the_player() {
    let mut police = Police::default();
    police.begin(3);
    let roll = Sensed { support_random: 0.01, ..sensed(false) };
    for _ in 0..800 {
        police.update(0.1, true, &view(5.0), roll);
    }
    assert!(!police.take_roadblock_request());
}

#[test]
fn a_placed_roadblock_is_not_asked_for_again_in_the_chase() {
    let mut police = Police::default();
    police.begin(3);
    police.roadblock_placed();
    let roll = Sensed { support_random: 0.01, ..sensed(true) };
    for _ in 0..800 {
        police.update(0.1, true, &view(5.0), roll);
    }
    assert!(!police.take_roadblock_request());
}

#[test]
fn an_escape_carries_the_heat_into_the_next_chase_at_0_95() {
    let mut police = Police::default();
    police.begin(1);
    police.heat = Some(Heat::new(3.0, 10.0));
    // Unseen for longer than level 3's timeout (75 s): the chase ends, and the heat it had is kept at 0.95 of it.
    let mut evaded = false;
    for _ in 0..1000 {
        evaded |= police.update(0.1, true, &view(5.0), sensed(false));
    }
    assert!(evaded, "got away");
    assert!(
        (2.85..=3.8).contains(&police.carried_heat),
        "0.95 of the heat at the escape (3 plus a little growth): {}",
        police.carried_heat
    );
    police.begin(1);
    assert!(police.heat.map(|h| h.value()).unwrap_or(0.0) >= police.carried_heat - 1e-4);
}

#[test]
fn a_nose_along_physics_z_is_render_forward_and_up_stays_up() {
    // Rotating physics X onto physics Z is a turn of a quarter about Y; the render frame must carry that
    // into render +X (forward), with the physics up (Y) staying render +Z (up).
    let q = Quat::from_rotation_y(-std::f32::consts::FRAC_PI_2);
    let r = render_rotation(q);
    assert!((r * Vec3::X - Vec3::X).length() < 1e-5, "nose: {:?}", r * Vec3::X);
    assert!((r * Vec3::Z - Vec3::Z).length() < 1e-5, "up: {:?}", r * Vec3::Z);
}

#[test]
fn the_tables_cover_their_levels() {
    assert_eq!(time_per_level(10), 2000.0);
    assert_eq!(time_per_level(0), 600.0, "levels are clamped to 1");
}

#[test]
fn the_helicopter_reports_present_once_it_has_joined() {
    let mut police = Police::default();
    police.begin(5);
    assert!(!police.helicopter_present());
    police.heli_wait = 0.0;
    police.try_join(&view(5.0));
    assert!(police.helicopter_present());
}

#[test]
fn a_roadblock_reports_standing_once_placed() {
    let mut police = Police::default();
    police.begin(7);
    assert!(!police.roadblock_standing());
    police.roadblock_placed();
    assert!(police.roadblock_standing());
}

#[test]
fn the_helicopter_takes_its_spawn_distance_and_fuel_from_the_settings() {
    let mut police = Police::default();
    let mut settings = HeliSettings::default();
    settings.spawn_distance = crate::settings::helicopter::Levels::flat(120.0);
    police.set_heli_settings(settings);
    police.begin(8);
    police.heli_wait = 0.0;
    police.try_join(&view(5.0));
    let heli = police.heli.expect("joined");
    assert!(heli.body.position.z < -110.0 && heli.body.position.z > -130.0, "120 m behind: {:?}", heli.body.position);
    assert_eq!(heli.fuel, 75.0, "heat 8 fuel from the install table");
}

#[test]
fn the_chase_is_tense_while_seen_and_calms_as_the_cops_lose_the_player() {
    use super::world::chase_intensity;
    assert_eq!(chase_intensity(true, 0.0, 20.0), 1.0);
    assert_eq!(chase_intensity(true, 15.0, 20.0), 1.0, "being seen is full intensity whatever the progress");
    assert_eq!(chase_intensity(false, 0.0, 20.0), 1.0);
    assert_eq!(chase_intensity(false, 5.0, 20.0), 0.75);
    assert_eq!(chase_intensity(false, 20.0, 20.0), 0.0);
    assert_eq!(chase_intensity(false, 99.0, 20.0), 0.0, "past the timeout it stays at zero");
    assert_eq!(chase_intensity(false, 1.0, 0.0), 0.0, "a zero timeout does not divide by zero");
}

#[test]
fn a_chase_counts_its_seconds_only_while_it_runs() {
    let mut police = Police::default();
    police.begin(1);
    police.update(2.0, true, &view(0.0), sensed(false));
    assert_eq!(police.pursuit_time, 2.0);
    police.end(EndReason::None);
    police.update(2.0, false, &view(0.0), sensed(false));
    assert_eq!(police.pursuit_time, 2.0, "no clock runs without a chase");
}

#[test]
fn an_escape_is_evaded_and_the_reason_and_the_card_clock_stay_after_the_chase() {
    use blackbox_pursuit::CooldownStatus;
    let mut police = Police::default();
    police.begin(1);
    // Level 1 needs 20 s unseen; the sight timer starts at -5 s, so the escape is reached near 25 s.
    let mut evaded = false;
    for _ in 0..400 {
        evaded |= police.update(0.1, true, &view(5.0), sensed(false));
    }
    assert!(evaded, "got away");
    assert_eq!(police.cooldown_status(), CooldownStatus::Evaded);
    assert_eq!(police.end_reason(), EndReason::None, "the world ends the chase; the reason is set then");
    police.end(EndReason::Evaded);
    assert_eq!(police.end_reason(), EndReason::Evaded);
    police.update(0.5, false, &view(5.0), sensed(false));
    assert!((police.since_end() - 0.5).abs() < 1e-4, "the card's clock runs once the chase is over");
    police.begin(1);
    assert_eq!(police.end_reason(), EndReason::None, "a new chase clears the reason");
}

#[test]
fn a_bust_ends_the_chase_busted_and_the_escape_does_not_tick() {
    use blackbox_pursuit::busted::BustInput;
    let mut police = Police::default();
    police.begin(1);
    let bust = BustInput { cop_distance: 5.0, seen: true, player_speed: 0.0, slow_speed: 5.0, flashing: false };
    let mut busted = false;
    for _ in 0..100 {
        busted |= police.tick_busted(0.1, true, bust);
    }
    assert!(busted, "busted after the 5 s timeout");
    let mut evaded = false;
    for _ in 0..100 {
        evaded |= police.update(0.1, true, &view(5.0), sensed(false));
    }
    assert!(!evaded, "a busted chase does not evade");
    assert_eq!(police.escape_progress(), -5.0, "the escape stayed at its start");
}
