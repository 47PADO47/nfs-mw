//! A roadblock's sawhorse breaks into its four pieces through the installed event sequence (`genericsmackable`).
//! Spec: `docs/specs/ai-pursuit-roadblocks.md` section 6.1.

use super::super::ram::strike;
use super::*;
use crate::scenes::world::{
    drive::{FixedClock, STEP},
    props::PlacedSpawn,
};
use blackbox_vehicle::FlatGround;
use game_install::GameDir;
use nfsmw_data::world::{DEFAULT_TRACK, PropCatalog, WorldIndex};

/// The pieces a sawhorse breaks into (`SAWHORSE_FRAG01..04`).
const PARTS: [u32; 4] = [0x863C_2522, 0x8395_A1C1, 0x4832_AE9D, 0x4C1B_1E37];

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_sawhorse_breaks_into_its_four_pieces_when_a_car_hits_it() {
    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let mut data = PhysicsData::load(&dir).unwrap();
    data.set_sequences(index.sequences.clone());
    let catalog =
        PropCatalog::new(index.prop_bounds.clone(), data.database()).with_hierarchies(index.model_hierarchies.clone());
    let (bounds, spec) = catalog.placeable("XO_Sawhorse_1b_00", "sawhorse").expect("the sawhorse's bounds");
    let hierarchy = catalog.hierarchy_of("XO_Sawhorse_1b_00").expect("the sawhorse has a hierarchy");
    let mut names: Vec<_> = hierarchy.nodes.iter().skip(1).map(|n| n.name_hash).collect();
    names.sort_unstable();
    let mut expected = PARTS;
    expected.sort_unstable();
    assert_eq!(names, expected, "its children are the four pieces");
    let available = (0..hierarchy.nodes.len()).collect();
    let parts = Arc::new(ModelParts { hierarchy, available, meshes: BTreeMap::new() });

    let mut world = PropWorld::default();
    let owner = Rc::new(());
    let frame = Mat4::from_translation(Vec3::new(100.0, 100.0, 1.0));
    let id = world
        .spawn_placed(PlacedSpawn {
            bounds,
            spec,
            model: frame,
            instances: Vec::new(),
            owner: Rc::downgrade(&owner),
            parts: Some(parts),
        })
        .expect("the sawhorse is placed");
    assert!(world.scenery.models.contains_key(&id), "the sawhorse has a scenery model");
    assert_eq!(world.fragment_count(), 1, "only the sawhorse itself so far");
    strike(&mut world, id);
    let mut clock = FixedClock::default();
    let ground = FlatGround::new(-1000.0);
    for _ in 0..180 {
        for _ in 0..clock.advance(1.0 / 60.0) {
            world.step(&ground, STEP);
            world.update_scenery(&data);
        }
    }
    let pieces = world.scenery.models.get(&id).map_or(0, |m| m.spawned.len());
    eprintln!("pieces released: {pieces}, bodies {}", world.fragment_count());
    assert_eq!(pieces, 4, "the sequence releases the four pieces its parent has");
    assert_eq!(world.fragment_count(), 5, "the sawhorse and its four pieces");
    assert!(world.scenery.models[&id].hidden, "the whole sawhorse is hidden once the pieces fly");
    drop(owner);
    world.remove_orphaned_fragments();
    assert_eq!(world.fragment_count(), 0, "the pieces live as long as the roadblock");
}
