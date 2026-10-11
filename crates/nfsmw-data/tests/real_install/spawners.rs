//! Complete placement-to-scenery join on the user's track data.

use std::collections::{HashMap, HashSet};

use blackbox_attrib::{Database, vlt_hash};
use blackbox_scenery::{layout::MOST_WANTED, read_object_spawners, read_scenery_sections};
use nfsmw_data::world::{DEFAULT_TRACK, WorldIndex};

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn every_physics_placement_joins_to_scenery_and_attributes() {
    let Some(dir) = crate::install() else { return };
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let bounds: HashSet<_> = index.prop_bounds.iter().map(|b| b.name_hash).collect();
    let mut ids = HashSet::new();
    let mut total = 0;
    let mut missing_bounds = 0;
    for s in &index.sections {
        let bytes = dir.read_range(&index.stream_file, u64::from(s.file_offset), s.size as usize).unwrap();
        let scenery = read_scenery_sections(&bytes, &MOST_WANTED).unwrap();
        let hooks: HashMap<_, _> = scenery
            .iter()
            .flat_map(|section| {
                section
                    .override_hooks
                    .iter()
                    .map(|h| (u32::from(h.override_index), &section.instances[usize::from(h.instance_index)]))
            })
            .collect();
        for p in read_object_spawners(&bytes, &MOST_WANTED.spawners).unwrap() {
            total += 1;
            assert!(ids.insert(p.unique_id), "duplicate ID {}", p.unique_id);
            let inst = hooks.get(&p.override_index).unwrap();
            assert_eq!([p.position[2], -p.position[0], p.position[1]], inst.position);
            let attrs = db.collection_by_key(vlt_hash("smackable"), p.attributes).unwrap();
            if bounds.contains(&p.collision) {
                continue;
            }
            missing_bounds += 1;
            assert!(matches!(attrs.name(), Some("lightwoodenpost" | "wood_slat_fence")));
        }
    }
    assert_eq!(total, 13_484);
    assert_eq!(missing_bounds, 5);
}
