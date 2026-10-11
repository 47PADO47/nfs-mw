//! Acceleration observed by the contact camera after every shared-world response.

use glam::Vec3;

use super::CarSim;
use crate::scenes::world::space;

impl CarSim {
    pub fn camera_acceleration(&self) -> Vec3 {
        space::to_render(
            ((self.body().linear_velocity - self.before_velocity) / blackbox_vehicle::FIXED_STEP).to_array(),
        )
    }
}
