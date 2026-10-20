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
