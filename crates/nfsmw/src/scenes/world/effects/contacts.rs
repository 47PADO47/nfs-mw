//! Project a loaded tire patch onto the same resident faces the vehicle ground query uses.

use blackbox_collision::{CollisionWorld, GROUP_EXCLUSION, RayOptions, SURFACE_NO_GROUND};
use blackbox_vehicle::WheelState;
use glam::Vec3;

use super::super::space;

#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub point: Vec3,
    pub normal: Vec3,
    pub forward: Vec3,
    pub section: u32,
    pub skid: f32,
    pub smoke: f32,
    /// Displayed tread width; the physics-only fallback keeps the original visual width.
    pub width: f32,
}

/// `forward` is the steered wheel direction in physics space.
pub fn project(wheel: WheelState, forward: Vec3, collision: &CollisionWorld) -> Option<Contact> {
    if !wheel.on_ground || !wheel.position.is_finite() {
        return None;
    }
    let options = RayOptions {
        barriers: false,
        exclude: u32::from(SURFACE_NO_GROUND) | u32::from(GROUP_EXCLUSION),
        ..RayOptions::default()
    };
    let from = wheel.position + Vec3::Y * 0.35;
    let to = wheel.position - Vec3::Y * 0.35;
    let hit = collision.ray_cast(from.to_array(), to.to_array(), &options)?;
    let normal = space::to_render(hit.normal).normalize_or_zero();
    let forward = space::to_render(forward.to_array());
    let forward = (forward - normal * forward.dot(normal)).normalize_or_zero();
    if normal.length_squared() < 0.9 || forward.length_squared() < 0.9 {
        return None;
    }
    Some(Contact {
        point: space::to_render(hit.point),
        normal,
        forward,
        section: hit.section,
        skid: wheel.skid.clamp(0.0, 1.0),
        smoke: wheel.smoke.clamp(0.0, 1.0),
        width: 0.24,
    })
}
