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
    out.extend(placed.filter(|p| frustum.intersects(&p.bounds) && !props.hidden(p.prop_id)).filter_map(|p| {
        let slots = p.lods.map(|m| m.is_some());
        let slot = rules.choose(&view, p.position, p.radius, p.flags, slots, p.detailed)?;
        Some(Instance { mesh: p.lods[slot]?, transform: p.transform })
    }));
    out.sort_unstable_by_key(|i| i.mesh);
}
