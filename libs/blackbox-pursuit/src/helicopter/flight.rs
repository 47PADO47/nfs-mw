//! The helicopter's flight model: a point mass with a 0.5 s velocity response and a bounded acceleration, plus an
//! attitude that leans into the acceleration and turns towards the facing direction. Spec: `ai-helicopter.md` §5.
//! The rotor lift cancels gravity, so the model has no vertical gravity term.

use glam::{Quat, Vec3};

/// The flight constants of the simple chopper controller (`chopperspecs/default`, install-verified).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Specs {
    /// Top speed, m/s. Speeds above it are cut to it, climbing included.
    pub max_speed: f32,
    pub pitch_ang: f32,
    pub roll_ang: f32,
    pub pitch_align: f32,
    pub roll_align: f32,
    /// The acceleration limit `min(cap, 0.6 v + base)`.
    pub accel_cap: f32,
    pub accel_base: f32,
    /// The yaw rate limit (rad/s) and the rate per radian of heading error.
    pub yaw_limit: f32,
    pub yaw_gain: f32,
}

impl Specs {
    /// The specs with the top speed and the numbers a tuning file may change.
    pub fn with_gains(max_speed: f32, gains: &super::gains::Gains) -> Self {
        Self {
            max_speed,
            accel_cap: gains.acceleration[0],
            accel_base: gains.acceleration[1],
            yaw_limit: gains.turning[0],
            yaw_gain: gains.turning[1],
            ..Self::default()
        }
    }
}

impl Default for Specs {
    fn default() -> Self {
        Self {
            max_speed: 100.0,
            pitch_ang: 0.05,
            roll_ang: 0.06,
            pitch_align: 6.0,
            roll_align: 4.5,
            accel_cap: 80.0,
            accel_base: 30.0,
            yaw_limit: YAW_LIMIT,
            yaw_gain: YAW_GAIN,
        }
    }
}

/// The acceleration limit: `min(80, 0.6 v + 30)`, or 80 when braking hard. [decomp]: 0.6 pool, 80 pool
///, 30 pool.
pub fn max_acceleration(speed: f32, max_decel: bool) -> f32 {
    limit(speed, max_decel, 80.0, 30.0)
}

fn limit(speed: f32, max_decel: bool, cap: f32, base: f32) -> f32 {
    if max_decel {
        return cap;
    }
    (speed * 0.6 + base).min(cap)
}

/// Desired acceleration is this many times the velocity error (`Chopper_Ratio`, a 0.5 s response). [decomp] 2, pool.
const RESPONSE_RATIO: f32 = 2.0;
/// Scale of the acceleration read back as lean (`meas_local *= 0.2`, the memoryless low-pass of spec Q2). [decomp]
const LEAN_LOW_PASS: f32 = 0.2;
/// Pitch lean per m/s^2 of forward-back acceleration: `9 / 80` of `pitch_ang`. [decomp] 9 pool, 80 the divisor.
const PITCH_LEAN: f32 = 9.0 / 80.0;
/// Roll lean per m/s^2 of side acceleration: `12 / 80` of `roll_ang`. [decomp] 12 pool, 80 the divisor.
const ROLL_LEAN: f32 = 12.0 / 80.0;
/// Limit of the pitch and roll lean, in turns (36 degrees). [decomp] 0.1, pool.
const LEAN_LIMIT: f32 = 0.1;
/// Gain of the climb-pitch term (`meas_local.y * 0.035`, above 15 m/s with a rising velocity). [unconfirmed]: spec value, not
/// found in the pool.
const CLIMB_PITCH: f32 = 0.035;
/// Yaw rate per radian of heading error (`cross * 8`). [decomp] 8, pool.
const YAW_GAIN: f32 = 8.0;
/// Limit of the yaw rate, rad/s. [unconfirmed]: spec value, not found in the pool.
const YAW_LIMIT: f32 = 1.3;
/// Angular velocity smoothing: `(new + 7 old) / 8`. [decomp] 7 pool, 0.125 pool.
const ANGULAR_OLD: f32 = 7.0;
const ANGULAR_SMOOTH: f32 = 0.125;

/// The helicopter as a body: where it is, how it moves and which way it points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chopper {
    pub position: Vec3,
    pub velocity: Vec3,
    pub orientation: Quat,
    /// The angular velocity of the last step, after smoothing (world frame, rad/s; composed from the body axes).
    angular: Vec3,
    prev_velocity: Vec3,
}

impl Chopper {
    /// A helicopter at rest at `position`, facing `heading` (radians from +X in the map plane, turned about +Y).
    pub fn new(position: Vec3, heading: f32) -> Self {
        Self {
            position,
            velocity: Vec3::ZERO,
            orientation: Quat::from_rotation_y(-heading),
            angular: Vec3::ZERO,
            prev_velocity: Vec3::ZERO,
        }
    }

    /// The direction the nose points.
    pub fn forward(&self) -> Vec3 {
        self.orientation * Vec3::X
    }

    /// Advances the body by `dt` seconds towards `desired` velocity, turning its nose towards `facing`.
    /// `max_decel` lifts the acceleration limit to its braking value.
    pub fn step(&mut self, dt: f32, desired: Vec3, facing: Vec3, max_decel: bool, specs: &Specs) {
        if !(dt > 0.0 && dt.is_finite()) {
            return;
        }
        let desired = clamp_length(desired, specs.max_speed);
        let speed = self.velocity.length();
        let limit = limit(speed, max_decel, specs.accel_cap, specs.accel_base);
        let dv = desired - self.velocity;
        let required = dv.length() * RESPONSE_RATIO;
        let accel = if required > 1.0 && (required > limit || max_decel) {
            dv * (limit / required)
        } else {
            dv * RESPONSE_RATIO
        };
        // The lean reads the acceleration of the last step, as the original reads it back from the body's velocity: the
        // velocity before that step's change is kept, so the read-back is not zero.
        let measured = self.measured_acceleration(dt);
        self.prev_velocity = self.velocity;
        self.velocity += accel * dt;
        self.position += self.velocity * dt;
        self.attitude(measured, facing, specs, dt);
    }

    /// The acceleration of the last step, read back from the velocity change over it (zero for a step too short to
    /// trust).
    fn measured_acceleration(&self, dt: f32) -> Vec3 {
        if dt <= 0.005 {
            return Vec3::ZERO;
        }
        (self.velocity - self.prev_velocity) / dt
    }

    /// Leans the body into the acceleration and turns it to face `facing`, with the 8-step smoothing of the spec. The body
    /// frame is nose +X, right -Z, up +Y: the pitch is read from the forward acceleration and turns the body about its
    /// right axis, the roll from the side acceleration and turns it about its nose, the yaw about the vertical.
    fn attitude(&mut self, measured: Vec3, facing: Vec3, specs: &Specs, dt: f32) {
        let local = self.orientation.inverse() * measured * LEAN_LOW_PASS;
        let (ahead, rightward, up) = (local.x, -local.z, local.y);
        let speed = self.velocity.length();
        let forward = self.forward();
        let right = self.orientation * Vec3::NEG_Z;

        let mut pitch = -(ahead * PITCH_LEAN) * specs.pitch_ang;
        if up > 0.0 && speed > 15.0 && self.velocity.y > 0.0 {
            pitch += up * CLIMB_PITCH;
        }
        let pitch = pitch.clamp(-LEAN_LIMIT, LEAN_LIMIT);
        let ideal_forward = Vec3::new(forward.x, (std::f32::consts::TAU * pitch).sin(), forward.z);
        let pitch_rate = (forward.cross(ideal_forward).dot(right) * specs.pitch_align).clamp(-1.0, 1.0);

        let roll = (-(rightward * ROLL_LEAN) * specs.roll_ang).clamp(-LEAN_LIMIT, LEAN_LIMIT);
        let ideal_right = Vec3::new(right.x, (std::f32::consts::TAU * roll).sin(), right.z);
        let roll_rate = (right.cross(ideal_right).dot(forward) * specs.roll_align).clamp(-1.0, 1.0);

        let yaw = yaw_rate(facing, forward, specs);
        let raw = right * pitch_rate + forward * roll_rate + Vec3::Y * yaw;
        self.angular = (raw + ANGULAR_OLD * self.angular) * ANGULAR_SMOOTH;
        let turn = Quat::from_scaled_axis(self.angular * dt);
        self.orientation = (turn * self.orientation).normalize();
    }
}

/// The yaw rate that turns the nose, flattened to the horizontal, towards `facing`.
fn yaw_rate(facing: Vec3, forward: Vec3, specs: &Specs) -> f32 {
    let ideal = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let actual = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    (-ideal.cross(actual).y * specs.yaw_gain).clamp(-specs.yaw_limit, specs.yaw_limit)
}

/// `v` shortened to `max` if it is longer.
fn clamp_length(v: Vec3, max: f32) -> Vec3 {
    let len = v.length();
    if len <= max {
        return v;
    }
    v * (max / len)
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn acceleration_limit_grows_with_speed_and_caps_at_80() {
        assert_eq!(max_acceleration(0.0, false), 30.0);
        assert!((max_acceleration(50.0, false) - 60.0).abs() < 1e-5);
        assert_eq!(max_acceleration(200.0, false), 80.0);
        assert_eq!(max_acceleration(0.0, true), 80.0, "braking lifts the limit");
    }

    #[test]
    fn hovers_still_with_nothing_asked() {
        let mut heli = Chopper::new(Vec3::new(0.0, 50.0, 0.0), 0.0);
        for _ in 0..120 {
            heli.step(DT, Vec3::ZERO, Vec3::X, false, &Specs::default());
        }
        assert!(heli.velocity.length() < 1e-3);
        assert!((heli.position.y - 50.0).abs() < 1e-3, "no gravity: the lift cancels it");
    }

    #[test]
    fn reaches_the_desired_velocity_in_about_half_a_second() {
        let mut heli = Chopper::new(Vec3::ZERO, 0.0);
        // From rest the limit is 30 m/s^2, so 20 m/s is reached in well under two seconds.
        let want = Vec3::new(20.0, 0.0, 0.0);
        for _ in 0..120 {
            heli.step(DT, want, Vec3::X, false, &Specs::default());
        }
        assert!((heli.velocity - want).length() < 0.5, "velocity {:?}", heli.velocity);
    }

    #[test]
    fn speed_is_capped_at_the_top_speed_climbing_included() {
        let mut heli = Chopper::new(Vec3::ZERO, 0.0);
        let want = Vec3::new(150.0, 150.0, 0.0);
        for _ in 0..600 {
            heli.step(DT, want, Vec3::X, false, &Specs::default());
        }
        assert!(heli.velocity.length() <= 100.5, "top speed {}", heli.velocity.length());
    }

    #[test]
    fn acceleration_is_limited_per_step() {
        let mut heli = Chopper::new(Vec3::ZERO, 0.0);
        heli.step(DT, Vec3::new(100.0, 0.0, 0.0), Vec3::X, false, &Specs::default());
        assert!(heli.velocity.length() <= 30.0 * DT + 1e-4, "from rest the limit is 30 m/s^2");
    }

    #[test]
    fn the_nose_turns_towards_the_facing_direction() {
        let mut heli = Chopper::new(Vec3::ZERO, 0.0);
        let facing = Vec3::new(0.0, 0.0, 1.0);
        let before = heli.forward().dot(facing);
        for _ in 0..240 {
            heli.step(DT, Vec3::ZERO, facing, false, &Specs::default());
        }
        assert!(heli.forward().dot(facing) > before + 0.9, "turned to within about 25 degrees");
    }

    /// The nose's height and the right side's height after `secs` of asking for `want` from rest, nose along +X.
    fn lean_after(want: Vec3, secs: f32) -> (f32, f32) {
        let mut heli = Chopper::new(Vec3::ZERO, 0.0);
        for _ in 0..(secs / DT) as u32 {
            heli.step(DT, want, Vec3::X, false, &Specs::default());
        }
        (heli.forward().y, (heli.orientation * Vec3::NEG_Z).y)
    }

    #[test]
    fn speeding_up_pitches_the_nose_down_and_braking_lifts_it() {
        let (nose, side) = lean_after(Vec3::new(30.0, 0.0, 0.0), 0.5);
        assert!(nose < -0.02, "accelerating forward dips the nose: {nose}");
        assert!(side.abs() < 1e-3, "and does not roll: {side}");
        let (nose, _) = lean_after(Vec3::new(-30.0, 0.0, 0.0), 0.5);
        assert!(nose > 0.02, "accelerating backward lifts it: {nose}");
    }

    #[test]
    fn a_sideways_push_rolls_the_body_into_it() {
        let right = Vec3::NEG_Z;
        let (nose, side) = lean_after(right * 30.0, 0.5);
        assert!(side < -0.02, "accelerating to the right drops the right side: {side}");
        assert!(nose.abs() < 0.01, "with little pitch: {nose}");
        let (_, side) = lean_after(-right * 30.0, 0.5);
        assert!(side > 0.02, "to the left lifts it: {side}");
    }

    #[test]
    fn the_lean_follows_the_body_whatever_its_heading() {
        // Facing +Z, the same forward push (now along +Z) still dips the nose rather than rolling the body.
        let mut heli = Chopper::new(Vec3::ZERO, std::f32::consts::FRAC_PI_2);
        let forward = heli.forward();
        for _ in 0..30 {
            heli.step(DT, forward * 30.0, forward, false, &Specs::default());
        }
        assert!(heli.forward().y < -0.02, "nose down: {}", heli.forward().y);
        assert!((heli.orientation * Vec3::NEG_Z).y.abs() < 1e-3, "no roll");
    }

    #[test]
    fn a_short_step_does_nothing() {
        let mut heli = Chopper::new(Vec3::ZERO, 0.0);
        heli.step(0.0, Vec3::X * 10.0, Vec3::X, false, &Specs::default());
        assert_eq!(heli.velocity, Vec3::ZERO);
    }
}
