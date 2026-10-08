//! With the install (`NFSMW_GAME_DIR`): the HUD package has every object the binding looks for.

use blackbox_feng::{Runtime, fe_hash_upper};
use game_install::GameDir;

use super::bind::HudBinding;
use crate::ui::Catalog;

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
