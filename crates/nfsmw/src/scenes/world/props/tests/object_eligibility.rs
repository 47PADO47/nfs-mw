use super::*;

#[test]
fn source_disabled_props_ignore_cars_and_both_prop_pair_directions() {
    for disabled in [0, 1] {
        let mut placed = [
            placement(101, Vec3::new(0.0, 0.6, 0.0), 500.0, false, 0.0),
            placement(102, Vec3::new(0.0, 0.6, 0.9), 500.0, false, 0.0),
        ];
        let shape = Arc::make_mut(placed[disabled].prop.as_mut().unwrap());
        Arc::make_mut(shape.dynamics.as_mut().unwrap()).no_object_collisions = true;
        let mut world = PropWorld::default();
        world.add_tile(1, &mut placed);
        let ids = placed.each_ref().map(|p| p.prop_id);
        let first = world.colliders.get_mut(&ids[0]).unwrap().body.as_mut().unwrap();
        first.rigid.wake();
        first.rigid.linear_velocity = Vec3::Z * 10.0;
        world.collide_props(&[ids[0]]);
        assert_eq!(world.colliders[&ids[0]].body.as_ref().unwrap().rigid.linear_velocity, Vec3::Z * 10.0);
        assert_eq!(world.colliders[&ids[1]].body.as_ref().unwrap().rigid.linear_velocity, Vec3::ZERO);
        assert!(world.live_lifetimes.is_empty());

        world.remove_tile(1);
        world.add_tile(1, &mut placed[disabled..=disabled]);
        let mut vehicle = car(20.0);
        vehicle.body_mut().position.z += placed[disabled].spawner.unwrap().position[2];
        let before = (vehicle.position(), vehicle.linear_velocity());
        let mut impact = Impact::default();
        world.collide_vehicle(&mut vehicle, None, &mut impact);
        assert_eq!((vehicle.position(), vehicle.linear_velocity()), before);
        assert!(impact.contacts.is_empty() && impact.damage.is_empty() && impact.sounds.is_empty());
    }
}

#[test]
fn asleep_and_attached_props_remain_eligible_but_frozen_props_leave_the_grid() {
    let mut placed = [placement(103, Vec3::new(0.0, 0.6, 0.0), 500.0, false, 1000.0)];
    let mut world = PropWorld::default();
    world.add_tile(1, &mut placed);
    let id = placed[0].prop_id;
    let bounds = world.colliders[&id].physics_bounds();
    assert!(candidate_ids(&world, &bounds).contains(&id));
    world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap().rigid.state = BodyState::Frozen;
    world.regrid(id);
    assert!(!candidate_ids(&world, &bounds).contains(&id));
    world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap().rigid.state = BodyState::Asleep;
    world.regrid(id);
    assert!(candidate_ids(&world, &bounds).contains(&id));
}

#[test]
fn object_disabled_props_still_fall_onto_ground() {
    let mut placed = [placement(104, Vec3::Y * 3.0, 500.0, false, 0.0)];
    let shape = Arc::make_mut(placed[0].prop.as_mut().unwrap());
    Arc::make_mut(shape.dynamics.as_mut().unwrap()).no_object_collisions = true;
    let mut world = PropWorld::default();
    world.add_tile(1, &mut placed);
    let id = placed[0].prop_id;
    world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap().rigid.wake();
    for _ in 0..240 {
        world.step(&FlatGround::new(0.0), 1.0 / 60.0);
    }
    let body = &world.colliders[&id].body.as_ref().unwrap().rigid;
    assert!((0.35..0.7).contains(&body.position.y), "ground remains independent: {}", body.position.y);
    assert!(world.colliders[&id].cells.is_empty());
}
