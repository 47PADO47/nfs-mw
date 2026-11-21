//! The options of an unattended career run, on `play`: `docs/testing.md`, "Unattended career run".

use std::path::PathBuf;

use clap::{Args, ValueEnum};

/// Where a new career of an unattended run starts.
#[derive(ValueEnum, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CareerStart {
    /// The prologue's first race, `16.1.0`, as Start New Career does.
    #[default]
    Prologue,
    /// The car lot the prologue ends in, with the state the prologue leaves: its six events won and the cash their data
    /// pays. The races, movies and scenes of the prologue are not played, and no reward of them is granted.
    FirstCarLot,
}

/// Options of an unattended career run.
#[derive(Args, Clone, Default)]
pub struct CareerRunArgs {
    /// Start a new career under this alias at once, as Start New Career on the career menu does, skipping the boot flow
    /// and the menus. The alias is made in the profile folder (BLACKBOX_PROFILE_DIR for a fresh one).
    #[arg(long, value_name = "ALIAS", conflicts_with_all = ["skip_boot", "drive", "aliases"])]
    pub new_career: Option<String>,
    /// Where the new career starts (with --new-career).
    #[arg(long, value_enum, default_value_t, requires = "new_career")]
    pub career_start: CareerStart,
    /// Write a JSON-lines record of the career's story flow to this file, replacing it [env NFSMW_CAREER_TRACE].
    #[arg(long, value_name = "FILE.jsonl")]
    pub career_trace: Option<PathBuf>,
    /// End the process when the career's first car is bought in the car lot the prologue ends in.
    #[arg(long)]
    pub exit_after_first_car: bool,
    /// End the process when the first safe house visit hands the career to the safe house menu.
    #[arg(long, conflicts_with = "exit_after_first_car")]
    pub exit_after_first_visit: bool,
    /// End every movie and NIS scene as soon as it starts, as a skip would, for short unattended runs.
    #[arg(long)]
    pub skip_cinematics: bool,
}
