//! The riding style of the rider of a bike (`riding_style`): how he leans into a corner. RIDE 6's riding style page offers three
//! (its `LEAN_STYLE` option: balanced, shoulders out and old school); each bike type has its own lean clips for them. Read by
//! the rider's animation (`scenes/world/drive/rider`), which picks the lean set of its graph by [`RidingStyle::blueprint`]
//! (`docs/specs/ride6-rider-styles.md`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RidingStyle {
    /// Upright and square on the bike, the knee out a little.
    #[default]
    Balanced,
    /// Shoulders and body hung off the bike into the corner.
    ShouldersOut,
    /// Upright on the seat, the inside foot off the peg on a supermoto.
    OldSchool,
}
names!(
    RidingStyle,
    "balanced, shoulders-out or old-school",
    [(Self::Balanced, "balanced"), (Self::ShouldersOut, "shoulders-out"), (Self::OldSchool, "old-school")]
);
impl RidingStyle {
    pub const ALL: [Self; 3] = [Self::Balanced, Self::ShouldersOut, Self::OldSchool];

    /// The style's value in RIDE 6's animation blueprint (`RidingStyle`, `LEAN_STYLE` of the options): 0, 1 or 2.
    pub const fn blueprint(self) -> u8 {
        match self {
            Self::Balanced => 0,
            Self::ShouldersOut => 1,
            Self::OldSchool => 2,
        }
    }

    /// The style's name in the current language, as the options show it.
    pub fn label(self) -> String {
        let id = match self {
            Self::Balanced => "options-riding-style-balanced",
            Self::ShouldersOut => "options-riding-style-shoulders-out",
            Self::OldSchool => "options-riding-style-old-school",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_styles_round_trip_default_to_balanced_and_reject_other_names() {
        assert_eq!(RidingStyle::default(), RidingStyle::Balanced);
        for style in RidingStyle::ALL {
            assert_eq!(style.to_string().parse::<RidingStyle>(), Ok(style));
        }
        assert_eq!("old-school".parse::<RidingStyle>(), Ok(RidingStyle::OldSchool));
        for name in ["leaning", "Balanced", "old_school", "", "0"] {
            assert!(name.parse::<RidingStyle>().is_err(), "{name:?} must not parse");
        }
    }

    #[test]
    fn the_blueprint_values_are_the_options_values_in_order() {
        let values: Vec<u8> = RidingStyle::ALL.iter().map(|style| style.blueprint()).collect();
        assert_eq!(values, [0, 1, 2]);
    }

    #[test]
    fn labels_name_every_style() {
        assert_eq!(RidingStyle::Balanced.label(), "Balanced");
        assert_eq!(RidingStyle::ShouldersOut.label(), "Shoulders Out");
        assert_eq!(RidingStyle::OldSchool.label(), "Old School");
    }
}
