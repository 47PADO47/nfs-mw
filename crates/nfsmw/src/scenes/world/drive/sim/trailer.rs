//! What towing needs of the car: its rigid body for the joint, its size and its posture.

use blackbox_vehicle::rigid_body::RigidBody;
use glam::Vec3;

use super::CarSim;

impl CarSim {
    /// Makes the car a body nobody drives (a trailer): it takes the brake and the handbrake it is given and
    /// otherwise rolls free, with no idle braking and no reversing by itself.
    pub fn configure_trailer(&mut self) {
        self.configure_ai();
        self.vehicle.config.auto_brake = false;
        self.vehicle.config.auto_reverse = false;
    }

    /// The rigid body, for a joint.
    pub fn body(&self) -> &RigidBody {
        self.vehicle.body()
    }

    /// The rigid body, for a joint to change its velocity.
    pub fn body_mut(&mut self) -> &mut RigidBody {
        self.vehicle.body_mut()
    }

    /// Half the size of the collision box, physics axes (x right, y up, z forward), metres.
    pub fn half_dimensions(&self) -> Vec3 {
        self.vehicle.spec().dimension
    }

    /// The car's up direction in the world (physics space).
    pub fn up(&self) -> Vec3 {
        self.vehicle.rotation().y_axis
    }

    /// Moves the car by `delta` (physics space) without touching its velocity.
    pub fn shift_by(&mut self, delta: Vec3) {
        self.vehicle.body_mut().position += delta;
    }
}
