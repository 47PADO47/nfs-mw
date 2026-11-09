//! The arcade help of a bike on loose and rough ground (`docs/specs/ride6-bike-rough-ground.md`): **a modification made exclusively for the
//! NFS:MW rewrite, not the behaviour of the reference game.** Part of [`super::ArcadeSteering`]; [`ArcadeRough::REFERENCE`] is neutral in every
//! field.
//!
//! The ground gives each tyre a grip multiplier (the host's surface table: dirt, gravel, grass and sand are under 1). The reference game's bike
//! multiplies the tyre's force by it and nothing else knows: the lean controller leans for tarmac and the arcade steering's cornering assist
//! still pulls at 1.6 g, so on dirt the tyres cannot hold what the controller asks and the bike goes down at once. This help makes the bike
//! know the ground, and gives the bikes made for it more of it:
//!
//! 1. **it follows the surface** (`follow_surface`): the lean controller plans with the front tyre's grip times the surface's, and the cornering
//!    assist is scaled by it, so a bike turns wider on dirt and slides before it falls (a progressive breakaway, `surface_rate` smooths the change
//!    when the ground changes);
//! 2. **the penalty is smaller for the offroad bikes** (`grip_relief`): the share of a surface's grip loss that is taken away, per class;
//! 3. **the suspension damps more on loose ground** (`damping`), so a bump is not thrown back at the rider;
//! 4. **the fall tests forgive loose ground** (`tolerance`, `lean_hold`): wider limits on the angular speed, the lean, the pitch and the heading
//!    angle, and a lean must be held `lean_hold` seconds beyond its limit to count.
//!
//! The tables are indexed by [`crate::bike::TerrainClass::index`]: road, motard, offroad.

use crate::bike::curve::ramp;

/// The values of the help on rough ground; see the module's documentation. `REFERENCE` is neutral.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcadeRough {
    /// The lean controller and the cornering assist follow the surface's lateral grip.
    pub follow_surface: bool,
    /// How fast (per second) what the bike believes about the surface follows the surface it is on.
    pub surface_rate: f32,
    /// The surface's lateral multiplier at which it starts to count as loose (looseness 0) and at which it is wholly loose (looseness 1).
    pub loose_range: [f32; 2],
    /// The share (0 to 1) of a surface's grip penalty (1 less its multiplier, lateral and driving) that is taken away, per class.
    pub grip_relief: [f32; 3],
    /// The factor on the suspension's damping on wholly loose ground, per class.
    pub damping: [f32; 3],
    /// The factor on the fall tests' limits on wholly loose ground, per class.
    pub tolerance: [f32; 3],
    /// The seconds a lean must stay beyond its fall limit to count, on wholly loose ground, per class.
    pub lean_hold: [f32; 3],
}

impl ArcadeRough {
    /// Everything neutral: the reference game's bike on any ground.
    pub const REFERENCE: Self = Self {
        follow_surface: false,
        surface_rate: 5.0,
        loose_range: [0.95, 0.75],
        grip_relief: [0.0; 3],
        damping: [1.0; 3],
        tolerance: [1.0; 3],
        lean_hold: [0.0; 3],
    };

    /// How loose a surface with the lateral multiplier `lateral` is, 0 to 1.
    pub fn looseness(&self, lateral: f32) -> f32 {
        ramp(lateral, self.loose_range)
    }

    /// The grip of a surface for a bike of class `class`: the multiplier `grip` with `relief` of its penalty taken away. Never below the
    /// surface's own, never above 1 when the surface is under it (a surface that gives more than the road is left alone).
    pub fn relieved(&self, class: usize, grip: f32) -> f32 {
        let relief = self.grip_relief[class];
        if grip >= 1.0 || relief <= 0.0 {
            return grip;
        }
        1.0 - (1.0 - grip) * (1.0 - relief)
    }

    /// The factor on the suspension's damping at `looseness` for class `class`.
    pub fn damping_factor(&self, class: usize, looseness: f32) -> f32 {
        1.0 + (self.damping[class] - 1.0) * looseness
    }

    /// The factor on the fall tests' limits at `looseness` for class `class`.
    pub fn tolerance_factor(&self, class: usize, looseness: f32) -> f32 {
        1.0 + (self.tolerance[class] - 1.0) * looseness
    }

    /// The seconds a lean must be held beyond its limit at `looseness` for class `class`.
    pub fn hold(&self, class: usize, looseness: f32) -> f32 {
        self.lean_hold[class] * looseness
    }
}

impl Default for ArcadeRough {
    /// This game's values (`docs/specs/ride6-bike-rough-ground.md`, section 4).
    fn default() -> Self {
        Self {
            follow_surface: true,
            surface_rate: 5.0,
            loose_range: [0.95, 0.75],
            grip_relief: [0.0, 0.30, 0.60],
            damping: [1.0, 1.15, 1.40],
            tolerance: [1.0, 1.30, 1.80],
            lean_hold: [0.0, 0.15, 0.30],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_is_neutral() {
        let reference = ArcadeRough::REFERENCE;
        for class in 0..3 {
            assert_eq!(reference.relieved(class, 0.6), 0.6);
            assert_eq!(reference.damping_factor(class, 1.0), 1.0);
            assert_eq!(reference.tolerance_factor(class, 1.0), 1.0);
            assert_eq!(reference.hold(class, 1.0), 0.0);
        }
    }

    #[test]
    fn the_relief_takes_a_share_of_the_penalty_and_leaves_a_good_surface_alone() {
        let game = ArcadeRough::default();
        assert_eq!(game.relieved(0, 0.6), 0.6, "a road bike gets the surface as it is");
        assert!((game.relieved(2, 0.6) - 0.84).abs() < 1e-6, "an offroad bike loses 0.4 x 0.4");
        assert!(game.relieved(1, 0.6) > game.relieved(0, 0.6) && game.relieved(2, 0.6) > game.relieved(1, 0.6));
        assert_eq!(game.relieved(2, 1.0), 1.0);
        assert_eq!(game.relieved(2, 1.2), 1.2);
        assert!(game.relieved(2, 0.0) > 0.0 && game.relieved(2, 0.0) < 1.0);
    }

    #[test]
    fn looseness_runs_from_tarmac_to_loose_gravel() {
        let game = ArcadeRough::default();
        assert_eq!(game.looseness(1.0), 0.0);
        assert_eq!(game.looseness(0.95), 0.0);
        assert_eq!(game.looseness(0.75), 1.0);
        assert_eq!(game.looseness(0.4), 1.0);
        assert!((game.looseness(0.85) - 0.5).abs() < 1e-5);
    }

    #[test]
    fn the_offroad_class_gets_the_most_of_every_help_on_loose_ground_and_none_on_tarmac() {
        let game = ArcadeRough::default();
        for class in 0..3 {
            assert_eq!(game.damping_factor(class, 0.0), 1.0);
            assert_eq!(game.tolerance_factor(class, 0.0), 1.0);
            assert_eq!(game.hold(class, 0.0), 0.0);
        }
        assert!(game.damping_factor(2, 1.0) > game.damping_factor(1, 1.0));
        assert!(game.tolerance_factor(2, 1.0) > game.tolerance_factor(0, 1.0));
        assert!(game.hold(2, 1.0) > game.hold(1, 1.0));
    }
}
