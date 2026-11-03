use super::*;
use crate::app::test_backend::NullBackend;
use crate::frontend::{look::CarLook, stage::Stage};

#[test]
#[ignore = "requires installed menu models and stage"]
fn batch17_car_retarget_keeps_room_and_empties_it_until_publication() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    let mut scene = MenuScene::with_look(Some(dir), Look::Car(CarLook::named("BMWM3GTR", None)));
    scene.init(&mut renderer).unwrap();
    scene.settle(&mut renderer);
    let before: Vec<_> = scene.frame(16.0 / 9.0).1.iter().map(|i| (i.mesh, i.transform)).collect();
    let room: Vec<_> = scene.room.as_ref().unwrap().instances().iter().map(|i| i.mesh).collect();
    assert!(before.len() > room.len(), "the outgoing preview contains both room and car");
    assert!(scene.retarget_menu(Look::Car(CarLook::named("CARRERAGT", None))));
    assert!(!scene.ready());
    // R2: as the recorded original car select does, the room stays and stands empty until the new car is uploaded.
    let pending: Vec<_> = scene.frame(16.0 / 9.0).1.iter().map(|i| i.mesh).collect();
    assert_eq!(pending, room, "only the room is drawn while the next car loads");
    scene.settle(&mut renderer);
    assert!(scene.frame(16.0 / 9.0).1.len() > room.len(), "the new car appears once uploaded");
    assert_eq!(scene.room.as_ref().unwrap().instances().iter().map(|i| i.mesh).collect::<Vec<_>>(), room);
    assert!(room.iter().all(|mesh| renderer.mesh_labels.contains_key(mesh)));
    assert!(!scene.retarget_menu(Look::SafeHouse(CarLook::default_car())), "different stage uses normal scene change");
    scene.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty(), "old and replacement car allocations are released");
}

#[test]
#[ignore = "requires installed assembled wheel setup"]
fn batch17_frontend_stands_do_not_sink_assembled_tires_twice() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    for folder in ["BMWM3GTR", "CARRERAGT", "GTO"] {
        let model =
            nfsmw_data::car::load(&dir, folder, &LoadOptions { lod: 'A', all_parts: false, preset: None }).unwrap();
        let setup = model.wheel_setup.as_ref().unwrap();
        let bottom = (0..4)
            .map(|i| setup.tire_offsets[i][2] + setup.fe_compressions[usize::from(i >= 2)] - setup.tire_offsets[i][3])
            .sum::<f32>()
            / 4.0;
        for stage in [Stage::Crib, Stage::SafeHouse, Stage::Customization, Stage::Backroom, Stage::CarLot] {
            let contact = bottom + model.floor_height - stage.car_stand().sink;
            assert!((contact + 0.025).abs() < 1e-4, "{folder} {stage:?}: tire sink is {contact}");
        }
    }
}

#[test]
fn batch17_still_cache_follows_the_effective_inspection_camera() {
    let renderer = NullBackend::new(crate::settings::test_caps::native());
    let mut scene = MenuScene::default();
    let first = crate::frontend::camera_pose::SAFE_HOUSE;
    assert!(!scene.still_unchanged_from(&renderer, Some(first)));
    assert!(scene.still_unchanged_from(&renderer, Some(first)));
    let moved = crate::frontend::camera_pose::CameraPose { yaw: first.yaw + 10.0, ..first };
    assert!(!scene.still_unchanged_from(&renderer, Some(moved)));
    assert!(scene.still_unchanged_from(&renderer, Some(moved)));
    assert!(!scene.still_unchanged_from(&renderer, None), "returning to the screen camera refreshes its capture");
}

#[test]
#[ignore = "requires installed menu models and stage"]
fn r4_a_car_change_keeps_the_room_casting_and_drops_only_the_car() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut renderer =
        NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_reflection_tracking();
    let on = crate::settings::fe_shadows::Snapshot {
        options: crate::settings::fe_shadows::FeShadows::default(),
        world_quality: blackbox_gfx::ShadowQuality::Medium,
    };
    let previous = crate::settings::fe_shadows::replace_active(on);
    let mut scene = MenuScene::with_look(Some(dir), Look::Car(CarLook::named("BMWM3GTR", None)));
    scene.init(&mut renderer).unwrap();
    scene.settle(&mut renderer);
    let room: Vec<_> = scene.room.as_ref().unwrap().shadow_casters().iter().map(|i| i.mesh).collect();
    assert!(!room.is_empty(), "the crib has stage casters");
    assert!(scene.retarget_menu(Look::Car(CarLook::named("CARRERAGT", None))));
    scene.update(&mut renderer, &crate::input::ActionState::default(), 1.0 / 60.0);
    let casters: Vec<_> =
        renderer.shadow_casters.as_ref().expect("casters stay submitted").iter().map(|i| i.mesh).collect();
    crate::settings::fe_shadows::replace_active(previous);
    assert_eq!(casters, room, "only the stage casts while the next car loads");
    scene.close(&mut renderer);
}
