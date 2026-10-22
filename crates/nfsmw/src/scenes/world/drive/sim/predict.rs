//! The car's physics as a multiplayer client predicts it (`docs/specs/multiplayer/sync-model.md` section 2): save and
//! restore everything a later step depends on, and read or overwrite the reduced state the server sends back, in the
//! render space the entity views use (x forward, y left, z up; the pose is the model's origin and axes, as
//! [`CarSim::pose`] draws it).

use blackbox_vehicle::Vehicle;
use glam::{Mat3, Quat, Vec3};

use super::CarSim;
use crate::scenes::world::object_geometry::BodyPose;
use crate::scenes::world::space;

/// Everything a later step of the car depends on, for rewinding. The ground caches are left out: they are caches, and
/// a restore clears them. The damage engine and the reset-collision window are not rewound (a prediction does not
/// take damage twice; sync-model.md section 4.2).
#[derive(Debug, Clone)]
pub struct SimSave {
    vehicle: Vehicle,
    spin: [f32; 4],
    before_velocity: Vec3,
    previous_contact_pose: BodyPose,
    brake_lamp_age: f32,
}

/// The reduced motion of the car in render space: the model's origin and axes, and the body's velocities.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SimMotion {
    pub position: Vec3,
    pub rotation: Quat,
    /// Metres per second.
    pub linear: Vec3,
    /// Radians per second, mapped axis by axis as the entity views map it (`net/export.rs`).
    pub angular: Vec3,
    pub rpm: f32,
    /// The wire's gear: reverse 0, neutral 1, first 2.
    pub gear: u8,
    /// Nitrous left, 0 to 1.
    pub nitrous: f32,
}

impl CarSim {
    pub fn save(&self) -> SimSave {
        SimSave {
            vehicle: self.vehicle.clone(),
            spin: self.spin,
            before_velocity: self.before_velocity,
            previous_contact_pose: self.previous_contact_pose,
            brake_lamp_age: self.brake_lamp_age,
        }
    }

    pub fn restore(&mut self, saved: &SimSave) {
        self.vehicle = saved.vehicle.clone();
        self.spin = saved.spin;
        self.before_velocity = saved.before_velocity;
        self.previous_contact_pose = saved.previous_contact_pose;
        self.brake_lamp_age = saved.brake_lamp_age;
        self.ground_reference.clear();
        for wheel in self.wheel_ground.get_mut() {
            wheel.clear();
        }
    }

    /// The reduced motion now.
    pub fn motion(&self) -> SimMotion {
        let pose = self.pose();
        let body = self.vehicle.body();
        let telemetry = self.telemetry();
        SimMotion {
            position: pose.position,
            rotation: pose.rotation,
            linear: space::to_render(body.linear_velocity.to_array()),
            angular: space::to_render(body.angular_velocity.to_array()),
            rpm: telemetry.rpm,
            gear: u8::try_from(telemetry.gear.saturating_add(1)).unwrap_or(0),
            nitrous: telemetry.nos,
        }
    }

    /// Overwrite the body's pose and velocities with `motion` (the inverse of [`CarSim::motion`]); the engine, gear,
    /// wheels and nitrous keep their own state. A motion that is not finite is ignored.
    pub fn set_motion(&mut self, motion: &SimMotion) {
        let finite = motion.position.is_finite()
            && motion.rotation.is_finite()
            && motion.linear.is_finite()
            && motion.angular.is_finite();
        if !finite {
            return;
        }
        let model = Mat3::from_quat(motion.rotation.normalize());
        let physics = |v: Vec3| Vec3::from_array(space::to_physics(v));
        // The model's axes (forward, left, up) are the body's z, -x and y (`CarSim::pose`).
        let rotation = Mat3::from_cols(-physics(model.y_axis), physics(model.z_axis), physics(model.x_axis));
        let orientation = Quat::from_mat3(&rotation);
        let origin = physics(motion.position);
        let body = self.vehicle.body_mut();
        body.set_orientation(orientation);
        body.position = origin + rotation * self.pivot;
        body.linear_velocity = physics(motion.linear);
        body.angular_velocity = physics(motion.angular);
        self.previous_contact_pose = (self.vehicle.position(), self.vehicle.body().orientation);
    }
}
