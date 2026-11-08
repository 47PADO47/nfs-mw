//! The bushes set: its families, the manifest the game reads and the split with the trees set. No game data.

use std::collections::HashSet;

use blackbox_modding::content::{FitLimits, SwapFit, parse_replace_set};

use super::bushes::{self, FAMILIES};
use super::manifest;
use super::species::SPECIES;
use super::stock::{self, LEGACY_BUSHES};
use super::trees_set;

const BUILT: [(&str, usize); 6] =
    [("POPLAR", 4), ("BUSH", 4), ("HEDGE", 3), ("SHRUB_GREEN", 3), ("DARK_SHRUB", 3), ("PALE_SHRUB", 4)];

#[test]
fn every_bush_family_names_a_species_and_every_solid_is_listed_once_across_the_two_sets() {
    let keys: HashSet<&str> = SPECIES.iter().map(|species| species.key).collect();
    for apart in [true, false] {
        let mut seen = HashSet::new();
        let families: Vec<_> = stock::tree_families(apart).collect();
        let bush_families: Vec<_> = if apart { FAMILIES.iter().collect() } else { Vec::new() };
        for family in families.into_iter().chain(bush_families) {
            assert!(keys.contains(family.species), "{}", family.stem);
            for (name, code) in family.names() {
                assert!(seen.insert(name.clone()), "{name} is listed twice (bushes apart: {apart})");
                assert!(name.contains(&format!("_{code}_")) && name.len() <= 63, "{name}");
            }
        }
    }
}

#[test]
fn the_old_round_bushes_move_to_the_bushes_set_and_stay_in_the_trees_without_it() {
    let apart: Vec<&str> = stock::tree_families(true).map(|family| family.stem).collect();
    let together: Vec<&str> = stock::tree_families(false).map(|family| family.stem).collect();
    for legacy in LEGACY_BUSHES {
        assert!(!apart.contains(&legacy.stem), "{} is in both sets", legacy.stem);
        assert!(together.contains(&legacy.stem), "{} left the trees without being replaced", legacy.stem);
        assert!(FAMILIES.iter().any(|family| family.stem == legacy.stem), "{} is not in the bushes set", legacy.stem);
    }
}

#[test]
fn the_bushes_manifest_is_a_set_of_kind_bushes_that_the_game_accepts() {
    let text = manifest::replace_manifest(&bushes::set(), &BUILT);
    let set = parse_replace_set("bushes-mw2012", &text).unwrap();
    assert_eq!(
        (set.kind.as_str(), set.value.as_str(), set.option().as_deref()),
        ("bushes", "mw2012", Some("bushes_variant"))
    );
    assert_eq!(set.kind_label.as_deref(), Some("nfsmw2012-bushes"));
    assert_eq!(set.geometry.as_deref(), Some("TREES/MW2012/GEOMETRY.BIN"), "the pack of the trees is the bushes' too");
    assert_eq!(set.limits, FitLimits { spread: 1.5, squeeze: 0.5, min_height: 0.5 });
    let fits: Vec<(String, SwapFit)> = set
        .models
        .iter()
        .flat_map(|entry| entry.targets.iter().map(|target| format!("{target:?}")).map(|name| (name, entry.fit)))
        .collect();
    let find = |part: &str| fits.iter().find(|(name, _)| name.contains(part)).unwrap_or_else(|| panic!("{part}")).1;
    assert_eq!(find("XT_HEDGEA4M_1B_00"), SwapFit::Row, "a hedge is a stretch of copies");
    assert_eq!(find("XT_BUSHYELLOWFLOWERS_1B_RB_00"), SwapFit::Height);
}

#[test]
fn the_species_a_set_lacks_have_no_swap() {
    let text = manifest::replace_manifest(&bushes::set(), &[("HEDGE", 3)]);
    let set = parse_replace_set("bushes-mw2012", &text).unwrap();
    assert!(set.models.iter().all(|entry| entry.model.starts_with("MW12_HEDGE_L")));
    assert!(parse_replace_set("bushes-mw2012", &manifest::replace_manifest(&bushes::set(), &[])).is_err());
}

#[test]
fn the_two_sets_name_the_same_pack_so_the_textures_exist_once() {
    let trees = parse_replace_set("trees-mw2012", &manifest::replace_manifest(&trees_set(true), &BUILT)).unwrap();
    let bushes = parse_replace_set("bushes-mw2012", &manifest::replace_manifest(&bushes::set(), &BUILT)).unwrap();
    assert_eq!((&trees.geometry, &trees.textures), (&bushes.geometry, &bushes.textures));
    assert_eq!((trees.kind.as_str(), bushes.kind.as_str()), ("trees", "bushes"));
}
