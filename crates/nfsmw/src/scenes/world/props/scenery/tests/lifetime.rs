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

/// A lamp knocked off a booth that has collapsed must not be put back on the missing roof when its offscreen
/// allowance runs out: it is retired with the booth. Beside an intact booth it is restored as before.
#[test]
fn a_released_lamp_from_a_destroyed_booth_is_retired_instead_of_restored_in_mid_air() {
    for destroyed in [true, false] {
        let mut booth = super::super::super::tests::placement(1, Vec3::Y, 400.0, false, 0.0);
        booth.bounds = blackbox_scene::Aabb::new([-1.0, -1.0, 0.0], [1.0, 1.0, 2.5]);
        let mut lamp = super::super::super::tests::placement(2, Vec3::Y * 3.0, 100.0, false, 0.0);
        lamp.bounds = blackbox_scene::Aabb::new([-0.5, -0.5, 2.5], [0.5, 0.5, 3.5]);
        let mut placed = vec![booth, lamp];
        let mut world = PropWorld::default();
        world.add_tile(0, &mut placed);
        let (booth, lamp) = (placed[0].prop_id, placed[1].prop_id);
        if destroyed {
            world.hide_model(booth);
        }
        let spec = placed[1].prop.as_ref().unwrap().dynamics.clone().unwrap();
        let body = world.colliders.get_mut(&lamp).unwrap().body.as_mut().unwrap();
        let initial = body.rigid.position;
        let fallen = Vec3::new(2.0, 0.5, 0.0);
        body.rigid.position = fallen;
        body.moved = true;
        body.rigid.wake();
        body.lifetime.activate(&spec);
        crate::scenes::world::props::lifetime::track_lifetime(&mut world.live_lifetimes, lamp);
        // Unseen for longer than its eight-second allowance while it still moves.
        world.update_lifetimes(&FlatGround::new(0.0), Mat4::from_translation(Vec3::splat(1000.0)), 9.0);
        let collider = &world.colliders[&lamp];
        let body = collider.body.as_ref().unwrap();
        assert!(!body.lifetime.active, "destroyed {destroyed}: the release ends the simulation");
        if !destroyed {
            assert_eq!(body.rigid.position, initial, "beside an intact booth the placement is restored");
            assert!(!world.scenery.models[&lamp].hidden && collider.collidable);
            continue;
        }
        assert!(world.scenery.models[&lamp].hidden, "retired with the collapsed booth");
        assert!(!collider.collidable && collider.cells.is_empty());
        assert_eq!(body.rigid.position, fallen, "never put back on the missing roof");
        assert_eq!(world.scenery.models[&lamp].streaming_state(body.moved), Some(false), "the section cache keeps it");
    }
}
