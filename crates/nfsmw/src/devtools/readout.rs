//! How much of the scene's debug readout is drawn (`--show-readout`). The original HUD shows speed, rpm and
//! gear itself, so the default readout is the small line that adds to it.

use std::fmt;
use std::str::FromStr;

/// The level of the scene readout (bottom left of the window).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShowReadout {
    /// Nothing.
    Off,
    /// One line: what the original HUD does not show (the car and where it is, the free camera).
    #[default]
    Minimal,
    /// The numbers the HUD shows too (speed, rpm, gear, nitrous) and the scripted run.
    Full,
}

impl ShowReadout {
    pub fn name(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Minimal => "minimal",
            Self::Full => "full",
        }
    }
}

impl fmt::Display for ShowReadout {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

impl FromStr for ShowReadout {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "off" | "0" | "false" | "no" => Ok(Self::Off),
            "minimal" | "min" | "on" | "1" | "true" | "yes" => Ok(Self::Minimal),
            "full" | "2" => Ok(Self::Full),
            _ => Err(format!("expected off, minimal or full, got {s:?}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_levels_and_round_trips() {
        for level in [ShowReadout::Off, ShowReadout::Minimal, ShowReadout::Full] {
            assert_eq!(level.to_string().parse::<ShowReadout>(), Ok(level));
        }
        assert_eq!("MIN".parse(), Ok(ShowReadout::Minimal));
        assert_eq!("2".parse(), Ok(ShowReadout::Full));
        assert!("loud".parse::<ShowReadout>().is_err());
    }

    #[test]
    fn the_default_is_the_small_line() {
        assert_eq!(ShowReadout::default(), ShowReadout::Minimal);
    }
}
