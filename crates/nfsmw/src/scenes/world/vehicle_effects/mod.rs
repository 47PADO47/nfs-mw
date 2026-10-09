//! Optional vehicle streaks, implemented from docs/specs/vehicle-visual-effects.md.

mod body;
mod geometry;
mod particle;
mod sparks;
mod sweep;
#[cfg(test)]
mod tests;
mod trails;

use blackbox_render::EffectVertex;
use glam::Vec3;
use nfsmw_data::vehicle_effects::VisualEffectsData;

use super::drive::{CarPose, VisualContact};
use sparks::Sparks;
use trails::Trails;

pub const MAX_SPARKS: usize = 2048;
pub const MAX_TRAILS: usize = 256;

pub struct VehicleEffects {
    data: VisualEffectsData,
    sparks: Sparks,
    trails: Trails,
    enabled: [bool; 2],
    body: Option<body::BodyClip>,
}

impl VehicleEffects {
    pub fn new(data: VisualEffectsData) -> Self {
        Self { data, sparks: Sparks::default(), trails: Trails::default(), enabled: [false; 2], body: None }
    }

    pub fn set_body(&mut self, pose: CarPose, bounds: nfsmw_data::car::physics::CarBounds) {
        self.body = body::BodyClip::new(pose, bounds);
    }

    pub fn set_enabled(&mut self, sparks: bool, trails: bool) {
        self.enabled = [sparks, trails];
        if !sparks {
            self.sparks.clear();
        }
        if !trails {
            self.trails.clear();
        }
    }

    pub fn clear(&mut self) {
        self.sparks.clear();
        self.trails.clear();
    }

    /// A reset or camera parking ends emission and disconnects old trail geometry.
    pub fn disconnect(&mut self) {
        self.sparks.disconnect();
        self.trails.clear();
    }

    pub fn age(&mut self, dt: f32) {
        if !dt.is_finite() || dt <= 0.0 {
            return;
        }
        self.sparks.age(dt);
        self.trails.age(dt);
    }

    pub fn step(&mut self, contacts: &[VisualContact], pose: CarPose, velocity: Vec3, dt: f32) {
        if !dt.is_finite()
            || dt <= 0.0
            || !velocity.is_finite()
            || !pose.position.is_finite()
            || !pose.rotation.is_normalized()
        {
            return;
        }
        self.age(dt);
        if self.enabled[0] {
            self.sparks.emit(contacts, &self.data.collision, velocity, dt);
        }
        if self.enabled[1] {
            self.trails.emit(self.data.trail, pose, velocity, dt);
        }
    }

    pub fn bounce(&mut self, world: &blackbox_collision::CollisionWorld, dt: f32) {
        self.sparks.bounce(world, dt);
    }

    pub fn geometry(&self, camera: Vec3, forward: Vec3, show_trails: bool, out: &mut Vec<EffectVertex>) {
        self.sparks.geometry(camera, forward, self.body, out);
        if show_trails {
            self.trails.geometry(camera, forward, out);
        }
    }

    pub fn glows(&self, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        self.sparks.glows(camera, forward, self.body, out);
    }

    pub fn status(&self) -> String {
        format!(
            "sparks {}: {}/{} live, {} emitted; speed trails {}: {}/{} live, {} emitted",
            self.enabled[0],
            self.sparks.len(),
            MAX_SPARKS,
            self.sparks.emitted,
            self.enabled[1],
            self.trails.len(),
            MAX_TRAILS,
            self.trails.emitted,
        )
    }
}
