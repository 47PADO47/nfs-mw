//! With the install (`NFSMW_GAME_DIR`): the HUD package has every object the binding looks for.

use blackbox_feng::{Runtime, fe_hash_upper};
use game_install::GameDir;

use super::bind::HudBinding;
use super::minimap::MinimapBinding;
use crate::ui::{Catalog, UiAssets};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_single_race_hud_has_the_objects_the_binding_drives() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let package = catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone();
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let by_name = |n: &str| rt.find(id, fe_hash_upper(n)).is_some();
    for name in [
        "SpeedometerGroup",
        "SPEED_DIGIT_1",
        "SPEED_DIGIT_2",
        "SPEED_DIGIT_3",
        "3rdPersonSpeedUnits",
        "GaugeCluster",
        "3rdPersonGear",
        "3rdPersonNeedle",
        "Shift_light",
        "TAC_Lines_7500",
        "RPM_REDLINE",
        "TURBO_GROUP",
        "3rdperson_TurboDial",
    ] {
        assert!(by_name(name), "{name} is missing");
    }
    for hash in [0x87c3_8e97u32, 0x27dd_f583, 0xedfb_6d37] {
        assert!(rt.find(id, hash).is_some(), "{hash:#x} is missing");
    }
    // Binding every object works.
    let _ = HudBinding::new(&mut rt, id);
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_minimap_finds_its_objects_and_its_tiles_in_the_install() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let package = catalog.find("HUD_SingleRace.fng").expect("HUD_SingleRace.fng").clone();
    let mut rt = Runtime::new();
    let id = rt.load(package);
    let assets = UiAssets::load(&dir).unwrap();
    assert!(MinimapBinding::open_city(&rt, id, &dir, &assets).is_some());
    // The textures the pieces' mask and the arrow are drawn with.
    for name in ["MINIMAP_MASK", "MINIMAP_ICON_CAR", "MINIMAP_BACKING_COLOR", "MINI_MAP_CHOP0", "MINI_MAP_CHOP63"] {
        assert!(assets.texture(fe_hash_upper(name)).is_some(), "{name} is missing");
    }
    assert!(rt.find(id, fe_hash_upper("TRACKMAPTARGETRING")).is_some());
}
