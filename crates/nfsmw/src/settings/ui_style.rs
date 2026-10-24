//! The HUD style: the look of the HUD and menu UI (`docs/specs/ui-themes.md` section 11). Three choices: the game's own
//! HUD, the rewrite's token-driven HUD, and the remastered HUD. The theme the token-driven HUD takes its colours from is
//! [`UiTheme`](super::UiTheme), and it applies only when the HUD is not Original. Independent of the graphics
//! `VisualStyle`. It replaces the old `radio_hud` key (see `radio_hud.rs`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiStyle {
    /// The game's own screens and HUD, drawn through the authored `.fng` packages. Pixel-faithful by default.
    #[default]
    Original,
    /// The rewrite's own modern HUD and menus, rewritten from scratch (`docs/decisions/0013-custom-hud.md`). Until the
    /// rewrite lands it draws the drive card, the radio card and the world map's legend; the other slots fall back to
    /// Original.
    Custom,
    /// The remastered look: its own cluster, minimap, pursuit timer, police radar, radio card, vertical menus and world
    /// map (`ui/theme/remastered`). A slot it does not fill falls back to Original.
    Remastered,
}
names!(
    UiStyle,
    "original, custom or remastered",
    [(Self::Original, "original"), (Self::Custom, "custom"), (Self::Remastered, "remastered")]
);
impl UiStyle {
    pub const ALL: [Self; 3] = [Self::Original, Self::Custom, Self::Remastered];
    /// The style's name in the current language, as the options show it.
    pub fn label(self) -> String {
        let id = match self {
            Self::Original => "options-original",
            Self::Custom => "options-custom",
            Self::Remastered => "options-remastered",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_style_round_trips_defaults_to_the_original_and_rejects_other_names() {
        assert_eq!(UiStyle::default(), UiStyle::Original);
        assert_eq!(UiStyle::ALL.len(), 3);
        for style in UiStyle::ALL {
            assert_eq!(style.to_string().parse::<UiStyle>(), Ok(style));
        }
        assert_eq!("original".parse::<UiStyle>(), Ok(UiStyle::Original));
        assert_eq!("custom".parse::<UiStyle>(), Ok(UiStyle::Custom));
        assert_eq!("remastered".parse::<UiStyle>(), Ok(UiStyle::Remastered));
        for name in ["ea_trax", "Original", "PC", "", "enhanced"] {
            assert!(name.parse::<UiStyle>().is_err(), "{name:?} must not parse");
        }
    }

    #[test]
    fn labels_are_the_three_hud_choices() {
        assert_eq!(UiStyle::Original.label(), "Original");
        assert_eq!(UiStyle::Custom.label(), "Custom");
        assert_eq!(UiStyle::Remastered.label(), "Remastered");
    }
}
