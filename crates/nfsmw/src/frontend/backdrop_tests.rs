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

/// The eye of `look`'s camera after the backdrop has run for `seconds` of 60 Hz frames.
fn eye_after(dir: &GameDir, look: Look, seconds: f32) -> glam::Vec3 {
    let mut renderer = backend();
    let mut menu = MenuScene::with_look(Some(dir.clone()), look);
    menu.init(&mut renderer).unwrap();
    for _ in 0..(seconds * 60.0).round() as u32 {
        menu.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
    }
    let eye = menu.frame(16.0 / 9.0).0.camera_position;
    menu.close(&mut renderer);
    eye
}

#[test]
fn the_safe_house_camera_moves_onto_its_pose_and_the_crib_keeps_the_orbit_camera() {
    let Some(dir) = install() else { return };
    let look = || Look::SafeHouse(CarLook::default_car());
    let rest = crate::frontend::camera_pose::SAFE_HOUSE.eye();
    let start = eye_after(&dir, look(), 0.0);
    assert!((start - rest).length() > 3.0, "the move starts away from the final pose: {start:?}");
    let half = eye_after(&dir, look(), 1.5);
    assert!((half - rest).length() > 0.1 && (half - start).length() > 0.1, "and is on its way: {half:?}");
    let end = eye_after(&dir, look(), 3.5);
    assert!((end - rest).length() < 1e-3, "and ends on the final pose: {end:?}");
    let crib = eye_after(&dir, Look::Car(CarLook::default_car()), 1.0);
    assert!((crib - rest).length() > 1.0, "the crib does not use the safe house camera");
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
