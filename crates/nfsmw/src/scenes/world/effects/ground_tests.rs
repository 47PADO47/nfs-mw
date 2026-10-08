//! Synthetic resident-road coverage for contact reprojection, axis conversion and exclusions.

use blackbox_collision::{Article, CollisionPack, CollisionWorld, Instance, PackedVert, Strip};
use blackbox_vehicle::{FIXED_STEP, FlatGround, InputState, Vehicle, VehicleSpec};
use glam::Vec3;

use super::project;

fn world(flags: u8, group: u16) -> CollisionWorld {
    let verts = [(-1280, -1280), (1280, -1280), (-1280, 1280), (1280, 1280)]
        .into_iter()
        .map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags })
        .collect();
    let article = Article {
        strips: vec![Strip { center: [0.0; 3], radius: 15.0, flags: 2, verts }],
        barriers: vec![],
        surfaces: vec![123],
        intermediate_object: 0,
        flags: 0,
    };
    let instance = Instance {
        row_x: [1.0, 0.0, 0.0],
        row_z: [0.0, 0.0, 1.0],
        half_width: 10.0,
        half_length: 10.0,
        translation: [0.0, -5.0, 0.0],
        radius: 15.0,
        half_height: 1.0,
        iter_stamp: 0,
        flags: 0,
        group,
        article: 0,
    };
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack { section: 7, instances: vec![instance], articles: vec![article], objects: vec![] });
    world
}

fn wheel() -> blackbox_vehicle::WheelState {
    let mut car = Vehicle::new(VehicleSpec::example());
    let ground = FlatGround::new(5.0);
    assert!(car.place_on_ground(&ground, 0.0, 0.0, 10.0, 0.0));
    for _ in 0..120 {
        car.step(FIXED_STEP, &InputState::default(), &ground);
    }
    car.wheel(0)
}

#[test]
fn loaded_patch_is_projected_to_exact_ground_normal_and_section() {
    let mut wheel = wheel();
    wheel.position = Vec3::new(2.0, 5.1, 3.0);
    wheel.skid = 0.7;
    wheel.smoke = 0.4;
    let contact = project(wheel, Vec3::Z, &world(0, 0)).expect("resident road");
    assert_eq!(contact.point, Vec3::new(3.0, -2.0, 5.0));
    assert_eq!(contact.normal, Vec3::Z);
    assert_eq!(contact.forward, Vec3::X);
    assert_eq!(contact.section, 7);
    assert_eq!((contact.skid, contact.smoke), (0.7, 0.4));
}

#[test]
fn air_excluded_ground_missing_sections_and_remote_surfaces_emit_nothing() {
    let mut wheel = wheel();
    wheel.position = Vec3::new(2.0, 5.1, 3.0);
    assert!(project(wheel, Vec3::Z, &world(blackbox_collision::SURFACE_NO_GROUND, 0)).is_none());
    // Grouped ground strips remain driveable in WorldGround; group number alone is not an exclusion flag.
    assert!(project(wheel, Vec3::Z, &world(0, 1)).is_some());
    let mut excluded = world(0, 0);
    let mut pack = excluded.remove(7).unwrap();
    pack.instances[0].flags = u16::from(blackbox_collision::GROUP_EXCLUSION);
    excluded.insert(pack);
    assert!(project(wheel, Vec3::Z, &excluded).is_none());
    assert!(project(wheel, Vec3::Z, &CollisionWorld::new(None)).is_none());
    wheel.on_ground = false;
    assert!(project(wheel, Vec3::Z, &world(0, 0)).is_none());
    wheel.on_ground = true;
    wheel.position.y = 6.0;
    assert!(project(wheel, Vec3::Z, &world(0, 0)).is_none());
}
