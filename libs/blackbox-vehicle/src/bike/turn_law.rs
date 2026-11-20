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
/// last segment. A project's choice, in two playtest rounds. Round 3 ("less steering radius when slow, more when fast") gave the circle wanted at 15,
/// 30, 50, 80, 120 and 180 km/h (3 m or less, 6 m or less, about 15, about 40, about 90 and 220 m or more). Round 4 ("going faster than 100 kph turn
/// way more") made it about half as wide from 80 km/h up: about 27 m at 80, 30 at 100, 45 at 120, 70 at 150, 100 at 180 and 190 at 250 km/h, the slow
/// end kept. Below 6 km/h the radius is that of 6 km/h (the handlebar's own lock turns the bike there). The lateral acceleration `v^2 / r` the table
/// asks for is 0.8 g at 15 km/h, 1.6 g at 30, 1.4 g at 50, 1.9 g at 80, and 2.5 to 2.6 g from 100 km/h up: more than the 1.65 g the earlier rounds
/// turned at, so the assist is allowed [`MAX_ASSIST_G`] and the lean and fall limits are checked at it (`docs/specs/ride6-bike-turn-radius.md`). What the
/// lean (0.7 g at the limit of 36 degrees) does not give is the assist's.
pub const RADIUS: &[(f32, f32)] = &[
    (6.0, 1.8),
    (15.0, 1.9),
    (30.0, 4.4),
    (50.0, 14.0),
    (80.0, 27.0),
    (100.0, 30.0),
    (120.0, 45.0),
    (150.0, 70.0),
    (180.0, 100.0),
    (250.0, 190.0),
];

/// The most the turn law's assist gives at speed (g), `TurnLaw::max_assist`. Round 3 held it to 1.7 g, the most that the rounds before it gave. The
/// circles of round 4 ask for 2.5 g and more above 100 km/h, of which the lean gives 0.7 g, so the assist must give 1.9 g in a steady turn; 2.4 g leaves
/// 0.5 g for the stick's first instant, for the trim and for the flip of the stick, where `wanted - expect` is more still (the assist is capped there).
pub const MAX_ASSIST_G: f32 = 2.4;
/// The most the assist gives at a low speed (g), `TurnLaw::slow_max_assist`: round 3's 1.7 g. The table asks for no more than 1.14 g of assist up to
/// 80 km/h, and a cap of 2.4 g there made 3 more of the 293 bikes fall in a flip of the stick at 15 km/h (9 against 6), so the cap rises with the speed
/// from [`MAX_ASSIST_SPEED`] only.
pub const SLOW_MAX_ASSIST_G: f32 = 1.7;
/// The speeds (m/s, 72 and 94 km/h) between which the cap goes from [`SLOW_MAX_ASSIST_G`] to [`MAX_ASSIST_G`].
pub const MAX_ASSIST_SPEED: [f32; 2] = [20.0, 26.0];

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
    /// The most the assist gives (g) at or over the second of `max_assist_speed`.
    pub max_assist: f32,
    /// The most the assist gives (g) at or under the first of `max_assist_speed`.
    pub slow_max_assist: f32,
    /// The speeds (m/s) between which the most the assist gives goes from `slow_max_assist` to `max_assist`.
    pub max_assist_speed: [f32; 2],
    /// The factor on the time scale of the lean controller's target filter at or under the first of `slow_lean_speed`; 1 at or over the second.
    pub slow_lean_rate: f32,
    /// The speeds (m/s) of the two ends of `slow_lean_rate`.
    pub slow_lean_speed: [f32; 2],
    /// The rate (1/s) at which the trim (below) follows the shortfall of the turn: the bike's tyres resist a side force at the centre of gravity
    /// (the heading lags the velocity until they have turned the bike), so the circle comes out wider than the table by a margin that depends on
    /// the bike and the speed. The trim is an integral term that adds to the assist whatever the turn lacks of the circle's acceleration.
    pub trim_gain: f32,
    /// The most (m/s^2) the shortfall counts as in one step of the trim, so a flip of the stick (a shortfall of 2 g for a moment) does not wind it up.
    pub trim_reach: f32,
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
        max_assist: MAX_ASSIST_G,
        slow_max_assist: SLOW_MAX_ASSIST_G,
        max_assist_speed: MAX_ASSIST_SPEED,
        slow_lean_rate: 2.2,
        slow_lean_speed: [4.0, 12.0],
        trim_gain: 3.0,
        trim_reach: 4.0,
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

    /// The most (m/s^2) the assist gives at `speed` m/s: `slow_max_assist` at a low speed, `max_assist` at a high one.
    pub fn assist_cap(&self, speed: f32) -> f32 {
        let along = ramp(speed.abs(), self.max_assist_speed);
        (self.slow_max_assist + (self.max_assist - self.slow_max_assist) * along) * 9.81
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

    /// The acceleration (m/s^2, positive right) of the circle the stick asks for at `speed` m/s: the share of the full stick's.
    pub fn wanted(&self, input: &TurnInput, speed: f32) -> f32 {
        self.share(input, speed) * self.full_acceleration(speed)
    }

    /// The trim's next value (m/s^2, along the stick; added to the assist's magnitude): it follows the shortfall of the measured turn
    /// (`measured`, the speed times the yaw rate, positive right) from `scale` times the circle's acceleration. `scale` is the surface's share (1
    /// on a good road), so on loose ground the trim does not make up what the surface takes away. It dies away with no stick or no assist.
    pub fn trim_step(&self, trim: f32, input: &TurnInput, speed: f32, measured: f32, scale: f32, dt: f32) -> f32 {
        let wanted = self.wanted(input, speed) * scale;
        if ramp(speed.abs(), self.fade) <= 0.0 {
            return 0.0;
        }
        if wanted.abs() < 0.2 {
            return trim * (-dt / 0.25).exp();
        }
        let short = ((wanted - measured) * wanted.signum()).clamp(-self.trim_reach, self.trim_reach);
        let assist = self.acceleration(input, speed).abs();
        let cap = self.assist_cap(speed);
        (trim + self.trim_gain * short * dt).clamp(-assist, (cap - assist).max(0.0))
    }

    /// The assist with the trim (m/s^2, positive right): the assist's magnitude plus the trim, towards the stick, held to `max_assist`.
    pub fn trimmed(&self, input: &TurnInput, speed: f32, trim: f32) -> f32 {
        let wanted = self.wanted(input, speed);
        let fade = ramp(speed.abs(), self.fade);
        if wanted == 0.0 || fade <= 0.0 {
            return 0.0;
        }
        let total = (self.acceleration(input, speed).abs() + trim * fade).clamp(0.0, self.assist_cap(speed));
        total * wanted.signum()
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
        let cap = self.assist_cap(speed);
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
