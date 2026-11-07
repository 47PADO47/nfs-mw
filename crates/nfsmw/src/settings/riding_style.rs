//! The riding style of the rider of a bike: how he leans into a corner. RIDE 6's riding style page offers three (its `LEAN_STYLE`
//! option: balanced, shoulders out and old school); each bike type has its own lean clips for them, and the page shows its own
//! pictures for road, motard and off-road bikes. The player picks one style per [`BikeClass`] (`riding_style_road`,
//! `riding_style_motard`, `riding_style_offroad`). Read by the rider's animation (`scenes/world/drive/rider`), which picks the lean
//! set of its graph by [`RidingStyle::blueprint`] (`docs/specs/ride6-rider-styles.md`).

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

/// The three groups of bikes the riding style is chosen for, as the riding style page of RIDE 6 shows them (its `Road`, `Motard` and
/// `Offroad` picture maps). Which bike is in which group is the rider's own table (`scenes/world/drive/rider/class.rs`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BikeClass {
    /// Superbikes, sport bikes, nakeds, classics and cruisers.
    Road,
    /// Supermotos.
    Motard,
    /// Touring, adventure and enduro bikes and scooters.
    Offroad,
}

impl BikeClass {
    pub const ALL: [Self; 3] = [Self::Road, Self::Motard, Self::Offroad];

    /// The class's place in [`BikeClass::ALL`], for tables indexed by class.
    pub const fn index(self) -> usize {
        match self {
            Self::Road => 0,
            Self::Motard => 1,
            Self::Offroad => 2,
        }
    }

    /// The config key and the console name of the class's riding style.
    pub const fn key(self) -> &'static str {
        match self {
            Self::Road => "riding_style_road",
            Self::Motard => "riding_style_motard",
            Self::Offroad => "riding_style_offroad",
        }
    }

    /// The environment variable of the class's riding style.
    pub const fn env(self) -> &'static str {
        match self {
            Self::Road => "NFSMW_RIDING_STYLE_ROAD",
            Self::Motard => "NFSMW_RIDING_STYLE_MOTARD",
            Self::Offroad => "NFSMW_RIDING_STYLE_OFFROAD",
        }
    }

    /// The title of the class's row in the current language.
    pub fn title(self) -> String {
        let id = match self {
            Self::Road => "options-riding-style-road",
            Self::Motard => "options-riding-style-motard",
            Self::Offroad => "options-riding-style-offroad",
        };
        crate::i18n::text(id, &crate::i18n::Args::new())
    }
}

impl super::Settings {
    /// The riding style the player chose for the bikes of `class`.
    pub const fn riding_style_of(&self, class: BikeClass) -> RidingStyle {
        match class {
            BikeClass::Road => self.riding_style_road,
            BikeClass::Motard => self.riding_style_motard,
            BikeClass::Offroad => self.riding_style_offroad,
        }
    }

    /// The riding style of `class`, to change.
    pub fn riding_style_mut(&mut self, class: BikeClass) -> &mut RidingStyle {
        match class {
            BikeClass::Road => &mut self.riding_style_road,
            BikeClass::Motard => &mut self.riding_style_motard,
            BikeClass::Offroad => &mut self.riding_style_offroad,
        }
    }
}

impl super::Partial {
    /// A change of the riding style of `class`, as it is saved.
    pub fn set_riding_style(&mut self, class: BikeClass, style: RidingStyle) {
        let slot = match class {
            BikeClass::Road => &mut self.riding_style_road,
            BikeClass::Motard => &mut self.riding_style_motard,
            BikeClass::Offroad => &mut self.riding_style_offroad,
        };
        *slot = Some(style);
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
    fn the_classes_have_their_own_keys_and_places() {
        let keys: Vec<&str> = BikeClass::ALL.iter().map(|class| class.key()).collect();
        assert_eq!(keys, ["riding_style_road", "riding_style_motard", "riding_style_offroad"]);
        assert_eq!(BikeClass::ALL.map(BikeClass::index), [0, 1, 2]);
        assert_eq!(BikeClass::Motard.env(), "NFSMW_RIDING_STYLE_MOTARD");
        assert_eq!(BikeClass::Offroad.title(), "Riding Style: Offroad");
    }

    #[test]
    fn a_class_reads_and_writes_its_own_style() {
        let mut settings = super::super::Settings::from(super::super::Partial::default());
        assert_eq!(BikeClass::ALL.map(|class| settings.riding_style_of(class)), [RidingStyle::Balanced; 3]);
        *settings.riding_style_mut(BikeClass::Motard) = RidingStyle::OldSchool;
        assert_eq!(
            BikeClass::ALL.map(|class| settings.riding_style_of(class)),
            [RidingStyle::Balanced, RidingStyle::OldSchool, RidingStyle::Balanced]
        );
        let mut changed = super::super::Partial::default();
        changed.set_riding_style(BikeClass::Offroad, RidingStyle::ShouldersOut);
        assert_eq!(changed.riding_style_offroad, Some(RidingStyle::ShouldersOut));
        assert_eq!((changed.riding_style_road, changed.riding_style_motard), (None, None));
    }

    #[test]
    fn labels_name_every_style() {
        assert_eq!(RidingStyle::Balanced.label(), "Balanced");
        assert_eq!(RidingStyle::ShouldersOut.label(), "Shoulders Out");
        assert_eq!(RidingStyle::OldSchool.label(), "Old School");
    }
}
