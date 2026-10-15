//! Existing helicopter position for camera-relative horizontal noise.

use glam::Vec3;

use crate::scenes::world::{ai::TrafficWorld, space};

impl TrafficWorld {
    pub fn camera_chopper_positions(&self) -> Vec<Vec3> {
        self.police
            .heli
            .as_ref()
            .map(|heli| space::to_render(heli.body.position.to_array()))
            .filter(|position| position.is_finite())
            .into_iter()
            .collect()
    }
}
