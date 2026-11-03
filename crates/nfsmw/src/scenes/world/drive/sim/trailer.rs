//! What towing needs of the car: its rigid body for the joint, its size and its posture.

use blackbox_vehicle::rigid_body::RigidBody;
use glam::Vec3;

use super::CarSim;

impl CarSim {
    #[cfg(test)]
    pub fn physics_wheel_states(&self) -> [blackbox_vehicle::vehicle::WheelState; 4] {
        std::array::from_fn(|index| self.wheel_state(index))
    }

    /// Original named joint node in this body's local frame.
    pub fn joint_anchor(&self, name: u32, male: bool) -> Option<Vec3> {
        self.geometry.as_ref()?.joint(name, male)
    }

    pub fn joint_limits(&self, other: &Self, name: u32) -> Vec<blackbox_vehicle::rigid_body::AngularLimit> {
        match (&self.geometry, &other.geometry) {
            (Some(a), Some(b)) => a.joint_limits(b, name),
            _ => Vec::new(),
        }
    }

    /// Makes the car a body nobody drives (a trailer): it takes the brake and the handbrake it is given and
    /// otherwise rolls free, with no idle braking and no reversing by itself.
    pub fn configure_trailer(&mut self) {
        self.configure_ai();
        // Trailers use their separate suspension behavior in the original.
        self.vehicle.set_simple_chassis(None);
        let config = nfsmw_data::car::physics::trailer_chassis_config(self.vehicle.spec());
        self.vehicle.set_rolling_chassis(Some(config));
        self.vehicle.config.auto_brake = false;
        self.vehicle.config.auto_reverse = false;
    }

    pub fn set_trailer_hitched(&mut self, hitched: bool) {
        let cog = match hitched {
            true => nfsmw_data::car::physics::trailer_cog(self.vehicle.spec()),
            false => Vec3::ZERO,
        };
        self.vehicle.set_rolling_cog(cog);
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
