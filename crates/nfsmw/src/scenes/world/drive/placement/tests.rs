//! The real player reset adapter enables grace after successful replacement placement only.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::drive::SpawnRequest;
use crate::settings::CarShading;
use blackbox_collision::{Article, CollisionPack, Instance, PackedVert, STRIP_UP_FACING, Strip};
use glam::Vec3;
use nfsmw_data::car::{LoadOptions, physics::PhysicsData};

fn ground() -> CollisionWorld {
    let mut collision = CollisionWorld::new(None);
    collision.insert(CollisionPack {
        section: 1,
        instances: vec![Instance {
            row_x: [1.0, 0.0, 0.0],
            row_z: [0.0, 0.0, 1.0],
            translation: [0.0; 3],
            half_width: 100.0,
            half_height: 10.0,
            half_length: 100.0,
            radius: 150.0,
            iter_stamp: 0,
            flags: 0,
            group: 0,
            article: 0,
        }],
        articles: vec![Article {
            strips: vec![Strip {
                center: [0.0; 3],
                radius: 150.0,
                flags: STRIP_UP_FACING,
                verts: [(-12_800, -12_800), (-12_800, 12_800), (12_800, -12_800), (12_800, 12_800)]
                    .into_iter()
                    .map(|(x, z)| PackedVert { x, y: 0, z, surface: 0, flags: 0 })
                    .collect(),
            }],
            surfaces: vec![0],
            intermediate_object: 0,
            flags: 0,
            barriers: Vec::new(),
        }],
        objects: Vec::new(),
    });
    collision
}

#[test]
#[ignore = "requires installed player geometry/physics; CPU-only reset placement"]
fn installed_player_reset_collision_grace_requires_a_successful_replacement() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let data = PhysicsData::load(&dir).unwrap();
    let physics = data.car("bmwm3gtr").unwrap();
    let model =
        nfsmw_data::car::load_driving(&dir, "BMWM3GTR", &LoadOptions { lod: 'A', all_parts: false, preset: None })
            .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    let rig = CarRig::upload(&mut renderer, 0, model, CarShading::Original, Vec3::Z);
    let mut drive = Drive::new(
        "BMWM3GTR".to_owned(),
        rig,
        physics,
        SpawnRequest { near: [0.0; 2], heading: None, exact: None, strict: false },
        None,
        Default::default(),
    );
    let spawn = Spawn { position: Vec3::ZERO, heading: 0.0 };
    let empty = CollisionWorld::new(None);
    assert!(!drive.spawn(spawn, &empty, &data.surfaces));
    assert!(!drive.sim.as_ref().unwrap().reset_collision_active());
    let collision = ground();
    assert!(drive.spawn(spawn, &collision, &data.surfaces));
    assert!(
        !drive.sim.as_ref().unwrap().reset_collision_active(),
        "a retry of first placement is still initial placement"
    );
    drive.respawn_near([0.0; 2], Some(0.0));
    assert!(!drive.spawn(spawn, &empty, &data.surfaces));
    assert!(!drive.sim.as_ref().unwrap().reset_collision_active(), "failed reset cannot grant protection");
    assert!(drive.spawn(spawn, &collision, &data.surfaces));
    assert!(drive.sim.as_ref().unwrap().reset_collision_active(), "successful reset reaches collision admission");
    let exit = Spawn { position: Vec3::new(5.0, -6.0, 0.0), heading: 0.73 };
    drive.respawn_at(exit);
    let request = drive.waiting_for_road().expect("garage exit waits for its exact marker");
    assert_eq!(request.exact, Some(exit));
    assert!(request.strict, "a garage exit cannot be relocated to a nearby road");
    assert_eq!(drive.place_strict(exit, &collision, &data.surfaces), Strict::Placed);
    // Spawn locates the rigid body; the drawn model origin is offset by its authored pivot.
    let pivot = crate::scenes::world::space::to_render(drive.physics.bounds.pivot.to_array());
    let body_position = drive.position() + drive.current.rotation * pivot;
    assert!(
        (body_position.truncate() - exit.position.truncate()).length() < 1e-4,
        "body {body_position:?}, marker {:?}, pivot {pivot:?}",
        exit.position
    );
    assert!((drive.heading() - exit.heading).abs() < 1e-4, "the stored marker heading is preserved");
    drive.rig.release(&mut renderer);
}
