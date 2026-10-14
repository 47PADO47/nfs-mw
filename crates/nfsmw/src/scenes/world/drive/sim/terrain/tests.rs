use blackbox_collision::{Article, CollisionPack, CollisionWorld, Instance, PackedVert, STRIP_UP_FACING, Strip};
use blackbox_vehicle::{FlatGround, NoGround, VehicleSpec};
use glam::{Quat, Vec3};
use nfsmw_data::car::physics::{CarBounds, CarPhysics, WallSpec};

use super::super::{CarSim, DriveInput, PropWorld, Spawn};

mod installed;

fn physics() -> CarPhysics {
    let spec = VehicleSpec::example();
    CarPhysics {
        bounds: CarBounds { half_dimensions: spec.dimension, pivot: Vec3::ZERO },
        spec,
        tire_integration: Default::default(),
        jump_config: Default::default(),
        walls: WallSpec::default(),
        geometry: None,
        events: None,
        reactions: None,
    }
}

fn ground_face() -> CollisionWorld {
    let vertex = |x, z| PackedVert { x, y: 0, z, surface: 0, flags: 0 };
    let mut world = CollisionWorld::new(None);
    world.insert(CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 20.0,
            half_height: 1.0,
            half_length: 20.0,
            radius: 30.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![Strip {
                center: [0.0; 3],
                radius: 30.0,
                flags: STRIP_UP_FACING,
                verts: vec![vertex(-2560, -2560), vertex(2560, -2560), vertex(0, 2560)],
            }],
            barriers: vec![],
            surfaces: vec![7],
            intermediate_object: 0,
            flags: 0,
        }],
        objects: vec![],
    });
    world
}

#[test]
fn terrain_plane_is_available_to_jump_alignment_on_the_first_physics_step() {
    let world = ground_face();
    let mut props = PropWorld::default();
    let mut car = CarSim::new(physics(), [0.2; 4]);
    car.vehicle.jump_config = nfsmw_data::car::physics::jump_config();
    car.vehicle.place_moving(Vec3::new(0.0, 4.0, 0.0), Quat::from_rotation_x(0.2), 20.0);
    assert!(car.vehicle.ground_reference.is_none());
    assert_eq!(car.vehicle.body().pending_torque(), Vec3::ZERO);
    car.step(&DriveInput::default(), &NoGround, Some((&world, &mut props)));
    let plane = car.ground_reference.plane();
    assert!(plane.valid);
    assert_eq!(plane.surface_hash, 7);
    assert_eq!(plane.normal, [0.0, super::SETTINGS.maximum_normal_y, 0.0]);
    // This must use the pre-step body position. Updating after Vehicle::step would also delay
    // the alignment torque by one full physics tick.
    assert!((plane.separation + 4.0 * super::SETTINGS.maximum_normal_y).abs() < 1e-5);
    assert_eq!(car.vehicle.ground_reference.unwrap().normal.to_array(), plane.normal);
    assert!(car.vehicle.body().pending_torque().length() > 0.0, "terrain alignment must run on this first tick");
}

#[test]
fn terrain_cache_survives_hardware_updates_and_clears_on_placement() {
    let world = ground_face();
    let mut props = PropWorld::default();
    let mut car = CarSim::new(physics(), [0.2; 4]);
    car.vehicle.place(Vec3::new(0.0, 4.0, 0.0), Quat::IDENTITY);
    car.step(&DriveInput::default(), &NoGround, Some((&world, &mut props)));
    let plane = car.ground_reference.plane();
    let reference = car.vehicle.ground_reference;
    assert!(plane.valid);
    let mut upgraded = physics();
    upgraded.spec.engine.torque.iter_mut().for_each(|t| *t *= 1.2);
    car.set_performance(&upgraded);
    assert_eq!(car.ground_reference.plane(), plane);
    assert_eq!(car.vehicle.ground_reference, reference);
    assert!(car.place(&FlatGround::new(0.0), Spawn { position: Vec3::ZERO, heading: 0.0 }));
    assert!(!car.ground_reference.plane().valid);
    assert!(car.vehicle.ground_reference.is_none());
}

#[test]
fn missing_collision_world_does_not_invent_a_plane_from_wheel_ground() {
    let ground = FlatGround::new(0.0);
    let mut car = CarSim::new(physics(), [0.2; 4]);
    assert!(car.place(&ground, Spawn { position: Vec3::ZERO, heading: 0.0 }));
    for _ in 0..90 {
        car.step(&DriveInput::default(), &ground, None);
    }
    assert_eq!(car.vehicle.wheels_on_ground(), 4);
    assert!(car.vehicle.ground_reference.is_none());
    assert!(!car.ground_reference.plane().valid);
    let world = ground_face();
    car.refresh_ground_reference(Some(&world));
    assert!(car.ground_reference.plane().valid);
    car.step(&DriveInput::default(), &ground, None);
    assert!(car.vehicle.ground_reference.is_none(), "disconnecting collision must disable the consumer reference");
}

#[test]
fn prepared_wheel_planes_use_resident_faces_even_without_a_ray_ground() {
    let world = ground_face();
    let mut props = PropWorld::default();
    let mut car = CarSim::new(physics(), [0.2; 4]);
    car.vehicle.place(Vec3::new(0.0, 0.49, 0.0), Quat::IDENTITY);
    car.step(&DriveInput::default(), &NoGround, Some((&world, &mut props)));
    assert_eq!(car.vehicle.wheels_on_ground(), 4, "all four wheels must consume prepared planes");
    assert!(car.wheel_ground.borrow().iter().all(|cache| cache.plane().valid));
    let planes: Vec<_> = car.wheel_ground.borrow().iter().map(|cache| cache.plane()).collect();
    let mut upgraded = physics();
    upgraded.spec.engine.torque.iter_mut().for_each(|torque| *torque *= 1.2);
    car.set_performance(&upgraded);
    assert_eq!(car.wheel_ground.borrow().iter().map(|cache| cache.plane()).collect::<Vec<_>>(), planes);
    assert!(car.place(&FlatGround::new(0.0), Spawn { position: Vec3::ZERO, heading: 0.0 }));
    assert!(car.wheel_ground.borrow().iter().all(|cache| !cache.plane().valid));
}

#[test]
fn original_driver_role_masks_replace_blanket_group_exclusion() {
    let mut car = CarSim::new(physics(), [0.2; 4]);
    assert_eq!(car.terrain_exclusion(), 0);
    car.collision_role.cop = true;
    assert_eq!(car.terrain_exclusion(), 0, "human driving a cop car remains a human role");
    car.collision_role.player = false;
    assert_eq!(car.terrain_exclusion(), 0x80);
    car.collision_role.cop = false;
    assert_eq!(car.terrain_exclusion(), 0x40);
    car.collision_racer = true;
    assert_eq!(car.terrain_exclusion(), 0);
}

#[test]
fn sleeping_global_query_uses_zero_mask_even_for_a_cop_role() {
    let mut world = ground_face();
    let mut pack = world.remove(1).unwrap();
    pack.instances[0].flags = 0x80;
    world.insert(pack);
    let mut car = CarSim::new(physics(), [0.2; 4]);
    car.collision_role.player = false;
    car.collision_role.cop = true;
    car.vehicle.place(Vec3::new(0.0, 4.0, 0.0), Quat::IDENTITY);
    car.refresh_ground_reference(Some(&world));
    assert!(!car.ground_reference.plane().valid, "awake cop collider excludes instance bit 0x80");
    car.vehicle.body_mut().state = blackbox_vehicle::rigid_body::BodyState::Asleep;
    car.refresh_ground_reference(Some(&world));
    assert!(car.ground_reference.plane().valid, "original sleeping point query uses exclusion zero");
    assert_eq!(car.ground_reference.plane().surface_hash, 7);
}
