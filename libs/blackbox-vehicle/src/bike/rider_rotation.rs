//! The rider's body turning on the bike (`docs/specs/ride6-rider-body-aero.md`, "Rider rotation"): a rigid body held to the
//! frame's attitude by an angular spring and damper. It lags the bike's roll, pitch and yaw, and puts the reaction torque on
//! the bike once the bike is moving.

use glam::Vec3;

use super::curve::ramp;
use super::rider_spec::RiderSpec;

/// The speeds (m/s) over which the reaction torque on the bike fades in: none at a standstill, all of it at 5 km/h.
pub const REACTION_FADE_SPEED: [f32; 2] = [0.0, 5.0 / 3.6];

/// The most the rider's body may turn from the bike's attitude about an axis (rad): a guard against a collision's spin.
const MAX_ANGLE: f32 = std::f32::consts::FRAC_PI_2;

/// The rider's attitude against the bike's, as a small rotation vector in the body frame (rad; the axes of the physics space:
/// x right = pitch, y up = yaw, z forward = roll, each in the right-handed sense), and his own angular velocity (rad/s).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct RiderRotation {
    pub angle: Vec3,
    pub velocity: Vec3,
}

impl RiderRotation {
    /// Advance by `dt` while the bike turns at `omega` (body frame, rad/s) at `speed` m/s; `mass` is the rider's mass (kg).
    /// Returns the torque the rider's body puts on the bike (body frame, N m).
    ///
    /// The rider's body is a rigid body of moment `inertia_scale * mass` per axis. The spring torque is `-K angle`; the damper
    /// works on his angular velocity against the bike's scaled by the chassis factor: `-C (w_r - f w_bike)`, applied
    /// implicitly. The torque on the bike is the opposite of the rider's, per axis scaled, and fades in with the speed.
    pub fn step(&mut self, rider: &RiderSpec, mass: f32, omega: Vec3, speed: f32, dt: f32) -> Vec3 {
        if rider.angular_k <= 0.0 || mass <= 0.0 || dt <= 0.0 || !omega.is_finite() {
            *self = Self::default();
            return Vec3::ZERO;
        }
        let inertia = (rider.inertia_scale * mass).max(Vec3::splat(1e-3));
        let factor = rider.angular_damp_chassis_factor;
        let spring = -self.angle * rider.angular_k;
        let toward = omega * factor;
        let damp = rider.angular_damp.max(0.0);
        // Implicit in the damper: w' = (w + dt (spring + C f w_bike) / I) / (1 + dt C / I).
        let numerator = self.velocity + (spring + toward * damp) * dt / inertia;
        self.velocity = numerator / (Vec3::ONE + dt * damp / inertia);
        self.angle = (self.angle + (self.velocity - omega) * dt).clamp(Vec3::splat(-MAX_ANGLE), Vec3::splat(MAX_ANGLE));
        let on_rider = -self.angle * rider.angular_k - (self.velocity - toward) * damp;
        -on_rider * rider.torque_scale * ramp(speed.abs(), REACTION_FADE_SPEED)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// The rider's lean against the bike's (rad), positive to the right: a roll to the right is a negative turn about the
    /// forward axis.
    pub fn roll_right(&self) -> f32 {
        -self.angle.z
    }

    /// His pitch against the bike's (rad), positive nose up: a turn about the right axis lowers the nose.
    pub fn pitch_up(&self) -> f32 {
        -self.angle.x
    }

    /// His yaw against the bike's (rad), positive to the right.
    pub fn yaw_right(&self) -> f32 {
        self.angle.y
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec() -> RiderSpec {
        RiderSpec {
            inertia_scale: Vec3::new(0.23, 0.28, 0.19),
            angular_k: 2500.0,
            angular_damp: 250.0,
            angular_damp_chassis_factor: 0.0,
            ..RiderSpec::default()
        }
    }

    fn run(rotation: &mut RiderRotation, omega: Vec3, seconds: f32) -> Vec3 {
        let mut torque = Vec3::ZERO;
        for _ in 0..(seconds * 120.0) as usize {
            torque = rotation.step(&spec(), 70.0, omega, 20.0, 1.0 / 120.0);
        }
        torque
    }

    #[test]
    fn a_bike_rolling_at_a_steady_rate_drags_the_rider_behind_by_the_damper() {
        // The damper works against the world (factor 0): the rider holds back, the spring pulls him along.
        let mut rider = RiderRotation::default();
        let omega = Vec3::new(0.0, 0.0, -0.5);
        run(&mut rider, omega, 3.0);
        // Steady: he turns with the bike, so the spring torque equals the damper's -C w: angle = -C w / K.
        assert!((rider.velocity - omega).length() < 1e-3, "{:?}", rider.velocity);
        assert!((rider.angle.z - 250.0 * 0.5 / 2500.0).abs() < 1e-3, "{:?}", rider.angle);
        assert_eq!(rider.angle.x, 0.0);
    }

    #[test]
    fn against_the_bike_the_rider_follows_it_and_the_lag_is_the_spring_over_the_damper() {
        let mut rider = RiderRotation::default();
        let against_bike = RiderSpec { angular_damp_chassis_factor: 1.0, ..spec() };
        for _ in 0..360 {
            rider.step(&against_bike, 70.0, Vec3::new(0.0, 0.0, -0.5), 20.0, 1.0 / 120.0);
        }
        assert!(rider.angle.length() < 1e-3, "no lag when the damper works against the bike: {:?}", rider.angle);
    }

    #[test]
    fn he_settles_back_to_the_bikes_attitude() {
        let mut rider = RiderRotation { angle: Vec3::new(0.1, -0.2, 0.3), velocity: Vec3::ZERO };
        run(&mut rider, Vec3::ZERO, 3.0);
        assert!(rider.angle.length() < 1e-3 && rider.velocity.length() < 1e-2, "{rider:?}");
    }

    #[test]
    fn the_reaction_torque_opposes_the_rider_and_fades_in_with_the_speed() {
        let displaced = RiderRotation { angle: Vec3::new(0.0, 0.0, 0.1), velocity: Vec3::ZERO };
        let torque_at = |speed: f32| {
            let mut rider = displaced;
            rider.step(&spec(), 70.0, Vec3::ZERO, speed, 1e-4)
        };
        // The spring pulls the rider back toward -z, so the bike is pushed toward +z.
        assert!(torque_at(20.0).z > 200.0, "{:?}", torque_at(20.0));
        assert_eq!(torque_at(0.0), Vec3::ZERO);
        let half = torque_at(5.0 / 3.6 / 2.0).z;
        assert!((half / torque_at(20.0).z - 0.5).abs() < 0.02);
        let mut scaled = spec();
        scaled.torque_scale = Vec3::new(1.0, 1.0, 0.0);
        let mut rider = displaced;
        assert_eq!(rider.step(&scaled, 70.0, Vec3::ZERO, 20.0, 1e-4).z, 0.0);
    }

    #[test]
    fn a_rigid_rider_or_a_bad_step_does_nothing() {
        let mut rider = RiderRotation { angle: Vec3::X * 0.1, velocity: Vec3::ZERO };
        let rigid = RiderSpec { angular_k: 0.0, ..spec() };
        assert_eq!(rider.step(&rigid, 70.0, Vec3::ONE, 20.0, 0.01), Vec3::ZERO);
        assert_eq!(rider, RiderRotation::default());
        assert_eq!(rider.step(&spec(), 70.0, Vec3::splat(f32::NAN), 20.0, 0.01), Vec3::ZERO);
        assert_eq!(rider.step(&spec(), 0.0, Vec3::ZERO, 20.0, 0.01), Vec3::ZERO);
    }

    #[test]
    fn the_readouts_follow_the_conventions_of_the_bike() {
        let rider = RiderRotation { angle: Vec3::new(0.1, 0.2, -0.3), velocity: Vec3::ZERO };
        assert_eq!(rider.roll_right(), 0.3);
        assert_eq!(rider.pitch_up(), -0.1);
        assert_eq!(rider.yaw_right(), 0.2);
    }
}
