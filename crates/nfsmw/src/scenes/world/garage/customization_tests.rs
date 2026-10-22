use super::*;
use crate::viewer::Scene;
use nfsmw_data::car::customization::Category as Part;

#[test]
#[ignore = "requires an installed game"]
fn installed_garage_customization_preserves_driving_state_and_rejects_incompatible_parts() {
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
    let mut renderer =
        crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking();
    scene.init(&mut renderer).unwrap();
    scene.set_garage_open(true);
    scene.apply_garage_selection(&mut renderer, Selection { car: "COBALTSS".into(), ..Default::default() }).unwrap();
    for _ in 0..2000 {
        scene.update(&mut renderer, &crate::input::ActionState::default(), 1.0 / 60.0);
        if scene.ready() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    assert!(scene.ready());
    let drive = scene.drive.as_ref().unwrap();
    assert!(
        drive.camera().position.distance(drive.pose().position) < 10.0,
        "Chase must already follow the spawned car in the frozen garage"
    );
    scene.set_garage_open(false);
    let catalog = scene.garage.selected().unwrap().parts.clone().unwrap();
    let stock_wheel_handles: Vec<_> = renderer
        .mesh_labels
        .iter()
        .filter(|(_, name)| name.as_str() == "COBALTSS_KIT00_FRONT_TIRE_A")
        .map(|(handle, _)| *handle)
        .collect();
    assert!(!stock_wheel_handles.is_empty());
    let mut selection = scene.garage.applied.clone();
    selection.performance = scene.garage.selected().unwrap().performance.maximum();
    selection.tuning.profiles[1] = [-3, 2, 1, -2, 4, -1, 2];
    selection.tuning.active = 1;
    selection.color = Some([30, 80, 230]);
    scene.apply_garage_selection(&mut renderer, selection.clone()).unwrap();
    for _ in 0..30 {
        scene.update(&mut renderer, &crate::input::ActionState::default(), 1.0 / 60.0);
    }
    let mut throttle = crate::input::ActionState::default();
    throttle.update_for_test(&crate::input::Bindings::default(), &[bevy_input::keyboard::KeyCode::ArrowUp], &[], false);
    for _ in 0..300 {
        scene.update(&mut renderer, &throttle, 1.0 / 60.0);
        if scene.drive.as_ref().unwrap().speed().abs() > 3.0 {
            break;
        }
    }
    assert!(scene.drive.as_mut().unwrap().inflict(0, 2, true));
    let drive = scene.drive.as_ref().unwrap();
    let pose = drive.pose();
    let speed = drive.speed();
    assert!(speed.abs() > 3.0, "probe must actually be driving: speed {speed}, pose {pose:?}");
    let damage = drive.test_damage();
    let telemetry = drive.hud_state().unwrap();
    for category in Part::ALL {
        let choice = catalog.choices[category as usize].last().expect("Cobalt supports each category");
        selection.parts.parts[category as usize] = Some(choice.hash);
    }
    selection.parts.rim_color = Some([30, 180, 220]);
    selection.parts.vinyl_colors = [Some([240, 20, 30]), Some([20, 240, 70]), None];
    selection.gauges.skin = 3;
    selection.gauges.digits = Some([40, 230, 100]);
    scene.apply_garage_selection(&mut renderer, selection.clone()).unwrap();
    assert_eq!(scene.hud_frame().unwrap().skin, 3);
    assert_eq!(scene.hud_frame().unwrap().gauge_colors[2], Some([40, 230, 100]));
    let drive = scene.drive.as_ref().unwrap();
    assert!(
        stock_wheel_handles.iter().all(|h| !renderer.mesh_labels.contains_key(h)),
        "old stock wheel GPU meshes must be released"
    );
    assert!(
        renderer.mesh_labels.values().all(|name| name != "COBALTSS_KIT00_FRONT_TIRE_A"),
        "no stock wheel mesh may survive the replacement"
    );
    assert!(drive.test_model().solids.values().all(|s| s.name != "COBALTSS_KIT00_FRONT_TIRE_A"));
    assert_eq!(drive.test_model().paint.as_ref().unwrap().rgb, [30, 80, 230]);
    assert!(drive.test_model().window_tint_material.is_some());
    assert_eq!(drive.pose(), pose);
    assert_eq!(drive.speed(), speed);
    assert_eq!(drive.test_damage(), damage);
    assert_eq!(drive.performance, selection.performance);
    assert_eq!(drive.tuning, selection.tuning);
    let after = drive.hud_state().unwrap();
    assert_eq!((after.nos, after.gear, after.rpm), (telemetry.nos, telemetry.gear, telemetry.rpm));
    for category in Part::MODELS {
        assert!(drive.test_model().placements.iter().any(|p| p.slot == category.slot()));
    }
    let body = drive.test_model().placements.iter().find(|p| p.slot == slot::BODY).unwrap().solid;
    let mut repaint = selection.clone();
    repaint.color = Some([240, 20, 30]);
    scene.apply_garage_selection(&mut renderer, repaint.clone()).unwrap();
    assert_eq!(
        scene.drive.as_ref().unwrap().test_model().placements.iter().find(|p| p.slot == slot::BODY).unwrap().solid,
        body
    );
    repaint.color = None;
    scene.apply_garage_selection(&mut renderer, repaint.clone()).unwrap();
    assert_eq!(
        scene.drive.as_ref().unwrap().test_model().placements.iter().find(|p| p.slot == slot::BODY).unwrap().solid,
        body
    );
    let handles = renderer.mesh_labels.clone();
    repaint.gauges.skin = 4;
    repaint.gauges.needle = Some([230, 100, 10]);
    scene.apply_garage_selection(&mut renderer, repaint.clone()).unwrap();
    assert_eq!(renderer.mesh_labels, handles, "gauge-only changes must not reload the vehicle");
    assert_eq!(scene.hud_frame().unwrap().skin, 4);
    let mut bad_gauge = repaint.clone();
    bad_gauge.gauges.skin = 255;
    assert!(scene.apply_garage_selection(&mut renderer, bad_gauge).is_err());
    assert_eq!(scene.garage.applied, repaint);
    let mut invalid = repaint.clone();
    invalid.parts.parts[Part::Body as usize] = selection.parts.parts[Part::Spoiler as usize];
    invalid.color = Some([0, 0, 0]);
    assert!(scene.apply_garage_selection(&mut renderer, invalid).is_err());
    assert_eq!(scene.garage.applied, repaint);
    assert_eq!(scene.drive.as_ref().unwrap().pose(), pose);
    assert!(command::run(&mut scene, &mut renderer, &["body", "1", "spoiler", "99999"]).is_err());
    assert_eq!(scene.garage.applied, repaint, "bulk commands must validate before replacement");
    command::run(&mut scene, &mut renderer, &["stock"]).unwrap();
    assert!(scene.garage.applied.parts.is_stock());
    assert_eq!(scene.drive.as_ref().unwrap().pose(), pose);
    assert_eq!(scene.drive.as_ref().unwrap().performance, selection.performance);
    assert_eq!(scene.drive.as_ref().unwrap().tuning, selection.tuning);
    let temporary = std::env::temp_dir().join(format!("nfsmw-garage-command-{}.toml", std::process::id()));
    let payload = r#"version = 1
car = "COBALTSS"
color = [20, 20, 24]
levels = [0, 0, 0, 0, 0, 0, 0]
junkman = 0
tuning = [[0, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0, 0], [0, 0, 0, 0, 0, 0, 0]]
active_tuning = 0
parts = []
"#;
    crate::preset_store::save(&temporary, "Matte test", payload).unwrap();
    let bytes = std::fs::read(&temporary).unwrap();
    scene.garage.preset_picker.path = Some(temporary.clone());
    command::run(&mut scene, &mut renderer, &["preset", "Matte", "test"]).unwrap();
    assert_eq!(scene.garage.applied.color, Some([20, 20, 24]));
    assert_eq!(scene.drive.as_ref().unwrap().pose(), pose);
    let applied = scene.garage.applied.clone();
    assert!(command::run(&mut scene, &mut renderer, &["preset", "Missing"]).is_err());
    assert_eq!(scene.garage.applied, applied);
    assert_eq!(std::fs::read(&temporary).unwrap(), bytes, "loading must not rewrite the preset");
    std::fs::remove_file(temporary).unwrap();
    scene.update(&mut renderer, &throttle, 1.0 / 60.0);
    assert!(scene.drive.as_mut().unwrap().sim_mut().unwrap().is_finite());
    println!("five installed model categories swapped while moving; physics, damage, paint and tuning preserved");
}
