use super::*;

#[test]
fn fresh_point_height_has_no_artificial_lower_depth_and_allows_a_tolerated_face_above() {
    let mut world = world();
    let s = settings();
    let mut cache = GroundReferenceCache::default();
    let center = [0.0, 1.0, 0.0];
    assert_eq!(cache.point_height(&world, [0.0, 10000.0, 0.0], center, 3.0, &s, |_| false), Some(0.0));
    let mut pack = world.remove(1).unwrap();
    pack.articles[0].strips[0].center[1] = 1.0;
    world.insert(pack);
    assert_eq!(cache.point_height(&world, [0.0; 3], center, 3.0, &s, |_| false), Some(1.0));
    let mut pack = world.remove(1).unwrap();
    pack.articles[0].strips[0].center[1] = 1.03;
    world.insert(pack);
    assert_eq!(cache.point_height(&world, [0.0; 3], center, 3.0, &s, |_| false), None);
}

#[test]
fn point_and_collection_center_are_independent_and_missing_regions_do_not_reuse_a_face() {
    let mut world = world();
    let s = settings();
    let mut cache = GroundReferenceCache::default();
    let query = [0.0, 2.0, 0.0];
    assert_eq!(cache.point_height(&world, query, [0.0; 3], 3.0, &s, |_| false), Some(0.0));
    // The point still lies over the floor, but the independently collected region excludes it.
    assert_eq!(cache.point_height(&world, query, [100.0, 0.0, 0.0], 3.0, &s, |_| false), None);
    cache.clear();
    assert_eq!(cache.point_height(&world, query, [0.0; 3], 3.0, &s, |_| false), Some(0.0));
    world.remove(1);
    assert_eq!(cache.point_height(&world, query, [0.0; 3], 3.0, &s, |_| false), None);
}

#[test]
fn fresh_point_selection_retains_native_face_order_and_continuation() {
    let mut world = world();
    let s = settings();
    let mut pack = world.remove(1).unwrap();
    let mut upper = pack.articles[0].strips[0].clone();
    upper.center[1] = 2.0;
    pack.articles[0].strips.push(upper);
    world.insert(pack);
    let mut cache = GroundReferenceCache::default();
    let point = [0.0, 3.0, 0.0];
    assert_eq!(cache.point_height(&world, point, [0.0; 3], 3.0, &s, |_| false), Some(0.0));
    let mut pack = world.remove(1).unwrap();
    pack.articles[0].strips[0].verts[2].flags = 4;
    world.insert(pack);
    assert_eq!(cache.point_height(&world, point, [0.0; 3], 3.0, &s, |_| false), Some(2.0));
}

#[test]
fn fresh_point_query_does_not_replace_a_retained_body_reference() {
    let mut world = world();
    let s = settings();
    let mut cache = GroundReferenceCache::default();
    let old = cache.update(&world, [0.0, 2.0, 0.0], 3.0, &s, |_| false);
    assert!(old.valid);
    let mut pack = world.remove(1).unwrap();
    pack.articles[0].strips[0].center[1] = 3.0;
    world.insert(pack);
    assert_eq!(cache.point_height(&world, [0.0, 5.0, 0.0], [0.0; 3], 3.0, &s, |_| false), Some(3.0));
    assert_eq!(cache.plane(), old);
    assert_eq!(cache.point_height(&world, [f32::NAN, 0.0, 0.0], [0.0; 3], 3.0, &s, |_| false), None);
}
