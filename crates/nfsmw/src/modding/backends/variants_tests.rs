use std::rc::Rc;

use blackbox_modding::host::{OptionKind, OptionSpec, SettingsBackend, ValueSpec};
use toml::Value;

use super::super::menu_store::ValueStore;
use super::super::settings::{SettingsPort, change, visible};
use super::super::support::MemoryStore;
use super::*;

fn setup() -> (Rc<Bridge>, Rc<MemoryStore>, SettingsPort) {
    let store = Rc::new(MemoryStore::default());
    let bridge = Rc::new(Bridge::with_store(Box::new(store.clone())));
    (bridge.clone(), store, SettingsPort(bridge))
}

fn spec<'a>(kind: &EffectKind, value: &'a str) -> OptionSpec<'a> {
    OptionSpec {
        id: kind.variant.option,
        kind: OptionKind::List,
        label: "x",
        values: vec![ValueSpec { id: value, label: "A mod's" }],
        default_on: false,
    }
}

/// A folder with `TIRESMOKE/MYMOD/profile.toml`, opened as the install.
fn install_with(name: &str, text: &str) -> (std::path::PathBuf, game_install::GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-variants-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join("TIRESMOKE/MYMOD")).unwrap();
    std::fs::write(root.join("TIRESMOKE/MYMOD/profile.toml"), text).unwrap();
    let dir = game_install::GameDir::open(&root).unwrap();
    (root, dir)
}

const PROFILE: &str = "format = 1\n[[layer]]\ntexture = \"TIRESMOKE/MYMOD/A.PNG\"\nrate = 5.0\n";

#[test]
fn every_kind_has_its_own_option_and_folder_and_texts() {
    for (at, kind) in KINDS.iter().enumerate() {
        assert!(kind.variant.option.ends_with("_variant"), "{}", kind.name);
        assert_eq!(kind.label, format!("options-{}", kind.variant.option.replace('_', "-")));
        assert_eq!(kind.original, format!("{}-original", kind.label));
        for other in &KINDS[at + 1..] {
            assert_ne!(kind.variant.option, other.variant.option);
            assert_ne!(kind.variant.folder, other.variant.folder);
        }
    }
}

#[test]
fn without_a_mod_every_kind_is_the_original_and_no_option_is_listed() {
    let (bridge, _, _) = setup();
    assert!(visible(&bridge).is_empty(), "one value each: the menu is as it was");
    for kind in &KINDS {
        let chosen = choice(&bridge, kind);
        assert_eq!((chosen.value.as_str(), chosen.owner), ("original", None), "{}", kind.name);
    }
    assert!(bridge.tire_smoke_profile().is_none() && bridge.nitrous_profile().is_none());
}

#[test]
fn a_value_of_one_kind_lists_only_that_kinds_option_and_leaves_the_other_alone() {
    let (bridge, _, port) = setup();
    port.add_option("mymod", &spec(&TIRE_SMOKE_KIND, "mymod"), None).unwrap();
    assert_eq!(visible(&bridge).len(), 1);
    change(&bridge, visible(&bridge)[0], true);
    let smoke = choice(&bridge, &TIRE_SMOKE_KIND);
    assert_eq!((smoke.value.as_str(), smoke.owner.as_deref()), ("mymod", Some("mymod")));
    let nitrous = choice(&bridge, &NITROUS_KIND);
    assert_eq!((nitrous.value.as_str(), nitrous.owner), ("original", None), "the nitrous is untouched");
}

#[test]
fn the_choice_of_each_kind_is_saved_under_its_own_key() {
    let (bridge, store, port) = setup();
    port.add_option("mymod", &spec(&TIRE_SMOKE_KIND, "mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert_eq!(store.saved_text("nfsmw", "option_tire_smoke_variant").as_deref(), Some("mymod"));
    assert_eq!(store.saved_text("nfsmw", "option_nitrous_variant"), None);
    // A saved value of a mod that is not loaded reads as the original.
    store.save("nfsmw", "option_nitrous_variant", Value::String("gone".into())).unwrap();
    let fresh = Rc::new(Bridge::with_store(Box::new(store.clone())));
    assert_eq!(choice(&fresh, &NITROUS_KIND).value, "original");
    assert_eq!(choice(&fresh, &TIRE_SMOKE_KIND).value, "original", "the mod is not loaded yet");
}

#[test]
fn the_profile_of_the_chosen_value_is_read_once_through_the_install() {
    let (bridge, _, port) = setup();
    let (root, dir) = install_with("once", PROFILE);
    bridge.set_install(dir);
    assert!(bridge.tire_smoke_profile().is_none(), "the original: nothing is read");
    port.add_option("mymod", &spec(&TIRE_SMOKE_KIND, "mymod"), None).unwrap();
    assert!(bridge.tire_smoke_profile().is_none(), "the value is not chosen");
    change(&bridge, visible(&bridge)[0], true);
    let profile = bridge.tire_smoke_profile().expect("the mod's profile");
    assert_eq!((profile.layers.len(), profile.layers[0].rate), (1, 5.0));
    // Read once: the file can go and the profile stays, and it is the same read.
    std::fs::remove_dir_all(&root).unwrap();
    let again = bridge.tire_smoke_profile().expect("kept");
    assert!(Arc::ptr_eq(&profile, &again));
    assert!(bridge.effect_variants().tire_smoke.is_some());
}

#[test]
fn a_missing_or_refused_profile_is_none_and_the_games_effect_plays() {
    let (bridge, _, port) = setup();
    let (root, dir) = install_with("refused", "format = 9\n");
    bridge.set_install(dir);
    port.add_option("mymod", &spec(&TIRE_SMOKE_KIND, "mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert!(bridge.tire_smoke_profile().is_none(), "format 9 is refused");
    assert!(bridge.effect_variants().tire_smoke.is_none());
    let _ = std::fs::remove_dir_all(&root);

    let (bridge, _, port) = setup();
    let (root, dir) = install_with("missing", PROFILE);
    bridge.set_install(dir);
    port.add_option("other", &spec(&TIRE_SMOKE_KIND, "nothing"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert!(bridge.tire_smoke_profile().is_none(), "no TIRESMOKE/NOTHING/profile.toml");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_profile_that_was_not_there_is_looked_for_again_after_a_while() {
    let (bridge, _, port) = setup();
    let (root, dir) = install_with("late", PROFILE);
    bridge.set_install(dir.clone());
    port.add_option("late", &spec(&TIRE_SMOKE_KIND, "late"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert!(bridge.tire_smoke_profile().is_none());
    // The mod finishes converting: its files join the install's layers.
    let layer = std::env::temp_dir().join(format!("nfsmw-variants-layer-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&layer);
    std::fs::create_dir_all(layer.join("TIRESMOKE/LATE")).unwrap();
    std::fs::write(layer.join("TIRESMOKE/LATE/profile.toml"), PROFILE).unwrap();
    dir.add_late_overlay("late", &layer, 10).unwrap();
    let found = (0..PROFILE_RETRY_CALLS + 2).find_map(|_| bridge.tire_smoke_profile());
    assert!(found.is_some(), "found within {PROFILE_RETRY_CALLS} calls");
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&layer);
}

/// A folder with the profile of a spark kind at `<folder>/MYMOD/profile.toml`, opened as the install.
fn install_with_kind(name: &str, kind: &EffectKind, text: &str) -> (std::path::PathBuf, game_install::GameDir) {
    let root = std::env::temp_dir().join(format!("nfsmw-variants-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let dir = root.join(kind.variant.folder).join("MYMOD");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("profile.toml"), text).unwrap();
    (root.clone(), game_install::GameDir::open(&root).unwrap())
}

const SPARKS_PROFILE: &str = "format = 1\n[hit]\nclosing_speed_full = 20.0\nintensity_floor = 0.2\n[[hit.layer]]\nkind = \"streak\"\nburst = 30.0\n";
const XENON_PROFILE: &str = "format = 1\n[scrape]\nspeed_start = 1.0\nspeed_full = 2.0\n[[scrape.spark]]\ncolor = [1.0, 0.5, 0.2, 1.0]\nlife = 0.4\ncount = 50.0\n";

#[test]
fn the_spark_kinds_are_options_with_their_own_folders_and_texts() {
    assert_eq!(COLLISION_SPARKS_KIND.variant.option, "collision_spark_variant");
    assert_eq!(XENON_SPARKS_KIND.variant.option, "xenon_spark_variant");
    assert_eq!((COLLISION_SPARKS_KIND.variant.folder, XENON_SPARKS_KIND.variant.folder), ("SPARKS", "XENON"));
    assert!(KINDS.iter().any(|kind| kind.variant.option == "collision_spark_variant"));
    assert!(KINDS.iter().any(|kind| kind.variant.option == "xenon_spark_variant"));
}

#[test]
fn the_collision_spark_profile_is_read_once_for_the_chosen_value() {
    let (bridge, store, port) = setup();
    let (root, dir) = install_with_kind("sparks", &COLLISION_SPARKS_KIND, SPARKS_PROFILE);
    bridge.set_install(dir);
    assert!(bridge.collision_sparks_profile().is_none(), "the original: nothing is read");
    port.add_option("mymod", &spec(&COLLISION_SPARKS_KIND, "mymod"), None).unwrap();
    assert_eq!(visible(&bridge).len(), 1, "the row shows once a mod adds a value");
    change(&bridge, visible(&bridge)[0], true);
    assert_eq!(store.saved_text("nfsmw", "option_collision_spark_variant").as_deref(), Some("mymod"));
    let profile = bridge.collision_sparks_profile().expect("the mod's profile");
    assert_eq!((profile.hit.items.len(), profile.hit.items[0].count), (1, 30.0));
    assert!(profile.scrape.items.is_empty(), "a channel with no layer keeps the install's sparks");
    assert!(Arc::ptr_eq(&profile, &bridge.effect_variants().collision_sparks.unwrap()));
    assert!(bridge.effect_variants().xenon_sparks.is_none(), "the other spark kind is untouched");
    assert!(bridge.xenon_sparks_profile().is_none());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_restored_spark_profile_is_read_through_its_own_folder_and_a_refused_one_is_none() {
    let (bridge, _, port) = setup();
    let (root, dir) = install_with_kind("xenon", &XENON_SPARKS_KIND, XENON_PROFILE);
    bridge.set_install(dir);
    port.add_option("mymod", &spec(&XENON_SPARKS_KIND, "mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    let profile = bridge.xenon_sparks_profile().expect("the mod's profile");
    assert_eq!((profile.scrape.items.len(), profile.scrape.items[0].count), (1, 50.0));
    assert!(bridge.collision_sparks_profile().is_none(), "nothing under SPARKS/MYMOD");
    let _ = std::fs::remove_dir_all(&root);

    let (bridge, _, port) = setup();
    let (root, dir) = install_with_kind("xenon-bad", &XENON_SPARKS_KIND, "format = 1\n[trail]\nspeed = 5.0\n");
    bridge.set_install(dir);
    port.add_option("mymod", &spec(&XENON_SPARKS_KIND, "mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert!(bridge.xenon_sparks_profile().is_none(), "a trail table is not in the schema: the game's sparks play");
    assert!(bridge.effect_variants().xenon_sparks.is_none());
    let _ = std::fs::remove_dir_all(&root);
}

const WIND_PROFILE: &str = "format = 1
[[layer]]
file = \"WIND/MYMOD/W1.WAV\"
rate = 1.3
gain = [[0.0, 0.0], [60.0, 1.0]]
";

#[test]
fn the_wind_profile_is_read_through_its_own_folder_and_a_refused_one_is_none() {
    assert_eq!(WIND_KIND.variant.option, "wind_variant");
    assert_eq!(WIND_KIND.variant.folder, "WIND");
    assert!(KINDS.iter().any(|kind| kind.variant.option == "wind_variant"));
    let (bridge, store, port) = setup();
    let (root, dir) = install_with_kind("wind", &WIND_KIND, WIND_PROFILE);
    bridge.set_install(dir);
    assert!(bridge.wind_profile().is_none(), "the original: nothing is read");
    port.add_option("mymod", &spec(&WIND_KIND, "mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert_eq!(store.saved_text("nfsmw", "option_wind_variant").as_deref(), Some("mymod"));
    let profile = bridge.wind_profile().expect("the mod's profile");
    assert_eq!((profile.layers.len(), profile.layers[0].rate), (1, 1.3));
    assert!(Arc::ptr_eq(&profile, &bridge.effect_variants().wind.unwrap()));
    assert!(bridge.collision_sparks_profile().is_none(), "the other kinds are untouched");
    let _ = std::fs::remove_dir_all(&root);

    let (bridge, _, port) = setup();
    let (root, dir) = install_with_kind(
        "wind-bad",
        &WIND_KIND,
        "format = 1
[[layer]]
file = \"A.MP3\"
gain = [[0.0, 1.0], [1.0, 1.0]]
",
    );
    bridge.set_install(dir);
    port.add_option("mymod", &spec(&WIND_KIND, "mymod"), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert!(bridge.wind_profile().is_none(), "a file that is not a WAV is refused: the game's wind plays");
    assert!(bridge.effect_variants().wind.is_none());
    let _ = std::fs::remove_dir_all(&root);
}
