use super::*;
use blackbox_vehicle::{FlatGround, rigid_body::BodyState};

#[test]
fn hiding_a_parent_ends_only_its_physical_simulation_and_keeps_children_and_consumed_state() {
    let mut placed = super::super::super::tests::placement(7, Vec3::Y, 100.0, false, 0.0);
    let mut world = PropWorld::default();
    world.add_tile(0, std::slice::from_mut(&mut placed));
    let id = placed.prop_id;
    let owner = world.scenery.models[&id].owner.clone();
    let model = world.scenery.models.get_mut(&id).unwrap();
    model.fired = true;
    model.triggered = true;
    model.hidden_parts.insert(11);
    let spec = placed.prop.as_ref().unwrap().dynamics.clone().unwrap();
    let body = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    body.lifetime.activate(&spec);
    body.rigid.wake();
    crate::scenes::world::props::lifetime::track_lifetime(&mut world.live_lifetimes, id);
    let fragment = super::super::super::body::PropBody::fragment(
        placed.prop.as_ref().unwrap().bounds.as_ref().unwrap(),
        0,
        &spec,
        (body.rigid.position, body.rigid.orientation),
        body.transform(1.0),
    )
    .unwrap();
    let child = world.insert_fragment_with(fragment, spec, Vec::new(), owner.clone(), true).unwrap();
    world.hide_model(id);
    assert_eq!(owner.strong_count(), 1, "hiding does not invalidate descendant ownership");
    assert!(world.colliders.contains_key(&child));
    let collider = &world.colliders[&id];
    assert!(!collider.collidable && collider.cells.is_empty());
    let body = collider.body.as_ref().unwrap();
    assert_eq!(body.rigid.state, BodyState::Frozen);
    assert!(!body.lifetime.active);
    world.update_lifetimes(&FlatGround::new(0.0), Mat4::from_translation(Vec3::splat(1000.0)), 9.0);
    let model = &world.scenery.models[&id];
    assert!(model.hidden && model.fired && model.triggered);
    assert!(model.hidden_parts.contains(&11));
    assert_eq!(owner.strong_count(), 1, "expired children cannot restore the parent");
    world.remove_tile(0);
    assert_eq!(owner.strong_count(), 0);
    assert!(world.colliders.is_empty() && world.scenery.models.is_empty());
}
