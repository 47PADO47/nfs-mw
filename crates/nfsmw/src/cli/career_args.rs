//! The options of an unattended career run, on `play`: `docs/testing.md`, "Unattended career run".

use std::path::PathBuf;

use clap::Args;

/// Options of an unattended career run.
#[derive(Args, Clone, Default)]
pub struct CareerRunArgs {
    /// Start a new career under this alias at once, as Start New Career on the career menu does, skipping the boot flow
    /// and the menus. The alias is made in the profile folder (BLACKBOX_PROFILE_DIR for a fresh one).
    #[arg(long, value_name = "ALIAS", conflicts_with_all = ["skip_boot", "drive", "aliases"])]
    pub new_career: Option<String>,
    /// Write a JSON-lines record of the career's story flow to this file, replacing it [env NFSMW_CAREER_TRACE].
    #[arg(long, value_name = "FILE.jsonl")]
    pub career_trace: Option<PathBuf>,
    /// End the process when the career's first car is bought in the car lot the prologue ends in.
    #[arg(long)]
    pub exit_after_first_car: bool,
}
