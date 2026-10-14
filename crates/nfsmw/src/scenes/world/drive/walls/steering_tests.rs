use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, VehicleSpec};

use super::*;

fn car() -> Vehicle {
    let mut car = Vehicle::new(VehicleSpec::example());
    car.wall_steering_config = nfsmw_data::car::physics::wall_steering_config();
    assert!(car.place_on_ground(&FlatGround::new(0.0), 0.0, 0.0, 2.0, 0.0));
    for _ in 0..180 {
        car.step(FIXED_STEP, &InputState::default(), &FlatGround::new(0.0));
    }
    assert_eq!(car.wheels_on_ground(), 4);
    car.body_mut().linear_velocity = Vec3::Z;
    car
}

fn no_props(_: Vec3, _: Mat3, _: Vec3) -> Vec<WallContact> {
    vec![]
}

fn wall(z: f32) -> impl Fn(Vec3, Vec3) -> Option<Hit> {
    move |from, to| {
        if from.z >= z || to.z < z {
            return None;
        }
        let fraction = (z - from.z) / (to.z - from.z);
        Some(Hit { point: from.lerp(to, fraction), normal: Vec3::NEG_Z, prop: None, info: None })
    }
}

#[test]
fn world_response_queues_nose_yaw_for_the_next_tick_exactly_once() {
    let ground = FlatGround::new(0.0);
    let mut enabled = car();
    let z = enabled.position().z + enabled.spec().dimension.z - 0.1;
    let impact = resolve(&mut enabled, &wall(z), &no_props, &WallSpec::default());
    assert!(impact.impulse > 0.0);
    let mut disabled = enabled.clone();
    disabled.wall_steering_config = Default::default();
    let input = InputState { throttle: 1.0, steer: -0.5, ..Default::default() };
    enabled.step(FIXED_STEP, &input, &ground);
    disabled.step(FIXED_STEP, &input, &ground);
    let expected = 0.125 * enabled.controls().steering * enabled.controls().gas;
    assert!((enabled.angular_velocity().y - disabled.angular_velocity().y - expected).abs() < 1e-5);
    // Clone after consumption to isolate persistence from the newly changed tire forces.
    let mut without_config = enabled.clone();
    without_config.wall_steering_config = Default::default();
    enabled.step(FIXED_STEP, &input, &ground);
    without_config.step(FIXED_STEP, &input, &ground);
    assert_eq!(enabled.angular_velocity(), without_config.angular_velocity());
}

#[test]
fn an_identical_rigid_prop_response_does_not_queue_world_steering() {
    let ground = FlatGround::new(0.0);
    let mut enabled = car();
    let z = enabled.position().z + enabled.spec().dimension.z - 0.1;
    let props = |_: Vec3, _: Mat3, _: Vec3| {
        vec![WallContact {
            point: Vec3::new(0.0, enabled.position().y, z),
            normal: Vec3::NEG_Z,
            depth: 0.1,
            prop: Some((10, None)),
            info: None,
        }]
    };
    let contacts = props(Vec3::ZERO, Mat3::IDENTITY, Vec3::ONE);
    let props = |_: Vec3, _: Mat3, _: Vec3| contacts.clone();
    let impact = resolve(&mut enabled, &|_, _| None, &props, &WallSpec::default());
    assert!(impact.impulse > 0.0);
    let mut disabled = enabled.clone();
    disabled.wall_steering_config = Default::default();
    let input = InputState { throttle: 1.0, steer: -0.5, ..Default::default() };
    enabled.step(FIXED_STEP, &input, &ground);
    disabled.step(FIXED_STEP, &input, &ground);
    assert_eq!(enabled.angular_velocity(), disabled.angular_velocity());
}

fn role_walls() -> CollisionWorld {
    use blackbox_collision::{Article, Barrier, CollisionPack, Instance};

    let mut pack = CollisionPack { section: 7, instances: vec![], articles: vec![], objects: vec![] };
    for (index, (x, flags, group)) in [(0.0, 0, 1), (0.2, 0x40, 0), (0.4, 0x80, 0), (0.6, 0, 0)].into_iter().enumerate()
    {
        pack.instances.push(Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [-x, 0.0, 0.0],
            half_width: 4.0,
            half_height: 4.0,
            half_length: 4.0,
            radius: 8.0,
            iter_stamp: 0,
            flags,
            group,
            article: index as u16,
        });
        pack.articles.push(Article {
            strips: vec![],
            barriers: vec![Barrier {
                p0: [0.2, -2.0, 4.0],
                p1: [0.2, 2.0, -4.0],
                surface: 0,
                flags: 2,
                inv_xz_len: 0.125,
            }],
            surfaces: vec![7],
            intermediate_object: 0,
            flags: 0,
        });
    }
    let mut world = CollisionWorld::new(None);
    world.insert(pack);
    world
}

#[test]
fn body_role_masks_distinguish_world_contacts_without_hiding_farther_walls() {
    let world = role_walls();
    // Grouped wall at x=.2 is inactive for every role. The next two have distinct exclusion bits.
    for (mask, expected_instance, expected_x) in [(0, 1, 0.4), (0x80, 1, 0.4), (0x40, 2, 0.6), (0xc0, 3, 0.8)] {
        let cast = world_cast_with_mask(&world, mask);
        let hit = cast(Vec3::ZERO, Vec3::X * 2.0).unwrap();
        assert_eq!(hit.info.unwrap().instance, expected_instance);
        assert!((hit.point.x - expected_x).abs() < 1e-5);
        let mut car = car();
        car.body_mut().linear_velocity = Vec3::X;
        let impact = resolve(&mut car, &cast, &no_props, &WallSpec::default());
        assert!(impact.impulse > 0.0);
        assert_eq!(impact.deepest.unwrap().1.unwrap().instance, expected_instance);
    }
}
