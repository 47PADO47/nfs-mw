use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, VehicleSpec};

use super::*;

fn no_props(_: Vec3, _: Mat3, _: Vec3) -> Vec<WallContact> {
    vec![]
}

/// A wall: the plane z = `wall_z` (physics space), facing -z, like a barrier across the road.
fn wall(wall_z: f32) -> impl Fn(Vec3, Vec3) -> Option<Hit> {
    move |from, to| {
        let crosses = from.z < wall_z && to.z >= wall_z;
        let t = (wall_z - from.z) / (to.z - from.z);
        crosses.then(|| Hit { point: from + (to - from) * t, normal: Vec3::NEG_Z, prop: None, info: None })
    }
}

fn car() -> Vehicle {
    let mut v = Vehicle::new(VehicleSpec::example());
    assert!(v.place_on_ground(&FlatGround::new(0.0), 0.0, 0.0, 2.0, 0.0));
    v
}

#[test]
fn a_probe_past_the_wall_is_a_contact() {
    let half = Vec3::new(0.9, 0.6, 2.2);
    let cast = wall(2.0);
    let contacts = find(&cast, &no_props, Vec3::ZERO, Mat3::IDENTITY, half);
    // The front corners and the front middle poke 0.2 m through.
    assert_eq!(contacts.len(), 3);
    assert!(contacts.iter().all(|c| (c.depth - 0.2).abs() < 1e-4 && c.normal == Vec3::NEG_Z));
    assert!(find(&wall(3.0), &no_props, Vec3::ZERO, Mat3::IDENTITY, half).is_empty());
}

#[test]
fn floors_are_not_walls() {
    let floor = |from: Vec3, to: Vec3| Some(Hit { point: from.lerp(to, 0.5), normal: Vec3::Y, prop: None, info: None });
    assert!(find(&floor, &no_props, Vec3::ZERO, Mat3::IDENTITY, Vec3::ONE).is_empty());
}

#[test]
fn driving_into_a_wall_stops_the_car_and_keeps_it_out() {
    let ground = FlatGround::new(0.0);
    let mut v = car();
    let wall_z = 30.0;
    let cast = wall(wall_z);
    let mut worst_front = f32::MIN;
    let mut hit = 0.0f32;
    for _ in 0..600 {
        v.step(FIXED_STEP, &InputState { throttle: 1.0, ..InputState::default() }, &ground);
        hit = hit.max(resolve(&mut v, &cast, &no_props, &WallSpec::default()).impulse);
        let front = v.position().z + v.body().dimension().z;
        worst_front = worst_front.max(front);
    }
    assert!(hit > 1000.0, "an impulse was felt: {hit}");
    assert!(worst_front < wall_z + 0.5, "the nose went {} m into the wall", worst_front - wall_z);
    assert!(v.forward_speed() < 8.0, "still going {} m/s after ten seconds against a wall", v.forward_speed());
}

#[test]
fn every_contact_of_the_step_is_reported_with_its_kind() {
    let mut v = car();
    let impact = resolve(&mut v, &wall(2.0), &no_props, &WallSpec::default());
    // Three probes poke through the plane; with no collision data behind it the wall is a face.
    assert_eq!(impact.contacts.len(), 3);
    assert!(impact.contacts.iter().all(|c| c.kind == ContactKind::Face && c.normal == Vec3::NEG_Z));
    let barrier = WallContact {
        point: Vec3::ZERO,
        normal: Vec3::X,
        depth: 0.1,
        prop: None,
        info: Some(HitInfo { barrier: true, surface: 0, section: 0, instance: 0 }),
    };
    let prop = |mass| WallContact { prop: Some((1, mass)), info: None, ..barrier };
    assert_eq!(ContactPoint::from(&barrier).kind, ContactKind::Barrier);
    assert_eq!(ContactPoint::from(&prop(None)).kind, ContactKind::PropRigid);
    assert_eq!(ContactPoint::from(&prop(Some(80.0))).kind, ContactKind::PropMovable);
}

#[test]
fn no_wall_no_impulse() {
    let mut v = car();
    assert_eq!(resolve(&mut v, &wall(1000.0), &no_props, &WallSpec::default()), Impact::default());
}

/// A prop standing across the road at `z`: a contact whenever the front of the box reaches it.
/// `mass` of `None` is a rigid one.
fn prop(z: f32, mass: Option<f32>) -> impl Fn(Vec3, Mat3, Vec3) -> Vec<WallContact> {
    move |centre, _, half| {
        let depth = centre.z + half.z - z;
        if depth > 0.0 && centre.z < z {
            vec![WallContact {
                point: Vec3::new(centre.x, centre.y, z),
                normal: Vec3::NEG_Z,
                depth,
                prop: Some((7, mass)),
                info: None,
            }]
        } else {
            vec![]
        }
    }
}

#[test]
fn a_light_prop_is_pushed_aside_at_the_cost_of_some_speed() {
    let ground = FlatGround::new(0.0);
    let mut v = car();
    v.place_moving(Vec3::new(0.0, v.position().y, 0.0), glam::Quat::IDENTITY, 20.0);
    // The prop is gone once it has been knocked, as in the game.
    let gone = std::cell::Cell::new(false);
    let inner = prop(2.0, Some(100.0));
    let props = |c: Vec3, r: Mat3, h: Vec3| if gone.get() { vec![] } else { inner(c, r, h) };
    let before = v.forward_speed();
    let mut knocked = vec![];
    for _ in 0..3 {
        v.step(FIXED_STEP, &InputState { throttle: 1.0, ..InputState::default() }, &ground);
        let hit = resolve(&mut v, &no_cast, &props, &WallSpec::default()).knocked;
        gone.set(gone.get() || !hit.is_empty());
        knocked.extend(hit);
    }
    assert_eq!(knocked, [(7, 100.0)]);
    // The car keeps most of its speed: it lost about m_prop / (m_car + m_prop) of it, once.
    assert!(v.forward_speed() > before * 0.85, "{} -> {}", before, v.forward_speed());
    assert!(v.position().z > 0.5, "the car went through");
}

#[test]
fn a_rigid_prop_is_a_wall() {
    let ground = FlatGround::new(0.0);
    let mut v = car();
    v.place_moving(Vec3::new(0.0, v.position().y, 0.0), glam::Quat::IDENTITY, 20.0);
    let props = prop(10.0, None);
    let mut knocked = 0;
    for _ in 0..240 {
        v.step(FIXED_STEP, &InputState { throttle: 1.0, ..InputState::default() }, &ground);
        knocked += resolve(&mut v, &no_cast, &props, &WallSpec::default()).knocked.len();
    }
    assert_eq!(knocked, 0);
    assert!(v.position().z + v.body().dimension().z < 10.6, "stopped at the prop: z {}", v.position().z);
}

fn no_cast(_: Vec3, _: Vec3) -> Option<Hit> {
    None
}
