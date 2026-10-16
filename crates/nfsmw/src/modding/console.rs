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

fn loaded_line(summary: &ModSummary) -> String {
    if summary.quarantined {
        return format!("{} {} quarantined after {} faults", summary.id, summary.version, summary.faults);
    }
    format!("{} {} loaded", summary.id, summary.version)
}
