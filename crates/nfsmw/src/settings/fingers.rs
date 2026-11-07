//! How many fingers the rider pulls a bike's brake and clutch levers with (`brake_fingers`, `clutch_fingers`).
//!
//! RIDE 6's riding style page offers one, two or four fingers on the **brake** (its `BRAKE_FINGERS` option; two is the animation
//! blueprint's own default and the value of every riding style preset). The clutch has no option there: the left hand's clutch clip
//! pulls with the index and middle fingers, and the left hand of the other finger clips pulls with all four. `clutch_fingers` offers
//! those two (default: the clutch clip, two fingers). The rider's graph plays the finger clips of the numbers on the right and the
//! left hand (`docs/specs/ride6-rider-hands.md`).

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BrakeFingers {
    One,
    #[default]
    Two,
    Four,
}
names!(BrakeFingers, "1, 2 or 4", [(Self::One, "1"), (Self::Two, "2"), (Self::Four, "4")]);

impl BrakeFingers {
    pub const ALL: [Self; 3] = [Self::One, Self::Two, Self::Four];

    /// The number of fingers: the `BrakeFingers` of RIDE 6's animation blueprint.
    pub const fn count(self) -> u8 {
        match self {
            Self::One => 1,
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    /// The finger clip the rider's graph plays: the `brake_fingers` parameter of the graph (0 one finger, 1 two, 2 four).
    pub const fn graph_index(self) -> u8 {
        match self {
            Self::One => 0,
            Self::Two => 1,
            Self::Four => 2,
        }
    }

    /// The count as the options show it.
    pub fn label(self) -> String {
        self.count().to_string()
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ClutchFingers {
    #[default]
    Two,
    Four,
}
names!(ClutchFingers, "2 or 4", [(Self::Two, "2"), (Self::Four, "4")]);

impl ClutchFingers {
    pub const ALL: [Self; 2] = [Self::Two, Self::Four];

    /// The number of fingers.
    pub const fn count(self) -> u8 {
        match self {
            Self::Two => 2,
            Self::Four => 4,
        }
    }

    /// The finger clip the rider's graph plays: the `clutch_fingers` parameter of the graph (0 the clutch clip, 1 the four-finger clip's
    /// left hand).
    pub const fn graph_index(self) -> u8 {
        match self {
            Self::Two => 0,
            Self::Four => 1,
        }
    }

    /// The count as the options show it.
    pub fn label(self) -> String {
        self.count().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_counts_are_the_options_values_two_is_the_default_and_other_numbers_are_refused() {
        assert_eq!(BrakeFingers::default(), BrakeFingers::Two);
        assert_eq!(ClutchFingers::default(), ClutchFingers::Two);
        let counts: Vec<u8> = BrakeFingers::ALL.iter().map(|fingers| fingers.count()).collect();
        assert_eq!(counts, [1, 2, 4]);
        let counts: Vec<u8> = ClutchFingers::ALL.iter().map(|fingers| fingers.count()).collect();
        assert_eq!(counts, [2, 4]);
        for fingers in BrakeFingers::ALL {
            assert_eq!(fingers.to_string().parse::<BrakeFingers>(), Ok(fingers));
            assert_eq!(fingers.label(), fingers.count().to_string());
        }
        for fingers in ClutchFingers::ALL {
            assert_eq!(fingers.to_string().parse::<ClutchFingers>(), Ok(fingers));
        }
        for name in ["0", "3", "5", "two", ""] {
            assert!(name.parse::<BrakeFingers>().is_err(), "{name:?} must not parse");
        }
        for name in ["1", "3", "two", ""] {
            assert!(name.parse::<ClutchFingers>().is_err(), "{name:?} must not parse");
        }
    }

    #[test]
    fn the_graph_picks_its_clips_by_the_place_in_the_list() {
        let brake: Vec<u8> = BrakeFingers::ALL.iter().map(|fingers| fingers.graph_index()).collect();
        assert_eq!(brake, [0, 1, 2]);
        let clutch: Vec<u8> = ClutchFingers::ALL.iter().map(|fingers| fingers.graph_index()).collect();
        assert_eq!(clutch, [0, 1]);
    }
}
