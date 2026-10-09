use glam::Vec3;
use nfsmw_data::car::physics::CarPhysics;
use nfsmw_data::vehicle_effects::VisualEffectsData;

use super::super::{space, vehicle_effects::VehicleEffects};
use super::Drive;

pub(super) fn vehicle_effects(data: VisualEffectsData, physics: &CarPhysics) -> VehicleEffects {
    let half = physics.spec.dimension;
    let rear = space::to_render(physics.bounds.pivot.to_array()) - Vec3::X * half.z;
    VehicleEffects::new(data, rear, Vec3::new(half.z, half.x, half.y))
}

impl Drive {
    /// Age parked effects without emitting. Completed screenshot batches stay deterministic.
    pub fn age_effects(&mut self, dt: f32) {
        self.effects.disconnect();
        self.vehicle_effects.disconnect();
        if !self.batch_run {
            self.effects.age(dt);
            self.vehicle_effects.age(dt);
        }
    }
}
