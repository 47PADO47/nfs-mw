use super::*;
use blackbox_attrib::Database;

#[test]
#[ignore = "requires original global attribute and texture packs"]
fn installed_wet_tire_groups_have_runtime_textures() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let data = TireWaterData::read(&db);
    for name in ["asphalt", "asphalt_no_leaves", "wetpaved"] {
        let surface = db.collection("simsurface", name).expect("installed paved-road surface");
        let effect = data.effect(surface.key(), 0, true, false).expect("paved roads need authored wet driving spray");
        assert!(effect.emitters.iter().flatten().any(|p| p.rate > 0.0), "{name} needs a nonempty spray group");
    }
    let globals = crate::world::load_global_textures(&dir).unwrap();
    let mut effects = 0;
    for surface in db.collections_of("simsurface") {
        for kind in 0..3 {
            let Some(effect) = data.effect(surface.key(), kind, true, false) else { continue };
            effects += 1;
            for profile in effect.emitters.into_iter().flatten() {
                assert!(
                    globals.textures.iter().any(|t| t.name_hash == profile.texture),
                    "installed water emitter {:08x} needs texture {:08x}",
                    profile.key,
                    profile.texture
                );
            }
        }
    }
    assert!(effects > 0, "must resolve actual wet tire effects, not only empty fallbacks");
}

const LOOSE: [&str; 8] = ["grass", "golf_fairway", "golf_rough", "golf_green", "sand", "dirt", "mud", "gravel"];

#[test]
#[ignore = "requires original global attribute and texture packs"]
fn installed_dry_loose_surfaces_have_runtime_textures() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let data = TireWaterData::read(&db);
    let globals = crate::world::load_global_textures(&dir).unwrap();
    for name in LOOSE {
        let surface = db.collection("simsurface", name).expect("installed loose surface").key();
        let drive = data.effect(surface, 0, false, false).unwrap_or_else(|| panic!("{name} needs dry driving dust"));
        assert!(drive.emitters.iter().flatten().any(|e| e.rate > 0.0), "{name} dry drive group must emit");
        // Sand and dirt author a zero speed range for their wet driving record: rain silences it, as in the original.
        if let Some(wet) = data.effect(surface, 0, true, false) {
            assert_ne!(drive.key, wet.key, "{name}: rain must select the wet group, not the dry one");
        }
        for kind in 0..3 {
            let Some(effect) = data.effect(surface, kind, false, false) else { continue };
            for profile in effect.emitters.into_iter().flatten() {
                assert!(
                    globals.textures.iter().any(|t| t.name_hash == profile.texture),
                    "{name}: dry emitter {:08x} needs texture {:08x}",
                    profile.key,
                    profile.texture
                );
            }
        }
    }
}

#[test]
#[ignore = "requires original global attribute packs"]
fn installed_dry_paved_surfaces_keep_only_leaves() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let data = TireWaterData::read(&db);
    let leaves = blackbox_attrib::vlt_hash("emtd_dr_leaves");
    let asphalt = db.collection("simsurface", "asphalt").expect("installed asphalt").key();
    assert!(data.effect(asphalt, 0, false, false).is_none(), "dry asphalt has no driving dust");
    assert!(data.effect(asphalt, 1, false, false).is_none(), "tire smoke stays procedural");
    let slide = data.effect(asphalt, 2, false, false).expect("sliding on asphalt throws leaves");
    assert!(slide.emitters.iter().flatten().all(|e| e.key == leaves));
    let clean = db.collection("simsurface", "asphalt_no_leaves").expect("installed clean asphalt").key();
    assert!(data.effect(clean, 2, false, false).is_none(), "leaf-free asphalt has no leaves");
    let water = db.collection("simsurface", "water").expect("installed water").key();
    assert!(data.effect(water, 0, false, false).is_some(), "water surfaces always spray");
}

#[test]
fn unscaled_axles_pass_every_input_through() {
    let scale = AxleScale::default();
    assert!((0..4).all(|wheel| (0..3).all(|kind| scale.factor(wheel, kind) == 1.0)));
}

#[test]
#[ignore = "requires original global attribute packs"]
fn installed_axle_scales_damp_front_slide_and_heavy_vehicle_slip() {
    let dir = game_install::GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let car = AxleScale::read(&db, "BMWM3GTR");
    assert_eq!([car.factor(0, 2), car.factor(1, 2), car.factor(2, 2), car.factor(3, 2)], [0.1, 0.1, 1.0, 1.0]);
    assert_eq!([car.factor(0, 1), car.factor(3, 1), car.factor(0, 0)], [1.0, 1.0, 1.0]);
    let truck = AxleScale::read(&db, "truck");
    assert_eq!([truck.factor(0, 1), truck.factor(3, 1)], [0.1, 0.1]);
    assert_eq!(AxleScale::read(&db, "no_such_car"), AxleScale::read(&db, "default"));
}
