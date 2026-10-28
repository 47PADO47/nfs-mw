use super::*;
use crate::app::test_backend::NullBackend;

#[test]
#[ignore = "requires installed car assets; set NFSMW_GAME_DIR"]
fn my_cars_loads_previews_discards_confirms_and_frees_models() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("installed game directory")).unwrap();
    let mut scene = GarageScene::new(dir, false);
    let mut renderer =
        NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_texture_tracking();
    scene.init(&mut renderer).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !scene.ready() {
        assert!(std::time::Instant::now() < deadline, "My Cars loader did not finish");
        scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(scene.model.message.is_none(), "{:?}", scene.model.message);
    assert!(scene.model.selected().unwrap().performance.categories.iter().any(|part| part.max_level > 0));
    assert_room_and_car(&mut scene);
    for (stage, camera) in [
        (crate::garage::View::Customization, "customize_customizemain"),
        (crate::garage::View::Backroom, "customize_customizemain"),
        (crate::garage::View::Cars, "mycarsmanager"),
    ] {
        scene.garage_view(&mut renderer, stage, camera).unwrap();
        scene.update(&mut renderer, &ActionState::default(), 2.0);
        assert_room_and_car(&mut scene);
        assert_eq!(scene.model.applied, scene.baseline, "a backdrop change must not change the car transaction");
    }
    scene.garage_view(&mut renderer, crate::garage::View::Customization, "customize_customizemain").unwrap();
    scene.update(&mut renderer, &ActionState::default(), 8.0);
    scene.garage_view(&mut renderer, crate::garage::View::Customization, "customize_customizeparts_260").unwrap();
    scene.update(&mut renderer, &ActionState::default(), 8.0);
    let hood = nfsmw_data::frontend_camera::PoseBook::load(&scene.dir)
        .unwrap()
        .screen("customize_customizeparts_260")
        .expect("installed hood camera");
    let expected = crate::frontend::camera_pose::CameraPose::from_angle(&hood).eye();
    assert!((scene.frame(16.0 / 9.0).0.camera_position - expected).length() < 0.001);
    let folder = scene
        .model
        .cars
        .iter()
        .find(|car| car.paintable && car.unavailable.is_none())
        .expect("unlocked paintable installed car")
        .folder
        .clone();
    let baseline = Selection { car: folder, ..Selection::default() };
    scene.confirm_garage(&mut renderer, baseline.clone()).unwrap();
    scene.begin_garage(&mut renderer).unwrap();
    let painted = Selection { color: Some([22, 70, 190]), ..baseline.clone() };
    scene.preview_garage(&mut renderer, painted.clone()).unwrap();
    assert_eq!(scene.model.draft, painted);
    assert_eq!(scene.model.applied, baseline, "preview must not commit");
    scene.cancel_garage(&mut renderer).unwrap();
    assert_eq!(scene.model.draft, baseline);
    scene.begin_garage(&mut renderer).unwrap();
    scene.confirm_garage(&mut renderer, painted.clone()).unwrap();
    assert_eq!(scene.model.applied, painted);
    scene.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert_eq!(renderer.resources.lock().unwrap().textures, 0);
}

fn assert_room_and_car(scene: &mut GarageScene) {
    let (frame, instances) = scene.frame(16.0 / 9.0);
    assert!(frame.camera_position.is_finite());
    let room = |part: &blackbox_gfx::Instance| part.key.parts().0 & 0x3fff_ffff == (u32::MAX - 2) & 0x3fff_ffff;
    assert!(instances.iter().any(room), "the installed room is drawn");
    assert!(instances.iter().any(|part| !room(part)), "the customized car is drawn");
    assert!(
        instances.iter().all(|part| part.key.parts().1 != crate::scenes::car::FLOOR_PART),
        "the placeholder viewer floor must not cover the room"
    );
}

#[test]
fn batch12_preview_geometry_key_ignores_gauges_performance_and_tuning() {
    let previous = Selection { car: "BMWM3GTR".into(), ..Default::default() };
    let mut next = previous.clone();
    next.gauges.needle = Some([40, 80, 120]);
    next.tuning.profiles[0][0] = 3;
    next.performance.levels[0] = 1;
    assert!(same_geometry(&previous, &next));
    next.color = Some([30, 70, 180]);
    assert!(!same_geometry(&previous, &next));
    next = previous.clone();
    next.parts.parts[0] = Some(1234);
    assert!(!same_geometry(&previous, &next));
    next = previous.clone();
    next.car = "TT".into();
    assert!(!same_geometry(&previous, &next));
}

#[test]
#[ignore = "requires an installed game directory"]
fn batch12_my_cars_failed_initial_preview_cannot_open_an_empty_garage() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = GarageScene::new(dir, false);
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    scene.model.message = Some(Err("synthetic initial car load failure".into()));
    let error = scene.begin_garage(&mut renderer).unwrap_err();
    assert!(error.to_string().contains("synthetic initial car load failure"));
    assert!(scene.baseline.car.is_empty());
}

#[test]
#[ignore = "requires installed My Cars assets"]
fn batch12_my_cars_reuses_uploaded_geometry_for_unchanged_and_gauge_previews() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = GarageScene::new(dir, false);
    let mut renderer =
        NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_texture_tracking();
    scene.init(&mut renderer).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !scene.ready() {
        assert!(std::time::Instant::now() < deadline);
        scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(scene.model.message.is_none(), "{:?}", scene.model.message);
    let meshes = renderer.mesh_labels.clone();
    let textures = renderer.texture_labels.clone();
    let selection = scene.model.draft.clone();
    for _ in 0..3 {
        scene.preview_garage(&mut renderer, selection.clone()).unwrap();
        assert_eq!(renderer.mesh_labels, meshes, "an unchanged preview preserves uploaded mesh handles");
        assert_eq!(renderer.texture_labels, textures);
    }
    let mut changed = selection;
    changed.gauges.digits = Some([60, 120, 180]);
    scene.preview_garage(&mut renderer, changed.clone()).unwrap();
    assert_eq!(scene.model.draft, changed);
    assert_eq!(renderer.mesh_labels, meshes, "HUD color edits do not rebuild the 3D car");
    assert_eq!(renderer.texture_labels, textures);
    scene.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert!(renderer.texture_labels.is_empty());
}

#[test]
#[ignore = "requires installed My Cars assets"]
fn batch12_my_cars_restores_a_previous_car_without_uploading_it_again() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = GarageScene::new(dir, false);
    let mut renderer =
        NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_texture_tracking();
    scene.init(&mut renderer).unwrap();
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    while !scene.ready() {
        assert!(std::time::Instant::now() < deadline);
        scene.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    assert!(scene.model.message.is_none(), "{:?}", scene.model.message);
    let original = scene.model.draft.clone();
    let meshes = renderer.mesh_labels.clone();
    let other = scene.model.cars.iter().find(|car| car.folder != original.car).unwrap().folder.clone();
    scene.preview_garage(&mut renderer, Selection { car: other, ..Default::default() }).unwrap();
    let after_switch = renderer.mesh_labels.clone();
    assert!(meshes.keys().all(|handle| after_switch.contains_key(handle)), "the old preview stays parked");
    let textures = renderer.texture_labels.clone();
    scene.preview_garage(&mut renderer, original.clone()).unwrap();
    assert_eq!(scene.model.draft, original);
    assert_eq!(renderer.mesh_labels, after_switch, "returning to the old car allocates no meshes");
    assert_eq!(renderer.texture_labels, textures, "returning to the old car allocates no textures");
    scene.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert!(renderer.texture_labels.is_empty());
}
