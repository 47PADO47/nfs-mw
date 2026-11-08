//! The bike steering setting (`bike_steering`): how a bike turns. `arcade` is this game's modification (a quick lean from side to side
//! at every speed and a steady turn of about 1.6 g, `docs/specs/ride6-arcade-steering.md`), `reference` is the RIDE 6 behaviour the bikes'
//! data describes. Read when a bike is loaded: the arcade steering is part of the bike's tyres and controller
//! (`blackbox_vehicle::bike::ArcadeSteering`), so a change reaches the bike that is loaded next.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BikeSteering {
    /// Quick lean and a wide grip: bikes keep to the road's bends. This game's own tuning, not the reference's.
    #[default]
    Arcade,
    /// The reference game's steering, as the bikes' data states it.
    Reference,
}
names!(BikeSteering, "arcade or reference", [(Self::Arcade, "arcade"), (Self::Reference, "reference")]);

impl BikeSteering {
    /// The library's steering values for this setting (`ArcadeSteering::default()` or `ArcadeSteering::REFERENCE`).
    pub fn steering(self) -> blackbox_vehicle::bike::ArcadeSteering {
        match self {
            Self::Arcade => blackbox_vehicle::bike::ArcadeSteering::default(),
            Self::Reference => blackbox_vehicle::bike::ArcadeSteering::REFERENCE,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_steering_is_arcade_by_default_round_trips_and_rejects_other_names() {
        assert_eq!(BikeSteering::default(), BikeSteering::Arcade);
        for value in [BikeSteering::Arcade, BikeSteering::Reference] {
            assert_eq!(value.to_string().parse::<BikeSteering>(), Ok(value));
        }
        for name in ["Arcade", "real", "", "0"] {
            assert!(name.parse::<BikeSteering>().is_err(), "{name:?} must not parse");
        }
    }

    #[test]
    fn reference_switches_every_lever_off_and_arcade_is_the_games_tuning() {
        assert!(!BikeSteering::Reference.steering().is_enabled());
        assert!(BikeSteering::Arcade.steering().is_enabled());
    }
}
