//! A damped tip response; the nozzle always follows the current rendered marker.
use glam::{Mat4, Vec3};

#[derive(Default)]
pub(super) struct Motion {
    previous: Option<(Vec3, Vec3, Vec3)>,
    pub bend: Vec3,
    /// Damped heading rotation in world radians per simulation second.
    pub angular_velocity: Vec3,
    pub stretch: f32,
    pub speed: f32,
    pub velocity: Vec3,
}

impl Motion {
    pub fn step(&mut self, dt: f32, pose: Mat4, velocity: Vec3) {
        if dt <= 0.0 {
            return;
        }
        self.speed = velocity.length();
        self.velocity = velocity;
        let position = pose.transform_point3(Vec3::ZERO);
        let forward = pose.transform_vector3(Vec3::X).normalize_or_zero();
        let previous = self.previous.replace((position, forward, velocity));
        let Some((old_position, old_forward, old_velocity)) = previous else {
            self.stretch = 1.0;
            return;
        };
        if position.distance(old_position) > 10.0 + self.speed * dt * 4.0 {
            self.bend = Vec3::ZERO;
            self.angular_velocity = Vec3::ZERO;
            self.stretch = 1.0;
            return;
        }
        let acceleration = ((velocity - old_velocity) / dt).clamp_length_max(60.0);
        let turn = (forward - old_forward) / dt;
        // Turning already explains lateral acceleration during ordinary cornering.
        let residual = acceleration - turn * velocity.dot(forward);
        let cross = old_forward.cross(forward);
        let angle = cross.length().atan2(old_forward.dot(forward).clamp(-1.0, 1.0));
        let angular = cross.normalize_or_zero() * (angle / dt).min(12.0);
        // Respond promptly to a sharp turn, then let the tail settle more gradually.
        let damping = match angular.length_squared() > self.angular_velocity.length_squared() {
            true => 18.0,
            false => 8.0,
        };
        self.angular_velocity = self.angular_velocity.lerp(angular, 1.0 - (-damping * dt).exp());
        let target = (-residual * 0.0006).clamp_length_max(0.045);
        let weight = 1.0 - (-12.0 * dt).exp();
        self.bend = self.bend.lerp(target, weight);
        let stretch = (1.0 + acceleration.dot(forward) * 0.002).clamp(0.92, 1.08);
        self.stretch += (stretch - self.stretch) * weight;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn turning(hz: u32) -> Motion {
        let mut motion = Motion::default();
        for i in 0..=hz {
            let t = i as f32 / hz as f32;
            let pose = Mat4::from_rotation_z(t * 0.6);
            motion.step(1.0 / hz as f32, pose, pose.transform_vector3(Vec3::X * 40.0));
        }
        motion
    }

    #[test]
    fn turn_response_is_smooth_and_consistent_across_physics_rates() {
        let a = turning(60);
        let b = turning(144);
        assert!(a.angular_velocity.z > 0.5);
        assert!(a.angular_velocity.distance(b.angular_velocity) < 0.001);
        assert!((a.stretch - b.stretch).abs() < 0.001);
    }

    #[test]
    fn teleport_resets_tip_response() {
        let mut motion = turning(60);
        motion.step(1.0 / 60.0, Mat4::from_translation(Vec3::X * 1000.0), Vec3::ZERO);
        assert_eq!(motion.bend, Vec3::ZERO);
        assert_eq!(motion.angular_velocity, Vec3::ZERO);
        assert_eq!(motion.stretch, 1.0);
    }

    #[test]
    fn bump_response_lags_vertical_acceleration_and_settles() {
        let mut motion = Motion::default();
        motion.step(0.1, Mat4::IDENTITY, Vec3::X * 40.0);
        let velocity = Vec3::new(40.0, 0.0, 2.0);
        motion.step(0.1, Mat4::IDENTITY, velocity);
        assert!(motion.bend.z < -0.005);
        let peak = motion.bend.length();
        for _ in 0..20 {
            motion.step(0.1, Mat4::IDENTITY, velocity);
        }
        assert!(motion.bend.length() < peak * 0.01);
    }
}
