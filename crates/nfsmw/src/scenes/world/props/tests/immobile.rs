use super::*;
use blackbox_vehicle::rigid_body::RigidBody;

#[test]
fn source_immobile_vehicle_pushes_a_loose_prop_without_changing_its_motion() {
    let mut world = PropWorld::default();
    let mut placed = [placement(77, Vec3::new(0.0, 0.6, 0.0), 4000.0, false, 0.0)];
    world.add_tile(1, &mut placed);
    let mut vehicle = car(20.0);
    let mut spec = vehicle.spec().clone();
    spec.body.objects.immobile = true;
    vehicle.set_spec(spec);
    let position = vehicle.position();
    let velocity = vehicle.linear_velocity();
    let mut impact = Impact::default();
    world.collide_vehicle(&mut vehicle, None, &mut impact);
    assert!(!impact.contacts.is_empty());
    assert_eq!(vehicle.linear_velocity(), velocity);
    assert_eq!(vehicle.position(), position);
    let prop = world.colliders[&placed[0].prop_id].body.as_ref().unwrap();
    assert!(prop.rigid.linear_velocity.z > 0.0);
    assert!(impact.damage[0].force > 0.0, "notifications survive unchanged vehicle velocity");
}

#[test]
fn initial_immobility_survives_partner_state_but_breakaway_force_is_gated() {
    let placed = placement(78, Vec3::ZERO, 100.0, false, 500.0);
    let mut spec = placed.prop.as_ref().unwrap().dynamics.as_ref().unwrap().as_ref().clone();
    let body = RigidBody::new(spec.mass, Vec3::ONE, spec.tensor_scale, spec.body);
    let attached = contact::parameters(&body, Vec3::Z, &spec, true, false);
    assert!(attached.immobile);
    assert_eq!(attached.breaking_impulse, 500.0);
    let bilateral = contact::parameters(&body, Vec3::Z, &spec, true, true);
    assert!(bilateral.immobile);
    assert_eq!(bilateral.breaking_impulse, 0.0);
    spec.immobile = true;
    assert!(contact::parameters(&body, Vec3::Z, &spec, false, false).immobile);
    assert!(contact::parameters(&body, Vec3::Z, &spec, false, true).immobile);
}

#[test]
fn source_immobile_car_cannot_detach_an_attached_prop_or_receive_a_contact_response() {
    let mut world = PropWorld::default();
    let mut placed = [placement(79, Vec3::new(0.0, 0.6, 0.0), 400.0, true, 15_000.0)];
    world.add_tile(1, &mut placed);
    let mut vehicle = car(200.0);
    let mut spec = vehicle.spec().clone();
    spec.body.objects.immobile = true;
    vehicle.set_spec(spec);
    let before = (vehicle.position(), vehicle.linear_velocity());
    let prop_position = world.colliders[&placed[0].prop_id].body.as_ref().unwrap().rigid.position;
    let mut impact = Impact::default();
    world.collide_vehicle(&mut vehicle, None, &mut impact);
    assert_eq!((vehicle.position(), vehicle.linear_velocity()), before);
    let prop = world.colliders[&placed[0].prop_id].body.as_ref().unwrap();
    assert!(prop.attached && !prop.moved);
    assert_eq!(prop.rigid.position, prop_position);
    assert_eq!(prop.rigid.linear_velocity, Vec3::ZERO);
    assert!(impact.contacts.is_empty() && impact.damage.is_empty() && impact.sounds.is_empty());
}
