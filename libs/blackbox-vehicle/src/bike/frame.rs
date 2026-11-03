//! The frame of the bike as a host sees it: the rest geometry a renderer poses the wheels from.

use glam::Vec3;

use super::Bike;

/// The rest geometry of the frame in the body frame (x right, y up, z forward, the origin on the ground midway between
/// the contact patches at rest).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RestPose {
    /// The front hub at rest, steering straight.
    pub front_hub: Vec3,
    /// The rear hub at rest.
    pub rear_hub: Vec3,
    /// The unit steering axis, pointing up and back along the fork. Steering turns the front wheel about it and the fork's
    /// compression moves the front hub along it.
    pub steer_axis: Vec3,
    /// The point where the steering axis meets the ground at rest.
    pub steer_ground: Vec3,
}

impl Bike {
    /// The rest geometry of the frame.
    pub fn rest_pose(&self) -> RestPose {
        let geometry = &self.geometry;
        RestPose {
            front_hub: geometry.front_hub,
            rear_hub: geometry.rear_hub,
            steer_axis: geometry.axis,
            steer_ground: geometry.axis_ground,
        }
    }

    /// Whether a gear change is under way (the drive is cut for its first part).
    pub fn shifting(&self) -> bool {
        self.powertrain.shifting()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::FIXED_STEP;
    use crate::bike::{BikeInput, BikeSpec, FRONT, REAR};
    use crate::ground::FlatGround;

    #[test]
    fn the_rest_pose_is_the_geometry_of_the_spec() {
        let spec = BikeSpec::example();
        let bike = Bike::new(spec.clone());
        let rest = bike.rest_pose();
        assert!((rest.front_hub.z - spec.chassis.wheelbase / 2.0).abs() < 1e-6);
        assert!((rest.rear_hub.z + spec.chassis.wheelbase / 2.0).abs() < 1e-6);
        assert!((rest.front_hub.y - spec.front.radius).abs() < 1e-6);
        assert!((rest.steer_axis.length() - 1.0).abs() < 1e-5);
        assert!((rest.steer_ground.z - rest.front_hub.z - spec.chassis.trail).abs() < 1e-6);
        // The axis leans back: its top is behind its foot.
        assert!(rest.steer_axis.z < 0.0 && rest.steer_axis.y > 0.0);
    }

    #[test]
    fn the_two_halves_of_a_step_are_a_step() {
        let ground = FlatGround::new(0.0);
        let input = BikeInput { throttle: 0.7, steer: 0.2, ..Default::default() };
        let mut whole = Bike::new(BikeSpec::example());
        let mut halves = Bike::new(BikeSpec::example());
        assert!(whole.place_moving(&ground, 0.0, 0.0, 2.0, 0.3, 12.0));
        assert!(halves.place_moving(&ground, 0.0, 0.0, 2.0, 0.3, 12.0));
        for _ in 0..240 {
            whole.step(FIXED_STEP, &input, &ground);
            halves.begin_body(FIXED_STEP);
            halves.respond(FIXED_STEP, &input, &ground);
        }
        assert_eq!(whole.position(), halves.position());
        assert_eq!(whole.orientation(), halves.orientation());
        assert_eq!(whole.wheel(FRONT).compression, halves.wheel(FRONT).compression);
        assert_eq!(whole.wheel(REAR).spin, halves.wheel(REAR).spin);
    }

    #[test]
    fn the_body_is_the_one_the_bike_moves() {
        let ground = FlatGround::new(0.0);
        let mut bike = Bike::new(BikeSpec::example());
        assert!(bike.place_moving(&ground, 1.0, 2.0, 2.0, 0.0, 8.0));
        assert_eq!(bike.body().position, bike.position());
        assert_eq!(bike.body().linear_velocity, bike.velocity());
        bike.body_mut().linear_velocity = Vec3::ZERO;
        assert_eq!(bike.velocity(), Vec3::ZERO);
    }
}
