use super::*;
use nfsmw_career::story::chase_16_2_1::{CHASE_CALL_ID, COP_SPAWN_COUNT, CopKind, SABOTAGE_LEVEL};
use nfsmw_data::gameplay::PlaneTrigger;

const RACE: &str = "race_bin_16/16_2_1_d_day_intro";

/// A sphere volume of radius 5 m at the origin.
fn sphere() -> Option<Zone> {
    Some(Zone::Sphere(SphereTrigger {
        key: 1,
        name: format!("{RACE}/call_from_mia"),
        position: Vec3::ZERO,
        radius: 5.0,
        threshold_speed: 0.0,
    }))
}

fn inside() -> Vec3 {
    Vec3::ZERO
}

fn outside() -> Vec3 {
    Vec3::X * 100.0
}

/// A run whose volume is only the one at `slot` (0 call, 1 kill, 2 cops, 3 bail).
fn run_with(slot: usize) -> ChaseRun {
    let mut zones: [Option<Zone>; 4] = [None, None, None, None];
    zones[slot] = sphere();
    ChaseRun::from_zones(zones)
}

#[test]
fn a_trigger_fires_when_the_player_goes_in_not_while_inside() {
    let mut inside_now = [false; 4];
    assert_eq!(enter_edges(&mut inside_now, [true, false, false, false]), [true, false, false, false]);
    assert_eq!(enter_edges(&mut inside_now, [true, false, false, false]), [false; 4]);
    assert_eq!(enter_edges(&mut inside_now, [false; 4]), [false; 4]);
    assert_eq!(enter_edges(&mut inside_now, [true, false, false, false]), [true, false, false, false]);
}

#[test]
fn a_trigger_child_is_found_by_its_full_name_as_a_box_or_a_sphere() {
    let mut data = GameplayData::default();
    data.spheres.push(SphereTrigger {
        key: 2,
        name: format!("{RACE}/kill_engine"),
        position: Vec3::ZERO,
        radius: 3.0,
        threshold_speed: 0.0,
    });
    assert!(matches!(zone_named(&data, RACE, "kill_engine"), Some(Zone::Sphere(_))));
    assert!(zone_named(&data, RACE, "bail_trigger").is_none());
    assert!(zone_named(&data, "race_bin_16/other", "kill_engine").is_none());
}

#[test]
fn the_call_trigger_requests_phone_call_10_once() {
    let mut run = run_with(CALL);
    assert!(run.update(outside(), 1.0).is_empty());
    let events = run.update(inside(), 1.0);
    assert_eq!(events, vec![ChaseEvent::PlaceCall { id: CHASE_CALL_ID }]);
    assert!(run.update(inside(), 1.0).is_empty(), "the call is requested once");
}

#[test]
fn the_backup_sabotage_comes_15_seconds_after_the_call_trigger() {
    let mut run = run_with(CALL);
    run.step(inside(), 0.0, false);
    let mut sabotaged_at = None;
    for frame in 1..=20 {
        // The call never starts here, so only the backup timer can sabotage the engine.
        let events = run.step(inside(), 1.0, false);
        if events.contains(&ChaseEvent::SabotageEngine { level: SABOTAGE_LEVEL }) {
            sabotaged_at = Some(frame);
        }
    }
    assert_eq!(sabotaged_at, Some(15));
}

#[test]
fn the_kill_trigger_sabotages_the_engine_at_once() {
    let mut run = run_with(KILL);
    assert_eq!(run.update(inside(), 0.1), vec![ChaseEvent::SabotageEngine { level: SABOTAGE_LEVEL }]);
    assert!(run.update(outside(), 0.1).is_empty());
    assert!(run.update(inside(), 0.1).is_empty(), "one sabotage per run");
}

#[test]
fn the_cop_trigger_turns_the_cops_on_and_spawns_two_mid_size_cars() {
    let mut run = run_with(COPS);
    assert_eq!(
        run.update(inside(), 0.1),
        vec![ChaseEvent::CopsOn, ChaseEvent::SpawnCops { count: COP_SPAWN_COUNT, kind: CopKind::MidSize }]
    );
}

#[test]
fn a_started_call_ends_the_race_11_seconds_later_with_the_fade_at_8() {
    let mut run = run_with(CALL);
    assert_eq!(run.update(inside(), 0.0), vec![ChaseEvent::PlaceCall { id: CHASE_CALL_ID }]);
    let mut fade_at = None;
    let mut post_at = None;
    for frame in 1..=40 {
        for event in run.update(outside(), 1.0) {
            match event {
                ChaseEvent::FadeOn => fade_at = Some(frame),
                ChaseEvent::PostRace => post_at = Some(frame),
                _ => {}
            }
        }
    }
    let load = CALL_LOAD_SECS as i32;
    assert_eq!(fade_at, Some(load + 8), "8 s after the call starts, counting the frame it starts in as 0 s");
    assert_eq!(post_at, Some(load + 11), "3 s after the fade");
    assert!(run.is_finished());
    assert!(run.update(outside(), 1.0).is_empty(), "nothing is emitted after the race ends");
}

/// A plane across the x axis at the origin, facing along +z (heading 0), 20 m wide.
fn plane() -> PlaneTrigger {
    PlaneTrigger {
        key: 3,
        name: format!("{RACE}/call_from_mia"),
        position: Vec3::ZERO,
        heading_deg: 0.0,
        width: 20.0,
        resets_player: false,
    }
}

#[test]
fn the_prologue_triggers_are_planes_and_are_found_by_name() {
    let mut data = GameplayData::default();
    data.planes.push(plane());
    assert!(matches!(zone_named(&data, RACE, "call_from_mia"), Some(Zone::Plane(_))));
}

#[test]
fn a_fast_car_crossing_a_plane_in_one_step_is_caught() {
    let zone = Zone::Plane(plane());
    let before = Vec3::new(0.0, 0.0, -20.0);
    let after = Vec3::new(0.0, 0.0, 20.0);
    assert!(!zone.contains(before) && !zone.contains(after), "neither end is inside the 1 m slab");
    assert!(zone.touched(before, after), "the step passes through it");
}

#[test]
fn a_step_beside_or_short_of_the_plane_misses_it() {
    let zone = Zone::Plane(plane());
    assert!(!zone.touched(Vec3::new(30.0, 0.0, -20.0), Vec3::new(30.0, 0.0, 20.0)), "beside the 20 m width");
    assert!(!zone.touched(Vec3::new(0.0, 0.0, -20.0), Vec3::new(0.0, 0.0, -5.0)), "short of it");
    assert!(!zone.touched(Vec3::new(0.0, 80.0, -20.0), Vec3::new(0.0, 80.0, 20.0)), "far above it");
}

#[test]
fn the_opening_race_has_only_the_call_and_its_engine_gives_out_after_the_sabotage() {
    let mut zones: [Option<Zone>; 4] = [None, None, None, None];
    zones[CALL] = Some(Zone::Plane(plane()));
    let mut run = ChaseRun::from_zones(zones);
    run.set_speed(30.0);
    run.update(Vec3::new(0.0, 0.0, -10.0), 0.1);
    let mut sabotaged = None;
    let mut post = None;
    for frame in 1..=400 {
        let point = Vec3::new(0.0, 0.0, 10.0 + frame as f32);
        // The car slows once it is sabotaged, as a car with no throttle does.
        run.set_speed(if sabotaged.is_some() { 1.0 } else { 30.0 });
        for event in run.update(point, 0.1) {
            match event {
                ChaseEvent::SabotageEngine { .. } => sabotaged = Some(frame),
                ChaseEvent::PostRace => post = Some(frame),
                _ => {}
            }
        }
    }
    let (sabotaged, post) = (sabotaged.expect("the engine is sabotaged"), post.expect("the race ends"));
    assert!(post > sabotaged, "the car slows before the screen goes black");
    assert!(run.is_finished());
}

#[test]
fn a_sabotaged_car_that_still_runs_fast_blows_after_the_timeout() {
    let mut run = ChaseRun::from_zones([None, Some(Zone::Plane(plane())), None, None]);
    run.set_speed(40.0);
    assert!(!run.engine_blown(), "not before the sabotage");
    run.step(Vec3::new(0.0, 0.0, -1.0), 0.0, false);
    let events = run.step(Vec3::new(0.0, 0.0, 1.0), 0.0, false);
    assert!(events.contains(&ChaseEvent::SabotageEngine { level: SABOTAGE_LEVEL }));
    run.step(Vec3::new(0.0, 0.0, 2.0), ENGINE_BLOWN_AFTER_SECS - 0.1, false);
    assert!(!run.engine_blown());
    run.step(Vec3::new(0.0, 0.0, 3.0), 0.2, false);
    assert!(run.engine_blown());
}
