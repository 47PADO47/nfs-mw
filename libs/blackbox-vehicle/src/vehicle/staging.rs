//! Race staging restrains chassis motion after integration, independently of pedals and wheels.

use glam::Vec3;

use super::Vehicle;
use crate::rigid_body::BodyState;

impl Vehicle {
    pub(super) fn hold_staged_body(&mut self, previous_position: Vec3) {
        if !self.staging() || self.body.state == BodyState::Frozen {
            return;
        }
        let rotation = self.body.rotation();
        let local_velocity = rotation.transpose() * self.body.linear_velocity;
        let local_angular = rotation.transpose() * self.body.angular_velocity;
        self.body.linear_velocity = rotation * Vec3::new(0.0, local_velocity.y, 0.0);
        self.body.angular_velocity = rotation * Vec3::new(local_angular.x, 0.0, local_angular.z);
        self.body.position.x = previous_position.x;
        self.body.position.z = previous_position.z;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FlatGround, InputState, LaunchConfig, VehicleSpec};
    use glam::Quat;

    fn car() -> Vehicle {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.set_launch_config(Some(LaunchConfig {
            band_fraction: 0.25,
            redline_margin: 500.0,
            duration: 4.5,
            speed_cutoff: 26.8218,
            traction_gain: 0.25,
            staging_traction_scale: 0.25,
            cancel_on_shift: false,
        }));
        car
    }

    #[test]
    fn staging_preserves_integrated_height_and_local_vertical_roll_pitch_motion() {
        let mut car = car();
        car.place(Vec3::new(7.0, 4.0, -9.0), Quat::from_rotation_z(0.2) * Quat::from_rotation_y(0.6));
        car.body.linear_velocity = Vec3::new(3.0, -2.0, 5.0);
        car.body.angular_velocity = Vec3::new(0.4, 0.8, -0.3);
        car.body.apply_force(Vec3::new(1000.0, -2000.0, 500.0));
        car.body.apply_torque(Vec3::new(500.0, 1000.0, 1500.0));
        let previous = car.position();
        let mut normal = car.body.clone();
        normal.begin_frame(1.0 / 144.0);
        let rotation = normal.rotation();
        let velocity = rotation.transpose() * normal.linear_velocity;
        let angular = rotation.transpose() * normal.angular_velocity;
        car.set_staging(true);
        car.body.begin_frame(1.0 / 144.0);
        car.hold_staged_body(previous);
        assert_eq!(car.position(), Vec3::new(previous.x, normal.position.y, previous.z));
        assert_eq!(car.orientation(), normal.orientation);
        assert!((rotation.transpose() * car.linear_velocity() - Vec3::new(0.0, velocity.y, 0.0)).length() < 1e-6);
        assert!((rotation.transpose() * car.angular_velocity() - Vec3::new(angular.x, 0.0, angular.z)).length() < 1e-6);
    }

    #[test]
    fn staged_chassis_settles_vertically_while_the_engine_and_driven_wheels_rev() {
        let mut car = car();
        let ground = FlatGround::new(0.0);
        car.place_on_ground(&ground, 4.0, -3.0, 5.0, 0.0);
        let start = car.position();
        car.set_staging(true);
        for _ in 0..432 {
            car.step(1.0 / 144.0, &InputState { throttle: 0.7, ..Default::default() }, &ground);
        }
        assert!((car.position().x - start.x).abs() < 1e-5);
        assert!((car.position().z - start.z).abs() < 1e-5);
        assert!(car.position().y < start.y, "staging must allow suspension settling");
        assert!(car.powertrain.rpm() > car.spec.engine.idle + 500.0);
        assert!(car.tires.iter().any(|tire| tire.av.abs() > 1.0), "staging must preserve driven wheel spin");
        assert_eq!(car.controls.brake, 0.0);
        assert_eq!(car.controls.handbrake, 0.0);
    }

    #[test]
    fn staged_braking_does_not_invoke_the_chassis_sleep_wheel_reset() {
        let mut car = car();
        car.set_staging(true);
        car.wheels_on_ground = 4;
        car.controls.brake = 1.0;
        for tire in &mut car.tires {
            tire.av = 20.0;
        }
        car.damp_creep(false, Vec3::ZERO);
        assert!(car.tires.iter().all(|tire| tire.av == 20.0));
        car.set_staging(false);
        car.damp_creep(false, Vec3::ZERO);
        assert!(car.tires.iter().all(|tire| tire.av == 0.0));
    }
}
