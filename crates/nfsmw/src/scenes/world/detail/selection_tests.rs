use super::*;
use blackbox_gfx::MeshHandle;
use blackbox_scene::Aabb;
use glam::{Mat4, Vec3};

fn near_object(key: InstanceKey) -> Placed {
    let at = Vec3::new(0.0, 20.0, 0.0);
    Placed {
        lods: [Some(MeshHandle::from_raw(1)), Some(MeshHandle::from_raw(2)), None, None],
        detailed: None,
        transform: Mat4::from_translation(at),
        bounds: Aabb { min: at - Vec3::splat(3.0), max: at + Vec3::splat(3.0) },
        radius: 3.0,
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

#[test]
fn an_extended_range_remembers_only_what_the_last_pass_chose() {
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let view =
        LodView { position: [0.0; 3], forward: [0.0, 1.0, 0.0], pixel_scale: LodView::pixel_scale_for(1.0, 480.0) };
    let mut state = DetailState::default();
    state.configure(DetailSettings { geometry_range: DetailRange::Extended, ..Default::default() });
    let (a, b) = (near_object(InstanceKey::new(1, 1)), near_object(InstanceKey::new(1, 2)));
    state.begin();
    assert!(state.choose(&a, [0.0, 20.0, 0.0], &view, rules).is_some());
    assert!(state.choose(&b, [0.0, 20.0, 0.0], &view, rules).is_some());
    state.finish();
    assert_eq!(state.selections.len(), 2);
    state.begin();
    assert!(state.choose(&a, [0.0, 20.0, 0.0], &view, rules).is_some());
    state.finish();
    assert_eq!(state.selections.len(), 1, "an instance the pass did not reach is forgotten");
    assert!(state.selections.contains_key(&a.key));
}

#[test]
fn the_original_range_remembers_nothing() {
    let rules = &blackbox_scenery::layout::MOST_WANTED.lod;
    let view =
        LodView { position: [0.0; 3], forward: [0.0, 1.0, 0.0], pixel_scale: LodView::pixel_scale_for(1.0, 480.0) };
    let mut state = DetailState::default();
    state.begin();
    assert!(state.choose(&near_object(InstanceKey::new(1, 1)), [0.0, 20.0, 0.0], &view, rules).is_some());
    state.finish();
    assert!(state.selections.is_empty());
}
