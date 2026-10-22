//! The backdrop of each look builds, draws and frees what it uploaded (needs the install; the tests return at once without
//! `NFSMW_GAME_DIR`).

use game_install::GameDir;

use super::*;
use crate::app::test_backend::NullBackend;
use crate::frontend::camera_rig::CameraRig;
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

/// A backdrop of `look` on the camera `rig`, with its scene and renderer.
fn menu_on(dir: &GameDir, look: Look, rig: &CameraRig, renderer: &mut NullBackend) -> MenuScene {
    let mut menu = MenuScene::with_look(Some(dir.clone()), look).with_rig(rig.clone());
    menu.init(renderer).unwrap();
    menu
}

/// Runs `menu` for `seconds` of 60 Hz frames and returns the eye of its camera.
fn eye_after(menu: &mut MenuScene, renderer: &mut NullBackend, seconds: f32) -> glam::Vec3 {
    for _ in 0..(seconds * 60.0).round() as u32 {
        menu.update(renderer, &ActionState::default(), 1.0 / 60.0);
    }
    menu.frame(16.0 / 9.0).0.camera_position
}

/// The eye of the pose collection `name` in the install's `FE_ATTRIB.bin`.
fn eye_of(dir: &GameDir, name: &str) -> glam::Vec3 {
    let angle = nfsmw_data::frontend_camera::load(dir, &[name]).unwrap()[0];
    crate::frontend::camera_pose::CameraPose::from_angle(&angle).eye()
}

#[test]
fn the_safe_house_camera_moves_onto_its_pose() {
    let Some(dir) = install() else { return };
    let rest = eye_of(&dir, "angle_final_career_safehouse");
    let mut renderer = backend();
    let rig = CameraRig::default();
    let mut menu = menu_on(&dir, Look::SafeHouse(CarLook::default_car()), &rig, &mut renderer);
    let start = eye_after(&mut menu, &mut renderer, 0.0);
    assert!(
        (start - eye_of(&dir, "angle_entry_career_safehouse")).length() < 1e-3,
        "starts on the entry pose: {start:?}"
    );
    let half = eye_after(&mut menu, &mut renderer, 1.5);
    assert!((half - rest).length() > 0.1 && (half - start).length() > 0.1, "is on its way: {half:?}");
    let end = eye_after(&mut menu, &mut renderer, 2.0);
    assert!((end - rest).length() < 1e-3, "and ends on the final pose: {end:?}");
    menu.close(&mut renderer);
}

#[test]
fn the_crib_camera_plays_its_intro_then_follows_the_screens_and_survives_a_change_of_look() {
    let Some(dir) = install() else { return };
    let mut renderer = backend();
    let rig = CameraRig::default();
    let mut menu = menu_on(&dir, Look::Main, &rig, &mut renderer);
    rig.show("mainmenu");
    let start = eye_after(&mut menu, &mut renderer, 0.0);
    assert!((start - eye_of(&dir, "angle_entry_main_fe")).length() < 1e-3, "the intro starts far off: {start:?}");
    let main = eye_of(&dir, "angle_final_main_fe");
    let rest = eye_after(&mut menu, &mut renderer, 6.5);
    assert!((rest - main).length() < 1e-3, "and ends on the main menu pose: {rest:?}");
    // The career menu opens: a new backdrop with the car of the profile, on the same camera, which eases to its pose.
    menu.close(&mut renderer);
    let mut menu = menu_on(&dir, Look::Car(CarLook::default_car()), &rig, &mut renderer);
    assert!((eye_after(&mut menu, &mut renderer, 0.0) - main).length() < 1e-3, "the swap keeps the camera");
    rig.show("career_manager");
    let mid = eye_after(&mut menu, &mut renderer, 0.3);
    let career = eye_of(&dir, "angle_career_manager");
    assert!((mid - career).length() > 0.05 && (mid - main).length() > 0.05, "moves between the poses: {mid:?}");
    assert!((eye_after(&mut menu, &mut renderer, 1.0) - career).length() < 1e-3, "and settles on the career pose");
    // Back: the main menu's pose again.
    rig.show("mainmenu");
    assert!((eye_after(&mut menu, &mut renderer, 1.0) - main).length() < 1e-3, "backing out returns to the first pose");
    menu.close(&mut renderer);
}

#[test]
fn the_showcase_keeps_the_orbit_camera() {
    let Some(dir) = install() else { return };
    let mut renderer = backend();
    let rig = CameraRig::default();
    let mut menu = menu_on(&dir, Look::Main, &rig, &mut renderer);
    let main = eye_after(&mut menu, &mut renderer, 7.0);
    menu.close(&mut renderer);
    let mut showcase = menu_on(&dir, Look::Showcase(CarLook::default_car()), &rig, &mut renderer);
    let eye = eye_after(&mut showcase, &mut renderer, 1.0);
    assert!((eye - main).length() > 0.5, "the Showcase does not use the front end's camera: {eye:?}");
    showcase.close(&mut renderer);
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
