//! Installed material routing must exercise real paint, glass and customized parts.

use super::*;
use crate::app::test_backend::NullBackend;
use nfsmw_data::car::customization::{Category, Inventory, Selection};

#[test]
#[ignore = "needs installed cars selected by NFSMW_GAME_DIR"]
fn installed_original_car_materials_resolve_paint_glass_and_custom_parts() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
    let inventory = Inventory::load(&dir).unwrap();
    for folder in ["BMWM3GTR", "ELISE"] {
        let catalog = inventory.catalog(&dir, folder).unwrap();
        let mut custom = Selection::default();
        for category in [Category::Body, Category::Spoiler, Category::Wheels] {
            custom.parts[category as usize] = catalog.choices[category as usize].first().map(|choice| choice.hash);
        }
        assert!(!custom.is_stock(), "installed fixture must exercise customization");
        for selection in [Selection::default(), custom] {
            let model = nfsmw_data::car::load_customized(&dir, folder, &options, &selection).unwrap();
            let mut effects = std::collections::BTreeMap::new();
            for group in model.solids.values().flat_map(|solid| &solid.groups) {
                *effects.entry(group.effect_id).or_insert(0usize) += 1;
            }
            let mut paint_groups = 0;
            let mut glass_groups = 0;
            for mode in [CarShading::Original, CarShading::OriginalUnshadowed, CarShading::Glossy] {
                let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_glossy_tracking();
                let materials = CarMaterials::upload(&mut renderer, &model, mode, Vec3::Z);
                let lookup = materials.for_placement(&model.swaps, false);
                let expected_model = match mode {
                    CarShading::Original => GlossyModel::Shadowed,
                    CarShading::OriginalUnshadowed => GlossyModel::VertexLit,
                    _ => GlossyModel::Custom,
                };
                let mut resolved = 0;
                for solid in model.solids.values() {
                    for group in &solid.groups {
                        let Some(&hash) = solid.light_material_hashes.get(usize::from(group.light_material)) else {
                            continue;
                        };
                        let actual_hash = match hash {
                            CARSKIN => materials.paint.unwrap_or(hash),
                            WINDSHIELD => materials.window_tint.unwrap_or(hash),
                            _ => hash,
                        };
                        let Some(authored) = model.light_materials.get(&actual_hash) else { continue };
                        let Some(Shading::Glossy(handle)) = lookup.shading(hash) else {
                            panic!("unrouted installed light material")
                        };
                        let actual = renderer.glossy_materials[&handle];
                        assert_eq!(actual.model, expected_model);
                        let expected = match mode.is_original() {
                            true => original_material(authored, expected_model),
                            false => glossy_material(authored),
                        };
                        assert_eq!(actual, expected, "{folder} {} {}", solid.name, authored.name);
                        resolved += 1;
                        paint_groups += usize::from(hash == CARSKIN);
                        glass_groups +=
                            usize::from(authored.name.contains("GLASS") || authored.name.contains("WINDSHIELD"));
                    }
                }
                assert!(resolved > 10, "installed car needs representative material groups");
                materials.destroy(&mut renderer);
                assert!(renderer.glossy_materials.is_empty());
            }
            assert!(paint_groups > 0 && glass_groups > 0, "{folder} must exercise paint and glass profiles");
            eprintln!(
                "{folder}: stock={}, paint groups={paint_groups}, glass groups={glass_groups}, effect groups={effects:?}",
                selection.is_stock()
            );
        }
    }
}

#[test]
#[ignore = "needs installed preset tint parts selected by NFSMW_GAME_DIR"]
fn installed_original_car_tint_material_uses_selected_preset() {
    use nfsmw_data::car::{CarTables, LoadOptions, preset_parts, slot};
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let tables = CarTables::load(&dir).unwrap();
    let (preset, car, tint) = tables
        .presets
        .iter()
        .find_map(|preset| {
            let car = tables.types.iter().find(|car| car.type_name == preset.car_type_name)?;
            let parts = preset_parts(&tables, car, preset);
            let part = parts[slot::WINDOW_TINT]?;
            let tint = tables.parts.attribute(&part, "LIGHT_MATERIAL_NAME")?;
            (tint != WINDSHIELD && tables.light_materials.iter().any(|material| material.name_hash == tint))
                .then_some((preset, car, tint))
        })
        .expect("installed presets must exercise a non-stock tint profile");
    let geometry = car.geometry_filename.replace('\\', "/");
    let folder = geometry.split('/').nth(1).expect("installed car geometry folder");
    let options = LoadOptions { lod: 'A', all_parts: false, preset: Some(preset.preset_name.clone()) };
    let model = nfsmw_data::car::load(&dir, folder, &options).unwrap();
    assert_eq!(model.window_tint_material, Some(tint));
    for mode in [CarShading::Original, CarShading::OriginalUnshadowed, CarShading::Glossy] {
        let mut renderer = NullBackend::new(crate::settings::test_caps::native()).with_glossy_tracking();
        let materials = CarMaterials::upload(&mut renderer, &model, mode, Vec3::Z);
        let lookup = materials.for_placement(&model.swaps, false);
        let Some(Shading::Glossy(handle)) = lookup.shading(WINDSHIELD) else { panic!("windshield profile missing") };
        let expected_hash = match mode.is_original() {
            true => tint,
            false => WINDSHIELD,
        };
        let authored = &model.light_materials[&expected_hash];
        let actual = renderer.glossy_materials[&handle];
        assert_eq!(actual.diffuse_min, glossy_material(authored).diffuse_min);
        assert_eq!(actual.specular_min, glossy_material(authored).specular_min);
        materials.destroy(&mut renderer);
        assert!(renderer.glossy_materials.is_empty());
    }
    eprintln!("preset {} ({folder}) resolves windshield material {tint:08X}", preset.preset_name);
}
