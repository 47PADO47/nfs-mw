//! The grading variant from the player's choice to the look the scene is handed, over an install folder with authored files.

use std::rc::Rc;

use blackbox_modding::content::encode_lut;
use blackbox_modding::host::{OptionKind, OptionSpec, SettingsBackend, ValueSpec};

use super::super::settings::{SettingsPort, change, visible, visible_index};
use super::super::support::MemoryStore;
use super::super::variants::GRADING_KIND;
use super::*;

fn setup() -> (Rc<Bridge>, SettingsPort) {
    let bridge = Rc::new(Bridge::with_store(Box::new(Rc::new(MemoryStore::default()))));
    (bridge.clone(), SettingsPort(bridge))
}

fn spec(value: &str) -> OptionSpec<'_> {
    OptionSpec {
        id: GRADING_KIND.variant.option,
        kind: OptionKind::List,
        label: "Color grading",
        values: vec![ValueSpec { id: value, label: "A mod's look" }],
        default_on: false,
        section: "",
    }
}

const PROFILE: &str = "format = 1
[[cube]]
name = \"a\"
file = \"GRADING/MYMOD/A.LUT\"
[[set]]
weather = \"clear\"
[[set.key]]
hour = 12.0
cube = \"a\"
";

/// A folder with the grading variant `MYMOD`: the profile and, when `cube`, its one cube (an identity of 2).
fn install(name: &str, cube: bool) -> (std::path::PathBuf, game_install::GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-grading-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let folder = root.join("GRADING").join("MYMOD");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(folder.join("profile.toml"), PROFILE).unwrap();
    if cube {
        let identity = blackbox_gfx::ColourCube::identity(2).unwrap();
        std::fs::write(folder.join("A.LUT"), encode_lut(2, identity.rgb())).unwrap();
    }
    (root.clone(), game_install::GameDir::open(&root).unwrap())
}

#[test]
fn the_grading_kind_is_an_option_with_its_own_folder_and_off_as_the_original() {
    assert_eq!(GRADING_KIND.variant.option, "grading_variant");
    assert_eq!(GRADING_KIND.variant.folder, "GRADING");
    assert!(super::super::variants::KINDS.iter().any(|kind| kind.variant.option == "grading_variant"));
}

#[test]
fn the_chosen_look_is_read_once_and_the_original_is_none() {
    let (bridge, port) = setup();
    let (root, dir) = install("choice", true);
    bridge.set_install(dir);
    assert!(bridge.grading_look().is_none(), "no mod: the picture is the game's");
    assert!(visible_index(&bridge, "grading_variant").is_none(), "one value is not shown");
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    assert!(bridge.grading_look().is_none(), "the original is still chosen");
    let at = visible_index(&bridge, "grading_variant").expect("two values show the row");
    assert!(visible(&bridge).contains(&at));
    change(&bridge, at, true);
    let look = bridge.grading_look().expect("the mod's look");
    assert_eq!(look.cubes.len(), 1);
    assert!(look.cubes[0].is_identity(0.0));
    assert!(Arc::ptr_eq(&look, &bridge.grading_look().unwrap()), "read once");
    assert_eq!(bridge.effect_variants().grading.map(|g| Arc::ptr_eq(&g, &look)), Some(true));
    change(&bridge, at, true);
    assert!(bridge.grading_look().is_none(), "back to the original");
    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn a_look_whose_cube_is_missing_is_no_look() {
    let (bridge, port) = setup();
    let (root, dir) = install("missing", false);
    bridge.set_install(dir);
    port.add_option("mymod", &spec("mymod"), None).unwrap();
    change(&bridge, visible_index(&bridge, "grading_variant").unwrap(), true);
    assert!(bridge.grading_look().is_none());
    assert!(bridge.grading_look().is_none(), "and it is not read again every frame");
    let _ = std::fs::remove_dir_all(root);
}
