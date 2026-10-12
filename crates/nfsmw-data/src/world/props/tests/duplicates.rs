use blackbox_collision::{BodyGeometry, GeometryOptions};

use super::*;

fn fixture(key: u32, detailed: bool) -> BoundsSet {
    let mut root = Bounds {
        orientation: Quat::IDENTITY.to_array(),
        position: [0.0; 3],
        flags: bounds_flags::BOX,
        half_dimensions: [3.0, 5.0, 0.3],
        child_count: 0,
        point_cloud: None,
        pivot: [2.5, 5.0, 0.0],
        first_child: -1,
        radius: 6.0,
        surface: 0,
        name_hash: 0,
    };
    if !detailed {
        return BoundsSet { name_hash: key, nodes: vec![root], point_clouds: vec![] };
    }
    root.child_count = 1;
    root.first_child = 1;
    let pole = Bounds {
        flags: bounds_flags::BOX | bounds_flags::PRIM_VS_OBJECTS,
        position: [-2.5, -0.1, 0.0],
        pivot: [0.0, 4.9, 0.0],
        half_dimensions: [0.2, 4.8, 0.3],
        child_count: 0,
        first_child: -1,
        ..root.clone()
    };
    BoundsSet { name_hash: key, nodes: vec![root, pole], point_clouds: vec![] }
}

fn geometry(set: &BoundsSet) -> BodyGeometry {
    BodyGeometry::from_bounds(
        set,
        0,
        GeometryOptions {
            fallback_half: set.root().half_dimensions.into(),
            fallback_shape: Shape::Box,
            box_padding: Vec3::ZERO,
            surface: 0,
        },
    )
    .unwrap()
}

#[test]
fn duplicate_bounds_keep_first_collection_and_its_contact_geometry() {
    // Both orders matter: original selection follows file order, not detail count.
    for first_detailed in [true, false] {
        let first = fixture(1, first_detailed);
        let catalog =
            PropCatalog::new(vec![first.clone(), fixture(2, false), fixture(1, !first_detailed)], &Database::new());
        assert_eq!(catalog.len(), 2);
        let selected = &catalog.sets[&1];
        assert_eq!(selected.as_ref(), &first, "later duplicate replaced the first collection");
        let actual = geometry(selected);
        assert_eq!(actual.used_fallback, !first_detailed);
        assert_eq!(actual.primitives.len(), 1);
        let expected = match first_detailed {
            true => (Vec3::new(-2.5, -0.1, 0.0), Vec3::new(0.2, 4.8, 0.3)),
            false => (Vec3::ZERO, Vec3::new(3.0, 5.0, 0.3)),
        };
        assert_eq!((actual.primitives[0].centre, actual.primitives[0].half), expected);
    }
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn installed_duplicate_bounds_keep_authored_poles_and_fragment_hierarchies() {
    use crate::world::{DEFAULT_TRACK, WorldIndex};
    use game_install::GameDir;

    let Some(path) = std::env::var_os("NFSMW_GAME_DIR") else { return };
    let dir = GameDir::open(path).unwrap();
    let index = WorldIndex::open(&dir, DEFAULT_TRACK).unwrap();
    let db = Database::open(&dir.read("GLOBAL/ATTRIBUTES.BIN").unwrap()).unwrap();
    let catalog = PropCatalog::new(index.prop_bounds.clone(), &db);
    let mut seen = HashMap::new();
    let mut changed = 0;
    let mut narrowed = 0;
    for set in &index.prop_bounds {
        let Some(first) = seen.get(&set.name_hash).copied() else {
            seen.insert(set.name_hash, set);
            continue;
        };
        if first == set {
            continue;
        }
        changed += 1;
        let selected = &catalog.sets[&set.name_hash];
        assert_eq!(selected.as_ref(), first, "duplicate key {} lost its original hierarchy", set.name_hash);
        let actual = geometry(selected);
        let expected = geometry(first);
        assert_eq!(actual.used_fallback, expected.used_fallback);
        assert_eq!(actual.primitives.len(), expected.primitives.len());
        for (a, b) in actual.primitives.iter().zip(&expected.primitives) {
            assert_eq!((a.centre, a.half, a.flags), (b.centre, b.half, b.flags));
        }
        if !expected.used_fallback && geometry(set).used_fallback {
            narrowed += 1;
        }
    }
    assert!(changed > 0, "fixture must include conflicting duplicate collections");
    assert!(narrowed > 0, "fixture must include detailed collision followed by a broad fallback");
}
