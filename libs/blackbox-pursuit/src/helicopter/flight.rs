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
}

impl Default for Specs {
    fn default() -> Self {
        Self { max_speed: 100.0, pitch_ang: 0.05, roll_ang: 0.06, pitch_align: 6.0, roll_align: 4.5 }
    }
}

/// The acceleration limit: `min(80, 0.6 v + 30)`, or 80 when braking hard.
pub fn max_acceleration(speed: f32, max_decel: bool) -> f32 {
    if max_decel {
        return 80.0;
    }
    (speed * 0.6 + 30.0).min(80.0)
}

/// The helicopter as a body: where it is, how it moves and which way it points.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Chopper {
    pub position: Vec3,
    pub velocity: Vec3,
    pub orientation: Quat,
    /// The angular velocity of the last step, after smoothing (world frame, rad/s).
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
        let limit = max_acceleration(speed, max_decel);
        let dv = desired - self.velocity;
        let required = dv.length() * 2.0;
        let accel = if required > 1.0 && (required > limit || max_decel) { dv * (limit / required) } else { dv * 2.0 };
        let measured = self.measured_acceleration(dt);
        self.velocity += accel * dt;
        self.position += self.velocity * dt;
        self.attitude(measured, facing, specs, dt);
        self.prev_velocity = self.velocity;
    }

    /// The acceleration of the last step, read back from the velocity change (zero for a step too short to trust).
    fn measured_acceleration(&self, dt: f32) -> Vec3 {
        if dt <= 0.005 {
            return Vec3::ZERO;
        }
        (self.velocity - self.prev_velocity) / dt
    }

    /// Leans the body into the acceleration and turns it to face `facing`, with the 8-step smoothing of the spec.
    fn attitude(&mut self, measured: Vec3, facing: Vec3, specs: &Specs, dt: f32) {
        let inverse = self.orientation.inverse();
        let local = inverse * measured * 0.2;
        let speed = self.velocity.length();

        let mut pitch = -(local.z * 9.0 / 80.0) * specs.pitch_ang;
        if local.y > 0.0 && speed > 15.0 && self.velocity.y > 0.0 {
            pitch += local.y * 0.035;
        }
        let pitch = pitch.clamp(-0.1, 0.1);
        let forward = self.forward();
        let ideal_forward = Vec3::new(forward.x, (std::f32::consts::TAU * pitch).sin(), forward.z);
        let torque_x = (inverse * ideal_forward.cross(forward)).x;

        let mut roll = -(local.x * 12.0 / 80.0) * specs.roll_ang;
        roll = roll.clamp(-0.1, 0.1);
        let right = self.orientation * Vec3::NEG_Z;
        let ideal_right = Vec3::new(right.x, (std::f32::consts::TAU * roll).sin(), right.z);
        let torque_z = (inverse * ideal_right.cross(right)).z;

        let mut raw = Vec3::ZERO;
        raw.x = (-torque_x * specs.pitch_align).clamp(-1.0, 1.0);
        raw.z = (-torque_z * specs.roll_align).clamp(-1.0, 1.0);
        raw.y = yaw_rate(facing, forward);

        self.angular = (raw + 7.0 * self.angular) * 0.125;
        let turn = Quat::from_scaled_axis(self.angular * dt);
        self.orientation = (turn * self.orientation).normalize();
    }
}

/// The yaw rate that turns the nose, flattened to the horizontal, towards `facing`.
fn yaw_rate(facing: Vec3, forward: Vec3) -> f32 {
    let ideal = Vec3::new(facing.x, 0.0, facing.z).normalize_or_zero();
    let actual = Vec3::new(forward.x, 0.0, forward.z).normalize_or_zero();
    (-ideal.cross(actual).y * 8.0).clamp(-1.3, 1.3)
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

    #[test]
    fn a_short_step_does_nothing() {
        let mut heli = Chopper::new(Vec3::ZERO, 0.0);
        heli.step(0.0, Vec3::X * 10.0, Vec3::X, false, &Specs::default());
        assert_eq!(heli.velocity, Vec3::ZERO);
    }
}
