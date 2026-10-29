//! Installed preview reuse and resource ownership, without submitting a GPU frame.

use super::*;
use crate::app::test_backend::NullBackend;
use crate::settings::CarShading;
use blackbox_gfx::{GlossyModel, MeshHandle, Shading};
use game_install::GameDir;
use std::collections::HashSet;

fn setup() -> (GarageBackdrop, NullBackend) {
    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").expect("installed car assets")).unwrap();
    let mut backdrop = GarageBackdrop::new(dir);
    backdrop.set_car_shading(CarShading::Original);
    let renderer = NullBackend::new(crate::settings::test_caps::native())
        .with_mesh_tracking()
        .with_texture_tracking()
        .with_glossy_tracking()
        .with_normal_tracking();
    (backdrop, renderer)
}

fn show(backdrop: &mut GarageBackdrop, renderer: &mut NullBackend, selection: &Selection) {
    if backdrop.restore_preview(renderer, selection) {
        return;
    }
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    let mut model = nfsmw_data::car::load_customized(
        backdrop.scene.dir.as_ref().unwrap(),
        &selection.car,
        &options,
        &selection.parts,
    )
    .unwrap();
    if let Some(color) = selection.color {
        model.set_body_color(color).unwrap();
    }
    backdrop.show_preview(renderer, selection, model, options).unwrap();
}

fn car_meshes(backdrop: &GarageBackdrop) -> HashSet<MeshHandle> {
    backdrop.scene.car.as_ref().unwrap().snapshot(16.0 / 9.0).1.iter().map(|part| part.mesh).collect()
}

#[test]
#[ignore = "requires installed car and frontend room assets"]
fn batch12_parked_previews_reuse_handles_evict_oldest_and_close_once() {
    let (mut backdrop, mut renderer) = setup();
    let a = Selection { car: "BMWM3GTR".into(), ..Default::default() };
    let b = Selection { car: "CARRERAGT".into(), ..Default::default() };
    show(&mut backdrop, &mut renderer, &a);
    let a_meshes = car_meshes(&backdrop);
    let a_textures: HashSet<_> = renderer.texture_labels.keys().copied().collect();
    show(&mut backdrop, &mut renderer, &b);
    let b_meshes = car_meshes(&backdrop);
    let b_textures: HashSet<_> = renderer.texture_labels.keys().filter(|h| !a_textures.contains(*h)).copied().collect();
    assert!(a_meshes.is_disjoint(&b_meshes));
    assert!(!b_textures.is_empty(), "the second scene owns separate textures, including repeated names");
    let meshes = renderer.mesh_labels.clone();
    let textures = renderer.texture_labels.clone();
    let materials = renderer.glossy_materials.clone();
    assert!(backdrop.restore_preview(&mut renderer, &a));
    assert_eq!(car_meshes(&backdrop), a_meshes, "A to B to A reuses the original uploaded car");
    assert_eq!(renderer.mesh_labels, meshes);
    assert_eq!(renderer.texture_labels, textures);
    assert_eq!(renderer.glossy_materials, materials);

    let c = Selection { color: Some([180, 50, 30]), ..b.clone() };
    let d = Selection { color: Some([20, 60, 180]), ..b.clone() };
    show(&mut backdrop, &mut renderer, &c);
    let c_meshes = car_meshes(&backdrop);
    show(&mut backdrop, &mut renderer, &d);
    assert_eq!(backdrop.previews.parked.len(), PARKED_LIMIT);
    assert!(!backdrop.restore_preview(&mut renderer, &b), "B is the least recently used preview");
    assert!(b_meshes.iter().all(|h| !renderer.mesh_labels.contains_key(h)));
    assert!(b_textures.iter().all(|h| !renderer.texture_labels.contains_key(h)));
    assert!(a_meshes.iter().chain(&c_meshes).all(|h| renderer.mesh_labels.contains_key(h)));
    assert!(car_meshes(&backdrop).iter().all(|h| renderer.mesh_labels.contains_key(h)));
    assert!(backdrop.restore_preview(&mut renderer, &a), "evicting B leaves A's shared-name resources valid");
    assert_eq!(car_meshes(&backdrop), a_meshes);

    backdrop.close(&mut renderer);
    assert!(renderer.mesh_labels.is_empty());
    assert!(renderer.texture_labels.is_empty());
    assert!(renderer.glossy_materials.is_empty());
    let counts = renderer.resources.lock().unwrap();
    let destroyed = (counts.mesh_destructions, counts.texture_destructions, counts.glossy_destructions);
    assert_eq!(counts.textures, 0);
    drop(counts);
    backdrop.close(&mut renderer);
    let counts = renderer.resources.lock().unwrap();
    assert_eq!(destroyed, (counts.mesh_destructions, counts.texture_destructions, counts.glossy_destructions));
    assert!(backdrop.previews.current.is_none() && backdrop.previews.parked.is_empty());
}

#[test]
#[ignore = "requires installed car and frontend room assets"]
fn batch12_restored_preview_uses_current_shading_backroom_and_camera_cut() {
    let (mut backdrop, mut renderer) = setup();
    let a = Selection { car: "BMWM3GTR".into(), ..Default::default() };
    let b = Selection { car: "CARRERAGT".into(), ..Default::default() };
    show(&mut backdrop, &mut renderer, &a);
    let a_meshes = car_meshes(&backdrop);
    backdrop.frame(16.0 / 9.0);
    show(&mut backdrop, &mut renderer, &b);
    backdrop.set_stage(&mut renderer, GarageStage::Backroom).unwrap();
    backdrop.show_camera("customize_customizeparts_260");
    backdrop.update(&mut renderer, &crate::input::ActionState::default(), 10.0);
    let shared_rig = backdrop.scene.rig.clone();
    let pose = backdrop.scene.rig.pose();
    backdrop.set_car_shading(CarShading::OriginalUnshadowed);
    renderer.lighting_rig = None;
    assert!(backdrop.restore_preview(&mut renderer, &a));
    assert_eq!(backdrop.scene.rig.pose(), pose, "restoring a model keeps the shared stage mover");
    assert_eq!(shared_rig.pose(), pose, "the original handle still owns the same mover");
    assert_ne!(car_meshes(&backdrop), a_meshes, "a changed material mode refreshes a parked car on return");
    for mesh in car_meshes(&backdrop) {
        for draw in &renderer.mesh_draws[&mesh] {
            if let Shading::Glossy(handle) = draw.shading {
                assert_eq!(renderer.glossy_materials[&handle].model, GlossyModel::VertexLit);
            }
        }
    }
    let rig = renderer.lighting_rig.expect("the restored car puts back its light rig");
    assert_eq!(rig.lights, crate::scenes::car::original_lighting::backroom(1.0).lights);
    assert!(backdrop.frame(16.0 / 9.0).0.camera_cut);
    assert!(!backdrop.frame(16.0 / 9.0).0.camera_cut);
    backdrop.close(&mut renderer);
}

#[test]
#[ignore = "requires installed car and frontend room assets"]
fn batch12_restored_preview_keeps_its_own_fallback_orbit_rest_and_intro() {
    let (mut backdrop, mut renderer) = setup();
    let input = crate::input::ActionState::default();
    let a = Selection { car: "BMWM3GTR".into(), ..Default::default() };
    let b = Selection { car: "CARRERAGT".into(), ..Default::default() };
    show(&mut backdrop, &mut renderer, &a);
    // Model the existing fallback used when an install has no authored stage-camera poses.
    backdrop.scene.rig.reset();
    backdrop.scene.rest = (0.3, 7.0);
    backdrop.scene.intro = 0.1;
    backdrop.scene.update_car(&mut renderer, &input, 0.05);
    let a_orbit = (backdrop.scene.rest, backdrop.scene.intro);
    let a_eye = backdrop.frame(16.0 / 9.0).0.camera_position;
    show(&mut backdrop, &mut renderer, &b);
    backdrop.scene.rest = (1.1, 11.0);
    backdrop.scene.intro = 0.25;
    backdrop.scene.update_car(&mut renderer, &input, 0.03);
    let b_orbit = (backdrop.scene.rest, backdrop.scene.intro);
    let b_eye = backdrop.frame(16.0 / 9.0).0.camera_position;
    assert_ne!(a_eye, b_eye);

    assert!(backdrop.restore_preview(&mut renderer, &a));
    assert!(backdrop.scene.rig.pose().is_none(), "the cache does not create a replacement stage mover");
    assert_eq!((backdrop.scene.rest, backdrop.scene.intro), a_orbit);
    assert_eq!(backdrop.frame(16.0 / 9.0).0.camera_position, a_eye);
    backdrop.scene.update_car(&mut renderer, &input, 0.05);
    let continued_intro = backdrop.scene.intro;
    assert!((continued_intro - (a_orbit.1 + 0.05)).abs() < 0.00001);
    assert!(backdrop.restore_preview(&mut renderer, &b));
    assert_eq!((backdrop.scene.rest, backdrop.scene.intro), b_orbit);
    assert_eq!(backdrop.frame(16.0 / 9.0).0.camera_position, b_eye);
    assert!(backdrop.restore_preview(&mut renderer, &a));
    assert_eq!(backdrop.scene.intro, continued_intro, "a later visit resumes the latest parked progress");
    backdrop.close(&mut renderer);
}
