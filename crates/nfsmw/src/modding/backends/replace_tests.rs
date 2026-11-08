//! The replacement sets from the player's choice to the replacements the world applies, over an install folder with authored files.

use std::rc::Rc;

use blackbox_hash::bstring_hash;
use blackbox_modding::host::{OptionKind, OptionSpec, SettingsBackend, ValueSpec};
use blackbox_solid::{GroupSource, SolidSource, Vertex, write_geometry_pack};
use blackbox_tpk::{PixelFormat, TextureSource, write_tpk};

use super::super::settings::{self, SettingsPort, change, visible};
use super::super::support::MemoryStore;
use super::super::variants::TREES_KIND;
use super::*;
use crate::modding::content::ModDir;
use crate::modding::content::replace::{self, TEST_LOCK, active};

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
fn the_old_trees_profile_becomes_the_replacements_in_effect_and_the_original_takes_it_away() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (bridge, port) = setup();
    let (root, dir) = install("choice", PROFILE, true);
    bridge.set_install(dir);
    replace::set_active(None);
    bridge.publish_replacements();
    assert!(active().is_none(), "the original: nothing replaced");
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    assert!(visible(&bridge).len() == 1, "the row shows once a mod adds a value");
    bridge.publish_replacements();
    assert!(active().is_none(), "the value is not chosen");
    change(&bridge, visible(&bridge)[0], true);
    bridge.publish_replacements();
    assert_eq!(active().expect("the replacements").swaps(), 1);
    // The same choice is not read again: the pack in effect is the one that was made.
    let first = active().unwrap();
    bridge.publish_replacements();
    assert!(Arc::ptr_eq(&first, &active().unwrap()));
    change(&bridge, visible(&bridge)[0], false);
    bridge.publish_replacements();
    assert!(active().is_none(), "back to the original");
    replace::set_active(None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_variant_whose_packs_are_missing_or_refused_leaves_the_games_trees() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (bridge, port) = setup();
    let (root, dir) = install("no-pack", PROFILE, false);
    bridge.set_install(dir);
    replace::set_active(None);
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    bridge.publish_replacements();
    assert!(active().is_none(), "the pack files are not there");
    let _ = std::fs::remove_dir_all(&root);

    let (bridge, port) = setup();
    let (root, dir) = install("bad-profile", "format = 1\ngeometry = \"G.PNG\"\n", true);
    bridge.set_install(dir);
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    bridge.publish_replacements();
    assert!(bridge.trees_profile().is_none(), "a profile the schema refuses");
    assert!(active().is_none());
    let _ = std::fs::remove_dir_all(&root);
}

/// A mod folder that is also the install: manifests in `replace/`, files in `SETS/`, a lamp model and a picture.
fn mod_folder(name: &str, manifests: &[(&str, &str)]) -> (std::path::PathBuf, game_install::GameDir, ModDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-replace-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("SETS")).unwrap();
    std::fs::create_dir_all(root.join("replace")).unwrap();
    for (file, text) in manifests {
        std::fs::write(root.join("replace").join(file), text).unwrap();
    }
    let vertex =
        |position: [f32; 3]| Vertex { position, normal: [0.0, 0.0, 1.0], color_bgra: [128; 4], uv: [0.0, 0.0] };
    let post = [[-0.1, -0.1, 0.0], [0.1, -0.1, 0.0], [0.1, -0.1, 5.0], [-0.1, -0.1, 5.0]];
    let group = GroupSource::new(post.iter().map(|corner| vertex(*corner)).collect(), vec![0, 1, 2, 0, 2, 3]);
    let mut model = SolidSource::new("LAMP_NEW", vec![group]);
    model.texture_hashes = vec![bstring_hash("LAMP_TEX")];
    std::fs::write(root.join("SETS/GEOMETRY.BIN"), write_geometry_pack(&[model]).unwrap()).unwrap();
    let pack = [TextureSource::new("LAMP_TEX", 4, 4, PixelFormat::Dxt1, 1, vec![0; 8])];
    std::fs::write(root.join("SETS/TEXTURES.BIN"), write_tpk("SETS", &pack).unwrap()).unwrap();
    let dir = game_install::GameDir::open(&root).unwrap();
    let mod_dir = ModDir { id: "lamps".to_owned(), dir: root.clone(), priority: 0 };
    (root, dir, mod_dir)
}

const LAMP_SET: &str = "format = 1
[set]
kind = \"street_lamps\"
kind_label = \"Street lamps\"
value = \"slim\"
value_label = \"Slim lamps\"
fit = \"stand\"
[files]
geometry = \"SETS/GEOMETRY.BIN\"
textures = \"SETS/TEXTURES.BIN\"
[[model]]
target = \"XT_LAMP\"
model = \"LAMP_NEW\"
";

#[test]
fn a_declared_set_adds_a_value_to_its_kinds_option_and_the_choice_replaces_the_model() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (bridge, _port) = setup();
    let (root, dir, mod_dir) = mod_folder("declared", &[("lamp.toml", LAMP_SET)]);
    bridge.set_install(dir);
    replace::set_active(None);
    assert!(visible(&bridge).is_empty(), "no option before a mod declares one");
    bridge.declare_replacements(std::slice::from_ref(&mod_dir));
    let shown = visible(&bridge);
    assert_eq!(shown.len(), 1, "the kind's option shows: original and the mod's value");
    let info = settings::info(&bridge, shown[0]).unwrap();
    assert_eq!((info.label.as_str(), info.text.as_str()), ("Street lamps", "Original"));
    bridge.publish_replacements();
    assert!(active().is_none(), "the original is chosen");
    change(&bridge, shown[0], true);
    assert_eq!(settings::chosen(&bridge, "street_lamps_variant").as_deref(), Some("slim"));
    bridge.publish_replacements();
    let now = active().expect("the chosen set");
    assert_eq!((now.swaps(), now.sets()), (1, &["lamps:lamp".to_owned()][..]));
    assert_eq!(now.textures().len(), 1, "the set's pack texture is added once");
    assert!(bridge.replacement_report("lamps").contains("lamp: chosen"));
    change(&bridge, shown[0], false);
    bridge.publish_replacements();
    assert!(active().is_none(), "back to the original");
    replace::set_active(None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn an_always_set_applies_with_no_option_and_a_refused_manifest_is_reported() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (bridge, _port) = setup();
    let always =
        LAMP_SET.replace("kind = \"street_lamps\"", "always = true").replace("kind_label = \"Street lamps\"\n", "");
    let (root, dir, mod_dir) =
        mod_folder("always", &[("fix.toml", &always), ("bad.toml", "format = 1\n[set]\nkind = 1\n")]);
    bridge.set_install(dir);
    replace::set_active(None);
    bridge.declare_replacements(std::slice::from_ref(&mod_dir));
    assert!(visible(&bridge).is_empty(), "an always set has no row");
    bridge.publish_replacements();
    assert_eq!(active().expect("applies at once").swaps(), 1);
    let report = bridge.replacement_report("lamps");
    assert!(report.contains("bad.toml: refused"), "{report}");
    assert!(report.contains("fix: chosen"), "{report}");
    replace::set_active(None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_declared_value_of_the_trees_kind_joins_the_games_own_option_and_two_mods_share_a_kind() {
    let (bridge, _port) = setup();
    let trees = LAMP_SET.replace("street_lamps", "trees").replace("Street lamps", "Trees");
    let (root, dir, mod_dir) = mod_folder("trees", &[("t.toml", &trees)]);
    let other = ModDir { id: "other".to_owned(), dir: root.join("other"), priority: 0 };
    std::fs::create_dir_all(other.dir.join("replace")).unwrap();
    std::fs::write(other.dir.join("replace/u.toml"), trees.replace("slim", "wide")).unwrap();
    bridge.set_install(dir);
    bridge.declare_replacements(&[mod_dir, other]);
    let model = bridge.options.borrow();
    let option = model.get("trees_variant").unwrap();
    let ids: Vec<&str> = option.values.iter().map(|value| value.id.as_str()).collect();
    assert_eq!(ids, ["original", "slim", "wide"], "the game's own value stays first; each mod adds its own");
    assert_eq!(model.value_owner("trees_variant", "wide"), Some("other"));
    drop(model);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_set_whose_files_are_missing_leaves_the_games_own_and_says_why() {
    let _lock = TEST_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let (bridge, _port) = setup();
    let (root, dir, mod_dir) = mod_folder("missing", &[("lamp.toml", LAMP_SET)]);
    std::fs::remove_file(root.join("SETS/GEOMETRY.BIN")).unwrap();
    bridge.set_install(dir);
    replace::set_active(None);
    bridge.declare_replacements(std::slice::from_ref(&mod_dir));
    change(&bridge, visible(&bridge)[0], true);
    bridge.publish_replacements();
    assert!(active().is_none());
    assert!(bridge.replacement_report("lamps").contains("not read"), "{}", bridge.replacement_report("lamps"));
    let _ = std::fs::remove_dir_all(&root);
}
