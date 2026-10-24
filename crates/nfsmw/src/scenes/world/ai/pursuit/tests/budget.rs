//! The cops-to-evade budget, the backup that refills it and the surplus cops it sends away
//! (`docs/specs/ai-pursuit-cops.md` section 4).

use super::free_roam::{patrol, renderer, see};
use super::*;

/// A chase at heat 1 against `cops` patrols standing within the engagement radius, in a wave of four with five to evade.
/// `shape` changes the heat row before the chase starts.
fn chase_shaped(cops: usize, shape: impl FnOnce(&mut HeatRow)) -> TrafficWorld {
    let mut world = TrafficWorld::new(cop_road());
    for i in 0..cops {
        patrol(&mut world, &mut renderer(), Vec3::new(2.0, 0.0, 10.0 + 10.0 * i as f32));
    }
    let mut row = wave();
    row.cops[0].count = 4;
    row.full_engagement_cops = 5;
    shape(&mut row);
    world.player_position = Some(Vec3::ZERO);
    world.start_pursuit(1, vec![row], player());
    world
}

fn chase_with(cops: usize) -> TrafficWorld {
    chase_shaped(cops, |_| {})
}

fn evade(world: &mut TrafficWorld, cops: u32) {
    world.pursuit.as_mut().unwrap().budget.note_evaded(cops);
}

#[test]
fn the_wave_may_ask_for_no_more_cops_than_remain_to_evade() {
    let mut world = chase_with(0);
    assert_eq!(world.pursuit.as_ref().unwrap().wanted(world.police.settings(), false), 4, "the whole wave of four");
    evade(&mut world, 3);
    let pursuit = world.pursuit.as_ref().unwrap();
    assert_eq!(pursuit.wanted(world.police.settings(), false), 2, "five to evade, three gone");
    assert_eq!(pursuit.adjusted_wave(world.police.settings(), false), vec![("copx", 2, 100)]);
    evade(&mut world, 9);
    let pursuit = world.pursuit.as_ref().unwrap();
    assert_eq!(
        pursuit.wanted(world.police.settings(), false),
        0,
        "an empty budget asks for nothing until it is raised"
    );
}

#[test]
fn a_cop_that_was_engaged_and_leaves_the_chase_counts_as_evaded() {
    let mut world = chase_with(4);
    assert!(world.cars.iter().all(AiCar::is_cop));
    world.update_budget(0.1, focus());
    assert_eq!(world.cops_to_evade(), 5, "everyone is engaged, nobody has left");
    world.cars.pop();
    world.update_budget(0.1, focus());
    assert_eq!(world.cops_to_evade(), 4, "one engaged cop left");
    world.update_budget(0.1, focus());
    assert_eq!(world.cops_to_evade(), 4, "and is counted once");
}

#[test]
fn a_cop_that_never_came_close_is_not_counted_when_it_goes() {
    let mut world = chase_with(1);
    let mut far = focus();
    far.body.as_mut().unwrap().position = Vec3::new(0.0, 0.0, -400.0);
    world.update_budget(0.1, far);
    world.cars.clear();
    world.update_budget(0.1, far);
    assert_eq!(world.cops_to_evade(), 5);
}

#[test]
fn the_cops_over_what_the_chase_wants_are_sent_away_two_at_a_time() {
    let mut world = chase_with(4);
    evade(&mut world, 3);
    world.flee_surplus(focus(), false);
    let fleeing = |world: &TrafficWorld| world.cars.iter().filter(|car| car.is_fleeing()).count();
    assert_eq!(fleeing(&world), 2, "four cops, two wanted: two leave");
    assert!(world.cars.iter().filter(|car| car.is_fleeing()).all(|car| !car.emergency_lights() && car.is_searching()));
    assert!(world.cars.iter().filter(|car| !car.is_fleeing()).all(AiCar::emergency_lights));
    world.flee_surplus(focus(), false);
    assert_eq!(fleeing(&world), 2, "the surplus is already on its way");
}

#[test]
fn no_cop_is_sent_away_while_the_chase_wants_them_all() {
    let mut world = chase_with(4);
    world.flee_surplus(focus(), false);
    assert!(world.cars.iter().all(|car| !car.is_fleeing()));
}

#[test]
fn a_fleeing_cop_stays_out_of_the_chase_when_the_sight_changes() {
    let mut world = chase_with(4);
    evade(&mut world, 4);
    world.flee_surplus(focus(), false);
    let fleeing = world.cars.iter().position(AiCar::is_fleeing).expect("one leaves");
    let cruise = world.search_cruise();
    world.cars[fleeing].search_mode(false, cruise, &world.network, &world.index);
    assert!(world.cars[fleeing].is_searching() && world.cars[fleeing].is_fleeing(), "the chase cannot call it back");
}

#[test]
fn the_cool_down_keeps_only_the_patrol_count_and_the_rest_leave() {
    let mut world = chase_shaped(4, |row| row.patrol_cars = 1);
    let pursuit = world.pursuit.as_ref().unwrap();
    assert_eq!(pursuit.wanted(world.police.settings(), true), 1);
    world.flee_surplus(focus(), true);
    assert_eq!(world.cars.iter().filter(|car| car.is_fleeing()).count(), 2, "two of the three over leave this run");
}

#[test]
fn the_backup_is_called_when_few_remain_and_arrives_after_its_countdown() {
    let mut world = chase_with(4);
    world.update_budget(0.1, focus());
    assert!(world.backup_eta().is_none());
    evade(&mut world, 3);
    world.update_budget(0.1, focus());
    let eta = world.backup_eta().expect("two remain, the trigger is two");
    assert!((eta - 120.0).abs() < 1.0, "the default call time: {eta}");
    for _ in 0..13 {
        world.update_budget(10.0, focus());
    }
    assert!(world.backup_eta().is_none(), "it arrived");
    assert_eq!(world.cops_to_evade(), 5, "a full wave again");
}

#[test]
fn a_cool_down_drops_the_backup_call_and_the_chase_calls_it_again_afterwards() {
    let mut world = chase_with(2);
    evade(&mut world, 3);
    world.update_budget(0.1, focus());
    assert!(world.backup_eta().is_some());
    see(&mut world, true);
    for _ in 0..40 {
        see(&mut world, false);
    }
    assert_eq!(world.cooldown_status(), blackbox_pursuit::CooldownStatus::CoolDown);
    world.update_budget(0.1, focus());
    assert_eq!(world.backup_eta(), None, "the status the call set is overwritten by the cool-down");
    for _ in 0..80 {
        see(&mut world, true);
    }
    assert_eq!(world.cooldown_status(), blackbox_pursuit::CooldownStatus::InitialChase);
    world.update_budget(0.1, focus());
    assert!(world.backup_eta().is_some(), "back in the chase with few cops left, the backup is called again");
}
