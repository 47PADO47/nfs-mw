use super::*;

fn owned(p: &mut Placed, profile: u32, surface: u32) {
    let shape = Arc::make_mut(p.prop.as_mut().unwrap());
    Arc::make_mut(shape.dynamics.as_mut().unwrap()).sound_profile = profile;
    Arc::make_mut(shape.bounds.as_mut().unwrap()).nodes[0].surface = surface;
}

#[test]
fn prop_pair_queues_both_owners_with_partner_materials_once() {
    let mut placed = [
        placement(110, Vec3::new(0.0, 2.0, 0.0), 100.0, false, 0.0),
        placement(111, Vec3::new(0.0, 2.0, 0.9), 100.0, false, 0.0),
    ];
    owned(&mut placed[0], 71, 51);
    owned(&mut placed[1], 72, 52);
    let mut world = PropWorld::default();
    world.add_tile(1, &mut placed);
    let ids = placed.each_ref().map(|p| p.prop_id);
    let first = world.colliders.get_mut(&ids[0]).unwrap().body.as_mut().unwrap();
    first.rigid.wake();
    first.rigid.linear_velocity = Vec3::Z * 10.0;
    world.collide_props(&[ids[0]]);
    let contacts = world.take_sound_contacts();
    assert_eq!(contacts.len(), 2);
    assert_eq!(
        contacts.iter().map(|c| (c.sound_profile, c.identity, c.surface)).collect::<Vec<_>>(),
        vec![(71, u64::from(ids[0]), 52), (72, u64::from(ids[1]), 51)]
    );
    assert!(contacts.iter().all(|c| c.source == ContactSource::Object && c.delta_v > 0.0));
    assert_eq!(contacts[0].normal, -contacts[1].normal);
    assert_eq!(contacts[0].velocity, -contacts[1].velocity);
    assert!(world.take_sound_contacts().is_empty());
    let visuals = world.take_impact_effects();
    assert_eq!(visuals.len(), 2);
    assert_eq!(visuals[0].source.collection, 71);
    assert_eq!(visuals[1].source.collection, 72);
    assert_ne!(visuals[0].owner, visuals[1].owner);
    assert_eq!(visuals[0].raw.detached_normal, visuals[1].raw.detached_normal);
    assert_eq!(visuals[0].raw.normal, -visuals[1].raw.normal);
}

#[test]
fn first_successful_prop_ground_reaction_keeps_owned_profile_and_incoming_velocity() {
    let mut placed = [placement(112, Vec3::Y * 0.4, 100.0, false, 0.0)];
    owned(&mut placed[0], 73, 53);
    let mut world = PropWorld::default();
    world.add_tile(1, &mut placed);
    let id = placed[0].prop_id;
    let prop = world.colliders.get_mut(&id).unwrap().body.as_mut().unwrap();
    prop.rigid.wake();
    prop.rigid.linear_velocity = Vec3::NEG_Y * 10.0;
    world.finish_native(&FlatGround::new(0.0), 1.0 / 60.0);
    let contacts = world.take_sound_contacts();
    assert_eq!(contacts.len(), 1, "support corners must not duplicate one ground callback");
    assert_eq!((contacts[0].sound_profile, contacts[0].identity), (73, u64::from(id)));
    assert_eq!(contacts[0].source, ContactSource::Ground);
    assert!(contacts[0].velocity.y < -9.0 && contacts[0].delta_v > 0.0);
    assert!(!contacts[0].bottom && !contacts[0].detached);
}

#[test]
fn vehicle_prop_sound_survives_a_discarded_ai_impact_without_duplicating_the_car_endpoint() {
    let mut placed = [placement(113, Vec3::new(0.0, 0.6, 0.0), 100.0, false, 1000.0)];
    owned(&mut placed[0], 74, 54);
    let mut world = PropWorld::default();
    world.add_tile(1, &mut placed);
    let mut impact = Impact::default();
    world.collide_vehicle(&mut car(20.0), None, &mut impact);
    assert_eq!(impact.sounds.len(), 1, "only the car endpoint belongs to its local impact feed");
    assert_eq!(impact.sounds[0].sound_profile, 0);
    let car_contact = impact.sounds[0];
    drop(impact); // Traffic and trailers discard this return value; the prop still owns its sound.
    let contacts = world.take_sound_contacts();
    assert_eq!(contacts.len(), 1);
    let prop_contact = contacts[0];
    assert_eq!(prop_contact.sound_profile, 74);
    assert_eq!(prop_contact.identity, u64::from(placed[0].prop_id));
    assert!(prop_contact.detached && prop_contact.delta_v > 0.0);
    assert_eq!(prop_contact.normal, -car_contact.normal);
    assert_eq!(prop_contact.velocity, -car_contact.velocity);
    assert!(world.take_sound_contacts().is_empty(), "one completed tick consumes the prop endpoint once");
}
