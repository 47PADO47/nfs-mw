//! A support vehicle can inherit its geometry without inheriting the cache key.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::Options;

#[test]
#[ignore = "needs installed police aliases, geometry, physics and pursuit tables"]
fn installed_cop_alias_loads_once_and_discards_duplicate_completions_before_upload() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: None,
            height: 40.0,
            heading: 0.0,
            pitch: -15.0,
            fog_distance: 1500.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    let cars = nfsmw_data::car::list(&dir);
    assert!(!cars.iter().any(|folder| folder.eq_ignore_ascii_case("copcross")), "fixture needs an inherited model");
    let fallback = folders(&cars, "copcross", &scene.physics.lineage("copcross"))
        .into_iter()
        .find(|folder| cars.contains(folder))
        .expect("Cross inherits installed geometry");
    let mut renderer = NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_texture_tracking()
        .with_glossy_tracking();
    let job = Job::Cop(lookup(&scene, "copcross"));
    ask(&mut scene, Request::Cop("copcross".to_owned()), job);
    pump(&mut scene, &mut renderer, true);
    let traffic = scene.traffic.as_ref().unwrap();
    assert!(traffic.has_cop_model("copcross"), "the requested identity must survive folder fallback");
    assert_eq!(traffic.cop_models.len(), 1);
    assert_eq!(traffic.cop_models[0].model.name, "COPCROSS");
    let performance = traffic.cop_models[0].car.performance;
    let resources = || {
        let counts = renderer.resources.lock().unwrap();
        (counts.meshes, counts.textures, renderer.glossy_materials.len())
    };
    let installed = resources();
    assert!(installed.0 > 0 && installed.1 > 0, "the test must upload real assets");
    for _ in 0..1000 {
        scene.traffic.as_mut().unwrap().request_cop("CoPcRoSs");
        load_cop_models(&mut scene, &mut renderer, &["COPCROSS".to_owned()]).unwrap();
        load_requested(&mut scene, &mut renderer, true, true);
    }
    assert_eq!(scene.traffic.as_ref().unwrap().cop_models.len(), 1);
    assert!(scene.traffic.as_mut().unwrap().take_model_requests().is_empty());
    // A duplicate first-stage result must not schedule another extras trip either.
    let job = Job::Cop(lookup(&scene, "copcross"));
    ask(&mut scene, Request::Cop("copcross".to_owned()), job);
    pump(&mut scene, &mut renderer, true);
    assert_eq!(scene.traffic.as_ref().unwrap().cop_models.len(), 1);
    // Results already in flight, including one arriving after the traffic world went away,
    // must be discarded before creating renderer resources.
    for remove_traffic in [false, true] {
        let (_, model) = crate::scenes::world::load_car(&dir, &fallback).unwrap();
        let Job::CopExtras { staged, .. } =
            cop_job(&mut scene, "copcross", CopLoad { folder: fallback.clone(), model }).unwrap()
        else {
            panic!("cop model must stage its extras");
        };
        if remove_traffic {
            scene.traffic = None;
        }
        handle(
            &mut scene,
            &mut renderer,
            Done {
                request: Request::Cop("CoPcRoSs".to_owned()),
                result: Ok(Loaded::CopExtras(Box::new(CopExtrasLoad {
                    staged: *staged,
                    performance,
                    textures: Vec::new(),
                }))),
            },
        );
        let counts = renderer.resources.lock().unwrap();
        assert_eq!((counts.meshes, counts.textures, renderer.glossy_materials.len()), installed);
    }
    eprintln!(
        "Cross: 1000 repeat requests and stale completions retain {} meshes, {} textures",
        installed.0, installed.1
    );
}

#[test]
#[ignore = "needs installed heat-one-to-five police models, alias geometry, physics and pursuit tables"]
fn installed_cop_loading_keeps_requested_vehicle_physics_and_ai_when_geometry_is_inherited() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let mut scene = WorldScene::open(
        &dir,
        Options {
            at: None,
            height: 40.0,
            heading: 0.0,
            pitch: -15.0,
            fog_distance: 1500.0,
            wait_for_load: false,
            drive: None,
        },
    )
    .unwrap();
    let rows = nfsmw_data::pursuit::heat_rows(scene.physics.database());
    assert!(rows.len() >= 5, "all five ordinary heat fleets must be available");
    let mut names = vec!["copcross".to_owned()];
    for row in rows.iter().take(5) {
        for cop in &row.cops {
            if cop.name != "copheli" && !names.contains(&cop.name) {
                names.push(cop.name.clone());
            }
        }
    }
    let cars = nfsmw_data::car::list(&dir);
    assert!(!cars.iter().any(|name| name.eq_ignore_ascii_case("copcross")), "Cross must exercise geometry inheritance");
    let mut renderer = NullBackend::new(crate::settings::test_caps::native());
    load_cop_models(&mut scene, &mut renderer, &names).unwrap();
    pump(&mut scene, &mut renderer, true);
    let mut mismatches = Vec::new();
    for name in &names {
        let expected = scene.physics.car(name).unwrap_or_else(|error| panic!("{name}: {error:#}"));
        let expected_ai = ai_vehicle(scene.physics.database(), name);
        let expected_performance =
            nfsmw_data::car::physics::measured::ai_performance_of_spec(&expected.spec, expected.tire_integration);
        let loaded = scene
            .traffic
            .as_ref()
            .unwrap()
            .cop_models
            .iter()
            .find(|model| model.model.name.eq_ignore_ascii_case(name))
            .unwrap_or_else(|| panic!("production loader did not install requested vehicle {name}"));
        let actual = &loaded.model.physics;
        let same_spec = format!("{:?}", actual.spec) == format!("{:?}", expected.spec);
        let same_ai = loaded.car.ai == expected_ai;
        let same_performance = loaded.car.performance == expected_performance;
        eprintln!(
            "COP_LOAD name={name} requested_mass={} loaded_mass={} requested_ai={expected_ai:?} loaded_ai={:?} requested_top={} loaded_top={} spec_equal={same_spec} performance_equal={same_performance}",
            expected.spec.mass,
            actual.spec.mass,
            loaded.car.ai,
            expected_performance.top_speed,
            loaded.car.performance.top_speed,
        );
        if !same_spec || !same_ai || !same_performance || actual.tire_integration != expected.tire_integration {
            mismatches.push(name.clone());
        }
    }
    assert!(mismatches.is_empty(), "geometry donors must not replace requested vehicle physics or AI: {mismatches:?}");
}
