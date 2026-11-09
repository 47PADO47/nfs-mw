//! Air control: steering the body of a vehicle that has no wheel on the ground, as the open-world games of the genre do (a stick
//! pitches and rolls it, another axis spins it) (`docs/specs/air-control.md`).
//!
//! **This is a modification made exclusively for the NFS:MW rewrite; neither NFS: Most Wanted (2005) nor RIDE 6 has it.** The host decides
//! whether it is on (a setting) and for whom (the player's vehicle); this module only turns the three axes into a torque:
//!
//! - each axis asks for an angular speed (`rate`, rad/s, at full input) about its body axis, and the torque is the angular acceleration
//!   that closes the gap at `gain` per second, **held to `accel` (rad/s^2)** and multiplied by the body's inertia, so a heavy body and a
//!   light one turn the same way and nothing can spin up faster than `accel` allows;
//! - with no input on an axis the same bounded torque damps its angular speed (`damping`, per second), so the body settles instead of
//!   tumbling for ever;
//! - an input never pushes an axis that already turns faster than it asks for, and when the caller gives the attitude limits (a bike
//!   falls over beyond them) the rate it asks for fades out as the attitude nears them, so the body stops at the limit.
//!
//! Axes and signs, as the stick is used: **pitch** positive is the stick forward, the nose down; **roll** positive is the stick right,
//! the right side down; **yaw** positive is a turn to the right. The body frame is x right, y up, z forward.

use glam::{Mat3, Vec3};

#[cfg(test)]
mod tests;

/// Below this an axis counts as untouched and only the damping acts.
const DEAD: f32 = 1e-3;
/// The share of an attitude limit over which the push fades out (it is whole below 0.7 of the limit and gone at it).
const SOFT_LIMIT_BAND: f32 = 0.3;

/// The three axes of the player's input, each -1 to 1.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AirAxes {
    /// Positive: the stick forward, the nose down.
    pub pitch: f32,
    /// Positive: the stick right, the right side down.
    pub roll: f32,
    /// Positive: a spin to the right.
    pub yaw: f32,
}

impl AirAxes {
    /// No input.
    pub const ZERO: Self = Self { pitch: 0.0, roll: 0.0, yaw: 0.0 };

    /// The axes held to -1 to 1 (an axis that is not a number reads 0).
    pub fn clamped(self) -> Self {
        let held = |v: f32| match v.is_finite() {
            true => v.clamp(-1.0, 1.0),
            false => 0.0,
        };
        Self { pitch: held(self.pitch), roll: held(self.roll), yaw: held(self.yaw) }
    }

    /// Whether any axis is touched.
    pub fn is_active(&self) -> bool {
        self.pitch.abs() > DEAD || self.roll.abs() > DEAD || self.yaw.abs() > DEAD
    }
}

/// What one axis can do: the angular speed a full input asks for (rad/s) and the largest angular acceleration (rad/s^2).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AxisLimit {
    pub rate: f32,
    pub accel: f32,
}

/// The body the torque is for: its orientation (columns are the body axes), its angular velocity (world frame, rad/s) and its principal
/// moments of inertia (kg m^2, body frame).
#[derive(Clone, Copy, Debug)]
pub struct BodyFrame {
    pub rotation: Mat3,
    pub angular_velocity: Vec3,
    pub inertia: Vec3,
}

/// Where the body is in pitch and in roll and how far it may go (rad; a limit of 0 means none). A bike falls beyond its limits, so the
/// push fades out before them.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Attitude {
    /// The pitch, positive nose up.
    pub pitch_up: f32,
    /// The lean, positive towards the right.
    pub lean_right: f32,
    pub pitch_limit: f32,
    pub lean_limit: f32,
}

/// The limits of a class of vehicle (the named constants [`AirControlSpec::CAR`] and [`AirControlSpec::BIKE`]).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AirControlSpec {
    pub pitch: AxisLimit,
    pub roll: AxisLimit,
    pub yaw: AxisLimit,
    /// How fast an axis closes the gap to the angular speed it asks for (per second).
    pub gain: f32,
    /// How fast an untouched axis loses its angular speed (per second); bounded by the axis's `accel`.
    pub damping: f32,
}

impl AirControlSpec {
    /// A car: heavy and long, so slow to pitch and to spin; the angular accelerations are the ones a car's mass and a stick feel right
    /// with (a full stick takes about 0.3 s to reach its rate on the pitch axis, and the car rotates a quarter turn in about 1.4 s).
    pub const CAR: Self = Self {
        pitch: AxisLimit { rate: 1.1, accel: 3.5 },
        roll: AxisLimit { rate: 1.6, accel: 5.0 },
        yaw: AxisLimit { rate: 1.4, accel: 3.5 },
        gain: 6.0,
        damping: 0.5,
    };

    /// A bike with its rider: lighter and quicker than a car in all three axes, still bounded so the front wheel does not flick round.
    pub const BIKE: Self = Self {
        pitch: AxisLimit { rate: 1.8, accel: 8.0 },
        roll: AxisLimit { rate: 2.4, accel: 11.0 },
        yaw: AxisLimit { rate: 1.8, accel: 7.0 },
        gain: 8.0,
        damping: 0.7,
    };

    /// The angular acceleration (rad/s^2) one axis asks for: `command` is -1 to 1 along the axis, `omega` the body's angular speed about it.
    /// `held` is whether the player touches the axis: a held axis whose command has faded to nothing (an attitude limit) is still steered, to
    /// a standstill, so the body stops at the limit instead of drifting on.
    fn acceleration(&self, limit: AxisLimit, command: f32, omega: f32, held: bool) -> f32 {
        let bound = limit.accel;
        let damp = (-self.damping * omega).clamp(-bound, bound);
        if !held {
            return damp;
        }
        let target = command * limit.rate;
        let error = target - omega;
        // Already turning faster than the input asks for, the way it asks: no push, the damping alone.
        if error * target < 0.0 {
            return damp;
        }
        (self.gain * error).clamp(-bound, bound)
    }

    /// The torque (N m, world frame) the input puts on the body: the three accelerations through the inertia. The body's own attitude
    /// limits (`attitude`) fade out the push that goes towards them.
    pub fn torque(&self, axes: AirAxes, body: &BodyFrame, attitude: &Attitude) -> Vec3 {
        let axes = axes.clamped();
        let omega = body.rotation.transpose() * body.angular_velocity;
        // The stick forward turns the body about its right axis the way the nose goes down: positive about x. The stick right rolls the
        // right side down: negative about z. A spin to the right is positive about y.
        let pitch_toward = match axes.pitch >= 0.0 {
            true => -attitude.pitch_up,
            false => attitude.pitch_up,
        };
        let roll_toward = attitude.lean_right * axes.roll.signum();
        let pitch = axes.pitch * headroom(pitch_toward, attitude.pitch_limit);
        let roll = axes.roll * headroom(roll_toward, attitude.lean_limit);
        let accelerations = Vec3::new(
            self.acceleration(self.pitch, pitch, omega.x, axes.pitch.abs() > DEAD),
            self.acceleration(self.yaw, axes.yaw, omega.y, axes.yaw.abs() > DEAD),
            self.acceleration(self.roll, -roll, omega.z, axes.roll.abs() > DEAD),
        );
        body.rotation * (accelerations * body.inertia)
    }
}

/// How much of a push is left (0 to 1) for a body `angle` (rad) away from upright in the pushed direction, with a limit of `limit` (0: none).
fn headroom(angle: f32, limit: f32) -> f32 {
    if limit <= 0.0 {
        return 1.0;
    }
    ((limit - angle) / (SOFT_LIMIT_BAND * limit)).clamp(0.0, 1.0)
}
