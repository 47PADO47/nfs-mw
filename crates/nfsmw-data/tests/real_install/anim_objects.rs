//! `AnimObjects::load` on the install: one NIS bundle holds its three rigs, and the 87 NIS scene bundles together hold
//! the same 64 skeletons and 805 animations that the per-chunk tests count (`anim_skeletons`, `anim_banks`).

use nfsmw_data::anim::AnimObjects;

use crate::install;

/// The rigs of `Scene_ArrestF02_BundleB.bun` (`docs/formats/models.md`).
const ARREST_RIGS: [&str; 3] = ["Bip21", "Bip23", "Bip24"];

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_arrest_bundle_holds_its_three_rigs() {
    let Some(dir) = install() else { return };
    let objects = AnimObjects::load(&dir, "NIS/Scene_ArrestF02_BundleB.bun")
        .unwrap_or_else(|e| panic!("Scene_ArrestF02_BundleB.bun: {e:#}"));
    let names: Vec<&str> = objects.skeletons.iter().map(|skeleton| skeleton.name.as_str()).collect();
    assert_eq!(names, ARREST_RIGS, "skeleton names in chunk order");
    assert!(objects.skeletons.iter().all(|skeleton| !skeleton.bones.is_empty()), "every rig has bones");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_nis_bundles_hold_64_skeletons_and_805_animations() {
    let Some(dir) = install() else { return };
    let mut bundles = 0;
    let mut skeletons = 0;
    let mut animations = 0;
    for name in dir.files_in("NIS") {
        let lower = name.to_ascii_lowercase();
        if !(lower.starts_with("scene_") && lower.ends_with("_bundleb.bun")) {
            continue;
        }
        let objects = AnimObjects::load(&dir, &format!("NIS/{name}")).unwrap_or_else(|e| panic!("{name}: {e:#}"));
        bundles += 1;
        skeletons += objects.skeletons.len();
        animations += objects.banks.iter().map(|(_, bank)| bank.animations.len()).sum::<usize>();
    }
    eprintln!("{bundles} bundles, {skeletons} skeletons, {animations} animations");
    assert_eq!(bundles, 87, "NIS scene bundles");
    assert_eq!(skeletons, 64, "skeletons in the NIS bundles");
    assert_eq!(animations, 805, "animations in the NIS bundles");
}
