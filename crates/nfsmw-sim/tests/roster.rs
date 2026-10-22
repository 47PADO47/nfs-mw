//! Joining, leaving, entity ids and the entity list.

mod common;

use common::*;
use nfsmw_sim::toy::{FIRST_BOT_ID, MAX_PLAYERS, ToyConfig, ToyWorld};
use nfsmw_sim::{EntityKind, PlayerId, PlayerJoin, SimError, SimInputs, WorldSim};

#[test]
fn players_take_the_lowest_free_entity_id() {
    let mut world = bare_world(3);
    assert_eq!(world.entity_of(pid(0)), Some(0));
    assert_eq!(world.entity_of(pid(1)), Some(1));
    assert_eq!(world.entity_of(pid(2)), Some(2));
    world.remove_player(pid(1));
    world.add_player(pid(9), &join()).unwrap();
    assert_eq!(world.entity_of(pid(9)), Some(1), "the freed id is reused");
    world.add_player(pid(4), &join()).unwrap();
    assert_eq!(world.entity_of(pid(4)), Some(3));
}

#[test]
fn leave_and_rejoin_reuses_the_id_and_starts_fresh() {
    let mut world = bare_world(2);
    let mut inputs = SimInputs::default();
    inputs.set(pid(0), full_throttle());
    let mut events = Vec::new();
    for _ in 0..120 {
        world.step(&inputs, &mut events);
    }
    assert!(world.player_car(pid(0)).unwrap().speed > 5.0);
    world.remove_player(pid(0));
    assert!(world.own_state(pid(0)).is_none());
    assert_eq!(world.player_count(), 1);
    world.add_player(pid(0), &join()).unwrap();
    assert_eq!(world.entity_of(pid(0)), Some(0));
    assert_eq!(world.player_car(pid(0)).unwrap().speed, 0.0, "a rejoined car starts at rest");
}

#[test]
fn sixteen_players_fit_and_the_seventeenth_does_not() {
    let mut world = bare_world(16);
    assert_eq!(world.player_count(), MAX_PLAYERS);
    assert_eq!(world.add_player(pid(16), &join()), Err(SimError::Full));
    world.remove_player(pid(7));
    assert_eq!(world.add_player(pid(16), &join()), Ok(()));
    assert_eq!(world.entity_of(pid(16)), Some(7));
    assert_eq!(world.add_player(pid(17), &join()), Err(SimError::Full));
}

#[test]
fn joining_twice_and_unknown_cars_are_refused() {
    let mut world = bare_world(1);
    assert_eq!(world.add_player(pid(0), &join()), Err(SimError::AlreadyJoined(pid(0))));
    let bad = PlayerJoin { car_type: 64, ..PlayerJoin::default() };
    assert_eq!(world.add_player(pid(1), &bad), Err(SimError::UnknownCar(64)));
    assert_eq!(world.player_count(), 1, "a refused join leaves nothing behind");
    let good = PlayerJoin { car_type: 63, ..PlayerJoin::default() };
    assert_eq!(world.add_player(pid(1), &good), Ok(()));
}

#[test]
fn removing_an_absent_player_is_harmless() {
    let mut world = bare_world(2);
    let before = world.state_hash();
    world.remove_player(pid(9));
    assert_eq!(world.state_hash(), before);
}

#[test]
fn player_ids_are_not_entity_ids() {
    let mut world = bare_world(0);
    world.add_player(PlayerId(200), &join()).unwrap();
    world.add_player(PlayerId(3), &join()).unwrap();
    let views = entities(&world);
    assert_eq!(views[0].owner, Some(PlayerId(200)));
    assert_eq!(views[0].id, 0);
    assert_eq!(views[1].owner, Some(PlayerId(3)));
    assert_eq!(world.player_ids(), vec![PlayerId(200), PlayerId(3)], "listed in entity-id order");
}

#[test]
fn entities_are_ascending_unique_and_complete() {
    let config = ToyConfig { traffic: 12, police: 3, ..ToyConfig::default() };
    let mut world = world_with(5, config, 5);
    world.remove_player(pid(2));
    let views = entities(&world);
    assert_eq!(views.len(), 4 + 12 + 3);
    assert!(views.windows(2).all(|pair| pair[0].id < pair[1].id));
    let kinds: Vec<_> = views.iter().map(|v| v.kind).collect();
    assert_eq!(kinds.iter().filter(|k| **k == EntityKind::PlayerCar).count(), 4);
    assert_eq!(kinds.iter().filter(|k| **k == EntityKind::TrafficCar).count(), 12);
    assert_eq!(kinds.iter().filter(|k| **k == EntityKind::PoliceCar).count(), 3);
    assert_eq!(views[0].id, 0);
    assert_eq!(views[4].id, FIRST_BOT_ID);
    assert!(views.iter().filter(|v| v.kind == EntityKind::PlayerCar).all(|v| v.owner.is_some()));
    assert!(views.iter().filter(|v| v.kind != EntityKind::PlayerCar).all(|v| v.owner.is_none()));
}

#[test]
fn entities_stay_ascending_while_the_world_runs() {
    let mut world = world_with(5, ToyConfig::default(), 16);
    let mut events = Vec::new();
    for round in 0..600 {
        world.step(&SimInputs::default(), &mut events);
        if round == 200 {
            world.remove_player(pid(3));
            world.remove_player(pid(11));
        }
        if round == 400 {
            world.add_player(pid(30), &join()).unwrap();
        }
        let views = entities(&world);
        assert!(views.windows(2).all(|pair| pair[0].id < pair[1].id));
    }
}

#[test]
fn entity_views_are_physically_sensible() {
    let mut world = world_with(5, ToyConfig::default(), 2);
    let mut inputs = SimInputs::default();
    inputs.set(pid(0), full_throttle());
    let mut events = Vec::new();
    for _ in 0..90 {
        world.step(&inputs, &mut events);
    }
    for view in entities(&world) {
        let q = view.orientation;
        let length = (q[0] * q[0] + q[1] * q[1] + q[2] * q[2] + q[3] * q[3]).sqrt();
        assert!((length - 1.0).abs() < 1.0e-3, "unit quaternion, got {length}");
        assert_eq!(view.position[1], 0.0, "cars stay on the plane");
        assert!(view.position.iter().all(|v| v.is_finite()));
        assert!(view.linear_velocity.iter().all(|v| v.is_finite()));
    }
    let own = entities(&world).into_iter().find(|v| v.owner == Some(pid(0))).unwrap();
    assert!(own.linear_velocity[2] > 3.0, "car 0 drives towards +z");
    assert_eq!(own.flags & 1, 0, "no brake lights");
}

#[test]
fn own_state_reports_the_car() {
    let mut world = bare_world(1);
    let idle = world.own_state(pid(0)).unwrap();
    assert_eq!((idle.gear, idle.nitrous, idle.boost), (1, 1.0, 0.0));
    let mut inputs = SimInputs::default();
    inputs.set(pid(0), nfsmw_sim::InputState { throttle: 1.0, nos: true, ..Default::default() });
    let mut events = Vec::new();
    for _ in 0..180 {
        world.step(&inputs, &mut events);
    }
    let driving = world.own_state(pid(0)).unwrap();
    assert!(driving.gear >= 2);
    assert!(driving.rpm > idle.rpm);
    assert!(driving.nitrous < 1.0);
    assert_eq!(driving.flags & 2, 2, "nitrous burning");
    assert!(world.own_state(pid(1)).is_none());
}

#[test]
fn a_missing_input_means_neutral() {
    let mut world = bare_world(2);
    let mut inputs = SimInputs::default();
    inputs.set(pid(0), full_throttle());
    let mut events = Vec::new();
    for _ in 0..60 {
        world.step(&inputs, &mut events);
    }
    assert!(world.player_car(pid(0)).unwrap().speed > 1.0);
    assert_eq!(world.player_car(pid(1)).unwrap().speed, 0.0);
}

#[test]
fn spawn_requests_are_honoured_and_made_safe() {
    let mut world = ToyWorld::new(1, ToyConfig { bounds: 100.0, ..ToyConfig::bare() });
    add_at(&mut world, pid(0), 25.0, -30.0);
    add_at(&mut world, pid(1), f32::NAN, 1.0e30);
    add_at(&mut world, pid(2), f32::INFINITY, f32::NEG_INFINITY);
    assert_eq!(world.player_car(pid(0)).unwrap().pos, [25.0, -30.0]);
    for n in 1..3 {
        let pos = world.player_car(pid(n)).unwrap().pos;
        assert!(pos.iter().all(|v| v.is_finite() && v.abs() <= 100.0), "{pos:?}");
    }
}

#[test]
fn the_grid_gives_sixteen_players_distinct_spots() {
    let world = bare_world(16);
    let views = entities(&world);
    for (i, a) in views.iter().enumerate() {
        for b in &views[i + 1..] {
            let dx = a.position[0] - b.position[0];
            let dz = a.position[2] - b.position[2];
            assert!(dx * dx + dz * dz > 25.0, "{} and {} share a spot", a.id, b.id);
        }
    }
}

#[test]
fn the_config_is_made_safe() {
    let world = ToyWorld::new(
        1,
        ToyConfig {
            traffic: usize::MAX,
            police: usize::MAX,
            bounds: f32::NAN,
            car_types: u32::MAX,
            ..ToyConfig::default()
        },
    );
    assert_eq!(world.traffic().len(), nfsmw_sim::toy::MAX_TRAFFIC);
    assert_eq!(world.police().len(), nfsmw_sim::toy::MAX_POLICE);
    assert_eq!(world.config().bounds, 50.0);
    let views = entities(&world);
    assert!(views.windows(2).all(|pair| pair[0].id < pair[1].id));
    assert!(views.iter().all(|v| v.position[0].abs() <= 50.0 && v.position[2].abs() <= 50.0));
}
