//! The legacy `radio_hud` key, read for back-compat only. It is folded into [`UiStyle`](super::UiStyle): `ea_trax` is
//! `Original` and `custom` is `Custom`. Nothing writes this key any more, and a layer that also sets `ui_style` ignores it.
//! Specs: `docs/specs/ui-themes.md` section 11, `docs/specs/music-graph.md` section 7.

use std::str::FromStr;

use super::UiStyle;

/// The two values the old key accepted. Kept so an existing config file still loads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RadioHudStyle {
    /// The original EA Trax chyron. Maps to `UiStyle::Original`.
    EaTrax,
    /// The rewrite's card. Maps to `UiStyle::Custom`.
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

impl From<RadioHudStyle> for UiStyle {
    fn from(style: RadioHudStyle) -> Self {
        match style {
            RadioHudStyle::EaTrax => Self::Original,
            RadioHudStyle::Custom => Self::Custom,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_legacy_values_parse_and_map_onto_the_ui_style() {
        assert_eq!("EA-Trax".parse::<RadioHudStyle>(), Ok(RadioHudStyle::EaTrax));
        assert_eq!("ea_trax".parse::<RadioHudStyle>(), Ok(RadioHudStyle::EaTrax));
        assert_eq!("custom".parse::<RadioHudStyle>(), Ok(RadioHudStyle::Custom));
        assert!("chyron".parse::<RadioHudStyle>().is_err());
        assert_eq!(UiStyle::from(RadioHudStyle::EaTrax), UiStyle::Original);
        assert_eq!(UiStyle::from(RadioHudStyle::Custom), UiStyle::Custom);
    }
}
