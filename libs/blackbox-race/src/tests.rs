//! Whole-race tests: the pieces together.

use glam::Vec3;

use crate::*;

fn gate(z: f32) -> Gate {
    Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
}

fn course(laps: u32) -> Course {
    Course { start: Vec3::ZERO, checkpoints: vec![gate(100.0), gate(200.0)], finish: Some(gate(300.0)), laps }
}

/// Drives racer `index` straight along z through `zs` (x offset `x`), returning every event.
fn drive(race: &mut Race, index: usize, x: f32, zs: &[f32]) -> Vec<RaceEvent> {
    let mut events = Vec::new();
    race.place_racer(index, Vec3::new(x, 0.0, 0.0));
    for &z in zs {
        events.extend(race.move_racer(index, Vec3::new(x, 0.0, z)));
    }
    events
}

fn started(laps: u32, config: RaceConfig, humans: &[bool]) -> Race {
    let grid: Vec<(bool, Vec3)> = humans.iter().map(|&h| (h, Vec3::new(0.0, 0.0, 0.0))).collect();
    let mut race = Race::new(course(laps), config, &grid);
    race.start_now();
    race
}

const LAP: [f32; 5] = [10.0, 150.0, 250.0, 350.0, 400.0];

#[test]
fn the_cars_do_not_score_before_go() {
    let grid = [(true, Vec3::ZERO)];
    let mut race = Race::new(course(1), RaceConfig::default(), &grid);
    assert!(drive(&mut race, 0, 0.0, &LAP).is_empty());
    assert_eq!(race.racers()[0].progress, Progress::default());
}

#[test]
fn a_sprint_win_ends_the_race_for_the_human() {
    let mut race = started(1, RaceConfig::default(), &[true, false]);
    race.tick(30.0);
    let events = drive(&mut race, 0, 0.0, &LAP);
    assert!(events.contains(&RaceEvent::Finished { racer: 0, time: 30.0 }));
    assert_eq!(events.last(), Some(&RaceEvent::Over));
    assert!(race.is_over());
    let outcomes = race.outcomes();
    assert_eq!(outcomes[0].place, 1);
    assert!(outcomes[0].finished && !outcomes[1].finished);
}

#[test]
fn the_opponent_ahead_of_the_player_takes_first() {
    let mut race = started(1, RaceConfig::default(), &[true, false]);
    race.tick(20.0);
    drive(&mut race, 1, 0.0, &LAP);
    race.tick(5.0);
    drive(&mut race, 0, 0.0, &LAP);
    let outcomes = race.outcomes();
    assert_eq!((outcomes[1].place, outcomes[0].place), (1, 2));
}

#[test]
fn a_knockout_drops_the_last_racer_each_lap() {
    let config = RaceConfig { knockouts_per_lap: 1, ..Default::default() };
    let mut race = started(3, config, &[true, false, false, false]);
    race.tick(10.0);
    drive(&mut race, 0, 0.0, &LAP);
    drive(&mut race, 1, 0.0, &LAP);
    let events = drive(&mut race, 2, 0.0, &LAP);
    assert!(events.contains(&RaceEvent::KnockedOut { racer: 3 }), "{events:?}");
    assert!(race.racers()[3].out_at.is_some());
    assert!(race.racers()[0].racing(), "the human is not knocked out");
}

#[test]
fn a_human_who_falls_behind_is_not_knocked_out_by_the_rule() {
    let config = RaceConfig { knockouts_per_lap: 1, ..Default::default() };
    let mut race = started(3, config, &[true, false, false, false]);
    race.tick(10.0);
    for ai in 1..4 {
        drive(&mut race, ai, 0.0, &LAP);
    }
    assert!(race.racers()[0].racing());
    assert!(race.racers()[3].racing(), "the field was full: nobody dropped");
}

#[test]
fn a_time_limit_ends_the_race_with_nobody_finished() {
    let config = RaceConfig { time_limit: 20.0, ..Default::default() };
    let mut race = started(1, config, &[true, false]);
    let mut events = Vec::new();
    for _ in 0..30 {
        events.extend(race.tick(1.0));
    }
    assert_eq!(events, [RaceEvent::TimeUp, RaceEvent::Over]);
    assert!(race.outcomes().iter().all(|o| !o.finished));
}

#[test]
fn bonus_time_extends_the_limit() {
    let config = RaceConfig { time_limit: 20.0, ..Default::default() };
    let mut race = started(1, config, &[true]);
    race.tick(15.0);
    race.award_bonus(10.0);
    assert!(race.tick(10.0).is_empty(), "25 s of 30 used");
    assert!((race.time_left() - 5.0).abs() < 1e-3);
}

#[test]
fn a_speed_trap_race_ranks_by_points() {
    let config = RaceConfig { rank_by_points: true, speed_traps: 2, ..Default::default() };
    let mut race = started(1, config, &[true, false]);
    race.speed_trap(0, 0, 100.0);
    race.speed_trap(1, 0, 200.0);
    race.speed_trap(1, 1, 150.0);
    let places = race.places();
    assert_eq!(places, [2, 1]);
}

#[test]
fn a_totalled_human_ends_the_race() {
    let mut race = started(1, RaceConfig::default(), &[true, false]);
    let events = race.knock_out(0);
    assert_eq!(events, [RaceEvent::KnockedOut { racer: 0 }, RaceEvent::Over]);
    assert_eq!(race.outcomes()[0].place, 2, "behind the racer still racing");
}

#[test]
fn the_countdown_holds_the_race_until_go() {
    let grid = [(true, Vec3::ZERO)];
    let mut race = Race::new(course(1), RaceConfig::default(), &grid);
    race.begin_countdown();
    let mut events = Vec::new();
    for _ in 0..200 {
        events.extend(race.tick(1.0 / 60.0));
    }
    assert_eq!(events, [RaceEvent::Count(3), RaceEvent::Count(2), RaceEvent::Count(1), RaceEvent::Go]);
}
