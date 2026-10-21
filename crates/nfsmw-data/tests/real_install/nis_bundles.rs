//! `NisBundle::load` on every NIS scene of the install: the 87 bundles are listed by their scene names, and each one
//! loads with its header under the name the file gives (`docs/specs/nis-scenes.md`).

use nfsmw_data::nis::NisBundle;

use crate::install;

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn all_87_nis_bundles_load_by_scene_name() {
    let Some(dir) = install() else { return };
    let names = NisBundle::list(&dir);
    assert_eq!(names.len(), 87, "NIS scene bundles listed");
    for name in &names {
        let bundle = NisBundle::load(&dir, name).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        assert_eq!(bundle.name, *name, "the bundle's scene name");
        assert_eq!(bundle.scene.name, *name, "{name}: the header's name");
    }
    let lower = names[0].to_ascii_lowercase();
    let again = NisBundle::load(&dir, &lower).unwrap_or_else(|e| panic!("{lower}: {e:#}"));
    assert_eq!(again.scene.name, names[0], "the name matches without regard to case");
    assert!(NisBundle::load(&dir, "NoSuchScene").is_err(), "an unknown scene is an error");
}
