//! The trees variant from the player's choice to the pack the world swaps in, over an install folder with authored files.

use std::rc::Rc;

use blackbox_hash::bstring_hash;
use blackbox_modding::host::{OptionKind, OptionSpec, SettingsBackend, ValueSpec};
use blackbox_solid::{GroupSource, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::{PixelFormat, TextureSource, write_tpk};

use super::super::settings::{SettingsPort, change, visible};
use super::super::support::MemoryStore;
use super::super::variants::TREES_KIND;
use super::*;
use crate::modding::content::model_swap::{TEST_LOCK, active};

fn setup() -> (Rc<Bridge>, SettingsPort) {
    let bridge = Rc::new(Bridge::with_store(Box::new(Rc::new(MemoryStore::default()))));
    (bridge.clone(), SettingsPort(bridge))
}

fn spec(value: &str) -> OptionSpec<'_> {
    OptionSpec {
        id: TREES_KIND.variant.option,
        kind: OptionKind::List,
        label: "Trees variant",
        values: vec![ValueSpec { id: value, label: "A mod's trees" }],
        default_on: false,
    }
}

/// A folder with a trees variant `MYMOD` in it: a profile, a pack with one model and a pack with its texture.
fn install(name: &str, profile: &str, pack: bool) -> (std::path::PathBuf, game_install::GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-trees-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let folder = root.join("TREES").join("MYMOD");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("profile.toml"), profile).unwrap();
    if pack {
        let corners = [[-3.0, 0.0, 3.0], [3.0, 0.0, 3.0], [3.0, 0.0, 10.0], [-3.0, 0.0, 10.0]];
        let vertex =
            |position: [f32; 3]| Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv: [0.0, 0.0] };
        let group = GroupSource {
            effect_id: 6,
            flags: 0x4008,
            ..GroupSource::new(corners.iter().map(|corner| vertex(*corner)).collect(), vec![0, 1, 2, 0, 2, 3])
        };
        let mut model = SolidSource::new("MYTREE_L0", vec![group]);
        model.texture_hashes = vec![bstring_hash("MYLEAF")];
        std::fs::write(folder.join("GEOMETRY.BIN"), write_geometry_pack(&[model]).unwrap()).unwrap();
        let leaf = TextureSource::new("MYLEAF", 4, 4, PixelFormat::Dxt1, 1, vec![0; 8]);
        std::fs::write(folder.join("TEXTURES.BIN"), write_tpk("MYMOD", &[leaf]).unwrap()).unwrap();
    }
    (root.clone(), game_install::GameDir::open(&root).unwrap())
}

const PROFILE: &str = "format = 1
geometry = \"TREES/MYMOD/GEOMETRY.BIN\"
textures = \"TREES/MYMOD/TEXTURES.BIN\"
[[swap]]
solid = \"XT_THE_STOCK_TREE\"
model = \"MYTREE_L0\"
";

#[test]
fn the_trees_kind_is_an_option_with_its_own_folder_and_texts() {
    assert_eq!(TREES_KIND.variant.option, "trees_variant");
    assert_eq!(TREES_KIND.variant.folder, "TREES");
    assert!(super::super::variants::KINDS.iter().any(|kind| kind.variant.option == "trees_variant"));
}

#[test]
fn the_chosen_variant_becomes_the_pack_in_effect_and_the_original_takes_it_away() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (bridge, port) = setup();
    let (root, dir) = install("choice", PROFILE, true);
    bridge.set_install(dir);
    model_swap::set_active(None);
    bridge.publish_trees();
    assert!(active().is_none(), "the original: no pack");
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    assert!(visible(&bridge).len() == 1, "the row shows once a mod adds a value");
    bridge.publish_trees();
    assert!(active().is_none(), "the value is not chosen");
    change(&bridge, visible(&bridge)[0], true);
    bridge.publish_trees();
    assert_eq!(active().expect("the mod's pack").swaps(), 1);
    // The same choice is not read again: the pack in effect is the one that was made.
    let first = active().unwrap();
    bridge.publish_trees();
    assert!(Arc::ptr_eq(&first, &active().unwrap()));
    change(&bridge, visible(&bridge)[0], false);
    bridge.publish_trees();
    assert!(active().is_none(), "back to the original");
    model_swap::set_active(None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_variant_whose_packs_are_missing_or_refused_leaves_the_games_trees() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (bridge, port) = setup();
    let (root, dir) = install("no-pack", PROFILE, false);
    bridge.set_install(dir);
    model_swap::set_active(None);
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    bridge.publish_trees();
    assert!(active().is_none(), "the pack files are not there");
    let _ = std::fs::remove_dir_all(&root);

    let (bridge, port) = setup();
    let (root, dir) = install("bad-profile", "format = 1\ngeometry = \"G.PNG\"\n", true);
    bridge.set_install(dir);
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    bridge.publish_trees();
    assert!(bridge.trees_profile().is_none(), "a profile the schema refuses");
    assert!(active().is_none());
    let _ = std::fs::remove_dir_all(&root);
}
