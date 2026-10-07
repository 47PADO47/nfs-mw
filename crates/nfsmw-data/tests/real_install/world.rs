//! The whole streamed city: every section parses, and scenery resolves the way
//! `docs/formats/maps.md` says (own tile or shared sets, never another tile).

use std::collections::HashSet;

use blackbox_solid::Solid;
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex, load_section};

use crate::install;

pub fn assert_indices_in_range(s: &Solid, context: &str) {
    for g in &s.groups {
        let range = g.first_index as usize..(g.first_index + g.num_indices) as usize;
        let max = s.indices[range].iter().map(|&i| u32::from(i) + g.base_vertex).max().unwrap_or(0);
        assert!(g.num_indices == 0 || (max as usize) < s.vertices.len(), "{context}: {}: index out of range", s.name);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR); reads the whole 533 MB stream"]
fn every_world_section_parses_and_scenery_resolves() {
    let Some(dir) = install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    assert_eq!(index.sections.len(), 720);
    let mut file = dir.open_file(&index.stream_file).unwrap();

    let (mut solids, mut textures, mut instances) = (0, 0, 0);
    let mut shared_solids = HashSet::new();
    let mut tiles = Vec::new();
    for (i, section) in index.sections.iter().enumerate() {
        let data = load_section(&mut file, i, section).unwrap_or_else(|e| panic!("{}: {e:#}", section.name));
        for s in &data.solids {
            assert_indices_in_range(s, &section.name);
        }
        solids += data.solids.len();
        textures += data.textures.len();
        instances += data.scenery.iter().map(|s| s.instances.len()).sum::<usize>();
        if section.is_spatial() {
            tiles.push(data);
        } else {
            shared_solids.extend(data.solids.iter().map(|s| s.name_hash));
        }
    }
    eprintln!("{solids} solids, {textures} textures, {instances} scenery instances");
    assert_eq!(solids, 20_377);
    assert_eq!(instances, 77_783);

    let (mut resolved, mut missing) = (0, 0);
    for tile in &tiles {
        let own: HashSet<u32> = tile.solids.iter().map(|s| s.name_hash).collect();
        for section in &tile.scenery {
            for inst in &section.instances {
                match section.info_of(inst).and_then(|info| info.best_solid()) {
                    Some(k) if own.contains(&k) || shared_solids.contains(&k) => resolved += 1,
                    _ => missing += 1,
                }
            }
        }
    }
    eprintln!("tile instances: {resolved} resolved, {missing} not");
    assert!(missing * 1000 < resolved, "too many unresolved instances: {missing}");
}
