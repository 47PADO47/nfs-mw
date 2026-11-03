//! The police on the minimap (`docs/specs/hud-cop-blips.md` section 2): only during a running pursuit, only the cars
//! that came within the engagement radius, steady (not flashing) once wrecked or sent away, and the helicopter whenever
//! it flies in the pursuit.

use super::free_roam::{patrol, renderer};
use super::*;
use crate::hud::{COP, HELICOPTER, MapBlip};

fn blips(world: &TrafficWorld) -> Vec<MapBlip> {
    let mut out = Vec::new();
    world.map_blips(&mut out);
    out
}

fn cops(world: &TrafficWorld) -> Vec<MapBlip> {
    blips(world).into_iter().filter(|b| b.style == COP).collect()
}

/// Patrols `at` metres up the road, and a chase at heat 1 with them when `chase` is set.
fn world_with(at: &[f32], chase: bool) -> TrafficWorld {
    let mut world = TrafficWorld::new(cop_road());
    for z in at {
        patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, *z));
    }
    world.player_position = Some(Vec3::ZERO);
    if chase {
        world.start_pursuit(1, vec![wave()], player());
    }
    world
}

#[test]
fn no_police_icon_without_a_pursuit() {
    let world = world_with(&[10.0, 20.0], false);
    assert_eq!(world.cars.iter().filter(|c| c.is_police()).count(), 2);
    assert!(cops(&world).is_empty(), "patrols in free roam are not on the minimap");
}

#[test]
fn a_cop_is_shown_only_once_it_was_engaged() {
    let mut world = world_with(&[10.0, 20.0], true);
    assert!(world.cars.iter().all(AiCar::is_cop));
    assert!(cops(&world).is_empty(), "nobody has come within the engagement radius yet");
    let mut far = focus();
    far.body.as_mut().unwrap().position = Vec3::new(0.0, 0.0, -400.0);
    world.update_budget(0.1, far);
    assert!(cops(&world).is_empty(), "the cops are 400 m away");
    world.update_budget(0.1, focus());
    let shown = cops(&world);
    assert_eq!(shown.len(), 2);
    assert!(shown.iter().all(|b| b.active), "engaged cops chasing the player flash");
}

#[test]
fn a_roadblock_car_is_shown_once_its_roadblock_stands_well_beyond_the_chase_radius() {
    // The chase radius is the default 150 m here; the roadblock marks its cars within 300 m.
    let mut world = world_with(&[10.0, 20.0], true);
    world.cars[0].set_roadblock();
    let mut away = focus();
    away.body.as_mut().unwrap().position = Vec3::new(0.0, 0.0, -200.0);
    world.update_budget(0.1, away);
    let shown = cops(&world);
    assert_eq!(shown.len(), 1, "only the roadblock car 210 m away; the chase cop 220 m away is not engaged");
    assert!(world.cars[0].was_engaged() && !world.cars[1].was_engaged());
}

#[test]
fn an_engaged_wreck_stays_on_the_map_steady_until_it_goes() {
    let mut world = world_with(&[10.0], true);
    world.update_budget(0.1, focus());
    assert!(world.cars[0].destroy_by_breaker());
    let shown = cops(&world);
    assert_eq!(shown.len(), 1, "the wreck is still in the world");
    assert!(!shown[0].active, "a wreck does not flash");
    world.cars.clear();
    assert!(cops(&world).is_empty());
}

#[test]
fn a_cop_sent_away_leaves_the_map() {
    let mut world = world_with(&[10.0, 20.0, 30.0, 40.0], true);
    world.update_budget(0.1, focus());
    world.pursuit.as_mut().unwrap().budget.note_evaded(3);
    world.flee_surplus(focus(), false);
    let fleeing = world.cars.iter().filter(|c| c.is_fleeing()).count();
    assert!(fleeing > 0);
    let shown = cops(&world);
    assert_eq!(shown.len(), 4 - fleeing, "the cops that left are not drawn");
    assert!(shown.iter().all(|b| b.active), "the ones still chasing flash");
}

#[test]
fn every_police_icon_goes_when_the_pursuit_ends() {
    let mut world = world_with(&[10.0], true);
    world.update_budget(0.1, focus());
    assert_eq!(cops(&world).len(), 1);
    world.end_pursuit(blackbox_pursuit::EndReason::Evaded);
    assert!(blips(&world).iter().all(|b| b.style != COP && b.style != HELICOPTER));
}
