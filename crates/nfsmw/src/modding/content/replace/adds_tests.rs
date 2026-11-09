//! The objects of a set as map packs, over a folder with a generated geometry file.

use std::sync::Arc;

use blackbox_hash::bstring_hash;
use blackbox_modding::content::parse_replace_set;
use blackbox_solid::{GroupSource, SolidSource, Vertex, write_geometry_pack};

use super::super::active::LoadedSet;
use super::*;
use crate::modding::content::replace::{Active, TEST_LOCK, set_active};

fn vertex(position: [f32; 3]) -> Vertex {
    Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv: [0.0, 0.0] }
}

/// A folder holding `SETS/GEOMETRY.BIN` with one solid, `BENCH`.
fn folder(name: &str) -> (std::path::PathBuf, GameDir, Vec<blackbox_solid::Solid>) {
    let root = std::env::temp_dir().join(format!("nfsmw-adds-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("SETS")).unwrap();
    let square = [[-1.0, -1.0, 0.0], [1.0, -1.0, 0.0], [1.0, -1.0, 1.0], [-1.0, -1.0, 1.0]];
    let group = GroupSource::new(square.iter().map(|corner| vertex(*corner)).collect(), vec![0, 1, 2, 0, 2, 3]);
    let mut bench = SolidSource::new("BENCH", vec![group]);
    bench.texture_hashes = vec![bstring_hash("T")];
    let bytes = write_geometry_pack(&[bench]).unwrap();
    std::fs::write(root.join("SETS/GEOMETRY.BIN"), &bytes).unwrap();
    let dir = GameDir::open(&root).unwrap();
    (root, dir, blackbox_solid::read_solids(&bytes).unwrap())
}

fn active_with(adds: &str, models: Vec<blackbox_solid::Solid>) -> Arc<Active> {
    let text = format!("format = 1\n[set]\nkind = \"props\"\n[files]\ngeometry = \"SETS/GEOMETRY.BIN\"\n{adds}");
    let set = Arc::new(parse_replace_set("benches", &text).unwrap());
    Arc::new(Active::build(vec![LoadedSet {
        owner: "mod".to_owned(),
        order: 0,
        set,
        models,
        added: Vec::new(),
        replaced: Vec::new(),
    }]))
}

#[test]
fn objects_become_one_pack_per_cell_with_free_section_numbers_and_anchors_round_them() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (root, dir, models) = folder("cells");
    let adds = "[[add]]\nmodel = \"bench\"\npos = [10.0, 10.0, 1.0]\n[[add]]\nmodel = \"BENCH\"\npos = [30.0, 50.0, 1.0]\n[[add]]\nmodel = \"BENCH\"\npos = [900.0, 10.0, 1.0]\n[[add]]\nmodel = \"NOPE\"\npos = [0.0, 0.0, 0.0]\n";
    set_active(Some(active_with(adds, models)));
    let taken: HashSet<i16> = [2700].into_iter().collect();
    let made = packs(&dir, &taken);
    set_active(None);
    assert_eq!(made.len(), 2, "two cells: the object that names no solid of the file is left out");
    let numbers: Vec<i16> = made.iter().map(|pack| pack.section.number).collect();
    assert_eq!(numbers, [2701, 2702], "the number that is taken is skipped");
    assert_eq!(made[0].manifest.places.len(), 2);
    let [x, y, _, radius] = made[0].manifest.anchor;
    assert!((x - 20.0).abs() < 1e-3 && (y - 30.0).abs() < 1e-3, "centred on the two objects: {x} {y}");
    assert!(radius > 20.0, "it holds both: {radius}");
    assert!(made[0].key.starts_with("add-benches-"));
    assert!(matches!(made[0].source, crate::modding::content::scenery::PackSource::File(_)));
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn nothing_in_effect_places_nothing_and_prop_collision_is_refused() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (root, dir, models) = folder("none");
    set_active(None);
    assert!(packs(&dir, &HashSet::new()).is_empty());
    let adds = "[[add]]\nmodel = \"BENCH\"\npos = [1.0, 1.0, 1.0]\ncollision = \"prop\"\n";
    set_active(Some(active_with(adds, models)));
    assert!(packs(&dir, &HashSet::new()).is_empty(), "prop collision is not offered");
    set_active(None);
    let _ = std::fs::remove_dir_all(root);
}
