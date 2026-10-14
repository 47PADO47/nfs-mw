//! Actual stock trailer wheel materials resolve through the render upload path.

use super::*;
use crate::scenes::car::materials::CarMaterials;
use blackbox_scene::MaterialLookup;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_traffic_trailer_wheel_materials_are_textured() {
    let path = std::env::var_os("NFSMW_GAME_DIR").expect("set NFSMW_GAME_DIR to run installed tests");
    let dir = GameDir::open(path).unwrap();
    let options = LoadOptions { lod: 'A', all_parts: false, preset: None };
    for folder in ["TRAILERLOG", "TRAILERB"] {
        let model = nfsmw_data::car::load(&dir, folder, &options).unwrap();
        let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
        let materials = CarMaterials::upload(&mut renderer, &model, CarShading::default(), sun::to_sun());
        let wheels: Vec<_> = model.placements.iter().filter(|p| p.corner.is_some() && !p.is_brake()).collect();
        assert_eq!(wheels.len(), 4, "{folder} must have four authored wheel placements");
        for placement in wheels {
            let solid = &model.solids[&placement.solid];
            let lookup = materials.for_placement(&model.swaps, placement.left_brake);
            for group in &solid.groups {
                let hash = group.diffuse_texture(solid).expect("stock trailer wheel has a diffuse reference");
                assert!(
                    lookup.material(hash).is_some(),
                    "{folder}: {} corner {:?} draws an untextured wheel (diffuse {hash:08X})",
                    solid.name,
                    placement.corner
                );
                let texture = &model.textures[&model.swaps.resolve(hash)];
                assert_eq!(texture.name, "TRAFFIC_TIRE", "stock traffic wheel uses the shared tire atlas");
            }
        }
        materials.destroy(&mut renderer);
    }
}
