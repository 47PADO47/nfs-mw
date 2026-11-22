//! The words of the `ac-handling` console command. Pure, so the toggle logic is tested without a host.
//!
//! ```text
//! ac-handling on | off | status | ffb
//! ac-handling preset <auto|economy|road|sport|semislick|slick>
//! ac-handling <suspension|aids|steering|thermal> <on|off|preset>
//! ac-handling tc <0-5> | abs <0-5> | esc <0-4> | bias <-20..20>
//! ac-handling device <pad|wheel> | hud <on|off>
//! ```

use crate::aids::{MAX_CLICKS, MAX_ESC_LEVEL, MAX_LEVEL};
use crate::settings::{PresetChoice, Unit, UnitMode};
use crate::steering::Device;

/// The help text of the command.
pub const HELP: &str = "ac-handling on|off|status|ffb | preset <auto|economy|road|sport|semislick|slick> | \
<suspension|aids|steering|thermal> <on|off|preset> | tc|abs <0-5> | esc <0-4> | bias <-20..20> | device <pad|wheel> | hud <on|off>";

/// One level of the aids.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LevelKind {
    /// Traction control.
    Tc,
    /// ABS.
    Abs,
    /// Stability control.
    Esc,
    /// Brake bias in clicks.
    Bias,
}

/// One parsed command line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    /// Switch the model on.
    On,
    /// Switch it off.
    Off,
    /// Say what is going on.
    Status,
    /// Say the force-feedback signal of the steering unit.
    Ffb,
    /// Pick the compound.
    Preset(PresetChoice),
    /// Set the mode of an optional unit.
    Unit(Unit, UnitMode),
    /// Set a level of the aids.
    Level(LevelKind, i32),
    /// Pick the steering device.
    Device(Device),
    /// Show or hide the tyre temperatures on the HUD.
    Hud(bool),
}

fn usage() -> String {
    format!("usage: {HELP}")
}

fn level(kind: LevelKind, text: &str) -> Result<Command, String> {
    let value: i32 = text.parse().map_err(|_| usage())?;
    let (lo, hi) = match kind {
        LevelKind::Tc | LevelKind::Abs => (0, MAX_LEVEL as i32),
        LevelKind::Esc => (0, MAX_ESC_LEVEL as i32),
        LevelKind::Bias => (-MAX_CLICKS, MAX_CLICKS),
    };
    if value < lo || value > hi {
        return Err(format!("the level must be from {lo} to {hi}; {}", usage()));
    }
    Ok(Command::Level(kind, value))
}

/// Parse the words after the command name. The error is the usage line, to show the player.
pub fn parse(args: &[&str]) -> Result<Command, String> {
    match args {
        ["on"] => Ok(Command::On),
        ["off"] => Ok(Command::Off),
        ["status"] => Ok(Command::Status),
        ["ffb"] => Ok(Command::Ffb),
        ["preset", name] => PresetChoice::parse(name).map(Command::Preset).ok_or_else(usage),
        ["tc", value] => level(LevelKind::Tc, value),
        ["abs", value] => level(LevelKind::Abs, value),
        ["esc", value] => level(LevelKind::Esc, value),
        ["bias", value] => level(LevelKind::Bias, value),
        ["device", name] => Device::parse(name).map(Command::Device).ok_or_else(usage),
        ["hud", "on"] => Ok(Command::Hud(true)),
        ["hud", "off"] => Ok(Command::Hud(false)),
        [unit, mode] => match (Unit::parse(unit), UnitMode::parse(mode)) {
            (Some(unit), Some(mode)) => Ok(Command::Unit(unit, mode)),
            _ => Err(usage()),
        },
        _ => Err(usage()),
    }
}
