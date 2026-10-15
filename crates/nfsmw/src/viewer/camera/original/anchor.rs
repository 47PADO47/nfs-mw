//! Camera-anchor telemetry; this is distinct from chassis acceleration/assists.

use glam::Vec3;

#[derive(Debug, Clone, Default)]
pub struct History {
    previous: Option<(Vec3, f32)>,
}

impl History {
    pub fn acceleration(&mut self, position: Vec3, forward: Vec3, velocity: Vec3, dt: f32) -> Vec3 {
        let speed = velocity.length();
        let previous = self.previous.replace((position, speed));
        let Some((old_position, old_speed)) = previous else {
            return Vec3::ZERO;
        };
        if dt <= 0.0 || position.distance(old_position) / dt >= 300.0 {
            return Vec3::ZERO;
        }
        forward * ((speed - old_speed) / dt)
    }
}

pub fn drift(slip_radians: f32, absolute_speed: f32, grounded: bool) -> f32 {
    if !grounded {
        return 0.0;
    }
    (slip_radians.abs() / std::f32::consts::FRAC_PI_2).clamp(0.0, 1.0)
        * ((absolute_speed - 10.0) / 10.0).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_anchor_vectors_use_speed_derivative_along_forward() {
        for (old_speed, velocity, dt, position, forward, expected) in [
            (10.0, Vec3::new(20.0, 0.0, 0.0), 0.5, Vec3::ZERO, Vec3::X, Vec3::new(20.0, 0.0, 0.0)),
            (10.0, Vec3::new(0.0, 20.0, 0.0), 0.5, Vec3::ZERO, Vec3::X, Vec3::new(20.0, 0.0, 0.0)),
            (10.0, Vec3::new(20.0, 0.0, 0.0), 0.5, Vec3::ZERO, Vec3::Y, Vec3::new(0.0, 20.0, 0.0)),
            (20.0, Vec3::new(10.0, 0.0, 0.0), 0.25, Vec3::ZERO, Vec3::X, Vec3::new(-40.0, 0.0, 0.0)),
            (0.0, Vec3::new(20.0, 0.0, 0.0), 0.0, Vec3::ZERO, Vec3::X, Vec3::ZERO),
            (10.0, Vec3::new(20.0, 0.0, 0.0), 0.5, Vec3::new(150.0, 0.0, 0.0), Vec3::X, Vec3::ZERO),
            (10.0, Vec3::new(20.0, 0.0, 0.0), 0.5, Vec3::new(149.0, 0.0, 0.0), Vec3::X, Vec3::new(20.0, 0.0, 0.0)),
        ] {
            let mut history = History { previous: Some((Vec3::ZERO, old_speed)) };
            assert!((history.acceleration(position, forward, velocity, dt) - expected).length() < 2e-5);
        }
    }

    #[test]
    fn airborne_or_slow_vehicles_do_not_receive_grounded_drift_delay() {
        assert_eq!(drift(1.0, 30.0, false), 0.0);
        assert_eq!(drift(1.0, 5.0, true), 0.0);
        assert!((drift(std::f32::consts::FRAC_PI_4, 15.0, true) - 0.25).abs() < 1e-6);
    }
}
