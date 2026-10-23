//! Choosing what to draw this frame: frustum culling, the game's LOD rule
//! (`docs/specs/scenery-lod.md`, which also acts as the draw distance), then
//! sorting by mesh so the renderer can instance repeated models.
//!
//! The frame paths sweep the residency tile by tile ([`collect_main`], [`collect_rear`]): a tile's box is tested once,
//! and only the objects of a tile the view reaches are tested one by one, from compact cull data (`resident/cull.rs`).
//! The per-object rules live in `sweep.rs`; the flat collectors below run them on a plain list of objects.

use blackbox_gfx::Instance;
use blackbox_scene::Frustum;
use blackbox_scenery::layout::LodRules;
use glam::Mat4;

use super::detail::DetailState;
use super::props::PropWorld;
use super::resident::{CullTile, Placed};
use super::scenery_groups::SceneryGroups;

mod sort;
mod sweep;
#[cfg(test)]
mod tile_tests;

pub use sort::sort_by_mesh;
use sweep::Sweep;

/// Screen height the LOD thresholds are tuned for (`docs/specs/scenery-lod.md`).
pub(super) const REFERENCE_HEIGHT: f32 = 480.0;

/// The `prop_id` of a placed object that is not a registered prop. `PropWorld` hands out ids from 1 upwards
/// (`PropWorld::add_tile`), so no lookup in it ever finds this id.
pub(super) const NOT_A_PROP: u32 = 0;

pub struct Camera {
    pub view_proj: Mat4,
    pub position: [f32; 3],
    pub forward: [f32; 3],
    pub fov_y_radians: f32,
    pub light_dir: [f32; 3],
}

/// Include nearby casters outside the player frustum. Bevy culls these independently for each cascade.
/// The result is sorted by mesh. The frame paths append more after the scenery and use the tile collectors.
#[cfg(test)]
pub fn collect_with_shadows<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    shadow_distance: f32,
    out: &mut Vec<Instance>,
) {
    collect_flat(placed, camera, rules, props, None, (REFERENCE_HEIGHT, shadow_distance), out);
    sort_by_mesh(out);
}

/// A small environment capture needs its own projected-size LOD, rather than the player's 480px rule.
/// The result is sorted by mesh.
#[cfg(test)]
pub fn collect_for_height<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    height: f32,
    out: &mut Vec<Instance>,
) {
    collect_flat(placed, camera, rules, props, None, (height, 0.0), out);
    sort_by_mesh(out);
}

/// Where placed object `p` is drawn this frame, or `None` when its prop is knocked over. A non-prop has no entry in
/// `props`, so it skips the three lookups.
pub(super) fn current_pose(p: &Placed, props: &PropWorld) -> Option<(Mat4, blackbox_scene::Aabb)> {
    if p.prop_id == NOT_A_PROP {
        return Some((p.transform, p.bounds));
    }
    if props.hidden(p.prop_id) {
        return None;
    }
    // A moving prop is drawn where its body is, not where it was placed.
    Some(props.pose(p.prop_id).unwrap_or((p.transform, p.bounds)))
}

/// The per-object path over a plain list, the reference the tile collectors must match. `groups` decides which grouped
/// instances are drawn; `None` draws none of them.
#[cfg(test)]
fn collect_flat<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    groups: Option<&SceneryGroups>,
    sizes: (f32, f32),
    out: &mut Vec<Instance>,
) {
    let mut sweep = Sweep::new(camera, rules, props, groups, sizes, None, None, out);
    for p in placed {
        sweep.visit(p);
    }
}

/// The player's view: every tile of `tiles` (`Residency::cull_tiles`), unsorted. `original` lists the authored shadow
/// overlays too. `groups` decides which grouped instances are drawn.
#[allow(clippy::too_many_arguments)]
pub(super) fn collect_main<'a>(
    tiles: impl Iterator<Item = CullTile<'a>>,
    original: bool,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    groups: &SceneryGroups,
    shadow_distance: f32,
    detail: &mut DetailState,
    out: &mut Vec<Instance>,
) {
    let _trace = blackbox_trace::scope("World: visibility / caster selection");
    detail.begin();
    let mut sweep = Sweep::new(
        camera,
        rules,
        props,
        Some(groups),
        (REFERENCE_HEIGHT, shadow_distance),
        None,
        Some(&mut *detail),
        out,
    );
    for tile in tiles {
        sweep.visit_tile(&tile, original);
    }
    detail.finish();
}

/// A small filtered set, such as marked scenery, with the same main-view rules. Avoid collecting every unmarked mesh.
#[allow(clippy::too_many_arguments)]
pub(super) fn collect_subset<'a>(
    placed: impl Iterator<Item = &'a Placed>,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    groups: &SceneryGroups,
    shadow_distance: f32,
    detail: &mut DetailState,
    out: &mut Vec<Instance>,
) {
    detail.begin();
    let mut sweep = Sweep::new(
        camera,
        rules,
        props,
        Some(groups),
        (REFERENCE_HEIGHT, shadow_distance),
        None,
        Some(&mut *detail),
        out,
    );
    for p in placed {
        sweep.visit(p);
    }
    detail.finish();
}

/// The rear-view mirror's view, unsorted: nearby casters outside the frustum are included, and nothing whose origin is
/// farther than `far` from the camera is added (the mirror's far plane, `ReflectionDistance::mirror_far`), so the
/// capture needs no second pass to cut the list. The frame path appends more after it and sorts once.
#[allow(clippy::too_many_arguments)]
pub(super) fn collect_rear<'a>(
    tiles: impl Iterator<Item = CullTile<'a>>,
    original: bool,
    camera: &Camera,
    rules: &LodRules,
    props: &PropWorld,
    shadow_distance: f32,
    far: Option<f32>,
    out: &mut Vec<Instance>,
) {
    let _trace = blackbox_trace::scope("World: visibility / caster selection");
    let mut sweep = Sweep::new(camera, rules, props, None, (REFERENCE_HEIGHT, shadow_distance), far, None, out);
    for tile in tiles {
        sweep.visit_tile(&tile, original);
    }
}

/// The pose of a turned piece: its local x axis negated (the engine's `diag(-1, 1, 1) * M` in its row-vector
/// convention), the position kept. The bounds become the cube that holds the original box swung around the position,
/// which contains the mirrored box.
pub(super) fn turned_pose(transform: Mat4, bounds: &blackbox_scene::Aabb) -> (Mat4, blackbox_scene::Aabb) {
    let mirrored = transform * Mat4::from_scale(glam::Vec3::new(-1.0, 1.0, 1.0));
    let centre = transform.w_axis.truncate();
    let reach = (bounds.max - centre).abs().max((bounds.min - centre).abs()).length();
    (mirrored, blackbox_scene::Aabb { min: centre - glam::Vec3::splat(reach), max: centre + glam::Vec3::splat(reach) })
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
            group: 0,
            prop: None,
            prop_id: 0,
            key: InstanceKey::new(1, 1),
            original_shadow: false,
            casts_sun_shadow: true,
            spawner: None,
            parts: None,
            signal: None,
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
                shadow_tuning: None,
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

    #[test]
    fn a_turned_piece_is_mirrored_along_its_local_x_and_stays_inside_its_bounds() {
        // A piece 8 m long from its origin along a rotated x axis, as the race walls are placed.
        let transform = Mat4::from_rotation_translation(glam::Quat::from_rotation_z(0.4), Vec3::new(10.0, 20.0, 1.0));
        let far_end = transform.transform_point3(Vec3::new(8.0, 0.0, 0.0));
        let bounds =
            Aabb { min: Vec3::new(10.0, 20.0, 0.0).min(far_end), max: Vec3::new(10.0, 20.0, 2.0).max(far_end) };
        let (mirrored, swung) = turned_pose(transform, &bounds);
        let flipped_end = mirrored.transform_point3(Vec3::new(8.0, 0.0, 0.0));
        assert!(
            (flipped_end - (2.0 * Vec3::new(10.0, 20.0, 1.0) - far_end)).length() < 1e-4,
            "the far end swings over"
        );
        assert_eq!(mirrored.w_axis, transform.w_axis, "the position is kept");
        assert_eq!(mirrored.y_axis, transform.y_axis);
        assert!(swung.min.cmple(flipped_end).all() && swung.max.cmpge(flipped_end).all());
    }

    #[test]
    fn copies_appended_after_the_scenery_join_their_mesh_run_in_order() {
        let inst = |mesh: usize, key: u32| {
            Instance::keyed(MeshHandle::from_raw(mesh), Mat4::IDENTITY, InstanceKey::new(1, key))
        };
        // Scenery already sorted by mesh, then traffic and lamps appended after it as world_frame does.
        let mut list = vec![inst(1, 1), inst(1, 2), inst(2, 3)];
        list.extend([inst(2, 10), inst(1, 11), inst(3, 12), inst(2, 13)]);
        sort_by_mesh(&mut list);
        let meshes: Vec<usize> = list.iter().map(|i| i.mesh.raw()).collect();
        assert_eq!(meshes, [1, 1, 1, 2, 2, 2, 3], "every copy of a mesh is adjacent");
        let keys: Vec<u32> = list.iter().map(|i| i.key.parts().1).collect();
        assert_eq!(keys, [1, 2, 11, 3, 10, 13, 12], "the order inside each mesh is kept");
    }
}
