//! The world's collision as the ground the vehicle library drives on.

use std::cell::RefCell;

use blackbox_collision::{CandidateSet, CollisionWorld, GROUP_EXCLUSION, Hit, RayOptions, SURFACE_NO_GROUND};
use blackbox_vehicle::ground::{Ground, GroundHit};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

#[cfg(test)]
mod tests;

/// Ray casts against the resident collision packs. Both sides use physics space (x right, y up, z
/// forward), so no conversion is needed here.
pub struct WorldGround<'a> {
    pub collision: &'a CollisionWorld,
    pub surfaces: &'a SurfaceTable,
}

impl WorldGround<'_> {
    /// The tyre ray from `origin` along `dir`, cast by `cast`, as the vehicle library reads it.
    fn ground_hit(
        &self,
        origin: Vec3,
        dir: Vec3,
        max_distance: f32,
        cast: impl FnOnce([f32; 3], [f32; 3], &RayOptions) -> Option<Hit>,
    ) -> Option<GroundHit> {
        // Tyres meet faces only; walls are the body's business. Faces flagged 0x08 are not ground (the
        // original's ground-height queries skip them), and scenery-group geometry is off in free roam.
        let options = RayOptions {
            barriers: false,
            skip_groups: true,
            exclude: u32::from(SURFACE_NO_GROUND) | u32::from(GROUP_EXCLUSION),
            ..RayOptions::default()
        };
        let end = origin + dir * max_distance;
        let hit = cast(origin.to_array(), end.to_array(), &options)?;
        Some(GroundHit {
            distance: hit.t * max_distance,
            normal: Vec3::from(hit.normal),
            surface: self.surfaces.grip(hit.surface_hash),
        })
    }
}

impl Ground for WorldGround<'_> {
    fn surface_grip(&self, tag: u32) -> blackbox_vehicle::ground::SurfaceGrip {
        self.surfaces.grip(tag)
    }

    fn body_friction(&self, surface: u32) -> f32 {
        self.surfaces.body_friction(surface)
    }

    fn world_friction(&self, surface: u32) -> f32 {
        self.surfaces.world_friction(surface)
    }

    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        self.ground_hit(origin, dir, max_distance, |from, to, options| self.collision.ray_cast(from, to, options))
    }
}

/// [`WorldGround`] casting through a candidate window of its own rather than the collision world's shared one, so
/// cars integrated on several threads at once neither wait for nor rebuild each other's window. The hits are the same.
pub struct WindowedGround<'a> {
    pub world: WorldGround<'a>,
    pub window: RefCell<&'a mut CandidateSet>,
}

impl Ground for WindowedGround<'_> {
    fn surface_grip(&self, tag: u32) -> blackbox_vehicle::ground::SurfaceGrip {
        self.world.surface_grip(tag)
    }

    fn body_friction(&self, surface: u32) -> f32 {
        self.world.body_friction(surface)
    }

    fn world_friction(&self, surface: u32) -> f32 {
        self.world.world_friction(surface)
    }

    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        let collision = self.world.collision;
        let mut window = self.window.borrow_mut();
        self.world.ground_hit(origin, dir, max_distance, |from, to, options| {
            collision.ray_cast_in(from, to, options, &mut window)
        })
    }
}
