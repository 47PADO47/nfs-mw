//! The CustomHUD skin option (`custom_hud_skin`): whether the gauge skin of the third-party `CustomHud.asi` plugin
//! replaces the original HUD's gauges. Spec: `docs/specs/custom-hud-gauges.md` section 11.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CustomHudSkin {
    /// Draw the skin when the install has `scripts/CustomHud.asi` and `scripts/CustomHud.ini`; otherwise the stock gauges.
    #[default]
    Auto,
    /// Always draw the stock gauges.
    Off,
}
names!(CustomHudSkin, "auto or off", [(Self::Auto, "auto"), (Self::Off, "off")]);

impl CustomHudSkin {
    pub const ALL: [Self; 2] = [Self::Auto, Self::Off];

    /// The value as the options show it.
    pub fn label(self) -> String {
        let id = match self {
            Self::Auto => "options-auto",
            Self::Off => "options-off",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_choice_round_trips_defaults_to_auto_and_rejects_other_names() {
        assert_eq!(CustomHudSkin::default(), CustomHudSkin::Auto);
        for choice in CustomHudSkin::ALL {
            assert_eq!(choice.to_string().parse::<CustomHudSkin>(), Ok(choice));
        }
        for name in ["on", "Auto", "", "true"] {
            assert!(name.parse::<CustomHudSkin>().is_err(), "{name:?} must not parse");
        }
    }
}
