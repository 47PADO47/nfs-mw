//! The arcade help of a bike that has left the ground and lands again (`docs/specs/ride6-bike-jumps.md`): **a modification made exclusively
//! for the NFS:MW rewrite, not the behaviour of the reference game.** Part of [`super::ArcadeSteering`]; [`ArcadeAir::REFERENCE`] is neutral in
//! every field and gives the reference behaviour back exactly.
//!
//! A bike that jumps lands on its wheels, as in the open-world games of the genre, because of four things, each bounded:
//!
//! 1. **it levels in the air**: a torque (`level_*`) turns the bike's up towards the normal of the road below it, pitch and roll only (the
//!    yaw is the rider's), from `level_delay` seconds after both wheels left the road;
//! 2. **the bars centre in the air** (`handlebar_centre`): the lean controller, with no tyre to work through, would turn them to the lock, and the
//!    front wheel would land across the road;
//! 3. **the landing is soft**: for `landing_window` seconds after a landing the body's speed into the road is eaten (`landing_*`: an extra
//!    deceleration along the road's normal, proportional to the speed above `landing_soft_speed`, never above `landing_accel`), and the load
//!    of a tyre is held under `load_cap_g` times the bike's weight by a smooth curve (what the suspension and the tyre cannot take is not
//!    turned into an impulse);
//! 4. **the fall tests forgive the landing** (`landing_tolerance` for `landing_grace` seconds): the limits of the angular speed, of the lean and
//!    pitch and of the angle between the velocity and the heading are wider.
//!
//! Every strength is multiplied by the bike's class (`class_factor`: road, motard, offroad, from [`crate::bike::BikeSpec::terrain_class`]).

use crate::bike::class::TerrainClass;

/// The values of the help on jumps; see the module's documentation. `REFERENCE` is neutral.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcadeAir {
    /// The largest angular acceleration (rad/s^2) of the levelling torque about the pitch and the roll axes. 0: no levelling.
    pub level_accel: f32,
    /// How hard the levelling pulls a bike off the road's normal (rad/s^2 per rad of angle).
    pub level_gain: f32,
    /// How hard it damps the pitch and roll rates (per second).
    pub level_damping: f32,
    /// The seconds in the air over which the levelling comes in (none before the first, whole after the second): a bike keeps the attitude of
    /// its take-off for a moment, and a hop over a kerb is not levelled.
    pub level_delay: [f32; 2],
    /// The share (0 to 1) of the controller's handlebar that is taken away in the air: 1 centres the bars. 0: the controller's.
    pub handlebar_centre: f32,
    /// How much of the throttle the rider closes (0 to 1) in the air and, while a landing is forgiven, as the nose comes up: a bike that lands with
    /// the throttle open loops out in a wheelie (measured). 0: the rider's throttle as it is.
    pub landing_throttle_cut: f32,
    /// The least seconds in the air that make a jump (a shorter hop is not one: it gets no landing help).
    pub min_air_time: f32,
    /// The seconds after a landing in which the fall tests are wider, and the factor on their limits. 1: the data's limits.
    pub landing_grace: f32,
    pub landing_tolerance: f32,
    /// The most deceleration (m/s^2) the landing assist adds along the road's normal, the gain on the speed into the road above
    /// `landing_soft_speed` (per second), that speed (m/s) and the seconds after a landing the assist lasts. 0: no assist.
    pub landing_accel: f32,
    pub landing_gain: f32,
    pub landing_soft_speed: f32,
    pub landing_window: f32,
    /// The load on a tyre (as a multiple of the bike's weight) that the smooth cap approaches. 0: no cap.
    pub load_cap_g: f32,
    /// The factor of each strength above for a road bike, a motard and an offroad bike (index [`TerrainClass::index`]).
    pub class_factor: [f32; 3],
}

impl ArcadeAir {
    /// Everything neutral: the reference game's jumps.
    pub const REFERENCE: Self = Self {
        level_accel: 0.0,
        level_gain: 0.0,
        level_damping: 0.0,
        level_delay: [0.1, 0.35],
        handlebar_centre: 0.0,
        landing_throttle_cut: 0.0,
        min_air_time: 0.25,
        landing_grace: 0.0,
        landing_tolerance: 1.0,
        landing_accel: 0.0,
        landing_gain: 0.0,
        landing_soft_speed: 2.0,
        landing_window: 0.5,
        load_cap_g: 0.0,
        class_factor: [1.0; 3],
    };

    /// The factor of the bike's class.
    pub fn factor(&self, class: TerrainClass) -> f32 {
        self.class_factor[class.index()]
    }

    /// The factor on the fall tests' limits while a landing is forgiven: `landing_tolerance` scaled by the class (1 + (tolerance - 1) x
    /// factor).
    pub fn tolerance(&self, class: TerrainClass) -> f32 {
        1.0 + (self.landing_tolerance - 1.0) * self.factor(class)
    }
}

impl Default for ArcadeAir {
    /// This game's values (`docs/specs/ride6-bike-jumps.md`, section 4). Chosen from the rider's own limits: the lean and pitch fall limits
    /// are 70 and 65 degrees and the angular-speed limit 4.4 rad/s (times the arcade steering's 3), so a landing that rotates the bike by
    /// 90 degrees a second is a landing, not a tumble.
    fn default() -> Self {
        Self {
            level_accel: 10.0,
            level_gain: 12.0,
            level_damping: 4.5,
            level_delay: [0.0, 0.1],
            handlebar_centre: 1.0,
            landing_throttle_cut: 1.0,
            min_air_time: 0.25,
            landing_grace: 0.9,
            landing_tolerance: 1.8,
            landing_accel: 110.0,
            landing_gain: 25.0,
            landing_soft_speed: 2.0,
            landing_window: 0.5,
            load_cap_g: 14.0,
            class_factor: [1.0, 1.25, 1.5],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_changes_nothing_and_the_game_helps_more_the_more_a_bike_is_made_for_it() {
        let reference = ArcadeAir::REFERENCE;
        for class in [TerrainClass::Road, TerrainClass::Motard, TerrainClass::Offroad] {
            assert_eq!(reference.tolerance(class), 1.0);
        }
        let game = ArcadeAir::default();
        assert!(game.tolerance(TerrainClass::Road) > 1.0);
        assert!(game.tolerance(TerrainClass::Motard) > game.tolerance(TerrainClass::Road));
        assert!(game.tolerance(TerrainClass::Offroad) > game.tolerance(TerrainClass::Motard));
        assert!(game.factor(TerrainClass::Offroad) > game.factor(TerrainClass::Road));
    }
}
