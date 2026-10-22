//! The EAGL animation banks of the install, read with `blackbox-anim` and checked against the census in
//! `docs/specs/eagl-animation-bank.md` section 11: 848 animations (805 NIS, 39 L2RA, 4 InGame) in 125 bank objects,
//! every entry a Compound at 30 fps, and every child a delta codec with 1 to 3 children per compound.

use std::collections::BTreeMap;

use blackbox_anim::{Bank, Compound, Object, TypeId};

use crate::{install, unwrapped};

/// The chunk id of an `EAGLAnimations` chunk: one animation bank object per chunk.
pub(super) const EAGL_ANIMATIONS: u32 = 0x00E3_4010;
/// The type id of a compound channel: the type of every bank entry in the install.
const COMPOUND: u16 = 15;
/// The frame rate every compound holds.
const FPS: f32 = 30.0;

/// The census of one group of bank files (NIS, L2RA or InGame).
#[derive(Default)]
struct Census {
    banks: usize,
    animations: usize,
    /// How many compounds have each child count.
    child_counts: BTreeMap<usize, usize>,
    /// How many child records have each type id.
    child_types: BTreeMap<u16, usize>,
}

/// The bank files to read, as `(group, install path)`: every NIS scene bundle, then the L2RA track and InGame packs.
pub(super) fn bank_files(dir: &game_install::GameDir) -> Vec<(&'static str, String)> {
    let mut files: Vec<(&'static str, String)> = dir
        .files_in("NIS")
        .into_iter()
        .filter(|name| {
            let lower = name.to_ascii_lowercase();
            lower.starts_with("scene_") && lower.ends_with("_bundleb.bun")
        })
        .map(|name| ("NIS", format!("NIS/{name}")))
        .collect();
    files.push(("L2RA", "TRACKS/L2RA.BUN".to_string()));
    files.push(("InGame", "GLOBAL/InGameB.bun".to_string()));
    files
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_animation_bank_matches_the_census() {
    let Some(dir) = install() else { return };
    let mut groups: BTreeMap<&str, Census> = BTreeMap::new();
    for (group, rel) in bank_files(&dir) {
        let data = unwrapped(&dir, &rel);
        let census = groups.entry(group).or_default();
        for chunk in blackbox_chunk::find_all(&data, EAGL_ANIMATIONS) {
            let at = chunk.offset;
            let object = Object::read(chunk.payload).unwrap_or_else(|e| panic!("{rel} at {at:#x}: {e}"));
            let bank = Bank::read(&object).unwrap_or_else(|e| panic!("{rel} at {at:#x}: {e}"));
            census.banks += 1;
            for animation in bank.iter() {
                census.animations += 1;
                let name = &animation.name;
                assert_eq!(animation.type_id, COMPOUND, "{rel}: {name} has type {}", animation.type_id);
                let found = bank.find(name).map(|found| found.offset);
                assert_eq!(found, Some(animation.offset), "{rel}: {name} is not found by its name");
                let compound = Compound::read(&object, animation.offset)
                    .unwrap_or_else(|e| panic!("{rel}: {name} at {:#x}: {e}", animation.offset));
                assert_eq!(compound.fps, FPS, "{rel}: {name} frame rate");
                let children = compound.children.len();
                assert!((1..=3).contains(&children), "{rel}: {name} has {children} children");
                *census.child_counts.entry(children).or_default() += 1;
                for child in &compound.children {
                    let leaf = TypeId::from_u16(child.type_id).is_some_and(TypeId::is_supported_leaf);
                    assert!(leaf, "{rel}: {name} has child type {}", child.type_id);
                    *census.child_types.entry(child.type_id).or_default() += 1;
                }
            }
        }
    }

    let mut totals: BTreeMap<u16, usize> = BTreeMap::new();
    for (group, census) in &groups {
        eprintln!("{group}: {} banks, {} animations", census.banks, census.animations);
        eprintln!("  child count per compound: {:?}", census.child_counts);
        eprintln!("  child records per type: {:?}", census.child_types);
        for (&type_id, &count) in &census.child_types {
            *totals.entry(type_id).or_default() += count;
        }
    }
    eprintln!("all child records per type: {totals:?}");

    let expected = [("NIS", 111, 805), ("L2RA", 13, 39), ("InGame", 1, 4)];
    for (group, banks, animations) in expected {
        let census = &groups[group];
        assert_eq!(census.banks, banks, "{group} bank objects");
        assert_eq!(census.animations, animations, "{group} animations");
    }
    let animations: usize = groups.values().map(|census| census.animations).sum();
    assert_eq!(animations, 848, "animations in total");
    let census_types = [(10, 475), (11, 343), (17, 1), (20, 90), (21, 39)];
    let census_map: BTreeMap<u16, usize> = census_types.into_iter().collect();
    assert_eq!(totals, census_map, "child records per type");
}
