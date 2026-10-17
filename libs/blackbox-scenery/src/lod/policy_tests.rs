use super::*;
use crate::layout::MOST_WANTED;

fn view() -> LodView {
    LodView { position: [0.0; 3], forward: [1.0, 0.0, 0.0], pixel_scale: 343.0 }
}
fn object(distance: f32) -> LodObject {
    LodObject {
        position: [distance, 0.0, 0.0],
        radius: 4.0,
        flags: 0,
        slots: [true; 4],
        detailed: Some(LodModel { num_polys: 400, density: 30.0 }),
    }
}

#[test]
fn nearby_dense_geometry_has_a_distance_guard_and_missing_slots_stay_missing() {
    let rules = &MOST_WANTED.lod;
    let mut dense = object(24.9);
    dense.detailed.as_mut().unwrap().density = 1000.0;
    assert_eq!(rules.choose_with(&view(), dense, LodPolicy::default()), Some(0));
    dense.position[0] = 25.0;
    assert_eq!(rules.choose_with(&view(), dense, LodPolicy::default()), Some(2));
    dense.slots[2] = false;
    assert_eq!(rules.choose_with(&view(), dense, LodPolicy::default()), None);
    assert_eq!(rules.choose_with(&view(), dense, LodPolicy { highest: true, ..Default::default() }), Some(0));
}

#[test]
fn geometry_and_visibility_extensions_are_independent() {
    let rules = &MOST_WANTED.lod;
    let obj = object(35.0);
    assert_eq!(rules.choose_with(&view(), obj, LodPolicy::default()), Some(2));
    assert_eq!(rules.choose_with(&view(), obj, LodPolicy { geometry_scale: 2.0, ..Default::default() }), Some(0));
    assert_eq!(rules.choose_with(&view(), obj, LodPolicy { visibility_scale: 2.0, ..Default::default() }), Some(2));
    let distant = object(250.0);
    assert_eq!(rules.choose_with(&view(), distant, LodPolicy { geometry_scale: 2.0, ..Default::default() }), None);
    assert_eq!(rules.choose_with(&view(), distant, LodPolicy { visibility_scale: 2.0, ..Default::default() }), Some(2));
    assert_eq!(
        rules.choose_with(&view(), obj, LodPolicy { coarse_only: true, highest: true, ..Default::default() }),
        Some(2)
    );
}

#[test]
fn hysteresis_holds_either_side_only_inside_the_band() {
    let rules = &MOST_WANTED.lod;
    let obj = object(35.0); // 137 / 30, doubled = 9.13 (inside the 10% band).
    let policy = LodPolicy { geometry_scale: 2.0, hysteresis: 0.1, ..Default::default() };
    assert_eq!(rules.choose_with(&view(), obj, LodPolicy { previous: Some(0), ..policy }), Some(0));
    assert_eq!(rules.choose_with(&view(), obj, LodPolicy { previous: Some(2), ..policy }), Some(2));
    assert_eq!(rules.choose_with(&view(), object(60.0), LodPolicy { previous: Some(0), ..policy }), Some(2));
    assert_eq!(rules.choose_with(&view(), object(27.0), LodPolicy { previous: Some(2), ..policy }), Some(0));
    assert_eq!(
        rules.choose_with(&view(), LodObject { detailed: None, ..obj }, LodPolicy { previous: Some(2), ..policy }),
        Some(0)
    );
}

#[test]
fn full_scenery_keeps_one_authored_mesh_without_projected_size_pop() {
    let rules = &MOST_WANTED.lod;
    let policy = LodPolicy { highest: true, ..Default::default() };
    for distance in [25.0, 35.0, 80.0, 250.0, 2000.0] {
        let obj = object(distance);
        assert_eq!(rules.choose_with(&view(), obj, policy), Some(0));
        let coarse_only = LodObject { slots: [false, true, true, true], ..obj };
        assert_eq!(rules.choose_with(&view(), coarse_only, policy), Some(2));
        let special_view_only = LodObject { slots: [false, true, false, true], ..obj };
        assert_eq!(rules.choose_with(&view(), special_view_only, policy), None);
    }
    assert_eq!(rules.choose_with(&view(), object(2000.0), LodPolicy::default()), None);
}
