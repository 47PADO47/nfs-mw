//! Validate fixed-livery paint routing with the installed traffic assets.
use super::*;
use crate::app::test_backend::NullBackend;

#[test]
#[ignore = "needs installed traffic material groups"]
fn installed_traffic_cargo_material_groups() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let model = nfsmw_data::car::load(&dir, "TRAILERLOG", &options).unwrap();
    let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_glossy_tracking();
    let materials =
        CarMaterials::upload_with_background_policy(&mut renderer, &model, CarShading::Glossy, Vec3::Z, true);
    let mut wood_groups = 0;
    for solid in model.solids.values().filter(|s| s.name.contains("ATTACHMENT")) {
        for group in &solid.groups {
            let hash = solid.light_material_hashes[usize::from(group.light_material)];
            assert_eq!(model.light_materials[&hash].name, "DULLPLASTIC");
            let Some(Shading::Glossy(handle)) = materials.for_placement(&model.swaps, false).shading(hash) else {
                panic!("cargo material")
            };
            let material = renderer.glossy_materials[&handle];
            assert!(material.background_actor);
            assert_eq!(material.surface.unwrap().kind, blackbox_gfx::SurfaceKind::Diffuse);
            assert_eq!(material.surface.unwrap().coat, 0.0);
            for i in 0..3 {
                assert!(material.diffuse_min[i] <= 1.0);
                assert!(material.diffuse_min[i] + material.diffuse_range[i] <= 1.0);
            }
            wood_groups += 1;
        }
    }
    assert!(wood_groups >= 7, "real cargo groups must be covered");
    materials.destroy(&mut renderer);
    assert!(renderer.glossy_materials.is_empty());
}

#[test]
#[ignore = "needs installed traffic cars"]
fn installed_fixed_livery_traffic_gets_finish_without_changing_original_materials() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let mut enhanced_paints = 0;
    for folder in ["MINIVAN", "TAXI", "COPMIDSIZE"] {
        let model = nfsmw_data::car::load(&dir, folder, &options).unwrap();
        let paint = model.paint.as_ref().expect("installed base paint");
        eprintln!("{folder}: paint {} paintable={} material={:?}", paint.name, model.paintable, paint.light_material);
        for mode in [CarShading::Glossy, CarShading::Original] {
            let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_glossy_tracking();
            let materials = CarMaterials::upload_with_background_policy(&mut renderer, &model, mode, Vec3::Z, true);
            let hash = paint.light_material.expect("installed paint material");
            let actual = renderer.glossy_materials[&materials.glossy[&hash]];
            assert_eq!(actual.background_actor, mode == CarShading::Glossy);
            for (&key, handle) in &materials.glossy {
                if key == hash {
                    continue;
                }
                if mode == CarShading::Glossy && model.light_materials[&key].name == "DULLPLASTIC" {
                    assert_eq!(
                        renderer.glossy_materials[handle].surface.unwrap().kind,
                        blackbox_gfx::SurfaceKind::Diffuse
                    );
                    continue;
                }
                assert!(renderer.glossy_materials[handle].surface.is_none(), "traffic trim keeps authored response");
            }
            if mode == CarShading::Glossy {
                let expected = super::super::paint_surface::response(paint);
                assert_eq!(actual.surface, expected);
                enhanced_paints += usize::from(actual.surface.is_some());
            }
            if mode == CarShading::Original {
                assert_eq!(actual.surface, None);
                assert_eq!(actual.model, GlossyModel::Shadowed);
            }
            materials.destroy(&mut renderer);
            assert!(renderer.glossy_materials.is_empty());
        }
    }
    assert_eq!(enhanced_paints, 3, "all installed fixtures must receive the finish");
}
