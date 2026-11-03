//! Where the wheels are on the frame: the steering head, the fork and the swingarm, and the contact patch of a leaned
//! round tyre.
//!
//! Body frame: x right, y up, z forward, the origin on the ground midway between the two contact patches at rest.

use glam::{Quat, Vec3};

use super::spec::{ChassisSpec, WheelSpec};

/// The rest geometry of the frame.
#[derive(Clone, Copy, Debug)]
pub struct Geometry {
    /// The front hub at rest, steering straight.
    pub front_hub: Vec3,
    /// The rear hub at rest.
    pub rear_hub: Vec3,
    /// The unit steering axis, pointing up and back along the fork.
    pub axis: Vec3,
    /// The point where the steering axis meets the ground at rest.
    pub axis_ground: Vec3,
}

impl Geometry {
    pub fn new(chassis: &ChassisSpec, front: &WheelSpec, rear: &WheelSpec) -> Self {
        let half = chassis.wheelbase * 0.5;
        let (sin, cos) = chassis.rake.sin_cos();
        Self {
            front_hub: Vec3::new(0.0, front.radius, half),
            rear_hub: Vec3::new(0.0, rear.radius, -half),
            axis: Vec3::new(0.0, cos, -sin),
            axis_ground: Vec3::new(0.0, 0.0, half + chassis.trail),
        }
    }

    /// The front wheel's pose in the body frame at handlebar angle `steer` (positive turns right) and fork compression
    /// `compression` (m): the hub, and the rotation that takes the wheel's own axes (x axle, y up, z forward) to the frame.
    pub fn front_pose(&self, steer: f32, compression: f32) -> (Vec3, Quat) {
        let turn = Quat::from_axis_angle(self.axis, steer);
        let hub = self.axis_ground + turn * (self.front_hub - self.axis_ground) + self.axis * compression;
        (hub, turn)
    }

    /// The rear wheel's pose at swingarm compression `compression` (m, measured upwards).
    pub fn rear_pose(&self, compression: f32) -> (Vec3, Quat) {
        (self.rear_hub + Vec3::Y * compression, Quat::IDENTITY)
    }
}

/// The lowest point of a round tyre with its hub at `hub`, in a wheel plane whose axle is the unit vector `axle`, over
/// ground whose unit normal is `normal`. The tyre is a torus: a ring of radius `radius - tube` in the wheel plane with a
/// tube of radius `tube` around it, so a leaned wheel stands on the shoulder of the tread and the contact patch moves
/// sideways as the bike leans.
pub fn contact_point(hub: Vec3, axle: Vec3, normal: Vec3, wheel: &WheelSpec) -> Vec3 {
    let tube = (wheel.toroid_factor.clamp(0.0, 1.0) * wheel.width * 0.5).min(wheel.radius * 0.9);
    let down = -normal;
    let in_plane = down - axle * down.dot(axle);
    let direction = in_plane.try_normalize().unwrap_or(down);
    hub + direction * (wheel.radius - tube) + down * tube
}

/// The lean of the wheel plane from the ground normal (rad), positive when the top of the wheel leans right: the
/// camber the tyre model takes.
pub fn camber(axle: Vec3, normal: Vec3) -> f32 {
    (-axle.dot(normal)).clamp(-1.0, 1.0).asin()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bike::BikeSpec;

    fn spec() -> BikeSpec {
        BikeSpec::example()
    }

    #[test]
    fn at_rest_the_hubs_are_one_radius_over_the_patches() {
        let s = spec();
        let g = Geometry::new(&s.chassis, &s.front, &s.rear);
        assert!((g.front_hub.z - s.chassis.wheelbase / 2.0).abs() < 1e-6);
        assert!((g.rear_hub.y - s.rear.radius).abs() < 1e-6);
    }

    #[test]
    fn the_axis_meets_the_ground_one_trail_ahead_of_the_patch() {
        let s = spec();
        let g = Geometry::new(&s.chassis, &s.front, &s.rear);
        // The line from the ground point along the axis passes at the fork offset from the hub.
        let w = g.front_hub - g.axis_ground;
        let offset = w.cross(g.axis).length();
        let expected = (s.front.radius * s.chassis.rake.tan() - s.chassis.trail) * s.chassis.rake.cos();
        assert!((offset - expected).abs() < 1e-5, "{offset} {expected}");
    }

    #[test]
    fn steering_right_turns_the_wheel_towards_the_right() {
        let s = spec();
        let g = Geometry::new(&s.chassis, &s.front, &s.rear);
        let (_, turn) = g.front_pose(0.2, 0.0);
        assert!((turn * Vec3::Z).x > 0.15);
    }

    #[test]
    fn compressing_the_fork_lifts_the_hub_along_the_axis() {
        let s = spec();
        let g = Geometry::new(&s.chassis, &s.front, &s.rear);
        let (low, _) = g.front_pose(0.0, 0.0);
        let (high, _) = g.front_pose(0.0, 0.05);
        assert!((high - low - g.axis * 0.05).length() < 1e-6);
    }

    #[test]
    fn an_upright_tyre_touches_one_radius_below_the_hub() {
        let s = spec();
        let hub = Vec3::new(0.0, 0.32, 0.0);
        let p = contact_point(hub, Vec3::X, Vec3::Y, &s.front);
        assert!((p - Vec3::ZERO).length() < 1e-6, "{p}");
    }

    #[test]
    fn a_leaned_tyre_stands_on_its_shoulder() {
        let s = spec();
        let lean = 40.0_f32.to_radians();
        let axle = Vec3::new(lean.cos(), -lean.sin(), 0.0);
        let hub = Vec3::new(0.0, 0.3, 0.0);
        let p = contact_point(hub, axle, Vec3::Y, &s.front);
        // The hub sits over the side the bike leans towards, so the patch is on the other side of it, below the hub
        // and nearer than one radius.
        assert!(p.x < 0.0 && p.y < hub.y);
        assert!(hub.y - p.y < s.front.radius);
        assert!((camber(axle, Vec3::Y) - lean).abs() < 1e-5);
    }
}
