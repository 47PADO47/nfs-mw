//! Contacts, the plane's edge and the events a runner collects.

mod common;

use blackbox_vehicle::InputState;
use common::*;
use nfsmw_sim::runner::{PerPlayer, Runner, WanderScript};
use nfsmw_sim::toy::{CAR_RADIUS, ToyConfig, ToyWorld};
use nfsmw_sim::SimEvent;

fn contacts(runner: &Runner<ToyWorld>) -> Vec<(u32, u16, u16, f32)> {
    log_of(runner, |event| match event {
        SimEvent::Contact { a, b, speed } => Some((*a, *b, *speed)),
        _ => None,
    })
    .into_iter()
    .map(|(tick, (a, b, speed))| (tick, a, b, speed))
    .collect()
}

#[test]
fn a_car_driven_into_a_parked_one_makes_a_contact_event() {
    let mut world = ToyWorld::new(1, ToyConfig::bare());
    add_at(&mut world, pid(0), 0.0, -30.0);
    add_at(&mut world, pid(1), 0.0, 0.0);
    let mut runner = Runner::new(world);
    runner.run(240, &mut hold(pid(0), full_throttle()));
    let hits = contacts(&runner);
    assert!(!hits.is_empty());
    let (_, a, b, speed) = hits[0];
    assert_eq!((a, b), (0, 1), "the lower id is named first");
    assert!(speed > 5.0, "a real impact, got {speed}");
}

#[test]
fn touching_cars_are_pushed_apart() {
    let mut world = ToyWorld::new(1, ToyConfig::bare());
    add_at(&mut world, pid(0), 0.0, 0.0);
    add_at(&mut world, pid(1), 1.0, 0.0);
    let mut runner = Runner::new(world);
    runner.run(1, &mut idle());
    let a = runner.world().player_car(pid(0)).unwrap().pos;
    let b = runner.world().player_car(pid(1)).unwrap().pos;
    let gap = ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)).sqrt();
    assert!(gap >= 2.0 * CAR_RADIUS - 1.0e-3, "gap {gap}");
    assert!(contacts(&runner).is_empty(), "a nudge at rest is not an event");
    assert!(a[0] < 0.0 && b[0] > 1.0, "pushed along the line between the centres");
}

#[test]
fn cars_at_the_same_spot_separate_without_nan() {
    let mut world = ToyWorld::new(1, ToyConfig::bare());
    add_at(&mut world, pid(0), 10.0, 10.0);
    add_at(&mut world, pid(1), 10.0, 10.0);
    let mut runner = Runner::new(world);
    runner.run(10, &mut idle());
    for view in runner.entities() {
        assert!(view.position.iter().all(|v| v.is_finite()));
    }
    let a = runner.world().player_car(pid(0)).unwrap().pos;
    let b = runner.world().player_car(pid(1)).unwrap().pos;
    assert_ne!(a, b);
}

#[test]
fn a_head_on_hit_reports_the_closing_speed() {
    let mut world = ToyWorld::new(1, ToyConfig::bare());
    add_at(&mut world, pid(0), 0.0, -40.0);
    add_at(&mut world, pid(1), 0.0, 40.0);
    world.teleport(pid(1), [0.0, 40.0], [0.0, -1.0]);
    let mut runner = Runner::new(world);
    let mut script = PerPlayer::new([pid(0), pid(1)], |_, _| full_throttle());
    runner.run(300, &mut script);
    let hits = contacts(&runner);
    assert!(!hits.is_empty());
    let (_, _, _, speed) = hits[0];
    assert!(speed > 15.0, "two cars closing, got {speed}");
}

#[test]
fn every_contact_names_two_distinct_ordered_ids() {
    let config = ToyConfig { traffic: 30, police: 6, bounds: 150.0, ..ToyConfig::default() };
    let mut runner = Runner::new(world_with(21, config, 16));
    let players: Vec<_> = (0..16).map(pid).collect();
    runner.run(60 * 60, &mut WanderScript::new(5, players));
    let hits = contacts(&runner);
    assert!(hits.len() > 20, "a crowded small plane must see many contacts, got {}", hits.len());
    for (_, a, b, speed) in hits {
        assert!(a < b);
        assert!(speed.is_finite() && speed > 1.0);
    }
}

#[test]
fn cars_never_leave_the_plane() {
    let config = ToyConfig { traffic: 10, police: 4, bounds: 100.0, ..ToyConfig::default() };
    let mut runner = Runner::new(world_with(8, config, 16));
    let players: Vec<_> = (0..16).map(pid).collect();
    let mut script = WanderScript::new(11, players);
    for _ in 0..(60 * 120) {
        runner.step(&mut script);
        for view in runner.entities() {
            assert!(view.position[0].abs() <= 100.0 && view.position[2].abs() <= 100.0, "{view:?}");
        }
    }
}

#[test]
fn the_edge_stops_a_car() {
    let mut runner = Runner::new(world_with(1, ToyConfig { bounds: 100.0, ..ToyConfig::bare() }, 1));
    runner.run(60 * 20, &mut hold(pid(0), full_throttle()));
    let car = runner.world().player_car(pid(0)).unwrap();
    assert_eq!(car.pos[1], 100.0);
    assert!(car.speed < 30.0, "the wall bleeds speed");
}

#[test]
fn brake_lights_and_handbrake_show_in_the_flags() {
    let mut runner = Runner::new(bare_world(1));
    runner.run(5, &mut hold(pid(0), InputState { brake: 1.0, handbrake: 1.0, ..InputState::default() }));
    let flags = runner.entities()[0].flags;
    assert_eq!(flags & 0b101, 0b101);
}

#[test]
fn traffic_drives_and_police_close_in() {
    let config = ToyConfig { traffic: 6, police: 2, bounds: 300.0, ..ToyConfig::default() };
    let mut runner = Runner::new(world_with(17, config, 1));
    let start: Vec<_> = runner.entities();
    runner.run(60 * 60, &mut idle());
    let end = runner.entities();
    let moved = start.iter().zip(&end).filter(|(a, b)| a.position != b.position).count();
    assert!(moved >= 7, "traffic and police move without anyone driving, {moved} moved");
    let player = end[0].position;
    for cop in end.iter().filter(|v| v.kind == nfsmw_sim::EntityKind::PoliceCar) {
        let d = ((cop.position[0] - player[0]).powi(2) + (cop.position[2] - player[2]).powi(2)).sqrt();
        assert!(d < 120.0, "a police car a minute in must have reached the player, still {d} m away");
    }
}
