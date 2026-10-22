//! The skinned solids of `Scene_ArrestF02_BundleB.bun`: their skin, and the parts `rigid_split` makes of them
//! (`docs/formats/models.md`, "Skinned vertices and bones").

use blackbox_solid::{Solid, read_solids, rigid_split};

use crate::install;

/// The bundle with the three skinned solids.
const BUNDLE: &str = "NIS/Scene_ArrestF02_BundleB.bun";

/// The skinned solids, and the bone count of each rig: every bone number must be below it.
const SKINNED: [(&str, u8); 3] = [("FCOP01", 48), ("PERP01", 48), ("CUFFS01", 3)];

/// The solids of the bundle, or `None` without the install.
fn load() -> Option<Vec<Solid>> {
    let dir = install()?;
    let data = crate::unwrapped(&dir, BUNDLE);
    Some(read_solids(&data).unwrap_or_else(|e| panic!("{BUNDLE}: {e}")))
}

fn named<'a>(solids: &'a [Solid], name: &str) -> &'a Solid {
    solids.iter().find(|s| s.name == name).unwrap_or_else(|| panic!("{BUNDLE}: no solid {name}"))
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn the_arrest_solids_carry_skin_within_their_rigs() {
    let Some(solids) = load() else { return };
    for (name, bones) in SKINNED {
        let solid = named(&solids, name);
        let skin = solid.skin.as_ref().unwrap_or_else(|| panic!("{name} has no skin"));
        assert_eq!(skin.len(), solid.vertices.len(), "{name}: one skin entry per vertex");
        for (i, v) in skin.iter().enumerate() {
            assert!(v.bones.iter().all(|&b| b < bones), "{name} vertex {i}: bones {:?} past {bones}", v.bones);
            let sum: f32 = v.weights.iter().sum();
            assert!((sum - 1.0).abs() <= 0.01, "{name} vertex {i}: weights sum to {sum}");
        }
    }
    assert_eq!(named(&solids, "FCOP01").vertices.len(), 2547, "FCOP01 vertices");
}

#[test]
#[ignore = "needs the game (set NFSMW_GAME_DIR)"]
fn rigid_split_keeps_every_triangle_of_the_skinned_solids() {
    let Some(solids) = load() else { return };
    for (name, bones) in SKINNED {
        let solid = named(&solids, name);
        let parts = rigid_split(solid);
        let triangles: usize = parts.iter().map(|p| p.indices.len() / 3).sum();
        assert_eq!(triangles, solid.indices.len() / 3, "{name}: triangles across the parts");
        assert!(parts.iter().all(|p| p.bone < bones), "{name}: a part names a bone past {bones}");
        for part in &parts {
            assert!(part.indices.iter().all(|&i| (i as usize) < part.vertices.len()), "{name}: index out of part");
        }
    }
}
