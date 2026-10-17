use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::drive::{CarPose, CarRig, CarSim, SpawnRequest};
use crate::settings::CarShading;
use glam::{Quat, Vec3};
use nfsmw_data::car::{LoadOptions, WheelPose, physics::PhysicsData};

#[test]
#[ignore = "requires installed player model and physics"]
fn installed_player_road_feed_reuses_interpolated_parts_without_uploads() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let physics = PhysicsData::load(&dir).unwrap().car("bmwm3gtr").unwrap();
    let model =
        nfsmw_data::car::load_driving(&dir, "BMWM3GTR", &LoadOptions { lod: 'A', all_parts: false, preset: None })
            .unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    let rig = CarRig::upload(&mut renderer, 0, model, CarShading::Original, Vec3::Z);
    let mut drive = Drive::new(
        "BMWM3GTR".to_owned(),
        rig,
        physics,
        SpawnRequest { near: [1945.0, 1850.0], heading: None, exact: None },
        None,
        Default::default(),
    );
    let mut wet = Vec::new();
    drive.road_reflection_instances(&mut wet);
    assert!(wet.is_empty(), "unplaced player cannot enter reflection feed");
    drive.sim = Some(CarSim::for_rig(drive.physics.clone(), &drive.rig));
    drive.previous = CarPose {
        position: Vec3::new(1945.0, 1850.0, 149.9),
        rotation: Quat::from_euler(glam::EulerRot::XYZ, 0.12, -0.3, 0.7),
        wheels: [WheelPose { steer: 0.2, spin: 1.0, travel: 0.08 }; 4],
    };
    drive.current = CarPose {
        position: drive.previous.position + Vec3::new(0.3, -0.4, 0.1),
        rotation: Quat::from_euler(glam::EulerRot::XYZ, 0.15, -0.1, 0.8),
        ..drive.previous
    };
    drive.alpha = 0.37;
    let mut normal = Vec::new();
    drive.instances(&mut normal);
    drive.road_reflection_instances(&mut wet);
    assert!(!normal.is_empty());
    assert_eq!(normal.len(), wet.len());
    let root = drive.pose().transform();
    let mut reflected = root;
    reflected.z_axis = -reflected.z_axis;
    for (a, b) in wet.iter().zip(&normal) {
        assert_eq!((a.mesh, a.key), (b.mesh, b.key));
        let expected = reflected * (root.inverse() * b.transform);
        assert!(a.transform.abs_diff_eq(expected, 0.001));
    }
    let resources = renderer.mesh_labels.len();
    let shown = drive.pose();
    for _ in 0..360 {
        wet.clear();
        drive.road_reflection_instances(&mut wet);
    }
    assert_eq!(drive.pose(), shown, "capture must not advance physics or interpolation");
    assert_eq!(renderer.mesh_labels.len(), resources);
    drive.rig.release(&mut renderer);
}
