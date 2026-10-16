//! The `mods` console command: one line per mod, with its state.

use blackbox_modding::host::{ModSummary, Rejected};

use super::Mods;

/// What the `mods` command prints for the mods the game was started with.
pub fn listing(mods: Option<&Mods>) -> String {
    let Some(mods) = mods else {
        return "no mods: the game was started without them".to_owned();
    };
    lines(&mods.loaded(), mods.rejected())
}

/// One line per loaded mod (`id version state`), then one per refused mod with its reason. Empty lists say so.
pub fn lines(loaded: &[ModSummary], rejected: &[Rejected]) -> String {
    if loaded.is_empty() && rejected.is_empty() {
        return "no mods found in the mods folders".to_owned();
    }
    let mut lines: Vec<String> = loaded.iter().map(loaded_line).collect();
    lines.extend(rejected.iter().map(Rejected::summary));
    lines.join("\n")
}

/// The mods' console commands for `help`, as (usage, description) pairs. None without mods.
pub fn help_entries(mods: Option<&Mods>) -> Vec<(String, String)> {
    mods.map_or_else(Vec::new, Mods::command_help)
}

/// Whether a loaded mod registered a console command called `name` (lower case).
pub fn has_command(mods: Option<&Mods>, name: &str) -> bool {
    mods.is_some_and(|mods| mods.has_command(name))
}

/// Run a mod's console command, and give its output or its error.
pub fn run_command(mods: Option<&Mods>, name: &str, args: &[&str]) -> Result<String, String> {
    let Some(mods) = mods else {
        return Err(format!("unknown command {name:?}"));
    };
    mods.run_command(name, args)
}

fn loaded_line(summary: &ModSummary) -> String {
    if summary.quarantined {
        return format!("{} {} quarantined after {} faults", summary.id, summary.version, summary.faults);
    }
    format!("{} {} loaded", summary.id, summary.version)
}
