//! Patrols join the chase by their own sighting, one a manager run (spec `ai-pursuit-cops.md` section 2).

use super::free_roam::{patrol, renderer};
use super::*;

fn two_patrols() -> TrafficWorld {
    let mut world = TrafficWorld::new(cop_road());
    let mut renderer = renderer();
    patrol(&mut world, &mut renderer, Vec3::new(2.0, 0.0, 20.0));
    patrol(&mut world, &mut renderer, Vec3::new(2.0, 0.0, 40.0));
    world.player_position = Some(Vec3::ZERO);
    world
}

#[test]
fn a_chase_begins_with_the_one_patrol_that_saw_the_player() {
    let mut world = two_patrols();
    world.cars[0].report_sight(false, true);
    world.cars[1].report_sight(false, true);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.cars[0].is_cop(), "the first patrol in car order that saw the player starts the chase");
    assert!(world.cars[1].is_patrol(), "the other one waits for the next run");
    assert_eq!(world.pursuit.as_ref().unwrap().involved, 1);
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    world.update_pursuit(FRAME, focus(), &collision, &surfaces);
    assert!(world.cars[1].is_cop(), "it joins on the next run");
    assert_eq!(world.pursuit.as_ref().unwrap().involved, 2);
}

#[test]
fn a_near_patrol_that_does_not_see_the_player_stays_on_patrol() {
    let mut world = two_patrols();
    world.cars[1].report_sight(false, true);
    world.start_pursuit(1, vec![wave()], player());
    assert!(world.cars[0].is_patrol(), "20 m away but unsighted: not in the chase");
    assert!(world.cars[1].is_cop());
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    world.update_pursuit(FRAME, focus(), &collision, &surfaces);
    assert!(world.cars[0].is_patrol());
    assert_eq!(world.pursuit.as_ref().unwrap().involved, 1);
}

#[test]
fn a_patrol_that_spots_the_player_later_joins_the_running_chase_and_restarts_the_timer() {
    let mut world = two_patrols();
    world.cars[0].report_sight(false, true);
    let row = HeatRow { time_between_first_four_spawns: 10.0, time_between_cop_spawns: 5.0, ..wave() };
    world.start_pursuit(1, vec![row], player());
    let (collision, surfaces) = (floor(), SurfaceTable::default());
    world.update_pursuit(FRAME, focus(), &collision, &surfaces);
    assert!(world.cars[1].is_patrol());
    world.pursuit.as_mut().unwrap().spawn_timer = 0.5;
    world.cars[1].report_sight(false, true);
    world.update_pursuit(FRAME, focus(), &collision, &surfaces);
    assert!(world.cars[1].is_cop());
    let pursuit = world.pursuit.as_ref().unwrap();
    assert_eq!(pursuit.involved, 2);
    assert!(
        (pursuit.spawn_timer - (10.0 - FRAME)).abs() < 1e-4,
        "the join restarts the opening interval: {}",
        pursuit.spawn_timer
    );
}
