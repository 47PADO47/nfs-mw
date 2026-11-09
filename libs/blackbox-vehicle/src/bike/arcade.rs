//! Arcade steering: **a modification made exclusively for the NFS:MW rewrite, not the behaviour of the reference game (RIDE 6).**
//!
//! The reference's bikes turn like real ones: the lean comes in over a second or more, slower with the speed, and the radius of a steady
//! turn is `v^2 / (g tan(lean))` of a bike that cannot lean past its tyre's grip, which is 1.0 g and a radius of 40 m at 20 m/s, 300 m
//! at 60 m/s. On the roads of a street racing game that is "the bike only leans and goes on into the wall"
//! (`docs/specs/ride6-arcade-steering.md`). Every number that changes it is in [`ArcadeSteering`], and nowhere else:
//!
//! - [`ArcadeSteering::REFERENCE`] (also [`ArcadeSteering::disabled`]) is neutral in every field: a bike with it behaves exactly as the
//!   data of the reference game says (the data's curves, the data's tyre, no extra force). **A game that wants the original behaviour
//!   sets `spec.steering.arcade = ArcadeSteering::REFERENCE`** (the NFS:MW host does it for the setting `bike_steering = reference`).
//! - [`ArcadeSteering::default`] is this game's tuned values. They are a project's choices, tagged as such in the spec.
//!
//! The levers, from the most physical to the least:
//!
//! 1. `lateral_grip`: the tyres' lateral force is scaled in the tyre model itself (`Bike::new`), so the lean limit (the arctangent of the
//!    grip), the fall limit, the lean help's cap and the tyre curve that turns a wanted force into a slip all follow it.
//! 2. `lean_rate`, `time_scale_speed_floor`, `mirror_stick_curve`, `roll_spring`: how fast the rider's controller brings the lean to its
//!    target (the same controller, quicker filters, no slowing with the speed).
//! 3. `lean_help`, `lean_help_limit`: the reference's lean help (a side force at the front contact towards the one the controller wants)
//!    closes more of the gap and may give more, so a roll that the front tyre alone cannot deliver in time (it hops, its handlebar lags) is.
//! 4. `cornering_assist`: a side force at the centre of gravity (no roll torque) towards the lean, which is what a higher grip gives a
//!    bike that cannot lean further.
//! 5. `stick_reach`: the share of the pad stick's travel that reads as the whole stick (the host's input layer).
//! 6. `authority`, `authority_speed`: the cornering assist's factor over the speed, high when the bike is slow (a tighter circle) and low
//!    when it is fast (a wider one), a smooth curve with no step (`docs/specs/ride6-arcade-steering.md`, section 11).
//! 7. `wheelie_steer_drop`: a floor under a steered wheelie, so the stick lowers the front wheel only a little (section 12).
//! 8. `turn`: the turn law (`turn_law.rs`, `docs/specs/ride6-bike-turn-radius.md`): the cornering assist as the part of the circle's
//!    acceleration that the lean does not give, with the circle a full stick turns named at each speed by one table (tight when slow, wide when
//!    fast), a path that follows the stick at a low speed, and a quicker lean-in there. While `turn` is set it replaces the assist of
//!    `cornering_assist` and `authority` (levers 4 and 6), which stay for a steering that has none.
//! 9. `air`: the help of a bike that jumps (`arcade_air.rs`); 10. `rough`: the help of a bike on loose ground (`arcade_rough.rs`).

use super::turn_law::{TurnInput, TurnLaw};

#[path = "arcade_air.rs"]
mod air;
#[path = "arcade_rough.rs"]
mod rough;

pub use air::ArcadeAir;
pub use rough::ArcadeRough;

/// The values of the arcade steering; see the module's documentation. `REFERENCE` is neutral.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ArcadeSteering {
    /// The factor on the lateral force of both tyres (the peak, the stiffness and the camber thrust). 1: the data's tyre.
    pub lateral_grip: f32,
    /// The factor on the lean limit the stick can ask for (the arctangent of the front tyre's grip, held to the steepest lean). A bike that
    /// leans less has less far to flip from side to side, and `cornering_assist` supplies what the shallower lean does not. 1: the data's.
    pub lean_limit: f32,
    /// The factor on the time scale of the lean controller's target filter (how fast the rider brings the lean in). 1: the data's.
    pub lean_rate: f32,
    /// The least value of the data's speed curve of that time scale (`LeanTimeScaleSpeedTuning`: the data's slows the lean down to a third
    /// at 330 km/h). 0: the data's curve unchanged; 1: no slowing with the speed at all.
    pub time_scale_speed_floor: f32,
    /// Read the data's curve of the time scale over the stick for both sides as its quicker one, so a left stick leans as fast as a right
    /// one (the data's is slower to the left). false: the signed curve as the data states it.
    pub mirror_stick_curve: bool,
    /// The factor on the frequency of the roll spring-damper that pulls the lean to its target (the damping ratio is kept). 1: the data's.
    pub roll_spring: f32,
    /// The factor on the frequency of the filter on the stick (the rider's hand). 1: the data's.
    pub stick_response: f32,
    /// The factor on the frequency of the handlebar's filter (how fast the bars follow the controller). 1: the data's.
    pub handlebar_response: f32,
    /// The factor on the share of the gap between the front tyre's side force and the wanted one that the lean help closes (the data's
    /// 0.16 above 10 km/h; the share is held to 1). 1: the data's.
    pub lean_help: f32,
    /// The factor on the most force the lean help may give (the data's `LeanHelpLimit` of the tyre's peak at the rest load). 1: the data's.
    pub lean_help_limit: f32,
    /// The factor on the fall test's limit of the body's angular speed (`FallSpec::angular_speed`): a quick flip of the bike from lean to
    /// lean is the rider's own motion, not a tumble. 1: the data's limit.
    pub fall_angular_speed: f32,
    /// The side acceleration (in g) added at full lean, towards the lean, as a force at the centre of gravity. 0: none.
    pub cornering_assist: f32,
    /// The speeds (m/s) between which the cornering assist comes in (none below the first).
    pub assist_speed: [f32; 2],
    /// The share of the pad stick's travel that reads as the whole stick (the host's input layer). 1: the stick as it is.
    pub stick_reach: f32,
    /// The factor on the cornering assist at the slow and at the fast end of `authority_speed`: above 1 at the slow end the bike turns a tighter
    /// circle, below 1 at the fast end a wider one. Between the ends the factor follows a smooth curve over the logarithm of the speed
    /// ([`ArcadeSteering::authority_factor`]). `[1, 1]`: the assist as it is at every speed.
    pub authority: [f32; 2],
    /// The speeds (m/s) of the two ends of `authority`: the factor is the slow end's at or under the first and the fast end's at or over the second.
    pub authority_speed: [f32; 2],
    /// The most (rad) the stick lowers the front wheel of a wheelie: while the bike leans in a wheelie the pitch assist lifts the nose whenever it is
    /// under the pitch the wheelie had before the lean, less this (the data's assist fades out with the lean). `None`: the data's assist only.
    pub wheelie_steer_drop: Option<f32>,
    /// The turn law: the circle of a full stick at each speed, and what shapes the path's response ([`TurnLaw`]). While it is set it replaces the
    /// assist of `cornering_assist`, `assist_speed` and `authority`. `None`: the assist those fields give.
    pub turn: Option<TurnLaw>,
    /// The help on jumps: levelling in the air, the bars centred, a soft landing and fall tests that forgive it
    /// (`docs/specs/ride6-bike-jumps.md`). [`ArcadeAir::REFERENCE`]: none.
    pub air: ArcadeAir,
    /// The help on loose and rough ground: the lean and the assist follow the surface, a smaller penalty for the offroad bikes, more damping and
    /// wider fall tests on loose ground (`docs/specs/ride6-bike-rough-ground.md`). [`ArcadeRough::REFERENCE`]: none.
    pub rough: ArcadeRough,
}

impl ArcadeSteering {
    /// Everything neutral: the reference game's steering.
    pub const REFERENCE: Self = Self {
        lateral_grip: 1.0,
        lean_limit: 1.0,
        lean_rate: 1.0,
        time_scale_speed_floor: 0.0,
        mirror_stick_curve: false,
        roll_spring: 1.0,
        stick_response: 1.0,
        handlebar_response: 1.0,
        lean_help: 1.0,
        lean_help_limit: 1.0,
        fall_angular_speed: 1.0,
        cornering_assist: 0.0,
        assist_speed: [2.0, 8.0],
        stick_reach: 1.0,
        authority: [1.0, 1.0],
        authority_speed: [5.0, 70.0],
        wheelie_steer_drop: None,
        turn: None,
        air: ArcadeAir::REFERENCE,
        rough: ArcadeRough::REFERENCE,
    };

    /// The reference game's steering ([`ArcadeSteering::REFERENCE`]).
    pub const fn disabled() -> Self {
        Self::REFERENCE
    }

    /// Whether any lever is on.
    pub fn is_enabled(&self) -> bool {
        *self != Self::REFERENCE
    }

    /// The side acceleration (m/s^2, positive to the right) the cornering assist adds for a bike leaning `lean_share` of its limit
    /// (-1 to 1, positive to the right) at `speed` m/s.
    pub fn cornering_acceleration(&self, lean_share: f32, speed: f32) -> f32 {
        let fade = super::curve::ramp(speed.abs(), self.assist_speed);
        self.cornering_assist * 9.81 * lean_share.clamp(-1.0, 1.0) * fade * self.authority_factor(speed)
    }

    /// The side acceleration (m/s^2, positive to the right) of the cornering assist now: the turn law's when `turn` is set (`input` is what it reads),
    /// otherwise [`ArcadeSteering::cornering_acceleration`] for `lean_share`.
    pub fn assist_acceleration(&self, lean_share: f32, input: &TurnInput, speed: f32) -> f32 {
        match &self.turn {
            Some(law) => law.acceleration(input, speed),
            None => self.cornering_acceleration(lean_share, speed),
        }
    }

    /// The factor on the lean controller's target filter's time scale at `speed` m/s: `lean_rate`, times the turn law's quicker lean-in at a low
    /// speed when `turn` is set.
    pub fn lean_rate_at(&self, speed: f32) -> f32 {
        self.lean_rate * self.turn.map_or(1.0, |law| law.lean_rate(speed))
    }

    /// The factor of the speed on the cornering assist: the slow end of `authority` at or under `authority_speed[0]`, the fast end at or over
    /// `authority_speed[1]`, and between them a smoothstep of the logarithm of the speed (a progressive curve: it changes fast where the
    /// speed doubles from a little, with no step and no kink at the ends). 1 for the reference's `[1, 1]`.
    pub fn authority_factor(&self, speed: f32) -> f32 {
        let [slow, fast] = self.authority;
        if slow == fast {
            return slow;
        }
        let first = self.authority_speed[0].max(0.01);
        let second = self.authority_speed[1].max(first * 1.01);
        let along = super::curve::ramp(speed.abs().max(0.01).ln(), [first.ln(), second.ln()]);
        slow + (fast - slow) * along * along * (3.0 - 2.0 * along)
    }

    /// The speed factor of the lean time scale: the data's curve `table` at `speed`, held at `time_scale_speed_floor` at least (and never
    /// above the curve's own value at a standstill, so a table that starts low is not raised). 1 for an empty table.
    pub fn speed_scale(&self, table: &[(f32, f32)], speed: f32) -> f32 {
        let start = super::curve::factor(table, 0.0);
        let floor = self.time_scale_speed_floor.min(start);
        super::curve::factor(table, speed.abs()).max(floor)
    }

    /// The factor of the data's time-scale curve over the stick at `stick`.
    pub fn stick_scale(&self, table: &[(f32, f32)], stick: f32) -> f32 {
        match self.mirror_stick_curve {
            true => super::curve::factor_either_side(table, stick),
            false => super::curve::factor(table, stick),
        }
    }
}

impl Default for ArcadeSteering {
    /// This game's values (`docs/specs/ride6-arcade-steering.md`, section 5).
    fn default() -> Self {
        Self {
            lateral_grip: 1.0,
            lean_limit: 0.7,
            lean_rate: 1.38,
            time_scale_speed_floor: 1.0,
            mirror_stick_curve: true,
            roll_spring: 1.28,
            stick_response: 1.28,
            handlebar_response: 1.085,
            lean_help: 6.25,
            lean_help_limit: 3.0,
            fall_angular_speed: 3.0,
            cornering_assist: 1.1,
            assist_speed: [2.0, 8.0],
            stick_reach: 0.9,
            authority: [1.5, 0.6],
            authority_speed: [5.0, 70.0],
            wheelie_steer_drop: Some(7.0_f32.to_radians()),
            turn: Some(TurnLaw::GAME),
            air: ArcadeAir::default(),
            rough: ArcadeRough::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reference_is_neutral_and_the_default_is_not() {
        assert!(!ArcadeSteering::REFERENCE.is_enabled());
        assert_eq!(ArcadeSteering::disabled(), ArcadeSteering::REFERENCE);
        assert!(ArcadeSteering::default().is_enabled());
        assert_eq!(ArcadeSteering::REFERENCE.cornering_acceleration(1.0, 30.0), 0.0);
    }

    #[test]
    fn the_speed_scale_is_the_datas_curve_in_the_reference_and_flat_in_the_arcade() {
        let table = [(11.0, 1.0), (92.0, 0.333)];
        let reference = ArcadeSteering::REFERENCE;
        assert!((reference.speed_scale(&table, 92.0) - 0.333).abs() < 1e-6);
        assert!((ArcadeSteering::default().speed_scale(&table, 92.0) - 1.0).abs() < 1e-6);
        // A table that starts under the floor is not raised.
        assert!((ArcadeSteering::default().speed_scale(&[(2.0, 0.5), (50.0, 0.4)], 30.0) - 0.5).abs() < 0.1);
        assert_eq!(reference.speed_scale(&[], 30.0), 1.0);
    }

    #[test]
    fn the_cornering_assist_follows_the_lean_and_fades_in_with_the_speed() {
        let arcade = ArcadeSteering { cornering_assist: 0.5, authority: [1.0, 1.0], ..ArcadeSteering::default() };
        assert!((arcade.cornering_acceleration(1.0, 30.0) - 0.5 * 9.81).abs() < 1e-4);
        assert!((arcade.cornering_acceleration(-0.5, 30.0) + 0.25 * 9.81).abs() < 1e-4);
        assert_eq!(arcade.cornering_acceleration(1.0, 1.0), 0.0);
        assert!(arcade.cornering_acceleration(1.0, 5.0) > 0.0 && arcade.cornering_acceleration(1.0, 5.0) < 0.5 * 9.81);
    }

    #[test]
    fn the_authority_is_neutral_in_the_reference_and_falls_smoothly_with_the_speed() {
        let reference = ArcadeSteering::REFERENCE;
        for speed in [0.0_f32, 5.0, 20.0, 80.0] {
            assert_eq!(reference.authority_factor(speed), 1.0);
        }
        let arcade = ArcadeSteering::default();
        let [slow, fast] = arcade.authority;
        assert!(slow > 1.0 && fast < 1.0, "tighter when slow, wider when fast");
        assert_eq!(arcade.authority_factor(0.0), slow);
        assert_eq!(arcade.authority_factor(arcade.authority_speed[0]), slow);
        assert!((arcade.authority_factor(arcade.authority_speed[1]) - fast).abs() < 1e-6);
        assert!((arcade.authority_factor(300.0) - fast).abs() < 1e-6);
        // Never rises with the speed, and no step: a tenth of a m/s moves it by under 1.5 percent.
        let mut last = arcade.authority_factor(0.0);
        let mut speed = 0.1;
        while speed < 90.0 {
            let now = arcade.authority_factor(speed);
            assert!(now <= last + 1e-6, "{speed} m/s: {now} after {last}");
            assert!(last - now < 0.015, "a step at {speed} m/s: {last} to {now}");
            last = now;
            speed += 0.1;
        }
        // The crossing of 1 (the assist as before) lies between 50 and 90 km/h.
        let crossing = (1..400).map(|i| i as f32 * 0.25).find(|&v| arcade.authority_factor(v) < 1.0).unwrap();
        assert!((14.0..25.0).contains(&crossing), "the factor passes 1 at {crossing} m/s");
    }

    #[test]
    fn the_assist_acceleration_carries_the_authority() {
        let arcade = ArcadeSteering::default();
        let plain = ArcadeSteering { authority: [1.0, 1.0], ..arcade };
        assert!(arcade.cornering_acceleration(1.0, 9.0) > plain.cornering_acceleration(1.0, 9.0));
        assert!(arcade.cornering_acceleration(1.0, 60.0) < plain.cornering_acceleration(1.0, 60.0));
        assert_eq!(ArcadeSteering::REFERENCE.wheelie_steer_drop, None);
        assert!(arcade.wheelie_steer_drop.is_some_and(|drop| drop > 0.0));
    }

    #[test]
    fn the_stick_curve_is_signed_in_the_reference_and_mirrored_in_the_arcade() {
        let table = [(-1.0, 0.4), (0.0, 1.0), (1.0, 0.5)];
        assert!((ArcadeSteering::REFERENCE.stick_scale(&table, -1.0) - 0.4).abs() < 1e-6);
        assert!((ArcadeSteering::default().stick_scale(&table, -1.0) - 0.5).abs() < 1e-6);
    }
}
