use std::cell::RefCell;
use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{OptionKind, OptionSpec, SettingHandler, SettingsBackend, ValueSpec};
use toml::Value;

use super::bridge::Bridge;
use super::menu_store::ValueStore;
use super::settings::{NITROUS_VARIANT, SettingsPort, change, chosen, info, nitrous_variant, visible};
use super::support::MemoryStore;

fn setup() -> (Rc<Bridge>, Rc<MemoryStore>, SettingsPort) {
    let store = Rc::new(MemoryStore::default());
    let bridge = Rc::new(Bridge::with_store(Box::new(store.clone())));
    (bridge.clone(), store, SettingsPort(bridge))
}

fn variant<'a>() -> OptionSpec<'a> {
    OptionSpec {
        id: NITROUS_VARIANT,
        kind: OptionKind::List,
        label: "Nitrous variant",
        values: vec![ValueSpec { id: "mw2012", label: "NFS Most Wanted 2012" }],
        default_on: false,
    }
}

fn recorder() -> (Rc<RefCell<Vec<String>>>, SettingHandler) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let inside = seen.clone();
    (seen, Box::new(move |value: &str| inside.borrow_mut().push(value.to_owned())))
}

#[test]
fn without_a_mod_the_nitrous_variant_is_the_original_and_is_not_listed() {
    let (bridge, _, _) = setup();
    assert!(visible(&bridge).is_empty(), "one value: the menu is as it was");
    let choice = nitrous_variant(&bridge);
    assert_eq!((choice.value.as_str(), choice.owner), ("original", None));
}

#[test]
fn a_mod_value_makes_the_option_appear_after_the_games_own_value() {
    let (bridge, _, port) = setup();
    port.add_option("nfsmw-2012-content", &variant(), None).unwrap();
    let shown = visible(&bridge);
    assert_eq!(shown.len(), 1);
    let option = info(&bridge, shown[0]).unwrap();
    assert_eq!((option.label.as_str(), option.text.as_str(), option.toggle), ("Nitrous variant", "Original", false));
    change(&bridge, shown[0], true);
    let option = info(&bridge, shown[0]).unwrap();
    assert_eq!(option.text, "NFS Most Wanted 2012");
    let choice = nitrous_variant(&bridge);
    assert_eq!((choice.value.as_str(), choice.owner.as_deref()), ("mw2012", Some("nfsmw-2012-content")));
}

#[test]
fn the_choice_is_saved_and_survives_a_restart_while_the_mod_returns() {
    let (bridge, store, port) = setup();
    port.add_option("m", &variant(), None).unwrap();
    let at = visible(&bridge)[0];
    change(&bridge, at, true);
    assert_eq!(store.saved_text("nfsmw", "option_nitrous_variant").as_deref(), Some("mw2012"));

    // A new run with the mod: the saved choice applies as soon as the mod registers its value.
    let bridge2 = Rc::new(Bridge::with_store(Box::new(store.clone())));
    let port2 = SettingsPort(bridge2.clone());
    assert_eq!(chosen(&bridge2, NITROUS_VARIANT).as_deref(), Some("original"), "before the mod registers");
    port2.add_option("m", &variant(), None).unwrap();
    assert_eq!(chosen(&bridge2, NITROUS_VARIANT).as_deref(), Some("mw2012"));
}

#[test]
fn a_saved_variant_of_a_mod_that_is_not_loaded_leaves_the_game_unchanged() {
    let (bridge, store, _) = setup();
    store.save("nfsmw", "option_nitrous_variant", Value::String("mw2012".into())).unwrap();
    let choice = nitrous_variant(&bridge);
    assert_eq!((choice.value.as_str(), choice.owner), ("original", None));
    assert!(visible(&bridge).is_empty());
}

#[test]
fn a_mod_that_unloads_takes_its_value_and_the_choice_falls_back() {
    let (bridge, _, port) = setup();
    let handle = port.add_option("m", &variant(), None).unwrap();
    change(&bridge, visible(&bridge)[0], true);
    assert_eq!(nitrous_variant(&bridge).value, "mw2012");
    port.remove_option("m", handle).unwrap();
    assert_eq!(nitrous_variant(&bridge).value, "original");
    assert!(visible(&bridge).is_empty());
    assert_eq!(port.remove_option("m", handle), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn the_mods_callback_runs_with_the_new_value() {
    let (bridge, _, port) = setup();
    let (seen, handler) = recorder();
    port.add_option("m", &variant(), Some(handler)).unwrap();
    let at = visible(&bridge)[0];
    change(&bridge, at, true);
    change(&bridge, at, true);
    assert_eq!(*seen.borrow(), ["mw2012", "original"]);
}

#[test]
fn a_clash_of_kinds_is_refused_and_a_setting_the_game_lacks_is_appended() {
    let (bridge, _, port) = setup();
    let toggle =
        OptionSpec { id: NITROUS_VARIANT, kind: OptionKind::Toggle, label: "x", values: vec![], default_on: false };
    assert_eq!(port.add_option("m", &toggle, None), Err(BB_ERR_EXISTS));
    assert!(visible(&bridge).is_empty());

    let new = OptionSpec {
        id: "flame_color",
        kind: OptionKind::List,
        label: "Flame colour",
        values: vec![ValueSpec { id: "blue", label: "Blue" }, ValueSpec { id: "red", label: "Red" }],
        default_on: false,
    };
    port.add_option("m", &new, None).unwrap();
    let at = visible(&bridge)[0];
    assert_eq!(info(&bridge, at).unwrap().label, "Flame colour");
    assert_eq!(port.get("m", "flame_color").unwrap(), "blue");
    assert_eq!(port.get("m", "nope").unwrap_err(), BB_ERR_NOT_FOUND);
}

#[test]
fn a_toggle_the_mod_adds_is_shown_and_flips() {
    let (bridge, _, port) = setup();
    let (seen, handler) = recorder();
    let toggle =
        OptionSpec { id: "extra_hud", kind: OptionKind::Toggle, label: "Extra HUD", values: vec![], default_on: true };
    port.add_option("m", &toggle, Some(handler)).unwrap();
    let at = visible(&bridge)[0];
    assert!(info(&bridge, at).unwrap().on);
    change(&bridge, at, true);
    assert!(!info(&bridge, at).unwrap().on);
    assert_eq!(*seen.borrow(), ["off"]);
}
