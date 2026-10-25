//! Target tracking is shared before driver thinks and follows the published target, including search memory.

use super::*;
use crate::scenes::world::StateHasher;
use crate::scenes::world::ai::target_road::PrimaryRoad;
use crate::scenes::world::props::PropWorld;

fn tracked_hash(tracker: &PrimaryRoad) -> u64 {
    let mut hash = StateHasher::default();
    tracker.hash_into(&mut hash);
    hash.finish()
}

fn update(world: &mut TrafficWorld, body: Option<Body>) -> Option<crate::scenes::world::ai::cop::TargetRoad> {
    world.target_road.update(&world.network, &world.index, body, false, false, 1.0 / 60.0)
}

#[test]
fn primary_identity_and_vehicle_replacement_reset_only_the_affected_tracker() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let body = focus().body.unwrap();
    assert!(update(&mut world, Some(body)).is_some(), "fixtures may use an anonymous primary");
    world.note_primary_target(0);
    let empty = tracked_hash(&world.target_road);
    assert!(update(&mut world, Some(body)).is_some());
    let observed = tracked_hash(&world.target_road);
    assert_ne!(empty, observed);
    world.note_primary_target(0);
    assert_eq!(tracked_hash(&world.target_road), observed, "same seat retains the target walker");
    world.reset_primary_target_road(1);
    assert_eq!(tracked_hash(&world.target_road), observed, "another vehicle replacement must not reset the primary");
    world.reset_primary_target_road(0);
    assert_eq!(tracked_hash(&world.target_road), empty, "primary vehicle replacement resets its road state");
    update(&mut world, Some(body));
    world.note_primary_target(1);
    assert_ne!(tracked_hash(&world.target_road), empty, "primary identity participates in the hash");
}

#[test]
fn copied_observations_do_not_advance_seek_cache_or_random_streams() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let first = focus().body.unwrap();
    let first = Body { velocity: Vec3::Z * 20.0, ..first };
    let rng = world.rng.nav.0;
    let a = update(&mut world, Some(first)).expect("a physical target on a road is trackable");
    let observed = tracked_hash(&world.target_road);
    for _ in 0..8 {
        let copied = a;
        assert_eq!(copied, a);
        assert_eq!(tracked_hash(&world.target_road), observed);
    }
    let moved = Body { position: first.position + Vec3::Z * 10.0, ..first };
    let b = update(&mut world, Some(moved)).unwrap();
    assert_eq!(a.seek, b.seek, "one step retains the shared seek cache");
    let mut last = b;
    for _ in 0..24 {
        last = update(&mut world, Some(moved)).unwrap();
    }
    assert!(last.centre.z > a.centre.z, "current road observation follows the moving target");
    assert!(last.seek.z > a.seek.z, "cache refresh eventually uses the continued current road");
    assert_eq!(world.rng.nav.0, rng, "tracking is independent of cop think order and navigation RNG");
}

#[test]
fn reinforcement_seek_reuses_the_primary_targets_cached_road_prediction() {
    let mut world = TrafficWorld::new(straight_road(2000.0));
    let first = Body { velocity: Vec3::Z * 35.0, forward: Vec2::Y, ..focus().body.unwrap() };
    let cached = update(&mut world, Some(first)).unwrap();
    let moved = Body { position: first.position + Vec3::Z * 12.0, ..first };
    assert_eq!(update(&mut world, Some(moved)).unwrap().seek, cached.seek);
    let before = tracked_hash(&world.target_road);
    assert_eq!(world.reinforcement_seek(&moved), Some(cached.seek), "spawn and driving must use the same cache");
    assert_eq!(tracked_hash(&world.target_road), before, "spawn reads must not advance target history");
}

#[test]
fn reinforcement_seek_keeps_the_shorter_jerk_prediction() {
    let mut world = TrafficWorld::new(straight_road(2000.0));
    let target = Body { velocity: Vec3::Z * 35.0, forward: Vec2::Y, ..focus().body.unwrap() };
    let cached = world.target_road.update(&world.network, &world.index, Some(target), false, true, 1.0 / 60.0).unwrap();
    assert!(cached.seek.distance(target.position) < 150.0, "fixture must exercise the jerk horizon");
    assert_eq!(world.reinforcement_seek(&target), Some(cached.seek), "spawn must not rebuild the ordinary horizon");
}

#[test]
fn missing_target_and_racing_publish_no_free_roam_snapshot_but_age_state() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let body = focus().body.unwrap();
    assert!(update(&mut world, Some(body)).is_some());
    let before = tracked_hash(&world.target_road);
    assert!(update(&mut world, None).is_none());
    assert_ne!(tracked_hash(&world.target_road), before);
    assert!(world.target_road.update(&world.network, &world.index, Some(body), true, false, 0.25).is_none());
}

#[test]
fn road_observation_follows_remembered_search_target_without_reading_hidden_pose() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.start_pursuit(1, vec![wave()], player());
    let first = focus().body.unwrap();
    free_roam::see(&mut world, true);
    let target = world.pursuit_target(Some(first));
    let visible = update(&mut world, target).unwrap();
    for _ in 0..34 {
        free_roam::see(&mut world, false);
    }
    assert!(!world.police.is_perp_in_sight());
    let hidden = Body { position: Vec3::new(200.0, 0.0, 500.0), velocity: Vec3::Z * 80.0, ..first };
    let target = world.pursuit_target(Some(hidden)).unwrap();
    assert_eq!(target.position, first.position);
    assert_eq!(target.velocity, Vec3::ZERO);
    let remembered = update(&mut world, Some(target)).unwrap();
    assert!(remembered.centre.distance(visible.centre) < 1.0);
    assert!(remembered.centre.distance(hidden.position) > 100.0);
}

#[test]
fn the_step_shares_primary_snapshot_and_keeps_secondary_target_fallback_separate() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    let mut renderer = free_roam::renderer();
    for z in [-20.0, 20.0] {
        free_roam::patrol(&mut world, &mut renderer, Vec3::new(2.0, 0.0, z));
        let key = world.cars.len() as u32;
        world.cars.last_mut().unwrap().set_entity(key);
    }
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.cars.iter().all(AiCar::is_cop));
    free_roam::see(&mut world, true);
    let primary = focus();
    let secondary = Body { position: Vec3::new(2.0, 0.0, 30.0), ..primary.body.unwrap() };
    world.set_secondary_targets([(blackbox_pursuit::targets::TargetId(1), secondary)]);
    let assignments = world.secondary_assignments(primary.body);
    assert!(assignments.iter().any(Option::is_some));
    assert!(assignments.iter().any(Option::is_none));
    world.step(primary, &floor(), &mut PropWorld::default(), &SurfaceTable::default());
    for (car, secondary) in world.cars.iter().zip(assignments) {
        let Role::Cop(cop) = &car.role else { panic!("expected a pursuit driver") };
        let (shared, snapshot) = cop.target_road_observation();
        assert_eq!(shared, secondary.is_none());
        assert_eq!(snapshot.is_some(), secondary.is_none());
    }
}

fn production_step(world: &mut TrafficWorld, body: Option<Body>) {
    world.step(Focus { body, ..focus() }, &floor(), &mut PropWorld::default(), &SurfaceTable::default());
}

#[test]
fn free_roam_steps_establish_road_history_that_continues_into_a_chase() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.note_primary_target(0);
    let first = Body { forward: Vec2::Y, velocity: Vec3::Z * 20.0, ..focus().body.unwrap() };
    production_step(&mut world, Some(first));
    let before = world.target_road.observation().expect("normal driving must establish target road history");
    let history = tracked_hash(&world.target_road);
    world.start_pursuit(1, vec![wave()], player());
    assert_eq!(tracked_hash(&world.target_road), history, "pursuit entry must retain this vehicle's history");
    let moved = Body { position: first.position + Vec3::Z * 10.0, ..first };
    production_step(&mut world, Some(moved));
    let chasing = world.target_road.observation().expect("the pursuit continues the same tracker");
    assert_eq!(chasing.seek, before.seek, "pursuit entry must not invalidate the existing seek cache");
    assert!(chasing.centre.z > before.centre.z, "the retained current road must continue toward the moved body");
}

#[test]
fn free_roam_steps_without_a_body_suppress_the_snapshot_and_keep_aging_clocks() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    production_step(&mut world, focus().body);
    assert!(world.target_road.observation().is_some());
    let observed = tracked_hash(&world.target_road);
    production_step(&mut world, None);
    assert!(world.target_road.observation().is_none());
    let absent = tracked_hash(&world.target_road);
    assert_ne!(absent, observed);
    production_step(&mut world, None);
    assert!(world.target_road.observation().is_none());
    assert_ne!(tracked_hash(&world.target_road), absent, "absent observations must still advance tracker clocks");
}

#[test]
fn active_pursuit_without_an_authorized_target_does_not_track_the_hidden_actual_body() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    production_step(&mut world, focus().body);
    world.start_pursuit(1, vec![wave()], player());
    // No unit has sighted this target: age the native -5-second initial timer through its full grace.
    let unseen_steps = ((blackbox_pursuit::cooldown::TUNING.grace - blackbox_pursuit::cooldown::SIGHT_AT_START) / 0.25)
        .ceil() as usize
        + 2;
    for _ in 0..unseen_steps {
        free_roam::see(&mut world, false);
    }
    assert!(!world.police.is_perp_in_sight());
    assert!(world.pursuit.as_ref().unwrap().last_seen.is_none());
    let hidden = Body { position: Vec3::Z * 500.0, velocity: Vec3::Z * 80.0, ..focus().body.unwrap() };
    production_step(&mut world, Some(hidden));
    assert!(world.pursuit.is_some());
    assert!(world.target_road.observation().is_none(), "a running pursuit must not fall back to the actual focus");
}

#[test]
fn production_step_tracks_stationary_search_memory_instead_of_hidden_motion() {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.start_pursuit(1, vec![wave()], player());
    let first = Body { forward: Vec2::Y, ..focus().body.unwrap() };
    free_roam::see(&mut world, true);
    production_step(&mut world, Some(first));
    let seen = world.target_road.observation().unwrap();
    for _ in 0..34 {
        free_roam::see(&mut world, false);
    }
    assert!(!world.police.is_perp_in_sight());
    let hidden = Body { position: Vec3::Z * 500.0, velocity: Vec3::Z * 80.0, ..first };
    production_step(&mut world, Some(hidden));
    let memory = world.target_road.observation().expect("the last observation remains a valid search target");
    assert!(memory.centre.distance(seen.centre) < 1.0);
    assert!(memory.centre.distance(hidden.position) > 100.0);
    assert_eq!(world.pursuit.as_ref().unwrap().last_seen.unwrap().position, first.position);
}
