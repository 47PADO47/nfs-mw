//! The world's collision as the ground the vehicle library drives on.

use blackbox_collision::{CollisionWorld, RayOptions};
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
        // Tyres meet faces only; walls are the body's business.
        let options = RayOptions { barriers: false, ..RayOptions::default() };
        let end = origin + dir * max_distance;
        let hit = self.collision.ray_cast(origin.to_array(), end.to_array(), &options)?;
        Some(GroundHit {
            distance: hit.t * max_distance,
            normal: Vec3::from(hit.normal),
            surface: self.surfaces.grip(hit.surface_hash),
        })
    }
}
