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
    world.start_pursuit(1, vec![wave()], player());
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
}
