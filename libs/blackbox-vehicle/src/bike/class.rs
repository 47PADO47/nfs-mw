//! The class of a bike as its own data tells it (`docs/specs/ride6-bike-jumps.md`, section 2): road, motard or offroad.
//!
//! The imported bikes carry no class in their physics (the kind and the input handler are the rider's data), but the two things that
//! make a bike a jumper and a dirt bike are in it: the travel of the fork and the size of the front wheel. Over the 293 imported bikes
//! the fork travel splits them cleanly: every road bike has 0.10 to 0.16 m of it (scooters and the Rivale, 0.15), every bike made to
//! leave the road has 0.18 m or more (adventure bikes 0.18 to 0.27, supermotos 0.22 to 0.285). The front wheel then tells the two kinds
//! of long-travel bike apart: the adventure and enduro bikes have a 19 to 21 inch front wheel (outer radius 0.327 to 0.349 m), every
//! supermoto a 17 inch one (0.288 to 0.300 m).

use super::BikeSpec;

/// The fork travel (m) from which a bike counts as made for rough ground (the data's gap lies between 0.16 and 0.18 m).
pub const LONG_TRAVEL: f32 = 0.18;
/// The front wheel's outer radius (m) from which a long-travel bike counts as offroad rather than a motard (the data's gap lies between
/// 0.300 and 0.327 m).
pub const BIG_FRONT_WHEEL: f32 = 0.32;

/// The rear wheel's outer radius (m) under which a bike counts as a scooter. Over the 293 imported bikes the scooters (the Italjet Dragster 200 and
/// 700, 0.249 and 0.286 m, and the Honda X-ADV, 0.286 m) have a rear wheel under 0.29 m, and every other bike one of 0.306 m or more: the gap is clean.
pub const SCOOTER_REAR_WHEEL: f32 = 0.30;

/// A bike's class for the arcade help on jumps and on rough ground.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TerrainClass {
    /// Short-travel bikes: superbikes, nakeds, classics, cruisers, scooters.
    Road,
    /// Long travel on 17 inch wheels: the supermotos.
    Motard,
    /// Long travel on a 19 inch or bigger front wheel: adventure, enduro and trial bikes.
    Offroad,
}

impl TerrainClass {
    /// The class's place in the tables of the arcade steering (`[road, motard, offroad]`).
    pub const fn index(self) -> usize {
        match self {
            Self::Road => 0,
            Self::Motard => 1,
            Self::Offroad => 2,
        }
    }
}

impl BikeSpec {
    /// Whether the bike is a scooter, by its small rear wheel ([`SCOOTER_REAR_WHEEL`]).
    pub fn is_scooter(&self) -> bool {
        self.rear.radius < SCOOTER_REAR_WHEEL
    }

    /// The class of the bike from its fork travel and its front wheel.
    pub fn terrain_class(&self) -> TerrainClass {
        if self.front_suspension.max_travel < LONG_TRAVEL {
            return TerrainClass::Road;
        }
        match self.front.radius >= BIG_FRONT_WHEEL {
            true => TerrainClass::Offroad,
            false => TerrainClass::Motard,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bike(travel: f32, radius: f32) -> BikeSpec {
        let mut spec = BikeSpec::example();
        spec.front_suspension.max_travel = travel;
        spec.front.radius = radius;
        spec
    }

    #[test]
    fn the_class_follows_the_fork_travel_and_the_front_wheel() {
        assert_eq!(bike(0.12, 0.300).terrain_class(), TerrainClass::Road);
        assert_eq!(bike(0.153, 0.300).terrain_class(), TerrainClass::Road, "a scooter's travel is not a dirt bike's");
        assert_eq!(bike(0.150, 0.349).terrain_class(), TerrainClass::Road, "a big wheel on a short fork is a classic");
        assert_eq!(bike(0.285, 0.288).terrain_class(), TerrainClass::Motard, "the CRF 450 R SMT");
        assert_eq!(bike(0.224, 0.300).terrain_class(), TerrainClass::Motard, "the KTM 690 SMC R");
        assert_eq!(bike(0.270, 0.349).terrain_class(), TerrainClass::Offroad, "the KTM 890 Adventure R");
        assert_eq!(bike(0.180, 0.349).terrain_class(), TerrainClass::Offroad, "the Tiger 900 Rally Pro");
        assert_eq!(bike(0.200, 0.327).terrain_class(), TerrainClass::Offroad, "the Multistrada V4 Rally");
    }

    #[test]
    fn a_small_rear_wheel_makes_a_scooter() {
        let mut spec = BikeSpec::example();
        spec.rear.radius = 0.286;
        assert!(spec.is_scooter());
        spec.rear.radius = 0.306;
        assert!(!spec.is_scooter());
    }

    #[test]
    fn the_indices_follow_the_order_of_the_tables() {
        assert_eq!(
            [TerrainClass::Road.index(), TerrainClass::Motard.index(), TerrainClass::Offroad.index()],
            [0, 1, 2]
        );
    }
}
