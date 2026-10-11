//! Choosing what to draw this frame: frustum culling, the game's LOD rule
//! (`docs/specs/scenery-lod.md`, which also acts as the draw distance), then
//! sorting by mesh so the renderer can instance repeated models.

use blackbox_render::Instance;
use blackbox_scene::Frustum;
use blackbox_scenery::LodView;
use blackbox_scenery::layout::LodRules;
use glam::Mat4;

use super::props::PropWorld;
use super::resident::Placed;

/// Screen height the LOD thresholds are tuned for (`docs/specs/scenery-lod.md`).
const REFERENCE_HEIGHT: f32 = 480.0;

pub struct Camera {
    pub view_proj: Mat4,
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub fov_y_radians: f32,
}

pub fn collect<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    out: &mut Vec<Instance>,
) {
    let frustum = Frustum::from_view_proj(&camera.view_proj);
    let view = LodView {
        position: camera.position,
        forward: camera.forward,
        pixel_scale: LodView::pixel_scale_for(camera.fov_y_radians, REFERENCE_HEIGHT),
    };
    out.clear();
    for p in placed {
        let (transform, bounds) = props.pose(p.prop_id).unwrap_or((p.transform, p.bounds));
        if !frustum.intersects(&bounds) {
            continue;
        }
        let slots = p.lods.map(|m| m.is_some());
        let position = transform.w_axis.truncate().to_array();
        let Some(slot) = rules.choose(&view, position, p.radius, p.flags, slots, p.detailed) else {
            continue;
        };
        if props.model_instances(p.prop_id, transform, out) {
            continue;
        }
        if let Some(mesh) = p.lods[slot] {
            out.push(Instance { mesh, transform });
        }
    }
    out.sort_unstable_by_key(|i| i.mesh);
}
