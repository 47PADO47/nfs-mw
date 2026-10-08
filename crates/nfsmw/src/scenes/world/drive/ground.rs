//! The world's collision as the ground the vehicle library drives on.

use blackbox_collision::{CollisionWorld, GROUP_EXCLUSION, RayOptions, SURFACE_NO_GROUND};
use blackbox_vehicle::ground::{Ground, GroundHit};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

/// Ray casts against the resident collision packs. Both sides use physics space (x right, y up, z
/// forward), so no conversion is needed here.
pub struct WorldGround<'a> {
    pub collision: &'a CollisionWorld,
    pub surfaces: &'a SurfaceTable,
}

impl Ground for WorldGround<'_> {
    fn hit(&self, origin: Vec3, dir: Vec3, max_distance: f32) -> Option<GroundHit> {
        // Tyres meet faces only; walls are the body's business. Faces flagged 0x08 are not ground (the
        // original's ground-height queries skip them), and scenery-group geometry is off in free roam.
        let options = RayOptions {
            barriers: false,
            exclude: u32::from(SURFACE_NO_GROUND) | u32::from(GROUP_EXCLUSION),
            ..RayOptions::default()
        };
        let end = origin + dir * max_distance;
        let hit = self.collision.ray_cast(origin.to_array(), end.to_array(), &options)?;
        Some(GroundHit {
            distance: hit.t * max_distance,
            normal: Vec3::from(hit.normal),
            surface: self.surfaces.grip(hit.surface_hash),
        })
    }
}
