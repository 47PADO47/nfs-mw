use blackbox_attrib::Database;
use blackbox_render::Renderer;
use game_install::GameDir;
use glam::Vec3;

use super::sim::Telemetry;
use super::{CarPose, Drive};
use crate::scenes::world::exhaust::CarState;

impl Drive {
    pub fn spark_body(&self) -> (super::CarPose, nfsmw_data::car::physics::CarBounds) {
        (self.pose(), self.physics.bounds)
    }

    /// The car was put somewhere else: break the tire tracks and drop the sparks and the flames.
    pub(super) fn reset_effects(&mut self) {
        self.effects.disconnect();
        self.vehicle_effects.clear();
        self.flames.clear();
    }

    /// Age parked effects without emitting. Completed screenshot batches stay deterministic.
    pub fn age_effects(&mut self, dt: f32) {
        self.effects.disconnect();
        self.vehicle_effects.disconnect();
        self.flames.disconnect();
        if !self.batch_run {
            self.effects.age(dt);
            self.vehicle_effects.age(dt);
            self.flames.age(dt);
        }
    }

    /// Free the flames' unloaded textures and load the car's flames when the setting is on and they are not
    /// loaded yet. A frame with the setting off and nothing to free does nothing.
    pub fn maintain_flames(&mut self, renderer: &mut Renderer, dir: &GameDir, db: &Database) {
        self.flames.release(renderer);
        self.flames.load(renderer, dir, db, self.rig.model());
    }
}

/// What the flames need of the car after a physics step.
pub(super) fn car_state(pose: &CarPose, t: &Telemetry, velocity: Vec3, throttle: f32) -> CarState {
    CarState { to_world: pose.transform(), velocity, gear: t.gear, nitrous: t.nos_burning, throttle }
}
