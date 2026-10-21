//! With the install (`NFSMW_GAME_DIR`): `PostRace_Results.fng` has every object the pages' logic fills or hides.

use game_install::GameDir;

use super::screen::used_objects;
use crate::ui::Catalog;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_results_screen_has_the_objects_the_pages_fill() {
    let root = std::env::var_os("NFSMW_GAME_DIR").expect("NFSMW_GAME_DIR");
    let dir = GameDir::open(std::path::PathBuf::from(root)).unwrap();
    let catalog = Catalog::load(&dir, &["GLOBAL/InGameB.bun", "GLOBAL/INGAMEC.BUN"]);
    let def = catalog.find("PostRace_Results.fng").expect("PostRace_Results.fng");
    for hash in used_objects() {
        assert!(def.objects.iter().any(|o| o.name_hash == hash), "{hash:#x} is not an object of PostRace_Results.fng");
    }
}
