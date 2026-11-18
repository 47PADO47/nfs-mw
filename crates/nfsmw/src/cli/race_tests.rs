//! The unattended race run's options on the command line.

use std::path::PathBuf;

use clap::Parser;

use super::{Cli, Command};
use crate::parity::race_run::{AUTOPILOT_SKILL, Autopilot, RACE_TRACE_ENV, RaceRequest, RaceRunOptions};

fn no_env(_: &str) -> Option<String> {
    None
}

/// The run options of a command line that must parse.
fn options(args: &[&str]) -> RaceRunOptions {
    let cli = Cli::try_parse_from(args).unwrap_or_else(|e| panic!("{args:?}: {e}"));
    let command = cli.command.expect("a command");
    command.race_run().expect("a command that drives").options(no_env)
}

fn fails(args: &[&str]) -> bool {
    Cli::try_parse_from(args).is_err()
}

#[test]
fn a_drive_with_no_race_options_is_an_ordinary_run() {
    assert_eq!(options(&["nfsmw", "view-world", "--drive"]), RaceRunOptions::default());
    assert_eq!(options(&["nfsmw", "play", "--drive"]), RaceRunOptions::default());
    assert_eq!(options(&["nfsmw", "play"]), RaceRunOptions::default());
}

#[test]
fn a_race_keeps_the_events_own_opponents_and_laps_unless_given() {
    let plain = options(&["nfsmw", "view-world", "--drive", "--race", "16.1.1"]);
    assert_eq!(plain.race, Some(RaceRequest { event: "16.1.1".into(), opponents: None, laps: None, difficulty: None }));
    let set = options(&["nfsmw", "play", "--drive", "--race", "16.1.1", "--race-opponents", "3", "--race-laps", "2"]);
    assert_eq!(
        set.race,
        Some(RaceRequest { event: "16.1.1".into(), opponents: Some(3), laps: Some(2), difficulty: None })
    );
    let alone = options(&["nfsmw", "view-world", "--drive", "BMWM3GTR", "--race", "16.1.1", "--race-opponents", "0"]);
    assert_eq!(alone.race.unwrap().opponents, Some(0), "a race against nobody can be asked for");
}

#[test]
fn the_quick_race_difficulty_is_a_name_or_a_level_and_keeps_the_events_own_unless_given() {
    let level = |args: &[&str]| options(args).race.and_then(|race| race.difficulty);
    let base = ["nfsmw", "view-world", "--drive", "--race", "16.1.1"];
    assert_eq!(level(&base), None, "the event's own DifficultyLevel");
    for (name, want) in [("easy", 33), ("Medium", 66), ("hard", 100), ("0", 0), ("50", 50)] {
        assert_eq!(level(&[&base[..], &["--race-difficulty", name]].concat()), Some(want), "{name}");
    }
    assert!(fails(&[&base[..], &["--race-difficulty", "101"]].concat()));
    assert!(fails(&[&base[..], &["--race-difficulty", "brutal"]].concat()));
    assert!(fails(&["nfsmw", "view-world", "--drive", "--race-difficulty", "hard"]), "it needs a race");
}

#[test]
fn a_race_needs_a_car_and_its_details_need_a_race() {
    assert!(fails(&["nfsmw", "view-world", "--race", "16.1.1"]), "flying has no car to race");
    assert!(fails(&["nfsmw", "play", "--race", "16.1.1"]), "the menus are not skipped without --drive");
    assert!(fails(&["nfsmw", "view-world", "--drive", "--race-laps", "2"]));
    assert!(fails(&["nfsmw", "view-world", "--drive", "--race-opponents", "2"]));
    assert!(fails(&["nfsmw", "view-world", "--drive", "--race", "16.1.1", "--race-laps", "0"]));
    assert!(fails(&["nfsmw", "view-car", "--race", "16.1.1"]), "only the commands that drive take a race");
}

#[test]
fn the_autopilot_starts_at_the_green_light_with_full_skill_unless_told() {
    let plain = options(&["nfsmw", "view-world", "--drive", "--race", "16.1.1", "--autopilot"]);
    assert_eq!(plain.autopilot, Some(Autopilot { after_secs: 0.0, skill: AUTOPILOT_SKILL }));
    let late = options(&[
        "nfsmw",
        "view-world",
        "--drive",
        "--autopilot",
        "--autopilot-after",
        "2.5",
        "--autopilot-skill",
        "0.4",
    ]);
    assert_eq!(late.autopilot, Some(Autopilot { after_secs: 2.5, skill: 0.4 }));
    assert!(fails(&["nfsmw", "view-world", "--drive", "--autopilot-after", "2"]), "a delay of no autopilot");
    assert!(fails(&["nfsmw", "view-world", "--drive", "--autopilot", "--autopilot-after", "-1"]));
    assert!(fails(&["nfsmw", "view-world", "--drive", "--autopilot", "--autopilot-skill", "1.5"]));
}

#[test]
fn the_trace_file_and_the_exit_are_asked_for_separately() {
    let run = options(&["nfsmw", "view-world", "--drive", "--race-trace", "out/race.jsonl", "--exit-after-race"]);
    assert_eq!(run.trace, Some(PathBuf::from("out/race.jsonl")));
    assert!(run.exit_after_race && run.race.is_none() && run.autopilot.is_none());
    let cli = Cli::try_parse_from(["nfsmw", "view-world", "--drive"]).unwrap();
    let Some(Command::ViewWorld { race_run, .. }) = cli.command else { panic!("wrong command") };
    let from_env = race_run.options(|name| (name == RACE_TRACE_ENV).then(|| "env.jsonl".to_owned()));
    assert_eq!(from_env.trace, Some(PathBuf::from("env.jsonl")), "the variable alone turns the trace on");
}

#[test]
fn only_the_commands_that_drive_have_a_race_run() {
    let cli = Cli::try_parse_from(["nfsmw", "view-car"]).unwrap();
    assert!(cli.command.unwrap().race_run().is_none());
}
