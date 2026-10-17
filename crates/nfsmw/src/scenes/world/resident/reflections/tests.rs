use super::*;
use blackbox_scenery::{LodModel, SceneryInfo, SceneryInstance, ScenerySection};
use nfsmw_data::world::PropCatalog;

#[test]
fn road_visibility_preserves_source_bit_alias_and_player_exclusion() {
    assert!(!visible(0, ROAD_VIEW_FLAGS));
    for flags in [0x40, 0x0800_0000, 0x42, 0x0800_0002] {
        assert!(visible(flags, ROAD_VIEW_FLAGS));
    }
    assert!(!visible(0x50, ROAD_VIEW_FLAGS), "common static exclusion still wins");
    assert!(!visible(0x41, ROAD_VIEW_FLAGS | 1), "optional source event/view exclusions remain active");
}

#[test]
fn reflection_only_placements_keep_all_slots_without_entering_normal_or_prop_feed() {
    let data = data();
    let mut own = SectionResources::default();
    for i in 1..=4 {
        own.meshes.insert(i, (MeshHandle::from_raw(i as usize), LodModel { num_polys: 1, density: 1.0 }, false));
    }
    let catalog = PropCatalog::new(vec![], &blackbox_attrib::Database::new());
    let (mut player, unresolved) = super::super::place(&data, &own, &SectionResources::default(), &catalog);
    assert_eq!(unresolved, 0);
    assert_eq!(player.len(), 3);
    let mut reflected = place_reflections(&data, &own, &SectionResources::default());
    assert_eq!(reflected.len(), 3);
    assert_eq!(reflected.iter().map(|p| p.player_index).collect::<Vec<_>>(), [Some(1), None, Some(2)]);
    assert!(reflected.iter().all(|p| p.lods[3] == Some(MeshHandle::from_raw(4))));
    player[1].prop_id = 77;
    player[1].key = InstanceKey::new(5, 9);
    for (i, p) in reflected.iter_mut().enumerate() {
        p.bind_player(&player, InstanceKey::new(5, (1 << 31) | i as u32));
    }
    assert_eq!((reflected[0].prop_id, reflected[0].key), (77, player[1].key));
    assert_eq!(reflected[1].prop_id, 0, "reflection-only geometry never registers a physical prop");
    assert!(!player.iter().any(|p| p.key == reflected[1].key));
}

fn data() -> SectionData {
    SectionData {
        solid_vertices: Vec::new(),
        decoded: Vec::new(),
        index: 0,
        solids: vec![],
        textures: vec![],
        anims: vec![],
        spawners: vec![],
        collision: vec![],
        scenery: vec![ScenerySection {
            section_number: 101,
            infos: vec![SceneryInfo {
                name: "authored fixture".into(),
                solid_keys: vec![1, 2, 3, 4],
                radius: 5.0,
                hierarchy_hash: 0,
            }],
            instances: [0, 0x40, 0x42, 0x50, 0x0800_0000]
                .into_iter()
                .enumerate()
                .map(|(i, flags)| SceneryInstance {
                    bbox_min: [-1.0; 3],
                    bbox_max: [1.0; 3],
                    exclude_flags: flags,
                    preculler_index: -1,
                    lighting_context: -1,
                    position: [i as f32, 0.0, 0.0],
                    rotation: [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
                    info_index: 0,
                })
                .collect(),
            override_hooks: vec![],
        }],
    }
}
