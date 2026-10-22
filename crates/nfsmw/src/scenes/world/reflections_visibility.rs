//! Choose all six current reflection views in one scenery traversal.
use blackbox_gfx::Instance;
use blackbox_scene::Frustum;
use blackbox_scenery::{LodView, layout::LodRules};

use super::{props::PropWorld, resident::Placed, visibility};

/// Fractured prop instances and traffic keep their allocations between captures.
#[derive(Default)]
pub(super) struct Scratch {
    pub traffic: Vec<Instance>,
    parts: Vec<Instance>,
}

/// The caller applies authored-overlay rules once before
/// this traversal. Each face still chooses its own LOD and frustum membership.
pub(super) fn collect<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    cameras: &[visibility::Camera; 6],
    rules: &LodRules,
    props: &PropWorld,
    capture: (f32, Option<f32>),
    faces: &mut [Vec<Instance>; 6],
    scratch: &mut Scratch,
) {
    let (height, far) = capture;
    let _trace = blackbox_trace::scope("Reflection visibility: scenery traversal");
    let frustums = cameras.each_ref().map(|c| Frustum::from_view_proj(&c.view_proj));
    let views = cameras.each_ref().map(|c| LodView {
        position: c.position,
        forward: c.forward,
        pixel_scale: LodView::pixel_scale_for(c.fov_y_radians, height),
    });
    for face in faces.iter_mut() {
        face.clear();
    }
    for p in placed {
        // Reflection views have always omitted inactive scenery groups.
        if p.group != 0 {
            continue;
        }
        let Some((transform, bounds)) = visibility::current_pose(p, props) else { continue };
        if far.is_some_and(|far| bounds.distance_to(cameras[0].position.into()) > far) {
            continue;
        }
        let position = transform.w_axis.truncate().to_array();
        let slots = p.lods.map(|mesh| mesh.is_some());
        let selected: [Option<usize>; 6] = std::array::from_fn(|face| {
            if !frustums[face].intersects(&bounds) {
                return None;
            }
            rules.choose(&views[face], position, p.radius, p.flags, slots, p.detailed)
        });
        if selected.iter().all(Option::is_none) {
            continue;
        }
        scratch.parts.clear();
        let replaced =
            p.prop_id != visibility::NOT_A_PROP && props.model_instances(p.prop_id, transform, &mut scratch.parts);
        for (face, slot) in faces.iter_mut().zip(selected) {
            let Some(slot) = slot else { continue };
            if replaced {
                face.extend_from_slice(&scratch.parts);
                continue;
            }
            if let Some(mesh) = p.lods[slot] {
                face.push(Instance::keyed(mesh, transform, p.key));
            }
        }
    }
}

#[cfg(test)]
#[path = "reflections_visibility_tests.rs"]
mod tests;
