//! Visibility follows each actual vehicle task, with its original fractional timer and sight grace.

use std::cell::RefCell;

use super::*;
use crate::scenes::world::drive::STEP;
use crate::scenes::world::drive::walls::Hit;
use crate::scenes::world::props::PropWorld;

fn world(phases: &[u32]) -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(180.0));
    let mut renderer = free_roam::renderer();
    for (index, &phase) in phases.iter().enumerate() {
        free_roam::patrol_phased(&mut world, &mut renderer, Vec3::new(2.0, 0.0, -10.0 - index as f32 * 20.0), phase);
    }
    world.player_position = Some(Vec3::ZERO);
    start_with_patrols(&mut world, 1, vec![wave()], player());
    world
}

#[test]
fn pursuit_ray_checks_follow_the_actual_car_staggers_instead_of_a_global_quarter_second() {
    let mut world = world(&[0, 4]);
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    let rays = RefCell::new(Vec::new());
    for tick in 1..=8 {
        world.step(focus(), &collision, &mut props, &surfaces);
        world.update_sight(STEP, &|from, _| {
            rays.borrow_mut().push((tick, from.z));
            None
        });
    }
    let rays = rays.into_inner();
    assert_eq!(rays.len(), 2, "one check for each staggered task: {rays:?}");
    assert_eq!(rays.iter().map(|&(tick, _)| tick).collect::<Vec<_>>(), [1, 5]);
    assert!(world.cars.iter().all(AiCar::sees_player));
}

#[test]
fn a_failed_query_keeps_recent_unit_sight_until_the_next_task_ages_it_past_zero() {
    let mut world = world(&[0]);
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    for tick in 1..=17 {
        world.step(focus(), &collision, &mut props, &surfaces);
        world.update_sight(STEP, &|from, to| {
            if tick == 1 {
                return None;
            }
            Some(Hit { point: from.lerp(to, 0.5), normal: Vec3::Y, prop: None, info: None })
        });
        if tick == 9 {
            assert!(world.cars[0].sees_player(), "the failed ray retains the negative last-seen age");
        }
    }
    assert!(!world.cars[0].sees_player());
    assert!(!world.sighting(1).seen);
    assert!(world.cars[0].sight_age() > 0.0);
}

#[test]
fn repeating_the_sensing_pass_does_not_run_an_unexecuted_vehicle_task() {
    let mut world = world(&[0]);
    let collision = floor();
    let surfaces = SurfaceTable::default();
    world.step(focus(), &collision, &mut PropWorld::default(), &surfaces);
    world.update_sight(STEP, &|_, _| None);
    let age = world.cars[0].sight_age();
    for _ in 0..30 {
        world.update_sight(STEP, &|_, _| panic!("no car task was executed"));
    }
    assert_eq!(world.cars[0].sight_age(), age);
}

#[test]
fn a_suspended_cop_cannot_hold_a_frozen_negative_sight_age_over_the_pursuit() {
    let mut world = world(&[0]);
    let collision = floor();
    let surfaces = SurfaceTable::default();
    world.step(focus(), &collision, &mut PropWorld::default(), &surfaces);
    world.update_sight(STEP, &|_, _| None);
    assert!(world.sighting(1).seen);
    assert!(world.cars[0].sight_age() < 0.0);
    world.cars[0].set_sight_active(false);
    world.update_sight(STEP, &|_, _| panic!("a suspended car must not cast"));
    assert!(!world.sighting(1).seen);
    assert!(world.police.observed_ground_age().unwrap().is_infinite());
    world.cars[0].set_sight_active(true);
    assert!(!world.cars[0].sees_player(), "reactivation requires a fresh observation");
    world.update_sight(STEP, &|_, _| panic!("no car task was executed"));
    let grace = blackbox_pursuit::cooldown::TUNING.grace;
    assert!(world.police.observed_ground_age().unwrap() > grace, "a resumed cop brings no sighting into the pursuit");
}

#[test]
fn a_cop_sent_away_still_looks_and_its_sighting_counts_for_the_chase() {
    let mut world = world(&[0]);
    let cruise = world.search_cruise();
    assert!(world.cars[0].start_flee(cruise, &world.network, &world.index));
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    for _ in 0..8 {
        world.step(focus(), &collision, &mut props, &surfaces);
        world.update_sight(STEP, &|_, _| None);
    }
    assert!(world.cars[0].is_fleeing());
    assert!(world.cars[0].sees_player(), "a fleeing cop keeps testing its sight");
    assert!(world.sighting(1).seen, "and its sighting is the chase's");
}

#[test]
fn a_cop_sent_away_turns_round_where_its_lane_ends() {
    let mut world = world(&[0]);
    let cruise = world.search_cruise();
    assert!(world.cars[0].start_flee(cruise, &world.network, &world.index));
    // The player stands off the road, so nothing blocks the lane.
    let aside = Vec3::new(60.0, 0.0, 0.0);
    world.player_position = Some(aside);
    let mut view = focus();
    view.position = [aside.x, aside.z];
    if let Some(body) = view.body.as_mut() {
        body.position = aside;
    }
    let collision = floor();
    let surfaces = SurfaceTable::default();
    let mut props = PropWorld::default();
    let mut farthest: f32 = 0.0;
    for _ in 0..(60.0 / STEP) as u32 {
        world.step(view, &collision, &mut props, &surfaces);
        farthest = farthest.max(world.cars[0].physics_position().z.abs());
    }
    let end = world.cars[0].physics_position().z.abs();
    assert!(farthest > 150.0, "it drove to the end of the 180 m lane: {farthest}");
    assert!(end < farthest - 20.0, "and came back instead of standing there: {end} of {farthest}");
}
