use super::*;
use crate::viewer::Scene;

#[test]
#[ignore = "requires an installed game"]
fn installed_garage_repaints_and_rejects_bad_requests_without_replacing_the_car() {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
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
    let entry = scene.garage.cars.iter().find(|car| car.folder == "COBALTSS").unwrap();
    let car = entry.folder.clone();
    let caps = crate::settings::test_caps::native();
    let mut renderer = crate::app::test_backend::NullBackend::new(caps);
    scene.init(&mut renderer).unwrap();
    let selection = Selection { car: car.clone(), color: Some([20, 140, 230]), ..Default::default() };
    scene.apply_garage_selection(&mut renderer, selection.clone()).unwrap();
    assert_eq!(scene.garage.applied, selection);
    for _ in 0..2000 {
        scene.update(&mut renderer, &crate::input::ActionState::default(), 1.0 / 60.0);
        if scene.ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(scene.ready(), "the selected car must actually be placed before checking repaint");
    let before = scene.drive.as_ref().unwrap().pose();
    let paint = Selection { car: car.clone(), color: Some([220, 40, 30]), ..Default::default() };
    scene.apply_garage_selection(&mut renderer, paint.clone()).unwrap();
    assert_eq!(scene.drive.as_ref().unwrap().pose(), before);
    assert_eq!(scene.garage.applied, paint);
    assert!(!scene.drive.as_ref().unwrap().hud_state().unwrap().has_nos);
    let mut upgraded = paint.clone();
    upgraded.performance = scene.garage.selected().unwrap().performance.maximum();
    scene.apply_garage_selection(&mut renderer, upgraded.clone()).unwrap();
    assert_eq!(scene.drive.as_ref().unwrap().pose(), before, "upgrades must not respawn the car");
    assert!(scene.drive.as_ref().unwrap().hud_state().unwrap().has_nos);
    assert_eq!(scene.drive.as_ref().unwrap().performance, upgraded.performance);
    let mut invalid = upgraded.clone();
    invalid.performance.levels[4] = 255;
    assert!(scene.apply_garage_selection(&mut renderer, invalid).is_err());
    assert_eq!(scene.garage.applied, upgraded);
    assert_eq!(scene.drive.as_ref().unwrap().pose(), before);
    scene.apply_garage_selection(&mut renderer, paint.clone()).unwrap();
    assert!(!scene.drive.as_ref().unwrap().hud_state().unwrap().has_nos, "restoring stock removes an installed kit");
    let mut tuned = paint.clone();
    tuned.performance = upgraded.performance;
    tuned.tuning.profiles[0] = [-5, 5, -5, 5, 0, 5, -5];
    tuned.tuning.profiles[1] = [5, -5, 5, -5, 0, -5, 5];
    scene.apply_garage_selection(&mut renderer, tuned.clone()).unwrap();
    assert_eq!(scene.drive.as_ref().unwrap().pose(), before, "tuning must not respawn the placed car");
    assert_eq!(scene.drive.as_mut().unwrap().sim_mut().unwrap().tunings(), tuned.tuning.normalized().unwrap());
    tuned.tuning.active = 1;
    scene.apply_garage_selection(&mut renderer, tuned.clone()).unwrap();
    assert_eq!(scene.drive.as_mut().unwrap().sim_mut().unwrap().tunings(), tuned.tuning.normalized().unwrap());
    let mut invalid_tuning = tuned.clone();
    invalid_tuning.tuning.profiles[2][0] = 6;
    invalid_tuning.color = Some([1, 2, 3]);
    assert!(scene.apply_garage_selection(&mut renderer, invalid_tuning).is_err());
    assert_eq!(scene.garage.applied, tuned, "invalid tuning must fail before paint or hardware changes");
    assert_eq!(scene.drive.as_ref().unwrap().pose(), before);
    tuned.performance = Default::default();
    scene.apply_garage_selection(&mut renderer, tuned.clone()).unwrap();
    assert_eq!(scene.drive.as_mut().unwrap().sim_mut().unwrap().tunings(), tuned.tuning.normalized().unwrap());
    scene.apply_garage_selection(&mut renderer, paint.clone()).unwrap();
    scene.set_garage_open(true);
    for _ in 0..10 {
        scene.update(&mut renderer, &crate::input::ActionState::default(), 1.0 / 60.0);
    }
    assert_eq!(scene.drive.as_ref().unwrap().pose(), before, "garage must freeze physics");
    let bogus = Selection { car: "MISSING_VEHICLE".into(), color: None, ..Default::default() };
    assert!(scene.apply_garage_selection(&mut renderer, bogus).is_err());
    assert_eq!(scene.drive.as_ref().unwrap().car_name, car);
    assert_eq!(scene.garage.applied, paint);
    let fixed =
        scene.garage.cars.iter().find(|car| !car.paintable && car.unavailable.is_none()).unwrap().folder.clone();
    assert!(
        scene
            .apply_garage_selection(
                &mut renderer,
                Selection { car: fixed, color: Some([1, 2, 3]), ..Default::default() }
            )
            .is_err()
    );
    assert_eq!(scene.garage.applied, paint);
    assert!(scene.garage.cars.len() > 20);
    let tables = CarTables::load(&dir).unwrap();
    for car in &scene.garage.cars {
        let vehicle = tables.car_type_for_folder(&car.folder).unwrap();
        assert!(matches!(vehicle.usage_type, 0 | 1));
        assert!(car.unavailable.is_none());
        println!("{:?}: {} / {} / {}", car.category, car.manufacturer, car.name, car.folder);
    }
    let racers = scene.garage.cars.iter().filter(|car| car.category == Category::Racing).count();
    let police = scene.garage.cars.iter().filter(|car| car.category == Category::Police).count();
    assert!(racers > 20 && police > 0);
    assert!(!scene.garage.cars.iter().any(|car| car.folder.starts_with("TRAF")));
    // Exercise the actual scene's resident collision packs, not the flat-ground data fixtures.
    scene.set_garage_open(false);
    let mut driving_tune = paint.clone();
    driving_tune.performance = upgraded.performance;
    driving_tune.tuning.profiles[0] = [-5, 5, 5, -5, 0, 5, 5];
    scene.apply_garage_selection(&mut renderer, driving_tune.clone()).unwrap();
    for _ in 0..30 {
        scene.update(&mut renderer, &crate::input::ActionState::default(), 1.0 / 60.0);
    }
    let on_road = scene.drive.as_ref().unwrap().pose();
    let mut throttle = crate::input::ActionState::default();
    throttle.update_for_test(&crate::input::Bindings::default(), &[bevy_input::keyboard::KeyCode::ArrowUp], &[], false);
    for _ in 0..120 {
        scene.update(&mut renderer, &throttle, 1.0 / 60.0);
    }
    let delta = scene.drive.as_ref().unwrap().pose().position - on_road.position;
    assert!(glam::Vec2::new(delta.x, delta.y).length() > 0.5, "resident-road drive must move: {delta:?}");
    let sim = scene.drive.as_mut().unwrap().sim_mut().unwrap();
    assert!(sim.is_finite());
    assert!(sim.telemetry().wheels_on_ground >= 3, "prepared wheel faces must support the installed car");
    assert_eq!(sim.tunings(), driving_tune.tuning.normalized().unwrap());
    println!("{racers} racing / {police} police choices; repaint and rejection checks passed");
}
