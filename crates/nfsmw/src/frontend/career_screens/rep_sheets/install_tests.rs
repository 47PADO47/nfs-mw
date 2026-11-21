//! With the install (`NFSMW_GAME_DIR`): each sheet's package has every object the sheet fills or hides.

use game_install::GameDir;

use super::{Kind, used_objects};
use crate::frontend::ids::screen;
use crate::ui::Catalog;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_sheet_packages_have_the_objects_the_sheets_fill() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["FRONTEND/FrontB.lzc"]);
    for (kind, package) in [
        (Kind::Race, screen::SAFE_HOUSE_RACE_SHEET),
        (Kind::Milestones, screen::SAFE_HOUSE_MILESTONES),
        (Kind::Bounty, screen::SAFE_HOUSE_BOUNTY),
    ] {
        let def = catalog.find(package).unwrap_or_else(|| panic!("{package} is not in the install"));
        for hash in used_objects(kind) {
            assert!(def.objects.iter().any(|o| o.name_hash == hash), "{hash:#x} is not an object of {package}");
        }
    }
}

#[test]
#[ignore = "needs the stock in-game and safe-house milestone packages"]
fn installed_milestone_twins_have_distinct_cursor_cues() {
    use blackbox_feng::{Runtime, fe_hash_upper, runtime::Outgoing};

    let dir = GameDir::open(std::env::var_os("NFSMW_GAME_DIR").unwrap()).unwrap();
    let catalog = Catalog::load(&dir, &["FRONTEND/FrontB.lzc", "GLOBAL/InGameB.bun"]);
    let mut runtime = Runtime::new();
    for (name, cursor, expected) in [
        (screen::SAFE_HOUSE_MILESTONES, 0x0674_5352, 0x1CFA_D52B),
        (screen::IN_GAME_MILESTONES, 0xD4FE_BDB2, 0x9197_0159),
    ] {
        let package = runtime.load(catalog.find(name).unwrap().clone());
        let cursor = runtime.find(package, cursor).unwrap();
        for slot in 1..=9 {
            runtime.run_script(cursor, fe_hash_upper(&format!("POS{slot}")));
            runtime.update(1.0 / 60.0);
            assert!(
                runtime.take_outgoing().iter().any(|event| {
                    matches!(event, Outgoing::Sound { message, authored: true, .. } if *message == expected)
                }),
                "{name} slot {slot}"
            );
        }
    }
}
