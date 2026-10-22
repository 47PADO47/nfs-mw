//! Installed previews must release their renderer resources when scenes are replaced.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::frontend::MenuScene;
use blackbox_gfx::GlossyModel;

fn backend() -> NullBackend {
    NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_texture_tracking()
        .with_glossy_tracking()
}

fn assert_closed(scene: &mut dyn Scene, renderer: &mut NullBackend) {
    assert!(!renderer.mesh_labels.is_empty(), "the installed fixture must upload actual meshes");
    assert!(!renderer.glossy_materials.is_empty(), "the installed fixture must allocate materials");
    assert!(renderer.resources.lock().unwrap().textures > 0);
    scene.close(renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert!(renderer.glossy_materials.is_empty());
    let counts = renderer.resources.lock().unwrap();
    assert_eq!(counts.textures, 0);
    let destroyed = (counts.mesh_destructions, counts.texture_destructions, counts.glossy_destructions);
    assert!(destroyed.0 > 0 && destroyed.1 > 0 && destroyed.2 > 0);
    drop(counts);
    scene.close(renderer);
    let counts = renderer.resources.lock().unwrap();
    assert_eq!(destroyed, (counts.mesh_destructions, counts.texture_destructions, counts.glossy_destructions));
}

#[test]
#[ignore = "needs installed cars selected by NFSMW_GAME_DIR"]
fn installed_car_preview_close_releases_meshes_textures_and_materials_once() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let model = nfsmw_data::car::load(&dir, "BMWM3GTR", &options).unwrap();
    let mut car = CarScene::new(model, 0.0).with_source(dir, options);
    car.set_car_shading(CarShading::Original);
    let mut renderer = backend();
    car.init(&mut renderer).unwrap();
    assert_closed(&mut car, &mut renderer);
}

#[test]
#[ignore = "needs installed frontend room and car selected by NFSMW_GAME_DIR"]
fn installed_menu_preview_applies_initial_and_live_shading_and_releases_room_once() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut menu = MenuScene::new(Some(dir));
    menu.set_car_shading(CarShading::Original);
    let mut renderer = backend();
    menu.init(&mut renderer).unwrap();
    menu.settle(&mut renderer);
    assert!(renderer.glossy_materials.values().all(|m| m.model == GlossyModel::Shadowed));
    let initial_meshes = renderer.mesh_labels.len();
    menu.set_car_shading(CarShading::OriginalUnshadowed);
    menu.update(&mut renderer, &ActionState::default(), 1.0 / 240.0);
    assert_eq!(renderer.mesh_labels.len(), initial_meshes);
    assert!(renderer.glossy_materials.values().all(|m| m.model == GlossyModel::VertexLit));
    assert_closed(&mut menu, &mut renderer);
}

#[test]
#[ignore = "needs installed paint definitions"]
fn installed_paint_previews_isolate_finish_response_and_keep_camera_and_resources() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let model = nfsmw_data::car::load(&dir, "CARRERAGT", &options).unwrap();
    let mut car = CarScene::new(model, 35.0).with_source(dir, options);
    car.set_car_shading(CarShading::Glossy);
    let mut renderer = backend();
    car.init(&mut renderer).unwrap();
    let meshes = renderer.mesh_labels.len();
    let yaw = car.camera.yaw;
    for name in ["MATTE01_PAINT", "GLOSS_L1_COLOR01", "CHROME01_PAINT"] {
        commands::run(&mut car, &mut renderer, "paint", &[name, "C04430"]).unwrap().unwrap();
        assert_eq!(car.camera.yaw, yaw);
        assert_eq!(car.model.paint.as_ref().unwrap().rgb, [192, 68, 48]);
        assert_eq!(renderer.mesh_labels.len(), meshes);
        let surfaced: Vec<_> = renderer.glossy_materials.values().filter(|m| m.surface.is_some()).collect();
        assert!(!surfaced.is_empty());
        let paint_handle = car.model.paint.as_ref().unwrap().light_material.unwrap();
        let paint_material = super::paint_surface::response(car.model.paint.as_ref().unwrap()).unwrap();
        assert!(surfaced.iter().any(|m| m.surface == Some(paint_material)));
        println!("{name} {paint_handle:08X}: {paint_material:?}");
    }
    let before = car.model.paint.clone();
    assert!(commands::run(&mut car, &mut renderer, "paint", &["NOT_A_PAINT"]).unwrap().is_err());
    assert_eq!(car.model.paint, before);
    assert_closed(&mut car, &mut renderer);
}
