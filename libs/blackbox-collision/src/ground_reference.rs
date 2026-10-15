//! Persistent body terrain reference built from a cached collision-triangle list.
//! Specification: docs/specs/vehicle-ground-reference.md. No ray or wheel-plane substitute.

mod candidates;
mod face;
mod geometry;
mod global;
pub use face::GroundFaceCache;
pub use global::PointQuerySettings;
#[cfg(test)]
mod install;
mod region;
#[cfg(test)]
mod tests;

use crate::math::Vec3;
use crate::{CollisionWorld, Triangle};
use region::Region;
pub use region::RegionSettings;

/// Caller-supplied query tolerances and cached-region sizing policy.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundReferenceSettings {
    pub region: RegionSettings,
    pub vertical_tolerance: f32,
    pub minimum_vertical_separation: f32,
    pub maximum_normal_y: f32,
    pub edge_length_epsilon: f32,
    pub continuation_mask: u8,
    pub instance_exclusion: u16,
    pub maximum_cell_span: u32,
    pub reuse_face: bool,
    pub preserve_face: bool,
}

/// A copied candidate face; streaming removal cannot invalidate its memory.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundFace {
    pub triangle: Triangle,
    pub surface_hash: u32,
    pub section: u32,
    pub instance: usize,
}

/// Saved normal and signed perpendicular separation from the body position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GroundPlane {
    pub normal: Vec3,
    pub separation: f32,
    pub surface_hash: u32,
    pub valid: bool,
}

impl Default for GroundPlane {
    fn default() -> Self {
        Self { normal: [0.0, 1.0, 0.0], separation: 0.0, surface_hash: 0, valid: false }
    }
}

/// Prepared region, copied candidate list and persistent selected face.
#[derive(Clone, Debug, Default)]
pub struct GroundReferenceCache {
    region: Region,
    candidates: Vec<GroundFace>,
    face: Option<GroundFace>,
    plane: GroundPlane,
    revision: Option<u64>,
    settings: Option<GroundReferenceSettings>,
}

impl GroundReferenceCache {
    pub fn update_global(
        &mut self,
        world: &CollisionWorld,
        position: Vec3,
        settings: &PointQuerySettings,
        group_enabled: impl FnMut(u16) -> bool,
    ) -> GroundPlane {
        global::update(&mut self.face, &mut self.plane, world, position, settings, group_enabled)
    }
    pub fn faces(&self) -> &[GroundFace] {
        &self.candidates
    }

    pub fn plane(&self) -> GroundPlane {
        self.plane
    }

    /// Clear on placement/reset; ordinary movement retains the selected plane.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Rebuild candidates after an external group/animation transition, retaining face history.
    pub fn invalidate_region(&mut self) {
        self.revision = None;
    }

    /// Refresh an awake body's terrain reference. Group state is supplied by the world owner.
    /// A negative/invalid radius or nonfinite position leaves the previous reference unchanged.
    pub fn update(
        &mut self,
        world: &CollisionWorld,
        position: Vec3,
        requested_radius: f32,
        settings: &GroundReferenceSettings,
        group_enabled: impl FnMut(u16) -> bool,
    ) -> GroundPlane {
        if !self.prepare(world, position, requested_radius, settings, group_enabled) {
            return self.plane;
        }
        self.select(position, settings);
        self.plane
    }

    /// A fresh selected face at `position` over a retained region centered elsewhere.
    /// No finite ray depth is used. Body selected-face history is left unchanged.
    pub fn point_height(
        &mut self,
        world: &CollisionWorld,
        position: Vec3,
        collection_center: Vec3,
        requested_radius: f32,
        settings: &GroundReferenceSettings,
        group_enabled: impl FnMut(u16) -> bool,
    ) -> Option<f32> {
        if !position.into_iter().all(f32::is_finite)
            || !self.prepare(world, collection_center, requested_radius, settings, group_enabled)
        {
            return None;
        }
        let mut selected = None;
        let mut plane = GroundPlane::default();
        face::select(&mut selected, &mut plane, &self.candidates, position, settings);
        let face = selected?;
        let normal = geometry::normal(face.triangle.pts, settings.maximum_normal_y);
        Some(geometry::height(face.triangle.pts[0], normal, position))
    }

    fn prepare(
        &mut self,
        world: &CollisionWorld,
        position: Vec3,
        requested_radius: f32,
        settings: &GroundReferenceSettings,
        group_enabled: impl FnMut(u16) -> bool,
    ) -> bool {
        if !position.into_iter().all(f32::is_finite) || !requested_radius.is_finite() || requested_radius < 0.0 {
            return false;
        }
        let changed = self.revision != Some(world.revision()) || self.settings != Some(*settings);
        if self.region.refresh(position, requested_radius, &settings.region, changed) {
            self.candidates =
                candidates::collect(world, self.region.center, self.region.radius, settings, group_enabled);
            self.revision = Some(world.revision());
            self.settings = Some(*settings);
        }
        true
    }

    fn select(&mut self, position: Vec3, settings: &GroundReferenceSettings) {
        face::select(&mut self.face, &mut self.plane, &self.candidates, position, settings);
    }
}
