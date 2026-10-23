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
