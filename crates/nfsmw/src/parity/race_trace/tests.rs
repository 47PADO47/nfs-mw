use blackbox_race::{Course, Gate, RaceConfig};
use glam::Vec3;
use serde_json::{Value, json};

use super::*;
use crate::parity::ParityOptions;

fn gate(z: f32) -> Gate {
    Gate::new(Vec3::new(0.0, 0.0, z), Vec3::Z, Vec3::Y, [2.0, 40.0, 10.0])
}

/// A two-lap race of the player and one opponent: a checkpoint at 100 m and a finish line at 200 m.
fn race() -> Race {
    let course = Course::new(Vec3::ZERO, vec![gate(100.0)], Some(gate(200.0)), 2);
    Race::new(course, RaceConfig::default(), &[(true, Vec3::ZERO), (false, Vec3::ZERO)])
}

fn stamp(tick: u64) -> Stamp {
    Stamp { tick, sim_time: tick as f64 / 60.0, wall_ms: 0.0 }
}

/// Runs `events` through the log and gives the lines as JSON.
fn lines(log: &mut RaceLog, race: &Race, events: &[RaceEvent], tick: u64) -> Vec<Value> {
    let records = events.iter().flat_map(|event| log.records(race, event, stamp(tick)));
    records.map(|record| serde_json::to_value(record).unwrap()).collect()
}

/// Moves `racer` across the gate at `z`, one second of race time after the last move.
fn cross(race: &mut Race, racer: usize, z: f32) -> Vec<RaceEvent> {
    let mut events = race.tick(1.0);
    race.place_racer(racer, Vec3::new(0.0, 0.0, z - 1.0));
    events.extend(race.move_racer(racer, Vec3::new(0.0, 0.0, z + 1.0)));
    events
}

#[test]
fn the_green_light_starts_the_timer_and_the_countdown_writes_nothing() {
    let mut race = race();
    let mut log = RaceLog::default();
    let board = race.begin_countdown();
    assert!(lines(&mut log, &race, &board, 1).is_empty(), "the board's numbers are not lines");
    let go = race.tick(3.0);
    assert!(go.contains(&RaceEvent::Go));
    let written = lines(&mut log, &race, &go, 180);
    assert_eq!(
        written,
        vec![json!({"kind": "timer_start", "tick": 180, "sim_time": 3.0, "wall_ms": 0.0, "race_time": 0.0})]
    );
}

#[test]
fn a_whole_race_is_recorded_per_racer_with_laps_and_checkpoints_from_one() {
    let mut race = race();
    let mut log = RaceLog::default();
    race.start_now();
    // The player: checkpoint at 1 s, lap at 2 s, checkpoint at 3 s.
    let mut written = Vec::new();
    for z in [100.0, 200.0, 100.0] {
        let events = cross(&mut race, 0, z);
        written.extend(lines(&mut log, &race, &events, 1));
    }
    let kinds: Vec<&str> = written.iter().map(|line| line["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["checkpoint", "lap", "checkpoint"]);
    assert_eq!(
        (&written[0]["racer"], &written[0]["lap"], &written[0]["checkpoint"], &written[0]["race_time"]),
        (&json!(0), &json!(1), &json!(1), &json!(1.0))
    );
    assert_eq!(
        (&written[1]["lap"], &written[1]["lap_time"], &written[1]["race_time"]),
        (&json!(1), &json!(2.0), &json!(2.0))
    );
    assert_eq!(
        (&written[2]["lap"], &written[2]["checkpoint"]),
        (&json!(2), &json!(1)),
        "the second lap's first checkpoint"
    );
    // The opponent's first checkpoint, at 4 s, is its own lap 1.
    let events = cross(&mut race, 1, 100.0);
    let theirs = lines(&mut log, &race, &events, 2);
    assert_eq!((&theirs[0]["racer"], &theirs[0]["lap"], &theirs[0]["race_time"]), (&json!(1), &json!(1), &json!(4.0)));
    // The player finishes at 5 s: the last lap, the finish with the place, and the end of the race.
    let events = cross(&mut race, 0, 200.0);
    let end = lines(&mut log, &race, &events, 3);
    let kinds: Vec<&str> = end.iter().map(|line| line["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["lap", "racer_finished", "race_finished"]);
    assert_eq!((&end[0]["lap"], &end[0]["lap_time"], &end[0]["race_time"]), (&json!(2), &json!(3.0), &json!(5.0)));
    assert_eq!((&end[1]["racer"], &end[1]["place"], &end[1]["total_time"]), (&json!(0), &json!(1), &json!(5.0)));
    assert_eq!(end[2]["race_time"], json!(5.0));
    assert_eq!(
        end[2]["results"],
        json!([
            {"racer": 0, "place": 1, "finished": true, "total_time": 5.0},
            {"racer": 1, "place": 2, "finished": false, "total_time": null},
        ])
    );
    assert!(end.iter().all(|line| line["tick"] == 3), "every line carries the tick it was raised in");
}

#[test]
fn a_checkpoint_that_closes_the_lap_in_the_same_tick_keeps_its_lap_number() {
    // No finish line: the last checkpoint closes the lap, so both events come from one move.
    let course = Course::new(Vec3::ZERO, vec![gate(100.0)], None, 2);
    let mut race = Race::new(course, RaceConfig::default(), &[(true, Vec3::ZERO)]);
    let mut log = RaceLog::default();
    race.start_now();
    let events = cross(&mut race, 0, 100.0);
    let written = lines(&mut log, &race, &events, 1);
    assert_eq!((&written[0]["kind"], &written[0]["lap"]), (&json!("checkpoint"), &json!(1)));
    assert_eq!((&written[1]["kind"], &written[1]["lap"]), (&json!("lap"), &json!(1)));
}

#[test]
fn a_knockout_is_a_line_and_the_other_events_are_not() {
    let race = race();
    let mut log = RaceLog::default();
    let quiet = [
        RaceEvent::Count(3),
        RaceEvent::TimeUp,
        RaceEvent::Bonus { racer: 0, seconds: 5.0 },
        RaceEvent::SpeedTrap { racer: 0, slot: 0, kmh: 200.0 },
        RaceEvent::Reset { racer: 0 },
    ];
    assert!(lines(&mut log, &race, &quiet, 1).is_empty());
    let out = lines(&mut log, &race, &[RaceEvent::KnockedOut { racer: 1 }], 1);
    assert_eq!((&out[0]["kind"], &out[0]["racer"]), (&json!("knocked_out"), &json!(1)));
}

fn header() -> Header {
    Header {
        schema: SCHEMA,
        created_unix_ms: 1,
        run: RunHeader::new(
            ParityOptions { seed: Some(7), stock_rules: true },
            Vec::new(),
            Default::default(),
            None,
            None,
        ),
        event: "16.1.1".into(),
        race: "race_bin_16/16_1_1_circuit".into(),
        race_type: "circuit".into(),
        opponents: 3,
        laps: 2,
        checkpoints_per_lap: 4,
        finish_line: true,
        rolling_start: false,
        racers: vec![RacerRow { racer: 0, name: "player".into(), car: Some("BMWM3GTR".into()), human: true }],
        autopilot: Some(Autopilot { after_secs: 1.5, skill: 1.0 }.into()),
    }
}

#[test]
fn the_header_line_carries_the_run_header_the_event_the_opponents_and_the_laps() {
    let line = serde_json::to_value(Record::Header { stamp: stamp(0), header: Box::new(header()) }).unwrap();
    assert_eq!((&line["kind"], &line["schema"]), (&json!("header"), &json!(SCHEMA)));
    assert_eq!((&line["run"]["seed"], &line["run"]["stock_rules"]), (&json!(7), &json!(true)));
    assert_eq!((&line["event"], &line["opponents"], &line["laps"]), (&json!("16.1.1"), &json!(3), &json!(2)));
    assert_eq!(line["racers"][0], json!({"racer": 0, "name": "player", "car": "BMWM3GTR", "human": true}));
    assert_eq!(line["autopilot"], json!({"after_secs": 1.5, "skill": 1.0}));
    assert_eq!(line["tick"], json!(0));
}

#[test]
fn the_file_holds_one_json_object_per_line_and_replaces_an_older_file() {
    let folder = std::env::temp_dir().join(format!("nfsmw-race-trace-{}", std::process::id()));
    let path = folder.join("nested").join("race.jsonl");
    for laps in [5_u32, 2] {
        let mut file = TraceFile::create(&path).unwrap();
        file.write(&Record::Header { stamp: stamp(0), header: Box::new(Header { laps, ..header() }) });
        file.write(&Record::TimerStart { stamp: stamp(180), race_time: 0.0 });
    }
    let text = std::fs::read_to_string(&path).unwrap();
    let _ = std::fs::remove_dir_all(&folder);
    let lines: Vec<Value> = text.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
    assert_eq!(lines.len(), 2, "the second file replaced the first");
    assert_eq!((&lines[0]["kind"], &lines[0]["laps"]), (&json!("header"), &json!(2)));
    assert_eq!((&lines[1]["kind"], &lines[1]["tick"]), (&json!("timer_start"), &json!(180)));
}
