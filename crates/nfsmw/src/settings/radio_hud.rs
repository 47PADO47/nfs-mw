//! How the radio announces the song on the HUD: the original EA Trax chyron, or the card this build draws itself.
//! The chyron is the package `EA_TRAX.fng` (`docs/specs/music-graph.md` section 7); the card is `hud/radio.rs`.

use std::{fmt, str::FromStr};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RadioHudStyle {
    /// The disc and tab of the original EA Trax screen, animated by the game's own scripts.
    #[default]
    EaTrax,
    /// The rewrite's text card: title, artist and album with the time, drawn in egui.
    Custom,
}

impl FromStr for RadioHudStyle {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "ea_trax" | "ea-trax" | "eatrax" => Ok(Self::EaTrax),
            "custom" => Ok(Self::Custom),
            _ => Err(format!("expected ea_trax or custom, got {s:?}")),
        }
    }
}

impl fmt::Display for RadioHudStyle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::EaTrax => "ea_trax",
            Self::Custom => "custom",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_style_round_trips_and_defaults_to_the_original() {
        assert_eq!(RadioHudStyle::default(), RadioHudStyle::EaTrax);
        for style in [RadioHudStyle::EaTrax, RadioHudStyle::Custom] {
            assert_eq!(style.to_string().parse::<RadioHudStyle>(), Ok(style));
        }
        assert_eq!("EA-Trax".parse::<RadioHudStyle>(), Ok(RadioHudStyle::EaTrax));
        assert!("chyron".parse::<RadioHudStyle>().is_err());
    }
}
