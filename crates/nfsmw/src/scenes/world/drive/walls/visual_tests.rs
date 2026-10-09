use super::*;
use blackbox_vehicle::VehicleSpec;

fn no_cast(_: Vec3, _: Vec3) -> Option<Hit> {
    None
}

fn contact(point: Vec3, depth: f32, prop: Option<(u32, Option<f32>)>) -> WallContact {
    WallContact {
        point,
        normal: Vec3::NEG_Z,
        depth,
        prop,
        info: Some(HitInfo { barrier: true, surface: 7, section: 12, instance: 3 }),
    }
}

#[test]
fn each_contact_keeps_its_point_velocity_before_response_and_only_the_reaction_has_impulse() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body_mut().linear_velocity = Vec3::new(7.0, 0.0, 20.0);
    car.body_mut().angular_velocity = Vec3::new(0.0, 2.0, 0.0);
    let cog = car.body().world_cog();
    let contacts =
        [contact(cog + Vec3::new(-1.0, 0.0, 2.0), 0.05, None), contact(cog + Vec3::new(1.0, 0.0, 2.0), 0.1, None)];
    let expected = contacts.map(|c| car.body().point_velocity(c.point));
    let mass = car.body().mass();
    let props = |_: Vec3, _: Mat3, _: Vec3| contacts.to_vec();
    let impact = resolve(&mut car, &no_cast, &props, &WallSpec::default());
    assert_eq!(impact.visuals.len(), 2);
    let first = impact.visuals[0];
    assert_eq!(first.point, contacts[1].point);
    assert_eq!(first.normal, Vec3::NEG_Z);
    assert_eq!(first.velocity, expected[1]);
    assert_eq!(impact.visuals[1].velocity, expected[0]);
    assert_ne!(car.body().point_velocity(first.point), first.velocity);
    assert!(first.impulse_delta_v > 0.0);
    assert!((first.impulse_delta_v * mass - impact.impulse).abs() < 0.01);
    assert_eq!(impact.visuals[1].impulse_delta_v, 0.0);
    assert_eq!(first.surface, Some(7));
    assert_eq!(first.info, contacts[1].info);
    assert_eq!(first.kind, ContactKind::Barrier);
    assert_eq!(first.prop, None);
}

#[test]
fn stationary_and_tangential_contacts_have_no_impact_impulse() {
    for velocity in [Vec3::ZERO, Vec3::new(8.0, 0.0, 0.0), Vec3::new(0.0, 0.0, -8.0)] {
        let mut car = Vehicle::new(VehicleSpec::example());
        car.body_mut().linear_velocity = velocity;
        let props = |_: Vec3, _: Mat3, _: Vec3| vec![contact(Vec3::new(0.0, 0.0, 2.0), 0.1, None)];
        let impact = resolve(&mut car, &no_cast, &props, &WallSpec::default());
        assert_eq!(impact.visuals.len(), 1);
        assert_eq!(impact.visuals[0].velocity, velocity);
        assert_eq!(impact.visuals[0].impulse_delta_v, 0.0);
        assert_eq!(impact.impulse, 0.0);
    }
}

#[test]
fn a_world_surface_zero_is_retained_and_is_distinct_from_absent_prop_metadata() {
    let mut car = Vehicle::new(VehicleSpec::example());
    let mut world = contact(Vec3::new(0.0, 0.0, 2.0), 0.1, None);
    world.info.as_mut().unwrap().surface = 0;
    let mut prop = contact(Vec3::new(1.0, 0.0, 2.0), 0.05, Some((9, None)));
    prop.info = None;
    let props = |_: Vec3, _: Mat3, _: Vec3| vec![world, prop];
    let impact = resolve(&mut car, &no_cast, &props, &WallSpec::default());
    assert_eq!(impact.visuals[0].surface, Some(0));
    assert_eq!(impact.visuals[1].surface, None);
}

#[test]
fn a_later_reacting_contact_gets_the_impulse_instead_of_the_deepest_separating_contact() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body_mut().linear_velocity = Vec3::new(0.0, 0.0, 8.0);
    let mut separating = contact(Vec3::new(0.0, 0.0, -2.0), 0.2, None);
    separating.normal = Vec3::Z;
    let props = |_: Vec3, _: Mat3, _: Vec3| vec![separating, contact(Vec3::new(0.0, 0.0, 2.0), 0.1, None)];
    let impact = resolve(&mut car, &no_cast, &props, &WallSpec::default());
    assert_eq!(impact.visuals[0].impulse_delta_v, 0.0);
    assert!(impact.visuals[1].impulse_delta_v > 0.0);
    assert_eq!(impact.visuals[1].point.z, 2.0);
}

#[test]
fn light_props_are_excluded_without_changing_debug_contacts_or_pre_response_velocity() {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.body_mut().linear_velocity = Vec3::new(8.0, 0.0, 0.0);
    let mut rigid = contact(Vec3::new(0.0, 0.0, 2.0), 0.1, Some((2, None)));
    rigid.info = None;
    let props = |_: Vec3, _: Mat3, _: Vec3| vec![contact(Vec3::new(0.0, 0.0, 2.0), 0.2, Some((1, Some(20.0)))), rigid];
    let impact = resolve(&mut car, &no_cast, &props, &WallSpec::default());
    assert_eq!(impact.contacts.len(), 2);
    assert_eq!(impact.knocked, vec![(1, 20.0)]);
    assert_eq!(impact.visuals.len(), 1);
    assert_eq!(impact.visuals[0].velocity, Vec3::new(8.0, 0.0, 0.0));
    assert_ne!(car.body().linear_velocity, impact.visuals[0].velocity);
    assert_eq!(impact.visuals[0].kind, ContactKind::PropRigid);
    assert_eq!(impact.visuals[0].prop, Some(2));
    assert_eq!(impact.visuals[0].surface, None);
}

#[test]
fn visual_budget_does_not_truncate_physics_or_debug_contacts() {
    let mut car = Vehicle::new(VehicleSpec::example());
    let props = |_: Vec3, _: Mat3, _: Vec3| {
        (1..=30)
            .map(|i| {
                let mut c = contact(Vec3::new(i as f32, 0.0, 2.0), i as f32 * 0.01, None);
                c.info.as_mut().unwrap().instance = i;
                c
            })
            .collect()
    };
    let impact = resolve(&mut car, &no_cast, &props, &WallSpec::default());
    assert_eq!(impact.rigid, 30);
    assert_eq!(impact.contacts.len(), 30);
    assert_eq!(impact.visuals.len(), MAX_VISUAL_CONTACTS);
    assert_eq!(impact.visuals.first().unwrap().info.unwrap().instance, 30);
    assert_eq!(impact.visuals.last().unwrap().info.unwrap().instance, 15);
}
