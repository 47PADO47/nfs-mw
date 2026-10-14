//! Short real-road support and launch check, independently of scene rendering and garage flow.

use super::*;
use crate::scenes::world::{drive::WorldGround, road, space};
use game_install::GameDir;
use nfsmw_data::car::{CarTables, physics::PhysicsData};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_default_area_cached_terrain_supports_idle_and_a_short_controlled_launch() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let near = index.centre();
    let mut collision = CollisionWorld::new(index.collision_grid.clone());
    let mut packs = 0;
    for section in
        index.sections.iter().filter(|section| section.is_spatial() && section.distance_to(near[0], near[1]) <= 500.0)
    {
        let bytes = dir.read_range(&index.stream_file, u64::from(section.file_offset), section.size as usize).unwrap();
        for pack in blackbox_collision::read_collision_packs(&bytes).unwrap() {
            collision.insert(pack);
            packs += 1;
        }
    }
    assert!(packs > 0, "default-area resident collision must be loaded");
    let spawn = road::find(near, |x, y| {
        let [px, _, pz] = space::to_physics(Vec3::new(x, y, 0.0));
        let top = collision.ground(px, pz, 1000.0, -1000.0)?.point[1] + 1.0;
        road::probe(&collision, x, y, top)
    })
    .expect("default area must contain a playable road");
    let physics = PhysicsData::load(&dir).unwrap();
    let tables = CarTables::load(&dir).unwrap();
    let kind = tables.car_type_for_folder("COBALTSS").unwrap();
    let mut car = CarSim::new(physics.car(&kind.type_name).unwrap(), [0.2; 4]);
    let ground = WorldGround { collision: &collision, surfaces: &physics.surfaces };
    let mut props = PropWorld::default();
    assert!(car.place(&ground, spawn));
    let start = car.body().position;
    for tick in 0..600 {
        car.step(&DriveInput::default(), &ground, Some((&collision, &mut props)));
        assert!(car.is_finite(), "idle tick {tick}");
        assert!(car.ground_reference.plane().valid, "missing body terrain on idle tick {tick}");
        if tick >= 60 {
            assert!(car.vehicle.wheels_on_ground() >= 3, "lost road support at idle tick {tick}");
        }
    }
    assert!(car.body().position.distance(start) < 2.0, "idle car must stay near its road placement");
    assert!(car.body().linear_velocity.length() < 1.0, "idle brake must hold the placed car");
    let resting = car.body().position;
    let heading = car.body().rotation().z_axis.x.atan2(car.body().rotation().z_axis.z);
    let right = car.body().rotation().x_axis;
    for tick in 0..120 {
        let current = car.body().rotation().z_axis.x.atan2(car.body().rotation().z_axis.z);
        let error = (heading - current + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        let lateral = (car.body().position - resting).dot(right);
        let steer = (3.0 * error - 0.3 * car.body().angular_velocity.y - 0.1 * lateral).clamp(-0.5, 0.5);
        // The installed default road is graded; use enough pedal for an actual hill start.
        car.step(&DriveInput { throttle: 0.6, steer, ..Default::default() }, &ground, Some((&collision, &mut props)));
        assert!(car.is_finite(), "launch tick {tick}");
        assert!(car.ground_reference.plane().valid, "missing body terrain on launch tick {tick}");
        assert!(car.vehicle.wheels_on_ground() >= 2, "lost road support at launch tick {tick}");
    }
    let distance = car.body().position.distance(resting);
    assert!(distance > 0.5 && distance < 30.0, "controlled two-second launch distance {distance}");
    assert!((car.body().position - resting).dot(right).abs() < 2.0, "short launch must stay on its road line");
    assert!(car.wheel_ground.borrow().iter().filter(|cache| cache.plane().valid).count() >= 3);
    println!("default area: {packs} packs, spawn {spawn:?}, two-second launch {distance:.2}m");
}
