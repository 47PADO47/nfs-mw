//! The options of an unattended race run, on the commands that drive (`play --drive`, `view-world --drive`):
//! `docs/testing.md`, "Unattended race runs".

use std::path::PathBuf;

use clap::Args;

use crate::parity::race_run::{AUTOPILOT_SKILL, Autopilot, RaceRequest, RaceRunOptions};

/// Seconds from 0 to an hour.
fn parse_seconds(text: &str) -> Result<f32, String> {
    let seconds: f32 = text.trim().parse().map_err(|_| format!("{text:?} is not a number of seconds"))?;
    if !(0.0..=3600.0).contains(&seconds) {
        return Err(format!("{seconds} is not between 0 and 3600 seconds"));
    }
    Ok(seconds)
}

/// A skill from 0 to 1.
fn parse_skill(text: &str) -> Result<f32, String> {
    let skill: f32 = text.trim().parse().map_err(|_| format!("{text:?} is not a number"))?;
    if !(0.0..=1.0).contains(&skill) {
        return Err(format!("{skill} is not between 0 and 1"));
    }
    Ok(skill)
}

/// Options of an unattended race run.
#[derive(Args, Clone, Default)]
pub struct RaceRunArgs {
    /// Start in this race, skipping the menus: a career event id such as 16.1.1, or a race name as `race list` shows
    /// it (the `race start` console command's race).
    #[arg(long, value_name = "EVENT", requires = "drive")]
    pub race: Option<String>,
    /// With --race: the number of opponents [default: the event's own].
    #[arg(long, value_name = "N", requires = "race")]
    pub race_opponents: Option<usize>,
    /// With --race: the number of laps [default: the event's own].
    #[arg(long, value_name = "N", requires = "race", value_parser = clap::value_parser!(u32).range(1..))]
    pub race_laps: Option<u32>,
    /// In a race, the opponents' AI driver drives the player's car from the green light.
    #[arg(long)]
    pub autopilot: bool,
    /// With --autopilot: the driver takes the car this many seconds of race time after the green light [default 0].
    #[arg(long, value_name = "SECS", requires = "autopilot", value_parser = parse_seconds)]
    pub autopilot_after: Option<f32>,
    /// With --autopilot: the driver's skill, 0 to 1, on the opponents' scale [default 1].
    #[arg(long, value_name = "0-1", requires = "autopilot", value_parser = parse_skill)]
    pub autopilot_skill: Option<f32>,
    /// Write a JSON-lines record of every race of the run to this file, replacing it [env NFSMW_RACE_TRACE].
    #[arg(long, value_name = "FILE.jsonl")]
    pub race_trace: Option<PathBuf>,
    /// End the process when a race is over (or was given up, or could not be started: that is a failure).
    #[arg(long)]
    pub exit_after_race: bool,
}

impl RaceRunArgs {
    /// The run's options: the command line, with the trace file from the environment (through `get`) when no flag names one.
    pub fn options(&self, get: impl Fn(&str) -> Option<String>) -> RaceRunOptions {
        let race = self.race.as_ref().map(|event| RaceRequest {
            event: event.trim().to_owned(),
            opponents: self.race_opponents,
            laps: self.race_laps,
        });
        let autopilot = self.autopilot.then(|| Autopilot {
            after_secs: self.autopilot_after.unwrap_or(0.0),
            skill: self.autopilot_skill.unwrap_or(AUTOPILOT_SKILL),
        });
        RaceRunOptions { race, autopilot, trace: None, exit_after_race: self.exit_after_race }
            .with_trace(self.race_trace.clone(), get)
    }
}
