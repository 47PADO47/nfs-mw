//! What a server replicates of a computer-driven car (`docs/specs/multiplayer/world-host.md`).

use glam::Vec3;

use super::AiCar;
use crate::scenes::world::drive::CarPose;
use crate::scenes::world::space;

impl AiCar {
    /// The pose after the last physics step (not interpolated) and the body's linear and angular velocity, all in
    /// render space.
    pub(in crate::scenes::world) fn stepped_motion(&self) -> (CarPose, Vec3, Vec3) {
        let body = self.sim.body();
        let linear = space::to_render(body.linear_velocity.to_array());
        let angular = space::to_render(body.angular_velocity.to_array());
        (self.current, linear, angular)
    }

    /// The brake pedal the driver holds, 0 to 1.
    pub(in crate::scenes::world) fn brake_pedal(&self) -> f32 {
        self.controls.brake
    }
}
