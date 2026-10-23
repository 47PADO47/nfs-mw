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
    NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_texture_tracking()
        .with_reflection_tracking()
}

/// Builds the backdrop of `look`, draws a frame and tears it down; returns the meshes it uploaded and the drawn parts.
fn build_and_draw(dir: &GameDir, look: Look) -> (usize, usize, glam::Vec3) {
    let mut renderer = backend();
    let mut menu = MenuScene::with_look(Some(dir.clone()), look.clone());
    menu.init(&mut renderer).unwrap();
    assert!(!menu.ready(), "the backdrop is not ready before its worker is read and uploaded");
    menu.settle(&mut renderer);
    assert!(menu.ready());
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
    // The car and the room are read on a worker; the camera starts when they are up.
    menu.settle(renderer);
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
    for stage in [Stage::SafeHouse, Stage::Crib] {
        // Stage files vary across installations. Compare the selected file's drawable solids in order,
        // rather than assuming a historical count of five crib solids.
        let bytes = dir.read(stage.file()).unwrap();
        let expected: Vec<String> = blackbox_solid::read_solids(&bytes)
            .unwrap()
            .into_iter()
            .filter(|solid| stage.draws(&solid.name))
            .filter(|solid| !solid.vertices.is_empty() && !solid.indices.is_empty())
            .map(|solid| solid.name)
            .collect();
        assert!(expected.iter().any(|name| name == "BACKDROP_CRIB"), "{stage:?} includes its room");
        let room = Room::load(&mut renderer, &dir, stage).unwrap();
        let actual: Vec<String> =
            room.instances().iter().map(|instance| renderer.mesh_labels[&instance.mesh].clone()).collect();
        assert_eq!(actual, expected, "{stage:?} uploads exactly the selected file's drawable solids in file order");
        room.destroy(&mut renderer);
        assert!(renderer.mesh_labels.is_empty(), "{stage:?} releases every room mesh");
    }
}

#[test]
fn the_room_is_drawn_in_its_files_order_whatever_the_mesh_handles_are() {
    use crate::frontend::room::Room;
    let Some(dir) = install() else { return };
    // The handles fall as the meshes are made, as a renderer that reuses freed slots may hand them out.
    let mut renderer = backend().with_descending_meshes();
    let room = Room::load(&mut renderer, &dir, Stage::SafeHouse).unwrap();
    let drawn: Vec<&str> = room.instances().iter().map(|i| renderer.mesh_labels[&i.mesh].as_str()).collect();
    // The mirrored garage and the sky come first, so the floor blends over them.
    let order = |name: &str| drawn.iter().position(|d| *d == name).unwrap();
    assert!(
        order("BACKDROP_REFLECTION") < order("BACKDROP_CURRGEN") && order("BACKDROP_CURRGEN") < order("BACKDROP_CRIB")
    );
    room.destroy(&mut renderer);
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
            menu.settle(&mut renderer);
            menu.close(&mut renderer);
            assert!(renderer.mesh_labels.is_empty(), "round {round}, {look:?}: meshes left");
            assert_eq!(renderer.resources.lock().unwrap().textures, 0, "round {round}, {look:?}: textures left");
        }
    }
}

#[test]
fn a_menu_closed_while_its_worker_reads_uploads_nothing() {
    let Some(dir) = install() else { return };
    let mut renderer = backend();
    let mut menu = MenuScene::with_look(Some(dir), Look::Main);
    menu.init(&mut renderer).unwrap();
    menu.close(&mut renderer);
    assert!(menu.ready(), "a closed backdrop has nothing left to wait for");
    menu.update(&mut renderer, &ActionState::default(), 1.0 / 60.0);
    assert!(renderer.mesh_labels.is_empty(), "the worker's result is dropped");
}

#[test]
fn the_backdrop_draws_nothing_until_the_car_is_up() {
    let Some(dir) = install() else { return };
    let mut renderer = backend();
    let mut menu = MenuScene::with_look(Some(dir), Look::SafeHouse(CarLook::default_car()));
    menu.init(&mut renderer).unwrap();
    assert!(menu.frame(16.0 / 9.0).1.is_empty(), "nothing is drawn while the worker reads");
    menu.settle(&mut renderer);
    assert!(!menu.frame(16.0 / 9.0).1.is_empty());
    menu.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
}

#[test]
fn a_backdrop_without_an_install_is_ready_at_once() {
    let mut menu = MenuScene::new(None);
    let mut renderer = backend();
    menu.init(&mut renderer).unwrap();
    assert!(menu.ready());
    assert!(menu.frame(16.0 / 9.0).1.is_empty());
}

#[test]
#[ignore = "requires installed menu camera and geometry; set NFSMW_GAME_DIR"]
fn navigating_before_the_backdrop_uploads_keeps_the_latest_camera_target() {
    let dir = install().expect("installed menu camera and geometry");
    let mut renderer = backend();
    let rig = CameraRig::default();
    let mut menu = MenuScene::with_look(Some(dir.clone()), Look::Main).with_rig(rig.clone());
    menu.init(&mut renderer).unwrap();
    assert!(!menu.ready());
    rig.show("mainmenu");
    rig.show("career_manager");
    menu.settle(&mut renderer);
    let eye = eye_after(&mut menu, &mut renderer, 8.0);
    assert!((eye - eye_of(&dir, "angle_career_manager")).length() < 1e-3, "latest screen wins: {eye:?}");
    menu.close(&mut renderer);
}

#[test]
#[ignore = "requires installed menu camera and geometry; set NFSMW_GAME_DIR"]
fn a_settled_direct_screen_uses_its_own_pose_instead_of_the_stage_intro() {
    let dir = install().expect("installed menu camera and geometry");
    let mut renderer = backend();
    let rig = CameraRig::settled();
    let mut menu = MenuScene::with_look(Some(dir.clone()), Look::Main).with_rig(rig.clone());
    menu.init(&mut renderer).unwrap();
    rig.show("career_manager");
    menu.settle(&mut renderer);
    let eye = eye_after(&mut menu, &mut renderer, 0.0);
    assert!((eye - eye_of(&dir, "angle_career_manager")).length() < 1e-3, "direct screen pose: {eye:?}");
    menu.close(&mut renderer);
}

#[test]
#[ignore = "requires installed menu camera and geometry; set NFSMW_GAME_DIR"]
fn menu_reflections_capture_only_the_room_and_follow_live_graphics_settings() {
    use blackbox_gfx::ReflectionQuality;
    use blackbox_gfx::reflection::ALL_FACES;
    let dir = install().expect("installed menu camera and geometry");
    let mut renderer = backend();
    let rig = CameraRig::settled();
    let mut menu = menu_on(&dir, Look::Main, &rig, &mut renderer);
    let set_quality = |renderer: &mut NullBackend, quality| {
        let mut graphics = *renderer.graphics();
        graphics.reflections = quality;
        renderer.apply_graphics(&graphics);
    };
    set_quality(&mut renderer, ReflectionQuality::High);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    let capture = renderer.reflection.as_ref().expect("menu submits a local room capture");
    assert_eq!(capture.quality, ReflectionQuality::High);
    assert_eq!(capture.refresh, ALL_FACES);
    for face in &capture.faces {
        let room = menu.room.as_ref().unwrap().instances();
        assert_eq!(face.len(), room.len(), "the car never reflects itself");
        for (actual, expected) in face.iter().zip(room) {
            assert_eq!((actual.mesh, actual.transform, actual.key), (expected.mesh, expected.transform, expected.key));
        }
    }
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(renderer.reflection.as_ref().unwrap().refresh, 0, "the stationary room reuses its capture");
    let mut graphics = *renderer.graphics();
    graphics.reflection_resolution.cube = blackbox_gfx::reflection::ReflectionSize::Auto;
    renderer.apply_graphics(&graphics);
    renderer.resize([2560, 1440]);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(renderer.reflection.as_ref().unwrap().refresh, ALL_FACES, "automatic resolution follows resize");
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(renderer.reflection.as_ref().unwrap().refresh, 0);
    renderer.set_reflection_scene(None);
    menu.reattached(&mut renderer);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(
        renderer.reflection.as_ref().unwrap().refresh,
        ALL_FACES,
        "returning from driving restores the room capture"
    );
    set_quality(&mut renderer, ReflectionQuality::Low);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert_eq!(renderer.reflection.as_ref().unwrap().refresh, ALL_FACES, "quality changes recapture");
    set_quality(&mut renderer, ReflectionQuality::Off);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(renderer.reflection.is_none(), "disabling reflections releases the room capture");
    set_quality(&mut renderer, ReflectionQuality::High);
    menu.set_car_shading(CarShading::Simple);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(renderer.reflection.is_none(), "simple shading follows the same rule as driving");
    menu.close(&mut renderer);
}

#[test]
#[ignore = "requires installed safe-house geometry; set NFSMW_GAME_DIR"]
fn safe_house_floor_capture_uses_only_the_car_and_keeps_main_camera_cuts() {
    let dir = install().expect("installed safe house");
    let mut renderer = backend();
    let mut graphics = *renderer.graphics();
    graphics.reflections = blackbox_gfx::ReflectionQuality::High;
    renderer.apply_graphics(&graphics);
    let rig = CameraRig::settled();
    let mut menu = menu_on(&dir, Look::SafeHouse(CarLook::default_car()), &rig, &mut renderer);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    let capture = renderer.planar.as_ref().expect("verified floor receives the car capture");
    assert!(!capture.instances.is_empty());
    assert_eq!(capture.clip_plane, [0.0, 0.0, 1.0, 0.0]);
    assert_eq!(capture.receivers, [menu.room.as_ref().unwrap().floor_texture().unwrap()]);
    assert!(capture.instances.iter().all(|part| matches!(car_part(part), Part::Car)));
    let (main, parts) = menu.frame(16.0 / 9.0);
    assert!(main.camera_cut, "reduced captures must not consume the main camera cut");
    let reflection = Mat4::from_scale(Vec3::new(1.0, 1.0, -1.0));
    assert_eq!(capture.frame.view, main.view * reflection);
    assert_eq!(capture.frame.camera_position, reflection.transform_point3(main.camera_position));
    for source in &capture.instances {
        assert!(parts.iter().any(|part| part.mesh == source.mesh && part.transform == source.transform));
    }
    let mut graphics = *renderer.graphics();
    graphics.reflections = blackbox_gfx::ReflectionQuality::Off;
    renderer.apply_graphics(&graphics);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(renderer.planar.is_none());
    graphics.reflections = blackbox_gfx::ReflectionQuality::High;
    renderer.apply_graphics(&graphics);
    menu.reattached(&mut renderer);
    menu.update(&mut renderer, &ActionState::default(), 0.0);
    assert!(renderer.planar.is_some());
    menu.close(&mut renderer);
    assert!(renderer.planar.is_none() && renderer.shadow_casters.is_none());
    assert!(renderer.mesh_labels.is_empty());
}

#[test]
#[ignore = "requires installed stage geometry; set NFSMW_GAME_DIR"]
fn floor_receiver_rejects_nonplanar_replacement_geometry_and_crib_helpers_are_hidden() {
    let dir = install().expect("installed stage");
    let file = dir.read(Stage::SafeHouse.file()).unwrap();
    let mut solids = blackbox_solid::read_solids(&file).unwrap();
    assert!(crate::frontend::room_floor::floor_texture(&solids, Stage::SafeHouse).is_some());
    for solid in &mut solids {
        for vertex in &mut solid.vertices {
            vertex.position[2] += 1.0;
        }
    }
    assert!(crate::frontend::room_floor::floor_texture(&solids, Stage::SafeHouse).is_none());
    let mut renderer = backend();
    let room = crate::frontend::room::Room::load(&mut renderer, &dir, Stage::Crib).unwrap();
    assert!(room.instances().iter().all(|part| matches!(
        renderer.mesh_labels[&part.mesh].as_str(),
        "BACKDROP_CRIB" | "BACKDROP_CURRGEN" | "BACKDROP_CAST_SHADOW_MAP"
    )));
    assert!(room.shadow_casters().iter().all(|part| renderer.mesh_labels[&part.mesh] == "BACKDROP_CAST_SHADOW_MAP"));
    assert!(!room.shadow_casters().is_empty());
    room.destroy(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
}
