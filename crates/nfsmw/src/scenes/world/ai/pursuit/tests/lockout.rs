//! The cop lockout: no patrols while it runs, an evade starts it and a chase clears it
//! (`docs/specs/ai-pursuit-cops.md` section 1).

use super::free_roam::{patrol, renderer, see};
use super::*;
use crate::scenes::world::ai::PatrolPlan;

fn patrol_world(patrols: u32) -> TrafficWorld {
    let mut world = TrafficWorld::new(straight_road(1000.0));
    world.add_cop_model(cop_model(&mut renderer(), "copx", player()));
    let mut row = wave();
    row.patrol_cars = patrols;
    let mut plan = PatrolPlan::new(vec!["copx".into()], 50.0, 71.0);
    plan.levels = vec![row];
    world.set_patrol(plan);
    world.configure(0, 5);
    world
}

/// A player standing 200 m down the road, so the production spawn band lies on the synthetic floor.
fn down_the_road() -> Focus {
    Focus {
        position: [-200.0, 0.0],
        body: Some(Body { position: -Vec3::Z * 200.0, ..focus().body.unwrap() }),
        ..focus()
    }
}

#[test]
fn no_patrol_spawns_while_the_lockout_runs_and_they_come_back_after_it() {
    let mut world = patrol_world(2);
    world.cops.start_session();
    for _ in 0..300 {
        world.manage(0.1, down_the_road(), &floor(), &SurfaceTable::default());
    }
    assert!(world.cars.is_empty(), "a session starts with the cops locked out");
    world.cops.unlock();
    for _ in 0..1000 {
        world.manage(0.1, down_the_road(), &floor(), &SurfaceTable::default());
    }
    assert_eq!(world.cars.iter().filter(|car| car.is_patrol()).count(), 2);
}

#[test]
fn a_lockout_removes_the_patrols_that_are_out() {
    let mut world = TrafficWorld::new(cop_road());
    patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 20.0));
    see(&mut world, false);
    assert_eq!(world.cars.len(), 1, "no lockout, the patrol stays");
    world.cops.lock_out();
    see(&mut world, false);
    assert!(world.cars.is_empty(), "every active cop goes while spawning is disabled");
}

#[test]
fn the_lockout_runs_down_on_the_police_clock() {
    let mut world = TrafficWorld::new(cop_road());
    world.cops.lock_out();
    for _ in 0..(110 * 4) {
        see(&mut world, false);
    }
    assert!(!world.cops.spawning_enabled(), "110 s of 120");
    for _ in 0..(11 * 4) {
        see(&mut world, false);
    }
    assert!(world.cops.spawning_enabled());
}

#[test]
fn an_evade_locks_the_cops_out_and_a_new_chase_clears_it() {
    let mut world = TrafficWorld::new(cop_road());
    world.start_pursuit(1, vec![wave()], player());
    world.end_pursuit(blackbox_pursuit::EndReason::Evaded);
    assert!((world.cop_lockout_left() - 120.0).abs() < 1e-3, "the free-roam time");
    world.start_pursuit(1, vec![wave()], player());
    assert_eq!(world.cop_lockout_left(), 0.0, "a chase that starts clears the lockout");
}

#[test]
fn only_an_evade_starts_the_lockout() {
    for reason in [blackbox_pursuit::EndReason::None, blackbox_pursuit::EndReason::Busted] {
        let mut world = TrafficWorld::new(cop_road());
        world.start_pursuit(1, vec![wave()], player());
        world.end_pursuit(reason);
        assert_eq!(world.cop_lockout_left(), 0.0, "{reason:?}");
    }
}
