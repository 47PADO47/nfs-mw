//! Explicit Custom Glossy surface classes from installed light material identities.
use blackbox_gfx::{GlossySurface, SurfaceKind};
use nfsmw_data::car::LightMaterial;

pub(super) fn response(material: &LightMaterial) -> Option<GlossySurface> {
    let (roughness, coat, kind) = classify(&material.name)?;
    Some(GlossySurface { roughness, coat, kind })
}

fn classify(name: &str) -> Option<(f32, f32, SurfaceKind)> {
    use SurfaceKind::*;
    Some(match name {
        "WINDSHIELD" | "WINDOWMASK" => (0.10, 0.0, Glass),
        n if n.starts_with("WINDSHIELD_TINT_") => (0.10, 0.0, Glass),
        "CARBONFIBER" | "CARBONFIBER2" => (0.38, 0.85, Composite),
        "MAGCHROME" | "CHROME" => (0.12, 0.0, Conductor),
        "MAGSILVER" => (0.27, 0.0, Conductor),
        "ALUMINUM" | "BRAKEDISC" => (0.42, 0.0, Conductor),
        "MAGGUNMETAL" => (0.38, 0.4, PaintedMetal),
        "MAGSILVERGLOSS" => (0.24, 0.7, PaintedMetal),
        "RUBBER" => (0.88, 0.0, Rubber),
        "INTERIOR" | "PLASTICHUBCAP" | "BOTTOM" => (0.65, 0.0, Diffuse),
        "__GARAGE_RIM_PAINT_MAGCHROME" | "__GARAGE_RIM_PAINT_CHROME" => (0.12, 0.0, Conductor),
        n if n.starts_with("__GARAGE_RIM_PAINT_") => (0.28, 0.65, PaintedMetal),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn explicit_classes_keep_lenses_and_unknown_mod_materials_unchanged() {
        assert_eq!(classify("WINDSHIELD_TINT_L2_BLACK").unwrap().2, SurfaceKind::Glass);
        assert_eq!(classify("CARBONFIBER2").unwrap().2, SurfaceKind::Composite);
        assert_eq!(classify("MAGCHROME").unwrap().2, SurfaceKind::Conductor);
        assert_eq!(classify("RUBBER").unwrap().2, SurfaceKind::Rubber);
        for name in ["HEADLIGHTGLASS", "BRAKELIGHT", "CHAR_RUBBER", "MOD_UNKNOWN"] {
            assert!(classify(name).is_none(), "{name}");
        }
    }
    #[test]
    #[ignore = "needs installed car material tables"]
    fn installed_car_surface_identities_are_present() {
        let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
        let tables = nfsmw_data::car::CarTables::load(&dir).unwrap();
        for material in &tables.light_materials {
            println!(
                "{}: diffuse {:?}/{:?} alpha {}/{} env {}/{} metal {} hotspot {}",
                material.name,
                material.diffuse_min,
                material.diffuse_max,
                material.diffuse_min_alpha,
                material.diffuse_max_alpha,
                material.envmap_min_scale,
                material.envmap_max_scale,
                material.metallic_scale,
                material.specular_hot_spot
            );
        }
        assert!(!tables.light_materials.is_empty());
        for (name, kind) in [
            ("WINDSHIELD", SurfaceKind::Glass),
            ("CARBONFIBER", SurfaceKind::Composite),
            ("MAGSILVER", SurfaceKind::Conductor),
            ("RUBBER", SurfaceKind::Rubber),
        ] {
            let material = tables.light_materials.iter().find(|m| m.name == name).expect("installed identity missing");
            assert_eq!(response(material).unwrap().kind, kind);
        }
        let options = nfsmw_data::car::LoadOptions { lod: 'A', all_parts: false, preset: None };
        use nfsmw_data::car::customization::{Catalog, Category, Selection};
        let catalog = Catalog::load(&dir, "911TURBO").unwrap();
        let carbon =
            catalog.choices[Category::Spoiler as usize].iter().find(|c| c.carbon).expect("carbon spoiler fixture");
        let mut selected = Selection::default();
        selected.parts[Category::Spoiler as usize] = Some(carbon.hash);
        let model = nfsmw_data::car::load_customized(&dir, "911TURBO", &options, &selected).unwrap();
        for placement in model.placements.iter().filter(|p| p.slot == nfsmw_data::car::slot::SPOILER) {
            let solid = &model.solids[&placement.solid];
            for group in &solid.groups {
                let light = solid
                    .light_material_hashes
                    .get(usize::from(group.light_material))
                    .and_then(|h| model.light_materials.get(h))
                    .map(|m| m.name.as_str());
                let texture = group
                    .diffuse_texture(solid)
                    .map(|h| model.swaps.resolve(h))
                    .and_then(|h| model.textures.get(&h))
                    .map(|t| t.name.as_str());
                println!("carbon spoiler {}: light {light:?}, texture {texture:?}", solid.name);
            }
        }
        for mode in [crate::settings::CarShading::Glossy, crate::settings::CarShading::Original] {
            let mut renderer =
                crate::app::test_backend::NullBackend::new(crate::settings::test_caps::native()).with_glossy_tracking();
            let materials = super::super::materials::CarMaterials::upload(&mut renderer, &model, mode, glam::Vec3::Z);
            let kinds: Vec<_> = renderer.glossy_materials.values().filter_map(|m| m.surface.map(|s| s.kind)).collect();
            if mode == crate::settings::CarShading::Glossy {
                for kind in [SurfaceKind::Glass, SurfaceKind::Composite, SurfaceKind::Rubber] {
                    assert!(kinds.contains(&kind), "911 parts must route {kind:?}: {kinds:?}");
                }
            }
            if mode == crate::settings::CarShading::Original {
                assert!(kinds.is_empty());
            }
            materials.destroy(&mut renderer);
            assert!(renderer.glossy_materials.is_empty());
        }
    }
}
