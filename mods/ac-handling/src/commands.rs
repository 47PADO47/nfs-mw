//! The words of the `ac-handling` console command: `on`, `off`, `status`, `preset <name>`. Pure, so the toggle logic is tested
//! without a host.

use crate::settings::PresetChoice;

/// The help text of the command.
pub const HELP: &str = "ac-handling on|off|status|preset <auto|economy|road|sport|semislick|slick>";

/// One parsed command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Switch the model on.
    On,
    /// Switch it off.
    Off,
    /// Say what is going on.
    Status,
    /// Pick the compound.
    Preset(PresetChoice),
}

/// Parse the words after the command name. The error is the usage line, to show the player.
pub fn parse(args: &[&str]) -> Result<Command, String> {
    let usage = || format!("usage: {HELP}");
    match args {
        ["on"] => Ok(Command::On),
        ["off"] => Ok(Command::Off),
        ["status"] => Ok(Command::Status),
        ["preset", name] => PresetChoice::parse(name).map(Command::Preset).ok_or_else(usage),
        _ => Err(usage()),
    }
}
