//! The theme of the token-driven HUD: the palette its colours come from (`docs/specs/ui-themes-tokens.md`). Two choices:
//! the Remastered palette and the Bevy palette. It applies only when the HUD style is not Original
//! ([`UiStyle`](super::UiStyle)).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiTheme {
    /// The remastered palette: gold accents on a dark panel.
    Remastered,
    /// The Bevy palette: the colours of the speedometer and the panels. The default.
    #[default]
    Bevy,
}
names!(UiTheme, "remastered or bevy", [(Self::Remastered, "remastered"), (Self::Bevy, "bevy")]);
impl UiTheme {
    pub const ALL: [Self; 2] = [Self::Remastered, Self::Bevy];
    pub fn label(self) -> &'static str {
        match self {
            Self::Remastered => "Remastered",
            Self::Bevy => "Bevy",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_theme_round_trips_defaults_to_bevy_and_rejects_other_names() {
        assert_eq!(UiTheme::default(), UiTheme::Bevy);
        for theme in UiTheme::ALL {
            assert_eq!(theme.to_string().parse::<UiTheme>(), Ok(theme));
        }
        for name in ["original", "gold", "Bevy", ""] {
            assert!(name.parse::<UiTheme>().is_err(), "{name:?} must not parse");
        }
    }
}
