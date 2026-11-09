//! The turn law of the arcade steering: how tight a full stick turns at each speed, as one documented table
//! (`docs/specs/ride6-bike-turn-radius.md`). **A modification made exclusively for the NFS:MW rewrite, not the behaviour of RIDE 6.**
//!
//! The cornering assist (`arcade_forces.rs`) is a side force at the centre of gravity. Under the law of the earlier rounds it was a fixed
//! multiple of the lean the rider asks for, so the circle a bike turned was whatever that, the lean limit and the tyres made of it, and it
//! changed from bike to bike. Under this law the assist is **the part of the lateral acceleration that the lean does not give**:
//!
//! ```text
//! wanted  = share x speed^2 / radius(speed)        the acceleration of the circle the table names, for the stick's share of its travel
//! expect  = g x tan(target lean)                   what the lean the controller aims at gives through the tyres
//! assist  = wanted - expect                        never against the stick, never more than `max_assist`, faded out below walking pace
//! ```
//!
//! In a steady turn the bike's lean is its target, so the total acceleration is `wanted` and the circle is the table's, whatever the bike's
//! mass, grip or lean limit. The table is the one tuning value (named constants, [`RADIUS`]); the other fields shape the transient:
//!
//! - at a low speed the share is the **stick's** (a hand's filter, 0.1 s), not the controller's target (which is the stick through three more
//!   slow filters, 0.6 s and more since the lean-in was slowed): a slow turn needs little lean, so the path must not wait for it
//!   (`quick`: the stick's share at and under the first speed, the target's at and over the second, linear between);
//! - the controller's own lean-in is made quicker at a low speed (`slow_lean_rate` on its target filter's time scale), for the same reason.

use super::curve::ramp;

/// The circle a full stick turns, against the speed: `(km/h, metres)`, joined by straight lines in `ln(speed)` and `ln(radius)` (a power law
/// between two points, so the radius never falls with the speed and has no step), and continued past the last point with the exponent of the
/// last segment. A project's choice: the playtest asked for "less steering radius when slow, more when fast" and gave the circle it wants at 15,
/// 30, 50, 80, 120 and 180 km/h (3 m or less, 6 m or less, about 15, about 40, about 90 and 220 m or more). Below 6 km/h the radius is that of
/// 6 km/h (the handlebar's own lock turns the bike there). The lateral acceleration `v^2 / r` that the table asks for is 0.8 g at 15 km/h, 1.6 g
/// at 30, 1.3 g at 50, 1.26 g at 80 and 120, 1.1 g at 180 km/h: never over the 1.65 g the earlier rounds turned at, so no bike's grip is asked for
/// more than it gave, and what the lean (0.7 g at the limit of 36 degrees) does not give is the assist's.
pub const RADIUS: &[(f32, f32)] =
    &[(6.0, 1.8), (15.0, 2.2), (30.0, 4.5), (50.0, 15.0), (80.0, 40.0), (120.0, 90.0), (180.0, 225.0)];

/// The numbers of the turn law; see the module's documentation.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TurnLaw {
    /// The circle of a full stick: `(km/h, m)` points, ascending in both, as [`RADIUS`].
    pub radius: &'static [(f32, f32)],
    /// The speeds (m/s) between which the assist comes in: none at or under the first, whole at or over the second.
    pub fade: [f32; 2],
    /// The speeds (m/s) between which the share the assist follows goes from the stick's (at or under the first) to the lean target's (at or
    /// over the second).
    pub quick: [f32; 2],
    /// The most the assist gives (g).
    pub max_assist: f32,
    /// The factor on the time scale of the lean controller's target filter at or under the first of `slow_lean_speed`; 1 at or over the second.
    pub slow_lean_rate: f32,
    /// The speeds (m/s) of the two ends of `slow_lean_rate`.
    pub slow_lean_speed: [f32; 2],
}

/// What the law reads of the bike each step: the stick's and the target lean's share of the lean limit (-1 to 1, positive right) and the
/// target lean itself (rad, positive right).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct TurnInput {
    pub target_share: f32,
    pub stick_share: f32,
    pub target_lean: f32,
}

impl TurnLaw {
    /// This game's values (`docs/specs/ride6-bike-turn-radius.md`).
    pub const GAME: Self = Self {
        radius: RADIUS,
        fade: [0.3, 1.5],
        quick: [4.0, 12.0],
        max_assist: 1.7,
        slow_lean_rate: 2.2,
        slow_lean_speed: [4.0, 12.0],
    };

    /// The radius (m) of the circle a full stick turns at `speed` m/s. Infinite for an empty table.
    pub fn radius(&self, speed: f32) -> f32 {
        let kmh = (speed.abs() * 3.6).max(1e-3);
        let table = self.radius;
        let (Some(&(first_speed, first_radius)), Some(&(last_speed, last_radius))) = (table.first(), table.last())
        else {
            return f32::INFINITY;
        };
        if kmh <= first_speed {
            return first_radius;
        }
        let Some(segment) = table.windows(2).find(|pair| kmh <= pair[1].0).or_else(|| table.windows(2).last()) else {
            return last_radius * (kmh / last_speed).powi(2);
        };
        let [(s0, r0), (s1, r1)] = [segment[0], segment[1]];
        let exponent = (r1 / r0).ln() / (s1 / s0).ln().max(1e-6);
        r0 * (kmh / s0).powf(exponent)
    }

    /// The acceleration (m/s^2) of the circle of a full stick at `speed` m/s: `speed^2 / radius`.
    pub fn full_acceleration(&self, speed: f32) -> f32 {
        speed * speed / self.radius(speed).max(1e-3)
    }

    /// The share of the stick's travel the assist follows at `speed` m/s: the stick's at a low speed, the lean target's at a high one.
    pub fn share(&self, input: &TurnInput, speed: f32) -> f32 {
        let quick = 1.0 - ramp(speed.abs(), self.quick);
        input.target_share + (input.stick_share - input.target_share) * quick
    }

    /// The side acceleration (m/s^2, positive right) the cornering assist gives a bike at `speed` m/s: the circle's acceleration for the share
    /// less what the target lean gives, `0` against the stick, held to `max_assist`, and faded in with the speed.
    pub fn acceleration(&self, input: &TurnInput, speed: f32) -> f32 {
        let fade = ramp(speed.abs(), self.fade);
        if fade <= 0.0 {
            return 0.0;
        }
        let wanted = self.share(input, speed) * self.full_acceleration(speed);
        let expected = 9.81 * input.target_lean.clamp(-1.2, 1.2).tan();
        let extra = wanted - expected;
        if extra * wanted <= 0.0 {
            return 0.0;
        }
        let cap = self.max_assist * 9.81;
        extra.clamp(-cap, cap) * fade
    }

    /// The factor on the lean controller's target-filter time scale at `speed` m/s: `slow_lean_rate` at a low speed, 1 at a high one.
    pub fn lean_rate(&self, speed: f32) -> f32 {
        1.0 + (self.slow_lean_rate - 1.0) * (1.0 - ramp(speed.abs(), self.slow_lean_speed))
    }
}

#[cfg(test)]
#[path = "turn_law_tests.rs"]
mod tests;
