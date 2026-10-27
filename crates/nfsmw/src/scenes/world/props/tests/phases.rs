use super::*;

#[test]
fn prop_drag_uses_the_velocity_after_ground_response() {
    let mut placed = placement(501, Vec3::Y * 0.45, 100.0, false, 0.0);
    let shape = Arc::make_mut(placed.prop.as_mut().unwrap());
    Arc::make_mut(shape.dynamics.as_mut().unwrap()).body.drag = Vec3::ONE;
    let mut world = PropWorld::default();
    world.add_tile(1, std::slice::from_mut(&mut placed));
    let body = world.colliders.get_mut(&placed.prop_id).unwrap().body.as_mut().unwrap();
    body.rigid.wake();
    body.rigid.linear_velocity = Vec3::new(30.0, -4.0, 0.0);
    world.begin_native(1.0 / 60.0);
    let before = world.colliders[&placed.prop_id].body.as_ref().unwrap().rigid.linear_velocity;
    world.resolve_native_pairs();
    world.finish_native(&FlatGround::new(0.0), 1.0 / 60.0);
    let body = &world.colliders[&placed.prop_id].body.as_ref().unwrap().rigid;
    assert!((body.linear_velocity - before).length() > 1.0, "the fixture must produce a ground impulse");
    let mut expected = body.clone();
    let force = expected.pending_force();
    expected.apply_drag();
    let drag = expected.pending_force() - force;
    assert!(drag.x.abs() > 0.01, "the fixture must retain horizontal drag");
    assert!((body.pending_force().x - drag.x).abs() < 0.001, "drag must use the corrected body velocity");
}

#[test]
fn prop_begin_only_integrates_and_the_shared_pair_pass_resolves_contacts() {
    let mut placed = [
        placement(502, Vec3::new(-0.48, 3.0, 0.0), 100.0, false, 0.0),
        placement(503, Vec3::new(0.48, 3.0, 0.0), 100.0, false, 0.0),
    ];
    let mut world = PropWorld::default();
    world.add_tile(1, &mut placed);
    for (placed, speed) in placed.iter().zip([3.0, -3.0]) {
        let prop = world.colliders.get_mut(&placed.prop_id).unwrap().body.as_mut().unwrap();
        prop.rigid.wake();
        prop.rigid.linear_velocity = Vec3::X * speed;
    }
    world.begin_native(1.0 / 60.0);
    assert!(placed.iter().all(|p| !world.colliders[&p.prop_id].body.as_ref().unwrap().object_contact));
    world.resolve_native_pairs();
    assert!(placed.iter().all(|p| world.colliders[&p.prop_id].body.as_ref().unwrap().object_contact));
    let velocity = world.colliders[&placed[0].prop_id].body.as_ref().unwrap().rigid.linear_velocity;
    assert!(velocity.x < 3.0, "the deferred pair pass must resolve its approaching contact");
}
