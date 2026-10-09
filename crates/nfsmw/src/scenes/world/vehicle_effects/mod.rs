//! Optional vehicle streaks, implemented from docs/specs/vehicle-visual-effects.md.

mod geometry;
mod sparks;
#[cfg(test)]
mod tests;
mod trails;

use blackbox_render::EffectVertex;
use glam::Vec3;
use nfsmw_data::vehicle_effects::VisualEffectsData;

use super::drive::{CarPose, VisualContact};
use sparks::Sparks;
use trails::Trails;

pub const MAX_SPARKS: usize = 768;
pub const MAX_TRAILS: usize = 256;

pub struct VehicleEffects {
    data: VisualEffectsData,
    sparks: Sparks,
    trails: Trails,
    enabled: [bool; 2],
    rear: Vec3,
    half: Vec3,
}

impl VehicleEffects {
    pub fn new(data: VisualEffectsData, rear: Vec3, half: Vec3) -> Self {
        Self { data, sparks: Sparks::default(), trails: Trails::default(), enabled: [false; 2], rear, half }
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
        if !dt.is_finite() || dt <= 0.0 || !velocity.is_finite() || !pose.position.is_finite() {
            return;
        }
        self.age(dt);
        if self.enabled[0] {
            self.sparks.emit(contacts, &self.data.collision, dt);
        }
        if self.enabled[1] {
            self.trails.emit(self.data.trail, pose, velocity, self.rear, self.half, dt);
        }
    }

    pub fn geometry(&self, camera: Vec3, forward: Vec3, show_trails: bool, out: &mut Vec<EffectVertex>) {
        self.sparks.geometry(camera, forward, out);
        if show_trails {
            self.trails.geometry(camera, forward, out);
        }
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
