//! The car tables of GLOBAL/GlobalB.lzc against docs/formats/cardata.md.

use blackbox_carparts::layout::MOST_WANTED as L;
use blackbox_carparts::{
    PartQuery, read_car_types, read_light_materials, read_parts_db, read_preset_rides, read_slot_types,
};
use blackbox_hash::bstring_hash;

use crate::{install, unwrapped};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn car_tables_match_documented_numbers() {
    let Some(dir) = install() else { return };
    let globalb = unwrapped(&dir, "GLOBAL/GLOBALB.LZC");

    let types = read_car_types(&globalb, &L);
    assert_eq!(types.len(), 91);
    assert!(types.iter().all(|t| t.type_name_hash == bstring_hash(&t.type_name)));
    let m3 = types.iter().find(|t| t.type_name == "BMWM3GTR").unwrap();
    assert_eq!((m3.skinnable, m3.default_base_paint, m3.usage_type), (true, 0xC7F2_884E, 0));
    assert_eq!(types.iter().filter(|t| t.skinnable).count(), 47);

    let db = read_parts_db(&globalb, &L).unwrap();
    assert_eq!((db.parts().len(), db.type_names().len()), (13_080, 106));
    let paint = db.by_name_hash(m3.default_base_paint).unwrap();
    assert_eq!(db.name(paint), "METAL_L1_COLOR02");
    let rgb = ["RED", "GREEN", "BLUE", "GLOSS"].map(|a| db.attribute(paint, a));
    assert_eq!(rgb, [Some(79), Some(79), Some(79), Some(128)]);
    // Name-valued attributes hold the name's hash, not a string offset.
    assert_eq!(db.attribute(paint, "LIGHT_MATERIAL_NAME"), Some(bstring_hash("METPAINTSILVER")));

    let body = db.find(&PartQuery::new(23, m3.type_name_hash).upgrade_level(0)).unwrap();
    assert_eq!(db.model_hash(body, 0), Some(bstring_hash("BMWM3GTR_KIT00_BODY_A")));
    let wheel = db.find(&PartQuery::new(67, m3.type_name_hash).upgrade_level(0)).unwrap();
    assert_eq!(db.model_hash(wheel, 0), Some(bstring_hash("BMWM3GTR_KIT00_FRONT_TIRE_A")));

    let slots = read_slot_types(&globalb, &L);
    assert_eq!((slots.defaults.len(), slots.overrides.len()), (139, 10));
    let spoiler = L.slot("SPOILER").unwrap();
    let porsche = bstring_hash("911TURBO");
    assert_eq!(slots.search_types(spoiler, porsche).collect::<Vec<_>>(), [porsche, bstring_hash("SPOILER_PORSCHES")]);

    let presets = read_preset_rides(&globalb, &L);
    assert_eq!(presets.len(), 82);
    assert!(presets.iter().any(|p| p.preset_name == "CE_GTRSTREET" && p.car_type_name == "BMWM3GTR"));

    let materials = read_light_materials(&globalb, &L);
    assert_eq!(materials.len(), 156);
    assert!(materials.iter().all(|m| m.name_hash == bstring_hash(&m.name)));
    let silver = materials.iter().find(|m| m.name == "METPAINTSILVER").unwrap();
    let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
    assert!(close(silver.diffuse_min_scale, 0.75) && close(silver.diffuse_max_scale, 1.0), "{silver:?}");
    assert!(close(silver.specular_power, 3.0) && close(silver.envmap_power, 0.15), "{silver:?}");
}

/// Aftermarket rims build their model names from the BRAND_NAME attribute; every rim's LOD A
/// model must exist in CARS/WHEELS.
#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn aftermarket_rim_models_exist() {
    let Some(dir) = install() else { return };
    let db = read_parts_db(&unwrapped(&dir, "GLOBAL/GLOBALB.LZC"), &L).unwrap();
    let solids: std::collections::HashSet<u32> =
        blackbox_solid::read_solids(&unwrapped(&dir, "CARS/WHEELS/GEOMETRY.BIN"))
            .unwrap()
            .iter()
            .map(|s| s.name_hash)
            .collect();
    let rims: Vec<_> = db.parts().iter().filter(|p| p.base_selector == 2).collect();
    assert_eq!(rims.len(), 164);
    let found = rims.iter().filter(|p| db.model_hash(p, 0).is_some_and(|h| solids.contains(&h))).count();
    assert_eq!(found, rims.len());
}
