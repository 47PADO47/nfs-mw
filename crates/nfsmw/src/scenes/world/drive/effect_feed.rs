use blackbox_attrib::Database;
use blackbox_gfx::RenderBackend;
use game_install::GameDir;
use glam::Vec3;

use super::sim::Telemetry;
use super::{CarPose, Drive};
use crate::scenes::world::exhaust::CarState;

impl Drive {
    pub(in crate::scenes::world) fn world_effect_interest(&self) -> Vec3 {
        self.position() + self.sim.as_ref().map_or(Vec3::ZERO, |sim| sim.effect_velocity()) * 0.15
    }

    pub fn spark_body(&self) -> (super::CarPose, nfsmw_data::car::physics::CarBounds) {
        (self.pose(), self.physics.bounds)
    }

    /// Age parked effects without emitting. Completed screenshot batches stay deterministic.
    pub fn age_effects(&mut self, dt: f32) {
        self.tail_trails.clear();
        self.effects.disconnect();
        self.vehicle_effects.disconnect();
        self.flames.disconnect();
        if !self.batch_run {
            self.effects.age(dt);
            self.vehicle_effects.age(dt);
            self.flames.age(dt);
        }
    }

    pub(in crate::scenes::world) fn take_impact_effects(
        &mut self,
    ) -> Vec<crate::scenes::world::impact_effects::Contact> {
        self.sim.as_mut().map_or_else(Vec::new, |sim| sim.take_impact_effects())
    }

    /// Free the flames' unloaded textures and load the car's flames when the setting is on and they are not
    /// loaded yet. A frame with the setting off and nothing to free does nothing.
    pub fn maintain_flames(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir, db: &Database) {
        self.flames.release(renderer);
        self.flames.load(renderer, dir, db, self.rig.model());
        self.flames.load_variant(renderer, dir);
    }
}

impl Drive {
    /// Load the pictures of the tire smoke variant when it changed, and free the ones of a variant that went.
    pub fn maintain_tire_smoke(&mut self, renderer: &mut dyn RenderBackend, dir: &GameDir) {
        self.effects.load_variant(renderer, dir);
    }
}

/// What the flames need of the car after a physics step.
pub(super) fn car_state(pose: &CarPose, t: &Telemetry, velocity: Vec3, throttle: f32) -> CarState {
    CarState { to_world: pose.transform(), velocity, gear: t.gear, throttle, nitrous: t.nos_burning }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn visual_engagement_uses_physics_burning_instead_of_available_nitrous() {
        let pose = CarPose { position: Vec3::ZERO, rotation: glam::Quat::IDENTITY, wheels: [Default::default(); 4] };
        let mut telemetry = Telemetry { has_nos: true, nos: 1.0, ..Default::default() };
        assert!(!car_state(&pose, &telemetry, Vec3::X, 1.0).nitrous);
        telemetry.nos_burning = true;
        assert!(car_state(&pose, &telemetry, Vec3::X, 1.0).nitrous);
        telemetry.nos = 0.0;
        telemetry.nos_burning = false;
        assert!(!car_state(&pose, &telemetry, Vec3::X, 1.0).nitrous);
    }
}
