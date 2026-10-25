//! The tile shortcut must draw exactly what the per-object path draws.

use super::sweep::TileView;
use super::*;
use crate::scenes::world::resident::TileCull;
use crate::viewer::camera::Lens;
use blackbox_collision::CollisionWorld;
use blackbox_gfx::reflection::distance::within;
use blackbox_gfx::{InstanceKey, MeshHandle};
use blackbox_scene::Aabb;
use glam::Vec3;

const GROUP: u16 = 3;
const TILES: i32 = 5;
const TILE_SIZE: f32 = 120.0;

/// A reproducible value in `[0, 1)`.
fn next(seed: &mut u32) -> f32 {
    *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
    (*seed >> 8) as f32 / (1u32 << 24) as f32
}

fn object(key: InstanceKey, at: Vec3, half: f32) -> Placed {
    Placed {
        lods: [Some(MeshHandle::from_raw(1 + key.parts().1 as usize % 3)), Some(MeshHandle::from_raw(10)), None, None],
        detailed: None,
        transform: Mat4::from_translation(at),
        bounds: Aabb { min: at - Vec3::splat(half), max: at + Vec3::splat(half) },
        radius: 6.0,
        flags: 0,
        group: 0,
        prop: None,
        prop_id: 0,
        key,
        original_shadow: false,
        casts_sun_shadow: true,
        spawner: None,
        parts: None,
        signal: None,
        sway_lods: 0,
    }
}

struct World {
    tiles: Vec<Vec<Placed>>,
    culls: Vec<TileCull>,
    collision: CollisionWorld,
}

impl World {
    /// A grid of tiles, each with objects of every kind the sweep treats differently: plain, authored shadow overlays,
    /// grouped, turnable and props.
    fn grid(group_in: bool) -> Self {
        let mut seed = 7;
        let mut tiles = Vec::new();
        for tx in -TILES / 2..=TILES / 2 {
            for ty in -TILES / 2..=TILES / 2 {
                let mut placed = Vec::new();
                for i in 0..40u32 {
                    let tile = tiles.len() as u32;
                    let at = Vec3::new(
                        (tx as f32 + next(&mut seed)) * TILE_SIZE,
                        (ty as f32 + next(&mut seed)) * TILE_SIZE,
                        next(&mut seed) * 10.0,
                    );
                    let half = 1.0 + next(&mut seed) * 12.0;
                    let mut p = object(InstanceKey::new(tile, i), at, half);
                    p.original_shadow = i % 5 == 0;
                    p.group = if i % 11 == 0 { GROUP } else { 0 };
                    p.flags = if i % 7 == 0 { blackbox_scenery::INSTANCE_TURNABLE } else { 0 };
                    p.prop_id = if i % 13 == 0 { 100 + i } else { 0 };
                    placed.push(p);
                }
                tiles.push(placed);
            }
        }
        let mut collision = CollisionWorld::new(None);
        collision.set_group_enabled(GROUP, group_in);
        let culls = tiles.iter().map(|placed| TileCull::build(placed)).collect();
        Self { tiles, culls, collision }
    }

    fn cull_tiles(&self) -> impl Iterator<Item = CullTile<'_>> {
        let collision = &self.collision;
        self.tiles.iter().zip(&self.culls).map(move |(placed, cull)| CullTile { placed, cull, collision })
    }
}

fn camera(eye: Vec3, target: Vec3) -> Camera {
    let lens = Lens::look_at(eye, target, 60.0, 0.1, 1.6);
    Camera {
        view_proj: lens.view_proj(),
        position: lens.position.to_array(),
        forward: lens.forward.to_array(),
        fov_y_radians: lens.fov_y_degrees.to_radians(),
        light_dir: [0.3, 0.2, -1.0],
    }
}

fn cameras() -> Vec<Camera> {
    vec![
        camera(Vec3::new(10.0, -40.0, 2.0), Vec3::new(10.0, 200.0, 2.0)),
        camera(Vec3::new(-150.0, 90.0, 5.0), Vec3::new(300.0, 120.0, 5.0)),
        camera(Vec3::new(0.0, 0.0, 30.0), Vec3::new(1.0, 1.0, 0.0)),
        camera(Vec3::new(900.0, 900.0, 2.0), Vec3::new(1000.0, 900.0, 2.0)),
    ]
}

fn digest(list: &[Instance]) -> Vec<(usize, u64, [f32; 16])> {
    list.iter().map(|i| (i.mesh.raw(), i.key.0, i.transform.to_cols_array())).collect()
}

/// The per-object path over the same objects, in the same order.
fn reference(
    world: &World,
    original: bool,
    camera: &Camera,
    groups: &SceneryGroups,
    shadow_distance: f32,
    far: Option<f32>,
    detail: Option<&mut DetailState>,
) -> Vec<Instance> {
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let mut out = Vec::new();
    let groups = detail.is_some().then_some(groups);
    let mut sweep =
        Sweep::new(camera, rules, &props, groups, (REFERENCE_HEIGHT, shadow_distance), None, detail, &mut out);
    for tile in world.cull_tiles() {
        for p in tile.placed.iter().filter(|p| (original || !p.original_shadow) && tile.group_shown(p.group)) {
            sweep.visit(p);
        }
    }
    if let Some(far) = far {
        let origin = Vec3::from_array(camera.position);
        out.retain(|i| within(origin, i.transform.w_axis.truncate(), Some(far)));
    }
    out
}

#[test]
fn the_main_view_draws_what_the_per_object_path_draws() {
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let mut drawn = 0;
    for group_in in [false, true] {
        let world = World::grid(group_in);
        let mut groups = SceneryGroups::default();
        if group_in {
            groups.switch(GROUP, Some(true));
        }
        for original in [false, true] {
            for shadow_distance in [0.0, 80.0] {
                for camera in cameras() {
                    let mut detail = DetailState::default();
                    let expected =
                        reference(&world, original, &camera, &groups, shadow_distance, None, Some(&mut detail));
                    let mut detail = DetailState::default();
                    let mut out = vec![Instance::new(MeshHandle::from_raw(99), Mat4::IDENTITY)];
                    collect_main(
                        world.cull_tiles(),
                        original,
                        &camera,
                        rules,
                        &props,
                        &groups,
                        shadow_distance,
                        &mut detail,
                        None,
                        &mut out,
                    );
                    assert_eq!(digest(&out), digest(&expected), "group {group_in}, original {original}");
                    drawn += out.len();
                }
            }
        }
    }
    assert!(drawn > 100, "the comparison must include drawn geometry, not only empty views");
}

#[test]
fn the_rear_view_cuts_at_the_far_plane_while_scanning() {
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let world = World::grid(false);
    let groups = SceneryGroups::default();
    let mut drawn = 0;
    let mut cut = 0;
    for camera in cameras() {
        for shadow_distance in [0.0, 80.0] {
            let mut unlimited = Vec::new();
            collect_rear(world.cull_tiles(), false, &camera, rules, &props, shadow_distance, None, &mut unlimited);
            for far in [192.0, 40.0] {
                let mut out = Vec::new();
                collect_rear(world.cull_tiles(), false, &camera, rules, &props, shadow_distance, Some(far), &mut out);
                // What the capture's own cut (`cull_instances`) would have left of the unlimited scan.
                let origin = Vec3::from_array(camera.position);
                let mut expected = unlimited.clone();
                expected.retain(|i| within(origin, i.transform.w_axis.truncate(), Some(far)));
                assert_eq!(digest(&out), digest(&expected), "far {far}");
                drawn += out.len();
                cut += unlimited.len() - out.len();
            }
        }
    }
    assert!(drawn > 50 && cut > 50, "the limit must both keep and drop geometry");
    // Unused by the rear view, but the reference must agree there too: no groups, no detail state.
    let reference = reference(&world, false, &cameras()[0], &groups, 80.0, None, None);
    let mut out = Vec::new();
    collect_rear(world.cull_tiles(), false, &cameras()[0], rules, &props, 80.0, None, &mut out);
    assert_eq!(digest(&out), digest(&reference));
}

#[test]
fn a_tile_is_classified_by_its_box() {
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let camera = camera(Vec3::new(10.0, -40.0, 2.0), Vec3::new(10.0, 200.0, 2.0));
    let mut out = Vec::new();
    let sweep = Sweep::new(&camera, rules, &props, None, (REFERENCE_HEIGHT, 0.0), None, None, &mut out);
    let around = |centre: Vec3, half: Vec3| Aabb { min: centre - half, max: centre + half };
    let ahead = around(Vec3::new(10.0, 10.0, 2.0), Vec3::splat(1.0));
    let behind = around(Vec3::new(10.0, -300.0, 2.0), Vec3::splat(1.0));
    let across = around(Vec3::new(10.0, 10.0, 2.0), Vec3::new(400.0, 1.0, 1.0));
    assert_eq!(sweep.classify(Some(&ahead), Some(&ahead)), TileView::Inside);
    assert_eq!(sweep.classify(Some(&behind), Some(&behind)), TileView::Outside);
    assert_eq!(sweep.classify(Some(&across), Some(&across)), TileView::Partial);
    assert_eq!(sweep.classify(None, None), TileView::Outside, "a tile of props only is swept by prop");
}

#[test]
fn a_prop_is_swept_even_when_its_tile_is_out_of_view() {
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let camera = camera(Vec3::ZERO, Vec3::Y * 100.0);
    // The tile's static object is behind the camera. The prop is ahead of it: the prop world may have moved it
    // anywhere, so the tile's box says nothing about it.
    let behind = object(InstanceKey::new(1, 1), Vec3::new(0.0, -200.0, 0.0), 3.0);
    let mut prop = object(InstanceKey::new(1, 2), Vec3::new(0.0, 30.0, 0.0), 3.0);
    prop.prop_id = 77;
    let placed = vec![behind, prop];
    let cull = TileCull::build(&placed);
    let collision = CollisionWorld::new(None);
    let tile = CullTile { placed: &placed, cull: &cull, collision: &collision };
    let mut out = Vec::new();
    collect_rear(std::iter::once(tile), false, &camera, rules, &props, 0.0, None, &mut out);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].key, InstanceKey::new(1, 2));
}

#[test]
fn cull_data_that_no_longer_matches_its_list_falls_back_to_the_per_object_path() {
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let camera = camera(Vec3::ZERO, Vec3::Y * 100.0);
    let mut placed = vec![object(InstanceKey::new(1, 1), Vec3::new(0.0, 30.0, 0.0), 3.0)];
    let cull = TileCull::build(&placed);
    placed.push(object(InstanceKey::new(1, 2), Vec3::new(2.0, 40.0, 0.0), 3.0));
    let collision = CollisionWorld::new(None);
    let tile = CullTile { placed: &placed, cull: &cull, collision: &collision };
    let mut out = Vec::new();
    collect_rear(std::iter::once(tile), false, &camera, rules, &props, 0.0, None, &mut out);
    assert_eq!(out.len(), 2);
}

#[test]
fn flagged_lods_sway_only_in_a_swaying_view() {
    let props = PropWorld::default();
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let camera = &cameras()[0];
    let frame = blackbox_scenery::sway::SwayFrame { phase: 90.0, axis: [0.0, 0.0, 0.0] };
    let draw = |sway_lods: u8, sway: Option<blackbox_scenery::sway::SwayFrame>| {
        let mut tree = object(InstanceKey::new(1, 1), Vec3::new(10.0, 0.0, 2.0), 2.0);
        tree.sway_lods = sway_lods;
        let mut out = Vec::new();
        Sweep::new(camera, rules, &props, None, (REFERENCE_HEIGHT, 0.0), None, None, &mut out)
            .with_sway(sway)
            .visit(&tree);
        assert_eq!(out.len(), 1);
        (tree.transform, out[0].transform)
    };
    let (rest, drawn) = draw(0b11, None);
    assert_eq!(drawn, rest, "a view without sway draws the resting pose");
    let (rest, drawn) = draw(0, Some(frame));
    assert_eq!(drawn, rest, "an unflagged solid does not sway");
    let (rest, drawn) = draw(0b11, Some(frame));
    assert_ne!(drawn, rest);
    assert_eq!(drawn.w_axis, rest.w_axis, "the sway turns about the object's origin");
    let turn = glam::Mat3::from_mat4(drawn);
    assert!((turn * turn.transpose()).abs_diff_eq(glam::Mat3::IDENTITY, 1e-5));
    assert!(turn.z_axis.angle_between(Vec3::Z) < 3.1_f32.to_radians());
}
