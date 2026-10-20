//! Synthetic and installed breaker verification.

use super::*;
mod census;
mod installed;
mod poles;
mod sawhorse;

#[test]
fn offscreen_release_resets_hidden_parts_and_invalidates_fragment_owners() {
    let mut placed = vec![super::super::tests::placement(7, Vec3::Y, 100.0, false, 0.0)];
    let mut world = PropWorld::default();
    world.add_tile(0, &mut placed);
    let id = placed[0].prop_id;
    let model = world.scenery.models.get_mut(&id).unwrap();
    model.hidden = true;
    model.hidden_parts.insert(11);
    let owner = model.owner.clone();
    world.reset_model(id);
    assert_eq!(owner.strong_count(), 0);
    let model = &world.scenery.models[&id];
    assert!(!model.hidden);
    assert!(model.hidden_parts.is_empty());
    world.remove_tile(0);
    assert!(world.scenery.models.is_empty());
    assert!(world.grid.is_empty());
}

#[test]
fn destroying_a_parent_invalidates_its_children() {
    let placement = super::super::tests::placement(1, Vec3::Y, 100.0, false, 0.0);
    let shape = placement.prop.as_ref().unwrap();
    let spec = shape.dynamics.as_ref().unwrap();
    let mut model = SceneryModel::placed(None, shape.bounds.clone().unwrap(), spec);
    let owner = model.owner.clone();
    assert_eq!(owner.strong_count(), 1);
    model.reset();
    assert_eq!(owner.strong_count(), 0);
    assert_eq!(model.owner.strong_count(), 1);
}

#[test]
fn placed_release_can_remove_later_fragment_ids_during_the_same_cleanup_pass() {
    let mut placed = vec![super::super::tests::placement(7, Vec3::Y, 100.0, false, 0.0)];
    let spec = Arc::make_mut(Arc::make_mut(placed[0].prop.as_mut().unwrap()).dynamics.as_mut().unwrap());
    spec.kill_off_screen = 0.005;
    let spec = placed[0].prop.as_ref().unwrap().dynamics.clone().unwrap();
    let mut world = PropWorld::default();
    world.add_tile(0, &mut placed);
    let id = placed[0].prop_id;
    let owner = world.scenery.models[&id].owner.clone();
    let body = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    body.lifetime.activate(&spec);
    crate::scenes::world::props::lifetime::track_lifetime(&mut world.live_lifetimes, id);
    let fragment = super::super::body::PropBody::fragment(
        placed[0].prop.as_ref().unwrap().bounds.as_ref().unwrap(),
        0,
        &spec,
        (body.rigid.position, body.rigid.orientation),
        body.transform(1.0),
    )
    .unwrap();
    world.insert_fragment_with(fragment, spec, Vec::new(), owner, true).unwrap();
    assert_eq!(world.len(), 2);
    world.update_lifetimes(
        &blackbox_vehicle::FlatGround::new(0.0),
        Mat4::from_translation(Vec3::splat(1000.0)),
        1.0 / 60.0,
    );
    assert_eq!(world.len(), 1);
    assert_eq!(world.fragment_count(), 0);
}
