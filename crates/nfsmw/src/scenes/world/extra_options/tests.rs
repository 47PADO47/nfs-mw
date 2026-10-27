use super::*;
use crate::viewer::Scene;
mod career_tests;
#[test]
#[ignore = "requires installed cars, roads and world attributes"]
fn installed_extra_options_preserve_views_pause_and_teleport_without_editing_density() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap();
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        super::super::Options {
            at: None,
            height: 30.0,
            heading: 0.0,
            pitch: 0.0,
            fog_distance: 1000.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    scene.extra = Default::default();
    let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    let car = scene.garage.cars.iter().find(|c| c.category == crate::garage::Category::Racing).unwrap().folder.clone();
    scene.apply_garage_selection(&mut renderer, crate::garage::Selection { car, ..Default::default() }).unwrap();
    let idle = crate::input::ActionState::default();
    for _ in 0..2000 {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
        if scene.ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(scene.ready(), "the installed player must actually reach a supported road");
    let pose = scene.players.lead().unwrap().physics_pose();
    let lens = scene.gameplay_lens(16.0 / 9.0);
    scene.extra_action(&mut renderer, Action::Freecam).unwrap();
    let detached = scene.gameplay_lens(16.0 / 9.0);
    assert!(lens.position.distance(detached.position) < 0.0001);
    assert!((lens.fov_y_degrees - detached.fov_y_degrees).abs() < 0.0001);
    assert!(lens.forward.distance(detached.forward) < 0.0001);
    scene.camera.position += Vec3::new(1000.0, 1000.0, 100.0);
    scene.extra_action(&mut renderer, Action::Freecam).unwrap();
    assert_eq!(scene.players.lead().unwrap().physics_pose(), pose, "return must not relocate the car");
    assert!(scene.players.lead().unwrap().waiting_for_road().is_none());
    scene.extra.testing.paused = true;
    let clock = scene.clock;
    for _ in 0..60 {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
    }
    assert_eq!(scene.clock, clock);
    assert_eq!(scene.players.lead().unwrap().physics_pose(), pose);
    let density = scene.traffic_setting;
    scene.extra.testing.traffic_off = true;
    scene.prepare_extra();
    assert_eq!(scene.traffic_setting, density);
    scene.extra_action(&mut renderer, Action::Clear).unwrap();
    scene.prepare_extra();
    assert_eq!(scene.traffic_setting, density);
    assert!(!scene.extra.testing.traffic_off);
    let at = [pose.position.x + 20.0, pose.position.y];
    scene.extra_action(&mut renderer, Action::Teleport(at, Some(1.0))).unwrap();
    let request = scene.players.lead().unwrap().waiting_for_road().unwrap();
    assert_eq!(request.near, at);
    assert_eq!(request.heading, Some(1.0));
    assert!(scene.extra.previous_location.is_some());
    scene.extra_action(&mut renderer, Action::Return).unwrap();
    let request = scene.players.lead().unwrap().waiting_for_road().unwrap();
    assert_eq!(request.near, [pose.position.x, pose.position.y]);
    for _ in 0..200 {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
        if scene.players.lead().unwrap().waiting_for_road().is_none() {
            break;
        }
    }
    let pose = scene.players.lead().unwrap().physics_pose();
    scene.extra_action(&mut renderer, Action::Teleport([pose.position.x, pose.position.y], Some(1.0))).unwrap();
    for _ in 0..200 {
        scene.update(&mut renderer, &idle, 1.0 / 60.0);
        if scene.players.lead().unwrap().waiting_for_road().is_none() {
            break;
        }
    }
    let drive = scene.players.lead().unwrap();
    assert!(drive.waiting_for_road().is_none(), "teleport must finish placing on the road");
    assert!((drive.heading() - 1.0).abs() < 0.01, "saved heading must be restored exactly");
    println!("Installed freecam/FOV retention, return, pause, traffic restore and teleport requests passed");
}

fn garage_fixture() -> (WorldScene, crate::app::test_backend::NullBackend) {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        super::super::Options {
            at: None,
            height: 30.0,
            heading: 0.0,
            pitch: 0.0,
            fog_distance: 1000.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    scene.extra = Default::default();
    let mut renderer = crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native());
    scene.init(&mut renderer).unwrap();
    scene
        .apply_garage_selection(
            &mut renderer,
            crate::garage::Selection { car: "COBALTSS".into(), ..Default::default() },
        )
        .unwrap();
    settle_car(&mut scene, &mut renderer);
    (scene, renderer)
}
fn settle_car(scene: &mut WorldScene, renderer: &mut dyn RenderBackend) {
    for _ in 0..2000 {
        scene.update(renderer, &crate::input::ActionState::default(), 1.0 / 60.0);
        if scene.ready() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("the swapped car must reach a supported road");
}
fn swap_car(scene: &mut WorldScene, renderer: &mut dyn RenderBackend) {
    let car = scene
        .garage
        .cars
        .iter()
        .find(|car| {
            car.category == crate::garage::Category::Racing
                && car.folder != scene.garage.applied.car
                && car.unavailable.is_none()
        })
        .unwrap()
        .folder
        .clone();
    scene.apply_garage_selection(renderer, crate::garage::Selection { car, ..Default::default() }).unwrap();
    settle_car(scene, renderer);
}
#[test]
#[ignore = "requires installed cars and road collision"]
fn installed_extra_options_car_swap_keeps_normal_camera_following() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap();
    let (mut scene, mut renderer) = garage_fixture();
    // Reproduce old saved preferences as well as the default: this must not imply a detached view.
    assert!(scene.extra.camera.preserve_framing);
    swap_car(&mut scene, &mut renderer);
    assert!(scene.extra.selected_bookmark.is_none(), "normal swaps must not create a fixed comparison view");
    assert!(!scene.extra.freecam);
    assert_eq!(scene.view, View::Chase);
    let before = scene.gameplay_lens(16.0 / 9.0).position;
    let car_before = scene.players.lead().unwrap().position();
    let mut throttle = crate::input::ActionState::default();
    throttle.update_for_test(&crate::input::Bindings::default(), &[bevy_input::keyboard::KeyCode::ArrowUp], &[], false);
    for _ in 0..120 {
        scene.update(&mut renderer, &throttle, 1.0 / 60.0);
    }
    assert!(scene.players.lead().unwrap().position().distance(car_before) > 0.5, "the player must actually drive");
    assert!(
        scene.gameplay_lens(16.0 / 9.0).position.distance(before) > 0.5,
        "the camera must follow without a manual recenter"
    );
}
#[test]
#[ignore = "requires installed cars and road collision"]
fn installed_extra_options_car_swap_preserves_explicit_bookmarks_and_freecam() {
    let _career = crate::career::SHARED_CAREER_TEST.lock().unwrap();
    let (mut scene, mut renderer) = garage_fixture();
    let lens = scene.gameplay_lens(16.0 / 9.0);
    scene.extra.selected_bookmark = Some(Bookmark {
        name: "Selected screenshot view".into(),
        relative: false,
        eye: lens.position.to_array(),
        forward: lens.forward.to_array(),
        up: lens.view.inverse().y_axis.truncate().to_array(),
        fov: lens.fov_y_degrees,
    });
    swap_car(&mut scene, &mut renderer);
    assert_eq!(scene.extra.selected_bookmark.as_ref().unwrap().name, "Selected screenshot view");
    assert!(scene.gameplay_lens(16.0 / 9.0).position.distance(lens.position) < 0.0001);
    scene.extra_action(&mut renderer, Action::Center).unwrap();
    scene.extra_action(&mut renderer, Action::Freecam).unwrap();
    let eye = scene.gameplay_lens(16.0 / 9.0).position;
    swap_car(&mut scene, &mut renderer);
    assert!(scene.extra.freecam);
    assert_eq!(scene.view, View::Fly, "freecam must remain movable after swapping cars");
    assert!(scene.extra.selected_bookmark.is_none(), "freecam must not be replaced with a fixed bookmark");
    assert!(scene.gameplay_lens(16.0 / 9.0).position.distance(eye) < 0.0001);
}
