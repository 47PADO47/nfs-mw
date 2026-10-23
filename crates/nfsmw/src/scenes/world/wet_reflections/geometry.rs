//! The road image has its own authored models, visibility and local reflection transform.
use super::super::{props::PropWorld, resident::ReflectionPlaced, visibility::Camera};
use blackbox_gfx::Instance;
use blackbox_scene::Frustum;
use blackbox_scenery::LodView;
use glam::Mat4;

pub(super) fn scenery<'a>(
    placed: impl Iterator<Item = &'a ReflectionPlaced>,
    camera: &Camera,
    props: &PropWorld,
    height: f32,
    out: &mut Vec<Instance>,
) {
    let frustum = Frustum::from_view_proj(&camera.view_proj);
    let view = LodView {
        position: camera.position,
        forward: camera.forward,
        pixel_scale: LodView::pixel_scale_for(camera.fov_y_radians, height),
    };
    out.clear();
    for p in placed {
        if props.hidden(p.prop_id) {
            continue;
        }
        let (transform, bounds) = props.pose(p.prop_id).unwrap_or((p.transform, p.bounds));
        if !frustum.intersects(&bounds) {
            continue;
        }
        let size = view.projected_size(transform.w_axis.truncate().to_array(), p.radius + 6.0);
        let Some(slot) = slot(p.flags, size) else { continue };
        let Some(mesh) = p.lods[slot] else { continue };
        let instance = Instance::keyed(mesh, scenery_transform(transform, p.flags), p.key);
        out.push(instance.with_flipped_winding(p.flip_winding()));
    }
    out.sort_unstable_by_key(|instance| instance.mesh);
}

fn slot(flags: u32, raw_size: i32) -> Option<usize> {
    if raw_size < 2 {
        return None;
    }
    let size = raw_size.saturating_add(if flags & 0x0200_0000 != 0 { 10 } else { 0 });
    if size < 32 {
        return None;
    }
    if flags & 0x80 != 0 {
        return Some(2);
    }
    if flags & 0x0100_0100 != 0 {
        return Some(0);
    }
    Some(3)
}

fn scenery_transform(mut transform: Mat4, flags: u32) -> Mat4 {
    if flags & 0x200 != 0 {
        return Mat4::IDENTITY;
    }
    transform.z_axis.z = -transform.z_axis.z;
    transform
}

/// The body callback keeps its own origin and flips its complete local Z axis.
/// Call on a body/root pose; child/wheel composition remains the vehicle adapter's job.
pub(super) fn vehicle_root(mut transform: Mat4) -> Mat4 {
    transform.z_axis = -transform.z_axis;
    transform
}

#[cfg(test)]
mod tests;
