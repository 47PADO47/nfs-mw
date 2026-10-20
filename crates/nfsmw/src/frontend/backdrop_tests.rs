//! The backdrop of each look builds, draws and frees what it uploaded (needs the install; the tests return at once without
//! `NFSMW_GAME_DIR`).

use game_install::GameDir;

use super::*;
use crate::app::test_backend::NullBackend;
use crate::frontend::look::CarLook;
use crate::frontend::stage::Stage;
use crate::input::ActionState;

fn install() -> Option<GameDir> {
    let dir = std::env::var_os("NFSMW_GAME_DIR")?;
    GameDir::open(dir).ok()
}

fn backend() -> NullBackend {
    NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_texture_tracking()
}

/// Builds the backdrop of `look`, draws a frame and tears it down; returns the meshes it uploaded and the drawn parts.
fn build_and_draw(dir: &GameDir, look: Look) -> (usize, usize, glam::Vec3) {
    let mut renderer = backend();
    let mut menu = MenuScene::with_look(Some(dir.clone()), look.clone());
    menu.init(&mut renderer).unwrap();
    menu.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
    let meshes = renderer.mesh_labels.len();
    let (params, parts) = menu.frame(16.0 / 9.0);
    let (drawn, eye) = (parts.len(), params.camera_position);
    menu.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty(), "close frees every mesh");
    assert_eq!(renderer.resources.lock().unwrap().textures, 0, "close frees every texture");
    (meshes, drawn, eye)
}

#[test]
fn every_look_builds_draws_and_frees_what_it_uploaded() {
    let Some(dir) = install() else { return };
    for look in [
        Look::Main,
        Look::Car(CarLook::default_car()),
        Look::Car(CarLook::career_start()),
        Look::SafeHouse(CarLook::default_car()),
        Look::SafeHouse(CarLook::named("TT", None)),
    ] {
        let (meshes, drawn, _) = build_and_draw(&dir, look.clone());
        assert!(meshes > 0 && drawn > 0, "{look:?} uploads and draws meshes ({meshes} meshes, {drawn} parts)");
    }
}

#[test]
fn the_safe_house_holds_its_own_camera_and_the_crib_the_orbit_camera() {
    let Some(dir) = install() else { return };
    let (_, _, safe) = build_and_draw(&dir, Look::SafeHouse(CarLook::default_car()));
    let pose = Stage::SafeHouse.camera().unwrap();
    assert!((safe - pose.eye()).length() < 1e-3, "{safe:?}");
    let (_, _, crib) = build_and_draw(&dir, Look::Car(CarLook::default_car()));
    assert!((crib - pose.eye()).length() > 1.0, "the crib does not use the safe house camera");
}

#[test]
fn the_rooms_draw_the_solids_their_stage_names() {
    use crate::frontend::room::Room;
    let Some(dir) = install() else { return };
    let mut renderer = backend();
    // The safe house file has sixteen solids; the drawn ones are the room, its floor reflection and the sky.
    for (stage, solids) in [(Stage::SafeHouse, 3), (Stage::Crib, 5)] {
        let room = Room::load(&mut renderer, &dir, stage).unwrap();
        assert_eq!(room.instances().len(), solids, "{stage:?}");
        room.destroy(&mut renderer);
    }
    assert!(renderer.mesh_labels.is_empty());
}

#[test]
fn switching_looks_over_and_over_leaks_nothing() {
    let Some(dir) = install() else { return };
    let mut renderer = backend();
    let looks = [Look::Main, Look::SafeHouse(CarLook::default_car()), Look::Car(CarLook::career_start())];
    for round in 0..3 {
        for look in &looks {
            let mut menu = MenuScene::with_look(Some(dir.clone()), look.clone());
            menu.init(&mut renderer).unwrap();
            menu.close(&mut renderer);
            assert!(renderer.mesh_labels.is_empty(), "round {round}, {look:?}: meshes left");
            assert_eq!(renderer.resources.lock().unwrap().textures, 0, "round {round}, {look:?}: textures left");
        }
    }
}
