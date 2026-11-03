//! Steering with the front wheel up (`docs/specs/ride6-wheelie-steer.md`).
//!
//! With the front wheel off the road the stick has no front tyre to work through. The reference game gives the bike three things for
//! that state and this module carries them, plus two choices of this port that make the result steerable:
//!
//! - **[B, D] The mid-air steer.** The handler's `MidAirSteerComponent` (the same in all eight input handlers) takes the handlebar
//!   away from the lean controller while the front wheel is more than 4 cm over the road, the lean is under 40 percent of its limit and
//!   the nose is not dropping faster than 0.4 rad/s, and sets it to 0.8 of its lock from the stick ([`mid_air_intensity`],
//!   [`mid_air_angle`]).
//! - **[B] The lean help factor of a wheelie** is `LeanHelpFactorWheelie`, 1 (the constructor default; no bike sets it or the range
//!   that would blend to it): the help is whole while the front wheel is up ([`LEAN_HELP_FACTOR_WHEELIE`]).
//! - **[B] The lean limit is cut** with the wheelie angle (`WheelieLeanLimitTuning`, 1 degree at 40 to 45 degrees: `lean_limit` of the
//!   wheelie spec). [C] This port keeps the cut but not below [`choices::WHEELIE_LEAN_FLOOR`], and
//! - **[C] holds the yaw to the lean** ([`Bike::apply_wheelie_yaw`]): the yaw rate a steady turn at the bike's lean needs, `g tan(lean) /
//!   v`, which is the stability help's own target (`ride6-chassis-round7.md`, section 3) and which the reference would apply in a wheelie
//!   through `StabilityHelpFactorWheelie` if a bike set it (constructor default 0, no bike does).

use glam::Vec3;

use super::curve::ramp;
use super::{Bike, FRONT, REAR, choices, lean_of};

/// The height of the front tyre over the road (m) over which the mid-air steer comes in: `GroundDistanceRange` 4.0 to 4.1 (centimetres,
/// as the other distances of the handler: `LeanHelpFactorAirborneRange` 20 to 35, `StabilityHelpMaxDistanceFromGround` 30). [D]
pub const MID_AIR_GROUND_RANGE: [f32; 2] = [0.040, 0.041];

/// The normalised lean (lean over the lean limit) over which the mid-air steer goes out: `NormalizedLeanRange` 0.4 to 0.41. [D]
pub const MID_AIR_LEAN_RANGE: [f32; 2] = [0.4, 0.41];

/// The pitch speed (rad/s, nose up positive) over which the mid-air steer comes in: `PitchSpeedRange` -0.4 to -0.3. The reference's sign
/// of the pitch speed was not traced; read as nose-up positive, a bike whose nose drops faster than 0.4 rad/s (a wheelie let down, a jump
/// coming in) gets no mid-air steer, which is what such a limit is for. [D, sign I]
pub const MID_AIR_PITCH_SPEED_RANGE: [f32; 2] = [-0.4, -0.3];

/// The handlebar the mid-air steer asks for at a full stick, as a share of the lock: `SteerInputTuning` maps the stick -1 to 1 onto 0.8 to
/// -0.8. With the handlebar positive to the right (as in `lean::handlebar_angle`) a stick to the right turns the bar to the left, the
/// counter-steer that rolls a bike to the right through the gyro of its front wheel. [D, polarity I]
pub const MID_AIR_STEER_SHARE: f32 = 0.8;

/// `LeanHelpFactorWheelie`, the factor of the lean help while the front wheel is up (the front gap over the unset range `(0, 0)` of
/// `LeanHelpFactorWheelieDistanceFromGroundRange` makes it the whole factor): the handler's constructor default, 1. [B]
pub const LEAN_HELP_FACTOR_WHEELIE: f32 = 1.0;

/// The weight the mid-air steer has over the controller's handlebar, 0 to 1: the front tyre's height over the road, the lean over its limit
/// and the nose-up pitch speed (rad/s), each through its range, multiplied (`0x1450f7680`).
pub fn mid_air_intensity(front_gap: f32, normalised_lean: f32, pitch_up_rate: f32) -> f32 {
    ramp(front_gap, MID_AIR_GROUND_RANGE)
        * (1.0 - ramp(normalised_lean.abs(), MID_AIR_LEAN_RANGE))
        * ramp(pitch_up_rate, MID_AIR_PITCH_SPEED_RANGE)
}

/// The handlebar angle (rad, positive to the right) the mid-air steer asks for with the stick at `stick` and a lock of `max_angle`.
pub fn mid_air_angle(stick: f32, max_angle: f32) -> f32 {
    -MID_AIR_STEER_SHARE * stick.clamp(-1.0, 1.0) * max_angle
}

/// The yaw rate (rad/s, positive to the right) of a steady turn at `lean` (rad, held to `max_lean`) at `speed` (m/s, at least `min_speed`):
/// `g tan(lean) / v`.
pub fn coordinated_yaw_rate(lean: f32, speed: f32, max_lean: f32, min_speed: f32) -> f32 {
    let held = match max_lean > 0.0 {
        true => lean.clamp(-max_lean, max_lean),
        false => lean,
    };
    held.tan() * 9.81 / speed.abs().max(min_speed.max(0.1))
}

/// How much of the wheelie's yaw torque a bike gets, 0 to 1: it comes in over `speed_range` (m/s) of the speed and over
/// `choices::WHEELIE_YAW_PITCH_RANGE` of the wheelie's lift angle (rad), so a front wheel that only hovers is a bike on the road.
pub fn yaw_weight(speed: f32, lift_angle: f32, speed_range: [f32; 2]) -> f32 {
    ramp(speed.abs(), speed_range) * ramp(lift_angle, choices::WHEELIE_YAW_PITCH_RANGE)
}

impl Bike {
    /// The weight of the mid-air steer now (see [`mid_air_intensity`]); 0 with the front wheel on the road.
    pub(super) fn mid_air_weight(&self) -> f32 {
        let front = &self.wheels[FRONT];
        if front.on_ground || self.fallen {
            return 0.0;
        }
        let rotation = self.body.rotation();
        let normalised = lean_of(rotation) / self.plan.limit.max(choices::MIN_LEAN_LIMIT);
        let pitch_up_rate = -self.body.angular_velocity.dot(rotation * Vec3::X);
        mid_air_intensity(front.gap, normalised, pitch_up_rate)
    }

    /// With the front wheel up and the rear wheel on the road, a yaw torque about the road's up that pulls the yaw rate to the one a steady
    /// turn at the bike's lean needs, `choices::WHEELIE_YAW_FACTOR` of the gap per sub-step, from the stability help's speed range up.
    /// [C] The reference's front tyre turns the bike into a corner; with it in the air nothing turns a leaned bike but the rear tyre, whose
    /// camber thrust is a few tens of newtons in the data, so the heading hardly follows the lean (`ride6-wheelie-steer.md`, section 3).
    pub(super) fn apply_wheelie_yaw(&mut self, dt: f32, speed: f32) {
        let factor = choices::WHEELIE_YAW_FACTOR;
        let rear = &self.wheels[REAR];
        if factor <= 0.0 || self.fallen || self.departure.off || self.wheels[FRONT].on_ground || !rear.on_ground {
            return;
        }
        let stability = &self.spec.steering.lean.stability;
        // It comes in with the speed (the stability help's range) and with the height of the wheelie: a front wheel that only hovers
        // over the road, as a throttle opened with the stick flicked makes it, is a bike on the road for this purpose.
        let coming_in = yaw_weight(speed, self.lift_angle(), stability.speed_range);
        if coming_in <= 0.0 {
            return;
        }
        let axis = rear.normal.try_normalize().unwrap_or(Vec3::Y);
        let rotation = self.body.rotation();
        let wanted = coordinated_yaw_rate(lean_of(rotation), speed, stability.max_lean, stability.speed_range[1]);
        let actual = self.body.angular_velocity.dot(axis);
        let acceleration = factor * coming_in * (wanted - actual) / dt.max(0.001);
        let local = rotation.transpose() * (axis * acceleration);
        self.body.apply_torque(rotation * (local * self.body.inertia()));
    }
}

#[cfg(test)]
#[path = "wheelie_steer_tests.rs"]
mod tests;
