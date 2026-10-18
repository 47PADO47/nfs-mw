//! The UI theme: the look of all HUD and menu UI (`docs/specs/ui-themes.md` section 11).
//! Independent of the graphics `VisualStyle`. It replaces the old `radio_hud` key (see `radio_hud.rs`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UiStyle {
    /// The game's own screens and HUD, drawn through the authored `.fng` packages. Pixel-faithful by default.
    #[default]
    Original,
    /// The rewrite's own elements (the radio card so far). Everything else falls back to Original.
    Custom,
    /// Reserved for the remastered theme. Not implemented yet: until it is, every slot falls back to Original.
    Remastered,
}
names!(
    UiStyle,
    "original, custom or remastered",
    [(Self::Original, "original"), (Self::Custom, "custom"), (Self::Remastered, "remastered")]
);
impl UiStyle {
    pub const ALL: [Self; 3] = [Self::Original, Self::Custom, Self::Remastered];
    pub fn label(self) -> &'static str {
        match self {
            Self::Original => "Original",
            Self::Custom => "Custom",
            Self::Remastered => "Remastered (incomplete)",
        }
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
    fn labels_name_the_incomplete_remastered_theme() {
        assert_eq!(UiStyle::Original.label(), "Original");
        assert_eq!(UiStyle::Custom.label(), "Custom");
        assert_eq!(UiStyle::Remastered.label(), "Remastered (incomplete)");
    }
}
