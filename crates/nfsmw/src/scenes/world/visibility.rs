//! Choosing what to draw this frame: frustum culling, the game's LOD rule
//! (`docs/specs/scenery-lod.md`, which also acts as the draw distance), then
//! sorting by mesh so the renderer can instance repeated models.

use blackbox_gfx::Instance;
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
    pub light_dir: [f32; 3],
}

/// Include nearby casters outside the player frustum. Bevy culls these independently for each cascade.
pub fn collect_with_shadows<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    shadow_distance: f32,
    out: &mut Vec<Instance>,
) {
    collect_inner(placed, camera, rules, props, REFERENCE_HEIGHT, shadow_distance, out);
}

/// A small environment capture needs its own projected-size LOD, rather than the player's 480px rule.
pub fn collect_for_height<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    height: f32,
    out: &mut Vec<Instance>,
) {
    collect_inner(placed, camera, rules, props, height, 0.0, out);
}

fn collect_inner<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    height: f32,
    shadow_distance: f32,
    out: &mut Vec<Instance>,
) {
    let frustum = Frustum::from_view_proj(&camera.view_proj);
    let caster_ray = glam::Vec3::from_array(camera.light_dir).normalize_or_zero() * 80.0;
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
        // A moving prop is drawn where its body is, not where it was placed.
        let (transform, bounds) = props.pose(p.prop_id).unwrap_or((p.transform, p.bounds));
        let in_view = frustum.intersects(&bounds);
        // A margin along the light ray admits casters beyond the last receiver cascade.
        let caster =
            near_caster(&bounds, camera.position, shadow_distance) && reaches_view(&bounds, &frustum, caster_ray);
        if !in_view && !caster {
            continue;
        }
        let slots = p.lods.map(|m| m.is_some());
        let position = transform.w_axis.truncate().to_array();
        // Keep the player's projected-size LOD for visible geometry. A shadow must not change it.
        let visible_lod =
            in_view.then(|| rules.choose(&view, position, p.radius, p.flags, slots, p.detailed)).flatten();
        let shadow_lod = || {
            if !caster {
                return None;
            }
            // Off-camera LOD must not disappear merely because the object is behind the player.
            let shadow_view = LodView {
                forward: (glam::Vec3::from_array(position) - glam::Vec3::from_array(camera.position))
                    .normalize_or_zero()
                    .to_array(),
                ..view
            };
            rules.choose(&shadow_view, position, p.radius, p.flags, slots, p.detailed)
        };
        let Some(slot) = visible_lod.or_else(shadow_lod) else {
            continue;
        };
        if props.model_instances(p.prop_id, transform, out) {
            continue;
        }
        if let Some(mesh) = p.lods[slot] {
            out.push(Instance::keyed(mesh, transform, p.key));
        }
    }
    out.sort_unstable_by_key(|i| i.mesh);
}

/// Conservative swept bounds reject nearby objects whose shadows cannot reach the camera frustum.
fn reaches_view(bounds: &blackbox_scene::Aabb, frustum: &Frustum, ray: glam::Vec3) -> bool {
    frustum.intersects(&blackbox_scene::Aabb {
        min: bounds.min.min(bounds.min + ray),
        max: bounds.max.max(bounds.max + ray),
    })
}

fn near_caster(bounds: &blackbox_scene::Aabb, position: [f32; 3], distance: f32) -> bool {
    if distance <= 0.0 {
        return false;
    }
    let point = glam::Vec3::from_array(position);
    let closest = point.clamp(bounds.min, bounds.max);
    point.distance_squared(closest) <= (distance + 80.0).powi(2)
}

#[cfg(test)]
mod tests {
    use super::*;
    use blackbox_gfx::{InstanceKey, MeshHandle};
    use blackbox_scene::Aabb;
    use glam::Vec3;

    #[test]
    fn nearby_casters_survive_turning_the_camera_away() {
        let mesh = MeshHandle::from_raw(1);
        let p = Placed {
            lods: [Some(mesh); 4],
            detailed: None,
            transform: Mat4::from_translation(Vec3::new(0.0, -20.0, 0.0)),
            bounds: Aabb::new([-2.0, -22.0, -2.0], [2.0, -18.0, 2.0]),
            radius: 2.0,
            flags: 0,
            prop: None,
            prop_id: 0,
            key: InstanceKey::new(1, 1),
            original_shadow: false,
            casts_sun_shadow: true,
            spawner: None,
            parts: None,
            curve_lamp: None,
        };
        let lens = blackbox_gfx::reflection::face_frame(
            Vec3::ZERO,
            2,
            &blackbox_gfx::FrameParams {
                view: Mat4::IDENTITY,
                projection: blackbox_gfx::Projection::Identity,
                camera_position: Vec3::ZERO,
                light_dir: Vec3::NEG_Z,
                clear_color: [0.0; 3],
                fog: None,
                scattering: None,
                surface_light: None,
                camera_cut: false,
            },
        );
        let camera = Camera {
            view_proj: lens.view_proj(),
            position: [0.0; 3],
            forward: [0.0, 1.0, 0.0],
            fov_y_radians: std::f32::consts::FRAC_PI_2,
            light_dir: [0.0, 1.0, -0.25],
        };
        let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
        let props = PropWorld::default();
        let mut out = Vec::new();
        collect_with_shadows([&p].into_iter(), &camera, rules, &props, 0.0, &mut out);
        assert!(out.is_empty(), "the object is behind the camera");
        collect_with_shadows([&p].into_iter(), &camera, rules, &props, 80.0, &mut out);
        assert_eq!(out.len(), 1, "the same object still casts into nearby receiver cascades");
        assert_eq!(out[0].key, p.key);
        let away = Camera { light_dir: [0.0, -1.0, -0.25], ..camera };
        collect_with_shadows([&p].into_iter(), &away, rules, &props, 80.0, &mut out);
        assert!(out.is_empty(), "casters pointing away from the receiver view stay culled");
        assert!(!near_caster(&p.bounds, [1000.0; 3], 80.0), "distant scenery stays culled");
    }
}
