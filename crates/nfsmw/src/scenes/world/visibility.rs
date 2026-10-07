//! Choosing what to draw this frame: distance and frustum culling, then sorting
//! by mesh so the renderer can instance repeated models.

use blackbox_render::Instance;
use blackbox_scene::Frustum;
use glam::{Mat4, Vec3};

use super::resident::Placed;

pub fn collect<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    view_proj: &Mat4,
    eye: Vec3,
    max_distance: f32,
    out: &mut Vec<Instance>,
) {
    let frustum = Frustum::from_view_proj(view_proj);
    out.clear();
    out.extend(
        placed
            .filter(|p| p.bounds.distance_to(eye) <= max_distance && frustum.intersects(&p.bounds))
            .map(|p| Instance { mesh: p.mesh, transform: p.transform }),
    );
    out.sort_unstable_by_key(|i| i.mesh);
}
