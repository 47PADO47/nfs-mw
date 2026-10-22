//! Whole-race tests: the pieces together.

use glam::Vec3;

use crate::*;

fn gate(z: f32) -> Gate {
    Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
}

fn course(laps: u32) -> Course {
    Course::new(Vec3::ZERO, vec![gate(100.0), gate(200.0)], Some(gate(300.0)), laps)
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

fn trap_course() -> Course {
    let mut course = course(1);
    let zone = Sphere { centre: Vec3::new(0.0, 0.0, 150.0), radius: 20.0 };
    course.traps = vec![Trap { zone, threshold_kmh: 88.0 }];
    course
}

#[test]
fn a_fast_pass_through_a_speed_trap_pays_and_a_slow_one_does_not() {
    let config = RaceConfig { rank_by_points: true, ..Default::default() };
    let grid = [(true, Vec3::ZERO), (false, Vec3::ZERO)];
    let mut race = Race::new(trap_course(), config, &grid);
    race.start_now();
    race.set_speed(0, 100.0 * KMH_TO_MS);
    race.set_speed(1, 60.0 * KMH_TO_MS);
    let fast = drive(&mut race, 0, 0.0, &[10.0, 140.0, 160.0]);
    let slow = drive(&mut race, 1, 0.0, &[10.0, 140.0, 160.0]);
    assert!(fast.iter().any(|e| matches!(e, RaceEvent::SpeedTrap { racer: 0, slot: 0, .. })), "{fast:?}");
    assert!(!slow.iter().any(|e| matches!(e, RaceEvent::SpeedTrap { .. })), "{slow:?}");
    assert_eq!(race.places(), [1, 2]);
}

#[test]
fn a_tollbooth_checkpoint_adds_time_for_the_human_only() {
    let mut course = course(1);
    course.bonuses = vec![25.0, 0.0];
    let config = RaceConfig { time_limit: 30.0, ..Default::default() };
    let grid = [(true, Vec3::ZERO), (false, Vec3::ZERO)];
    let mut race = Race::new(course, config, &grid);
    race.start_now();
    let ai = drive(&mut race, 1, 0.0, &[10.0, 140.0]);
    assert!(!ai.iter().any(|e| matches!(e, RaceEvent::Bonus { .. })));
    let human = drive(&mut race, 0, 0.0, &[10.0, 140.0]);
    assert!(human.contains(&RaceEvent::Bonus { racer: 0, seconds: 25.0 }), "{human:?}");
    assert!((race.time_left() - 55.0).abs() < 1e-3);
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

#[test]
fn a_racer_keeps_its_time_and_place_at_each_stage() {
    // The course is 300 m to the line: 20 % is 60 m, 40 % 120 m.
    let mut race = started(1, RaceConfig::default(), &[true, false]);
    race.tick(10.0);
    drive(&mut race, 1, 0.0, &[70.0, 100.0]);
    race.tick(5.0);
    drive(&mut race, 0, 0.0, &[70.0, 130.0]);
    let rival = race.racers()[1].stats.stages[0].expect("the rival passed 20 %");
    assert_eq!((rival.time, rival.place), (10.0, 1));
    let me = race.racers()[0].stats;
    assert_eq!(me.stages[0].map(|m| (m.time, m.place)), Some((15.0, 2)));
    assert!(me.stages[1].is_some(), "a later move fills the next stage");
    assert!(me.stages[2].is_none());
}

#[test]
fn speed_and_distance_count_only_while_racing() {
    let grid = [(true, Vec3::ZERO)];
    let mut race = Race::new(course(1), RaceConfig::default(), &grid);
    race.set_speed(0, 50.0);
    assert_eq!(race.racers()[0].stats.top_speed, 0.0, "not before GO");
    race.start_now();
    race.set_speed(0, 40.0);
    drive(&mut race, 0, 0.0, &[30.0, 60.0]);
    let stats = race.racers()[0].stats;
    assert_eq!((stats.top_speed, stats.distance), (40.0, 60.0));
    assert_eq!(stats.sixty_at, Some(0.0), "40 m/s is past 60 mph");
}

/// The last checkpoint is a goal like the others; the race ends at the finish line after it, never at the checkpoint
/// (`docs/specs/race-goals.md` section 2).
#[test]
fn the_last_checkpoint_of_a_sprint_does_not_end_the_race() {
    let mut race = started(1, RaceConfig::default(), &[true, false]);
    race.tick(30.0);
    let events = drive(&mut race, 0, 0.0, &[10.0, 150.0, 250.0]);
    assert_eq!(events, [RaceEvent::Checkpoint { racer: 0, number: 1 }, RaceEvent::Checkpoint { racer: 0, number: 2 }]);
    assert!(!race.is_over() && race.racers()[0].racing());
    assert_eq!(race.racers()[0].progress.reached, 2, "all the checkpoints, none of the finish");
    let events = drive(&mut race, 0, 0.0, &[250.0, 350.0]);
    assert_eq!(events.last(), Some(&RaceEvent::Over), "the finish line after it ends the race");
}

#[test]
fn standing_in_the_last_checkpoint_does_not_finish_the_race() {
    let mut race = started(1, RaceConfig::default(), &[true]);
    drive(&mut race, 0, 0.0, &[10.0, 150.0, 199.0]);
    for step in 0..600 {
        let z = 199.0 + (step % 3) as f32 * 0.5;
        assert!(race.move_racer(0, Vec3::new(0.0, 0.0, z)).iter().all(|e| !matches!(e, RaceEvent::Finished { .. })));
    }
    assert!(!race.is_over());
}

#[test]
fn a_far_finish_line_after_a_wide_last_checkpoint_is_reached_on_its_own() {
    // The prologue's sprint: a wide cube for the last checkpoint and the finish line hundreds of metres on.
    let cube = Gate::new(Vec3::new(0.0, 0.0, 1000.0), Vec3::Z, Vec3::Y, [80.0, 80.0, 80.0]);
    let finish = Gate::new(Vec3::new(0.0, 0.0, 1590.0), Vec3::Z, Vec3::Y, [6.0, 80.0, 6.0]);
    let course = Course::new(Vec3::ZERO, vec![cube], Some(finish), 1);
    let mut race = Race::new(course, RaceConfig::default(), &[(true, Vec3::ZERO)]);
    race.start_now();
    let mut events = Vec::new();
    for z in (0..1700).step_by(2) {
        events.extend(race.move_racer(0, Vec3::new(0.0, 0.0, z as f32)));
    }
    let checkpoint = events.iter().position(|e| matches!(e, RaceEvent::Checkpoint { number: 1, .. })).unwrap();
    let finished = events.iter().position(|e| matches!(e, RaceEvent::Finished { .. })).unwrap();
    assert!(finished > checkpoint, "the checkpoint comes first and the finish line is its own crossing");
    assert_eq!(events.iter().filter(|e| matches!(e, RaceEvent::Checkpoint { .. })).count(), 1);
}

/// A one metre slab across the road 50 m behind the grid, like the wrong-way trigger of a race
/// (`docs/specs/race-wrongway.md`).
fn wrong_way_course() -> Course {
    let mut course = course(1);
    course.resets = vec![Gate::new(Vec3::new(0.0, 0.0, -50.0), Vec3::Z, Vec3::Y, [1.0, 150.0, 100.0])];
    course
}

fn wrong_way_race(humans: &[bool]) -> Race {
    let grid: Vec<(bool, Vec3)> = humans.iter().map(|&human| (human, Vec3::ZERO)).collect();
    let mut race = Race::new(wrong_way_course(), RaceConfig::default(), &grid);
    race.start_now();
    race
}

#[test]
fn a_human_driving_backwards_into_the_wrong_way_zone_is_sent_back() {
    let mut race = wrong_way_race(&[true]);
    let events = drive(&mut race, 0, 0.0, &[-20.0, -49.0, -51.0]);
    assert_eq!(events, [RaceEvent::Reset { racer: 0 }]);
    assert!(!race.is_over(), "a reset is a restart, not an end");
}

#[test]
fn the_zone_raises_its_event_once_per_entry() {
    let mut race = wrong_way_race(&[true]);
    let first = drive(&mut race, 0, 0.0, &[-40.0, -49.8, -50.2, -52.0, -60.0]);
    assert_eq!(first, [RaceEvent::Reset { racer: 0 }], "once, however long the car stays in or past the slab");
    let mut inside = Vec::new();
    inside.extend(race.move_racer(0, Vec3::new(0.0, 0.0, -50.3)));
    inside.extend(race.move_racer(0, Vec3::new(0.0, 0.0, -50.1)));
    assert_eq!(inside.len(), 1, "going back in raises it again, the second step inside the slab does not");
}

#[test]
fn a_fast_car_cannot_jump_the_thin_zone() {
    let mut race = wrong_way_race(&[true]);
    let events = drive(&mut race, 0, 0.0, &[-30.0, -80.0]);
    assert_eq!(events, [RaceEvent::Reset { racer: 0 }], "the move from 30 to 80 m behind crosses the 1 m slab");
}

#[test]
fn only_a_human_racer_is_sent_back_and_only_while_racing() {
    let mut race = wrong_way_race(&[true, false]);
    assert!(drive(&mut race, 1, 0.0, &[-40.0, -60.0]).is_empty(), "the computer racers ignore the zone");
    let grid = [(true, Vec3::ZERO)];
    let mut staged = Race::new(wrong_way_course(), RaceConfig::default(), &grid);
    assert!(drive(&mut staged, 0, 0.0, &[-40.0, -60.0]).is_empty(), "nothing is armed before GO");
    let mut finished = wrong_way_race(&[true, false]);
    drive(&mut finished, 0, 0.0, &LAP);
    assert!(drive(&mut finished, 0, 0.0, &[-40.0, -60.0]).is_empty(), "a racer that is done is not sent back");
}

#[test]
fn a_zone_beside_the_road_is_not_touched_by_the_road() {
    let mut race = wrong_way_race(&[true]);
    assert!(drive(&mut race, 0, 100.0, &[-40.0, -60.0]).is_empty(), "the slab is 150 m wide, not 250 m");
}
