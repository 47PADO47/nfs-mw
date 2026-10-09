//! The minimap setting: how the HUD's minimap is shown (docs/specs/hud-minimap.md, section 6).

use std::fmt;
use std::str::FromStr;

use blackbox_minimap::Orientation;

/// The original keeps the minimap's mode per situation (free roam and races) as a player setting with the values
/// 0 fixed, 1 rotating, 2 off; free roam starts fixed. This rewrite has one setting for the free roam it has.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum MinimapMode {
    /// North is up the screen and the arrow turns.
    #[default]
    Fixed,
    /// The car's heading is up the screen and the map turns under the arrow.
    Rotating,
    /// No minimap.
    Off,
}

impl MinimapMode {
    /// How the picture is turned, or `None` when there is no minimap.
    pub fn orientation(self) -> Option<Orientation> {
        match self {
            Self::Fixed => Some(Orientation::North),
            Self::Rotating => Some(Orientation::Heading),
            Self::Off => None,
        }
    }
}

impl FromStr for MinimapMode {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "fixed" | "north" | "0" => Ok(Self::Fixed),
            "rotating" | "heading" | "1" => Ok(Self::Rotating),
            "off" | "none" | "2" => Ok(Self::Off),
            _ => Err(format!("expected fixed, rotating or off, got {s:?}")),
        }
    }
}

impl fmt::Display for MinimapMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Fixed => "fixed",
            Self::Rotating => "rotating",
            Self::Off => "off",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_names_and_the_originals_numbers() {
        assert_eq!("fixed".parse(), Ok(MinimapMode::Fixed));
        assert_eq!(" Rotating ".parse(), Ok(MinimapMode::Rotating));
        assert_eq!("2".parse(), Ok(MinimapMode::Off));
        assert_eq!("0".parse(), Ok(MinimapMode::Fixed));
        assert!("zoomed".parse::<MinimapMode>().is_err());
    }

    #[test]
    fn displays_what_it_parses_and_defaults_to_the_free_roam_mode_of_the_original() {
        for m in [MinimapMode::Fixed, MinimapMode::Rotating, MinimapMode::Off] {
            assert_eq!(m.to_string().parse(), Ok(m));
        }
        assert_eq!(MinimapMode::default(), MinimapMode::Fixed);
        assert_eq!(MinimapMode::Fixed.orientation(), Some(Orientation::North));
        assert_eq!(MinimapMode::Rotating.orientation(), Some(Orientation::Heading));
        assert_eq!(MinimapMode::Off.orientation(), None);
    }
}
