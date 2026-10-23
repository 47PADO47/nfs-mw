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
        let effect = data.wet(surface.key(), 0, false).expect("paved roads need authored wet driving spray");
        assert!(effect.emitters.iter().flatten().any(|p| p.rate > 0.0), "{name} needs a nonempty spray group");
    }
    let globals = crate::world::load_global_textures(&dir).unwrap();
    let mut effects = 0;
    for surface in db.collections_of("simsurface") {
        for kind in 0..3 {
            let Some(effect) = data.wet(surface.key(), kind, false) else { continue };
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
