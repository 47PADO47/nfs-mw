//! Laps, finishing order, moves and pursuits.

mod common;

use common::*;
use nfsmw_sim::runner::Runner;
use nfsmw_sim::toy::{RaceConfig, ToyConfig, ToyWorld};
use nfsmw_sim::{PlayerId, SimEvent, SimInputs, WorldSim};

fn laps(runner: &Runner<ToyWorld>) -> Vec<(PlayerId, u8, u32)> {
    log_of(runner, |event| match event {
        SimEvent::LapCompleted { player, lap, time_ms } => Some((*player, *lap, *time_ms)),
        _ => None,
    })
    .into_iter()
    .map(|(_, v)| v)
    .collect()
}

fn finishes(runner: &Runner<ToyWorld>) -> Vec<(PlayerId, u8, u32)> {
    log_of(runner, |event| match event {
        SimEvent::RaceFinished { player, place, time_ms } => Some((*player, *place, *time_ms)),
        _ => None,
    })
    .into_iter()
    .map(|(_, v)| v)
    .collect()
}

/// One checkpoint that covers the whole starting grid.
fn on_the_grid(laps: u8) -> ToyConfig {
    let race = RaceConfig { laps, checkpoints: vec![[0.0, -20.0]], radius: 100.0 };
    ToyConfig { race: Some(race), ..ToyConfig::bare() }
}

#[test]
fn two_drivers_finish_in_order_of_speed() {
    let config = ToyConfig { race: Some(RaceConfig::circuit(2)), ..ToyConfig::bare() };
    let mut runner = Runner::new(world_with(1, config, 2));
    let mut script = lap_driver(vec![(pid(0), 24.0), (pid(1), 18.0)]);
    for _ in 0..(60 * 400) {
        runner.step(&mut script);
        if runner.world().finished_count() == 2 {
            break;
        }
    }
    let finished = finishes(&runner);
    assert_eq!(finished.len(), 2, "both cars finish; laps: {:?}", laps(&runner));
    assert_eq!((finished[0].0, finished[0].1), (pid(0), 1));
    assert_eq!((finished[1].0, finished[1].1), (pid(1), 2));
    assert!(finished[0].2 < finished[1].2);
    let laps = laps(&runner);
    for player in [pid(0), pid(1)] {
        let own: Vec<_> = laps.iter().filter(|(p, _, _)| *p == player).collect();
        assert_eq!(own.iter().map(|(_, lap, _)| *lap).collect::<Vec<_>>(), vec![1, 2]);
        let total: u32 = own.iter().map(|(_, _, ms)| ms).sum();
        let finish = finished.iter().find(|(p, _, _)| *p == player).unwrap().2;
        assert!(total.abs_diff(finish) <= 2, "lap times sum to the race time: {total} vs {finish}");
    }
}

#[test]
fn a_lap_event_comes_before_the_finish_event() {
    let mut runner = Runner::new(world_with(1, on_the_grid(1), 1));
    runner.run(3, &mut idle());
    let kinds: Vec<_> = runner.events().iter().map(|(_, e)| *e).collect();
    assert_eq!(kinds.len(), 2);
    assert!(matches!(kinds[0], SimEvent::LapCompleted { lap: 1, time_ms: 16, .. }));
    assert!(matches!(kinds[1], SimEvent::RaceFinished { place: 1, time_ms: 16, .. }));
}

#[test]
fn players_finishing_on_the_same_tick_are_placed_by_id() {
    let mut runner = Runner::new(world_with(1, on_the_grid(1), 5));
    runner.run(5, &mut idle());
    let finished = finishes(&runner);
    let order: Vec<_> = finished.iter().map(|(p, place, _)| (p.0, *place)).collect();
    assert_eq!(order, vec![(0, 1), (1, 2), (2, 3), (3, 4), (4, 5)]);
    assert_eq!(runner.world().finished_count(), 5);
}

#[test]
fn one_checkpoint_per_tick_so_laps_count_up() {
    let mut runner = Runner::new(world_with(1, on_the_grid(3), 1));
    runner.run(10, &mut idle());
    let laps = laps(&runner);
    assert_eq!(laps.iter().map(|(_, lap, _)| *lap).collect::<Vec<_>>(), vec![1, 2, 3]);
    assert_eq!(laps.iter().map(|(_, _, ms)| *ms).collect::<Vec<_>>(), vec![16, 16, 16], "one tick each, in ms");
    assert_eq!(finishes(&runner).len(), 1, "finished once, then no more events");
}

#[test]
fn a_late_joiner_runs_its_own_clock() {
    let mut runner = Runner::new(world_with(1, on_the_grid(1), 0));
    runner.run(120, &mut idle());
    runner.world_mut().add_player(pid(0), &join()).unwrap();
    runner.run(2, &mut idle());
    let finished = finishes(&runner);
    assert_eq!(finished.len(), 1);
    assert_eq!(finished[0].2, 16, "timed from joining, not from tick 0");
}

#[test]
fn reset_race_starts_the_order_over() {
    let mut runner = Runner::new(world_with(1, on_the_grid(1), 2));
    runner.run(3, &mut idle());
    assert_eq!(runner.world().finished_count(), 2);
    runner.world_mut().reset_race();
    assert_eq!(runner.world().finished_count(), 0);
    runner.take_events();
    runner.run(3, &mut idle());
    let finished = finishes(&runner);
    assert_eq!(finished.iter().map(|(_, place, _)| *place).collect::<Vec<_>>(), vec![1, 2]);
}

#[test]
fn free_roam_has_no_race_events() {
    let mut runner = Runner::new(bare_world(4));
    runner.run(600, &mut idle());
    assert!(laps(&runner).is_empty() && finishes(&runner).is_empty());
}

#[test]
fn a_race_with_no_checkpoints_is_dropped() {
    let race = RaceConfig { laps: 2, checkpoints: Vec::new(), radius: 10.0 };
    let world = ToyWorld::new(1, ToyConfig { race: Some(race), ..ToyConfig::bare() });
    assert!(world.config().race.is_none());
}

#[test]
fn a_teleport_reports_moved_once() {
    let mut world = bare_world(2);
    assert!(world.teleport(pid(1), [50.0, 50.0], [1.0, 0.0]));
    assert!(world.teleport(pid(1), [60.0, 60.0], [1.0, 0.0]));
    let mut events = Vec::new();
    world.step(&SimInputs::default(), &mut events);
    assert_eq!(events, vec![SimEvent::Moved { player: pid(1) }]);
    events.clear();
    world.step(&SimInputs::default(), &mut events);
    assert!(events.is_empty());
    let car = world.player_car(pid(1)).unwrap();
    assert_eq!(car.pos, [60.0, 60.0]);
    assert_eq!(car.dir, [1.0, 0.0]);
    assert!(!world.teleport(pid(9), [0.0, 0.0], [0.0, 1.0]), "an absent player cannot be moved");
}

#[test]
fn a_reset_puts_the_car_back_on_its_grid_slot() {
    let mut runner = Runner::new(world_with(1, ToyConfig::bare(), 3));
    let grid = runner.world().player_car(pid(2)).unwrap().pos;
    runner.run(120, &mut hold(pid(2), full_throttle()));
    assert_ne!(runner.world().player_car(pid(2)).unwrap().pos, grid);
    assert!(runner.world_mut().reset_player(pid(2)));
    runner.run(1, &mut idle());
    assert_eq!(runner.world().player_car(pid(2)).unwrap().pos, grid);
    assert_eq!(runner.events().last().map(|(_, e)| *e), Some(SimEvent::Moved { player: pid(2) }));
}

#[test]
fn leaving_cancels_a_pending_move() {
    let mut world = bare_world(2);
    world.teleport(pid(1), [5.0, 5.0], [0.0, 1.0]);
    world.remove_player(pid(1));
    let mut events = Vec::new();
    world.step(&SimInputs::default(), &mut events);
    assert!(events.is_empty());
}

fn pursuit_world() -> ToyWorld {
    let config = ToyConfig { traffic: 0, police: 1, bounds: 1000.0, ..ToyConfig::default() };
    world_with(31, config, 1)
}

#[test]
fn police_start_a_pursuit_and_bust_a_parked_player() {
    let mut runner = Runner::new(pursuit_world());
    for _ in 0..(60 * 120) {
        runner.step(&mut idle());
        if runner.events().iter().any(|(_, e)| matches!(e, SimEvent::Busted { .. })) {
            break;
        }
    }
    let kinds: Vec<_> = runner
        .events()
        .iter()
        .filter_map(|(_, e)| match e {
            SimEvent::PursuitStarted { .. } => Some("started"),
            SimEvent::Busted { .. } => Some("busted"),
            SimEvent::PursuitEnded { .. } => Some("ended"),
            SimEvent::Moved { .. } => Some("moved"),
            _ => None,
        })
        .collect();
    assert_eq!(kinds, vec!["started", "busted", "ended", "moved"]);
    assert!(!runner.world().in_pursuit(pid(0)));
    let grid = nfsmw_sim::toy::grid_position(0);
    assert_eq!(runner.world().player_car(pid(0)).unwrap().pos, grid);
}

#[test]
fn a_player_who_gets_far_away_evades() {
    let mut runner = Runner::new(pursuit_world());
    for _ in 0..(60 * 120) {
        runner.step(&mut idle());
        if runner.world().in_pursuit(pid(0)) {
            break;
        }
    }
    assert!(runner.world().in_pursuit(pid(0)), "the chase must start");
    let cop = runner.world().police()[0].car.pos;
    let far = match cop[0] > 0.0 {
        true => [-900.0, -900.0],
        false => [900.0, 900.0],
    };
    runner.world_mut().teleport(pid(0), far, [0.0, 1.0]);
    runner.take_events();
    for _ in 0..(60 * 20) {
        runner.step(&mut idle());
    }
    let kinds: Vec<_> = runner
        .events()
        .iter()
        .filter_map(|(_, e)| match e {
            SimEvent::Evaded { .. } => Some("evaded"),
            SimEvent::PursuitEnded { .. } => Some("ended"),
            SimEvent::Busted { .. } => Some("busted"),
            _ => None,
        })
        .collect();
    assert_eq!(&kinds[..2], &["evaded", "ended"]);
}
