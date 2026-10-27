//! Existing live actors must observe shared appearance changes without a respawn.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::scenes::world::{Options, drive::CarPose, props::PropWorld};
use crate::settings::CarShading;
use blackbox_gfx::{GlossyModel, Instance};
use glam::{Quat, Vec3};
use std::rc::Rc;

#[test]
#[ignore = "needs installed traffic, cop and helicopter models selected by NFSMW_GAME_DIR"]
fn installed_shared_traffic_rigs_switch_mode_without_actor_reset() {
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
    let mut renderer =
        NullBackend::new(crate::settings::test_caps::native()).with_mesh_tracking().with_glossy_tracking();
    // The same path the game takes: request the model, then serve and install it.
    let job = traffic_job(&scene, "semilog");
    ask(&mut scene, Request::Traffic("semilog".to_owned()), job);
    pump(&mut scene, &mut renderer, true);
    load_cop_models(&mut scene, &mut renderer, &["copmidsize".to_owned()]).unwrap();
    load_helicopter_model(&mut scene, &mut renderer).unwrap();
    // Outside a warm-up both only queue their request, so install what the loader finishes.
    pump(&mut scene, &mut renderer, true);
    let traffic = scene.traffic.as_mut().unwrap();
    let live: Vec<Rc<CarRig>> = vec![
        traffic.models[0].rig.clone(),
        traffic.models[0].trailer.as_ref().unwrap().rig.clone(),
        traffic.cop_models[0].model.rig.clone(),
        traffic.helicopter_rig.as_ref().unwrap().clone(),
    ];
    let pose = CarPose {
        position: Vec3::new(25.0, 15.0, 4.0),
        rotation: Quat::from_rotation_z(0.3),
        wheels: Default::default(),
    };
    let draw = |rig: &CarRig| {
        let mut out = Vec::new();
        rig.instances(&pose, None, &Default::default(), &mut out);
        out
    };
    let mut before: Vec<Vec<Instance>> = live.iter().map(|rig| draw(rig)).collect();
    assert!(before.iter().all(|parts| !parts.is_empty()));
    let identities: Vec<_> = live.iter().map(Rc::as_ptr).collect();
    let mut props = PropWorld::default();
    for mode in [CarShading::Original, CarShading::OriginalUnshadowed, CarShading::Glossy] {
        traffic.reshade(&mut renderer, mode, Vec3::Z, &mut props);
        let next: Vec<_> = live.iter().map(|rig| draw(rig)).collect();
        for (old, new) in before.iter().zip(&next) {
            assert_eq!(old.len(), new.len());
            let old_by_key: std::collections::HashMap<_, _> = old.iter().map(|part| (part.key, part)).collect();
            for part in new {
                let old = old_by_key[&part.key];
                assert_eq!(part.transform, old.transform, "shared appearance cannot reset actor pose");
                assert_ne!(part.mesh, old.mesh);
                assert!(!renderer.mesh_labels.contains_key(&old.mesh), "retired meshes are destroyed");
            }
        }
        assert_eq!(identities, live.iter().map(Rc::as_ptr).collect::<Vec<_>>());
        let family = match mode {
            CarShading::Original => GlossyModel::Shadowed,
            CarShading::OriginalUnshadowed => GlossyModel::VertexLit,
            _ => GlossyModel::Custom,
        };
        assert!(renderer.glossy_materials.values().all(|material| material.model == family));
        assert!(
            renderer
                .glossy_materials
                .values()
                .all(|material| material.background_actor == (mode == CarShading::Glossy))
        );
        let resources = (renderer.mesh_labels.len(), renderer.glossy_materials.len(), renderer.lighting_updates);
        traffic.reshade(&mut renderer, mode, Vec3::X, &mut props);
        assert_eq!(
            resources,
            (renderer.mesh_labels.len(), renderer.glossy_materials.len(), renderer.lighting_updates),
            "unchanged mode must not upload again"
        );
        before = next;
    }
    // The window pass keeps clones of the rigs it prepared; the teardown still frees every one of them.
    drop(live);
    traffic.update_damaged_windows(&mut renderer, Vec3::Z);
    traffic.release(&mut renderer);
    assert!(renderer.mesh_labels.is_empty(), "every traffic rig is freed at teardown");
    eprintln!(
        "live shared appearance: traffic, loaded trailer, cop and helicopter preserve Rc identity/part poses across three shading modes"
    );
}
