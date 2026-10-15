use super::*;
use crate::{Article, CollisionPack, Instance, PackedVert, Strip};

mod point_height;

fn settings() -> GroundReferenceSettings {
    GroundReferenceSettings {
        region: RegionSettings {
            initial_expansion: 1.1,
            motion_threshold: 5.0,
            prediction: 2.5,
            padding: 0.1,
            maximum_radius: 25.0,
        },
        vertical_tolerance: 0.025,
        minimum_vertical_separation: -1.0,
        maximum_normal_y: 0.9999,
        edge_length_epsilon: 1e-6,
        continuation_mask: 4,
        instance_exclusion: 0xC0,
        maximum_cell_span: 20,
        reuse_face: true,
        preserve_face: true,
    }
}

fn face(y: f32, flags: u8, hash: u32) -> GroundFace {
    GroundFace {
        triangle: Triangle { pts: [[-2.0, y, -2.0], [2.0, y, -2.0], [0.0, y, 2.0]], surface: 0, flags },
        surface_hash: hash,
        section: 1,
        instance: 0,
    }
}

fn close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 2e-6, "{actual} != {expected}");
}

#[test]
fn original_update_reuse_missing_and_order_vectors() {
    // Ten PC instruction-replay fixtures: floor selection, cache reuse, misses,
    // forced search, continuation flag and winding-dependent first-edge boundary.
    let mut c = GroundReferenceCache::default();
    let mut s = settings();
    c.candidates = vec![face(0.0, 0, 11)];
    c.select([0.0, 2.0, 0.0], &s);
    assert!(c.plane.valid);
    assert_eq!(c.plane.surface_hash, 11);
    close(c.plane.normal[1], 0.9999);
    close(c.plane.separation, -1.9998);
    c.candidates = vec![face(1.0, 0, 22)];
    c.select([0.0, 5.0, 0.0], &s);
    assert_eq!(c.plane.surface_hash, 11);
    close(c.plane.separation, -4.9995);
    c.candidates.clear();
    c.select([100.0, 7.0, 0.0], &s);
    assert!(c.plane.valid);
    assert_eq!(c.plane.surface_hash, 0);
    close(c.plane.separation, -6.9993);
    s.preserve_face = false;
    c.select([100.0, 7.0, 0.0], &s);
    assert!(!c.plane.valid);
    close(c.plane.separation, -1.0);
    close(c.plane.normal[1], 0.9999);

    c.clear();
    s = settings();
    c.candidates = vec![face(0.0, 0, 11), face(1.0, 0, 22)];
    c.select([0.0, 2.0, 0.0], &s);
    assert_eq!(c.plane.surface_hash, 11);
    c.candidates = vec![face(1.0, 0, 22)];
    s.reuse_face = false;
    c.select([0.0, 2.0, 0.0], &s);
    assert_eq!(c.plane.surface_hash, 22);
    close(c.plane.separation, -0.9999);
    c.clear();
    c.candidates = vec![face(0.0, 4, 11), face(1.0, 0, 22)];
    c.select([0.0, 2.0, 0.0], &s);
    assert_eq!(c.plane.surface_hash, 22);
    c.clear();
    c.select([0.0, 2.0, 0.0], &s);
    assert!(!c.plane.valid);
    close(c.plane.separation, -1.0);
    c.candidates = vec![face(0.0, 0, 11)];
    c.select([0.0, 2.0, -2.0], &s);
    assert!(c.plane.valid);
    c.candidates[0].triangle.pts.swap(0, 1);
    s.preserve_face = false;
    c.select([0.0, 2.0, -2.0], &s);
    assert!(!c.plane.valid);
}

#[test]
fn original_boundary_winding_vectors() {
    let triangle = face(0.0, 0, 0).triangle.pts;
    let mut reversed = triangle;
    reversed.swap(0, 1);
    for (p, a, b) in [
        ([0.0, 2.0, 0.0], true, true),
        ([0.0, 2.0, -2.0], true, false),
        ([1.0, 2.0, 0.0], true, true),
        ([-1.0, 2.0, 0.0], true, true),
        ([-2.0, 2.0, -2.0], true, false),
        ([2.0, 2.0, -2.0], true, false),
        ([0.0, 2.0, 2.0], true, true),
    ] {
        assert_eq!(geometry::contains(triangle, p), a);
        assert_eq!(geometry::contains(reversed, p), b);
    }
}

#[test]
fn independent_wheel_faces_share_candidates_and_retain_planes_on_edge_misses() {
    let mut s = settings();
    s.vertical_tolerance = 0.35;
    s.reuse_face = false;
    let mut wheels: [GroundFaceCache; 4] = std::array::from_fn(|_| GroundFaceCache::default());
    let floor = [face(0.0, 8, 11)]; // Body triangle-list path includes face flag eight.
    let first = wheels[0].update([0.0, 0.1, 0.0], &floor, &s);
    assert!(first.valid);
    close(first.separation, -0.09999);
    s.reuse_face = true;
    let raised = [face(0.08, 0, 22)];
    assert_eq!(wheels[0].update([0.0, 0.1, 0.0], &raised, &s).surface_hash, 11);
    assert_eq!(wheels[1].update([0.0, 0.1, 0.0], &raised, &s).surface_hash, 22);
    let retained = wheels[0].update([10.0, 0.1, 0.0], &[], &s);
    assert!(retained.valid);
    assert_eq!(retained.surface_hash, 0);
    close(retained.separation, -0.09999);
    s.reuse_face = false;
    assert_eq!(wheels[0].update([0.0, 0.1, 0.0], &raised, &s).surface_hash, 22);
    wheels[0].clear();
    let missing = wheels[0].update([0.0, 0.1, 0.0], &[], &s);
    assert!(!missing.valid);
    assert_eq!(missing.separation, -1.0);
}

#[test]
fn original_global_point_query_upright_tilted_and_filter_vectors() {
    let settings = PointQuerySettings {
        cache: GroundReferenceSettings { instance_exclusion: 0, reuse_face: false, ..settings() },
        minimum_face_offset: 0.5,
        unknown_winding_height: 1.0,
        downward_distance: 750.0,
        upright_face_exclusion: 8,
    };
    for (point, strip_flags, face_flags, tilt, translation, valid) in [
        ([0.0, 2.0, 0.0], 1, 0, 0.0f32, [0.0; 3], true),
        ([0.0, -0.1, 0.0], 1, 0, 0.0, [0.0; 3], false),
        ([0.0, -0.01, 0.0], 1, 0, 0.0, [0.0; 3], true),
        ([0.0, 2.0, 0.0], 1, 8, 0.0, [0.0; 3], false),
        ([0.0, 2.0, 0.0], 0, 0, 0.0, [0.0; 3], false),
        ([0.0, 2.0, 0.0], 2, 0, 0.0, [0.0; 3], true),
        ([10.0, 5.0, 0.0], 1, 0, 0.0, [-10.0, -3.0, 0.0], true),
        ([0.0, 2.0, 0.0], 1, 0, 0.2, [0.0; 3], true),
        ([0.0, 2.0, 0.0], 1, 8, 0.2, [0.0; 3], true),
        ([0.0, -0.1, 0.0], 1, 0, 0.2, [0.0; 3], false),
        ([10.0, 2.0, 0.0], 1, 0, 0.2, [0.0; 3], false),
    ] {
        let mut world = world();
        let mut pack = world.remove(1).unwrap();
        let instance = &mut pack.instances[0];
        instance.translation = translation;
        instance.flags = u16::from(tilt != 0.0);
        instance.row_z = [0.0, -tilt.sin(), tilt.cos()];
        pack.articles[0].strips[0].flags = strip_flags;
        pack.articles[0].strips[0].verts[2].flags = face_flags;
        world.insert(pack);
        let mut cache = GroundFaceCache::default();
        let plane = cache.update_global(&world, point, &settings, |_| false);
        assert_eq!(plane.valid, valid, "point={point:?}, tilt={tilt}, flags={face_flags}");
        if plane.valid {
            assert_eq!(plane.surface_hash, 11);
        }
    }
}

#[test]
fn predictive_region_reuse_and_motion() {
    let s = settings().region;
    let mut r = Region::default();
    assert!(r.refresh([0.0; 3], 3.0, &s, false));
    close(r.radius, 3.3);
    assert!(!r.refresh([0.1, 0.0, 0.0], 3.0, &s, false));
    assert!(r.refresh([0.4, 0.0, 0.0], 3.0, &s, false));
    close(r.center[0], 1.15);
    close(r.radius, 3.85);
    assert!(r.refresh([10.0, 0.0, 0.0], 3.0, &s, false));
    close(r.center[0], 10.0);
    close(r.radius, 3.3);
    assert!(r.refresh([10.0, 1.0, 0.0], 3.0, &s, false));
    close(r.center[1], 3.5);
    close(r.radius, 5.6);
}

fn world() -> CollisionWorld {
    let strip = Strip {
        center: [0.0; 3],
        radius: 4.0,
        flags: 0xC0,
        verts: [(-256, -256), (256, -256), (0, 256)]
            .map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags: 8 })
            .to_vec(),
    };
    let instance = Instance {
        row_x: [1.0, 0.0, 0.0],
        half_width: 2.0,
        row_z: [0.0, 0.0, 1.0],
        half_length: 2.0,
        translation: [0.0; 3],
        radius: 3.0,
        half_height: 1.0,
        iter_stamp: 0,
        flags: 0,
        group: 0,
        article: 0,
    };
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack {
        section: 1,
        instances: vec![instance],
        objects: vec![],
        articles: vec![Article {
            strips: vec![strip],
            barriers: vec![],
            surfaces: vec![11],
            intermediate_object: 0,
            flags: 0,
        }],
    });
    world
}

#[test]
fn candidate_group_mask_translation_and_streaming_boundaries() {
    let mut w = world();
    let s = settings();
    let faces = candidates::collect(&w, [0.0, 1000.0, 0.0], 3.0, &s, |_| false);
    assert_eq!(faces.len(), 1); // No vertical, strip or triangle flag exclusion.
    let mut pack = w.remove(1).unwrap();
    pack.instances[0].translation = [-10.0, -3.0, 0.0];
    pack.instances[0].group = 7;
    w.insert(pack);
    assert!(candidates::collect(&w, [10.0, 5.0, 0.0], 3.0, &s, |_| false).is_empty());
    let faces = candidates::collect(&w, [10.0, 5.0, 0.0], 3.0, &s, |group| group == 7);
    assert_eq!(faces[0].triangle.pts[0], [8.0, 3.0, -2.0]);
    let mut c = GroundReferenceCache::default();
    assert!(c.update(&w, [10.0, 5.0, 0.0], 3.0, &s, |_| true).valid);
    let revision = c.revision;
    w.remove(1);
    assert!(c.update(&w, [10.0, 5.0, 0.0], 3.0, &s, |_| true).valid);
    assert_ne!(c.revision, revision);
    assert!(c.candidates.is_empty()); // Copied face remains safe after unload.
    c.invalidate_region();
    c.clear();
    assert!(!c.plane().valid);
}

#[test]
fn candidate_instance_exclusion_and_unrotated_face_path() {
    let mut w = world();
    let s = settings();
    let mut pack = w.remove(1).unwrap();
    pack.instances[0].flags = 0xC0;
    w.insert(pack);
    assert!(candidates::collect(&w, [0.0; 3], 3.0, &s, |_| true).is_empty());
    let mut pack = w.remove(1).unwrap();
    pack.instances[0].flags = 0;
    pack.instances[0].row_x = [0.0, 0.0, 1.0];
    pack.instances[0].row_z = [-1.0, 0.0, 0.0];
    w.insert(pack);
    let faces = candidates::collect(&w, [0.0; 3], 3.0, &s, |_| true);
    assert_eq!(faces[0].triangle.pts[0], [-2.0, 0.0, -2.0]);
    assert_ne!(faces[0].triangle.pts[0], w.pack(1).unwrap().instances[0].to_world([-2.0, 0.0, -2.0]));
}
