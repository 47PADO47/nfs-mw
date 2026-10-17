use std::cell::RefCell;
use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{MenuHandler, MenuKind, MenuRowSpec, MenusBackend};
use toml::Value;

use super::bridge::Bridge;
use super::menu_store::ValueStore;
use super::menus::{MenusPort, any_row, change, fit, rows};
use super::support::MemoryStore;

fn setup() -> (Rc<Bridge>, Rc<MemoryStore>, MenusPort) {
    let store = Rc::new(MemoryStore::default());
    let bridge = Rc::new(Bridge::with_store(Box::new(store.clone())));
    (bridge.clone(), store, MenusPort(bridge))
}

fn spec(kind: MenuKind, name: &str, default: f32) -> MenuRowSpec<'_> {
    MenuRowSpec { kind, name, label: "Row", default }
}

/// A callback that records the values it is called with.
fn recorder() -> (Rc<RefCell<Vec<f32>>>, MenuHandler) {
    let seen = Rc::new(RefCell::new(Vec::new()));
    let inside = seen.clone();
    (seen, Box::new(move |value| inside.borrow_mut().push(value)))
}

fn slider() -> MenuKind {
    MenuKind::Slider { min: 0.0, max: 10.0, step: 2.0 }
}

fn list() -> MenuKind {
    MenuKind::List { choices: vec!["low".into(), "mid".into(), "high".into()] }
}

#[test]
fn each_kind_keeps_its_value_in_range() {
    assert_eq!([fit(&MenuKind::Toggle, 0.7), fit(&MenuKind::Toggle, 0.2)], [1.0, 0.0]);
    assert_eq!([fit(&slider(), 5.1), fit(&slider(), -4.0), fit(&slider(), 42.0)], [6.0, 0.0, 10.0]);
    assert_eq!([fit(&list(), 1.6), fit(&list(), 9.0), fit(&list(), -1.0)], [2.0, 2.0, 0.0]);
    assert_eq!(fit(&MenuKind::Button, 5.0), 0.0);
}

#[test]
fn a_row_registers_with_its_default_or_the_value_saved_before() {
    let (_, store, port) = setup();
    let (_, on_change) = recorder();
    port.register("example.a", &spec(slider(), "speed", 4.0), on_change).unwrap();
    assert_eq!(rows(&port.0)[0].value, 4.0, "no saved value: the default");

    store.save("example.b", "speed", Value::Float(8.0)).unwrap();
    let (_, on_change) = recorder();
    port.register("example.b", &spec(slider(), "speed", 4.0), on_change).unwrap();
    assert_eq!(rows(&port.0)[1].value, 8.0, "a value saved by an earlier run wins");
    assert!(any_row(&port.0));
}

#[test]
fn a_name_must_be_plain_and_unique_per_mod() {
    let (_, _, port) = setup();
    let (_, on_change) = recorder();
    assert_eq!(
        port.register("example.a", &spec(MenuKind::Toggle, "two words", 0.0), on_change).map(|_| ()),
        Err(BB_ERR_ARG)
    );
    let (_, on_change) = recorder();
    port.register("example.a", &spec(MenuKind::Toggle, "fast", 0.0), on_change).unwrap();
    let (_, on_change) = recorder();
    assert_eq!(
        port.register("example.a", &spec(MenuKind::Toggle, "fast", 1.0), on_change).map(|_| ()),
        Err(BB_ERR_EXISTS)
    );
    let (_, on_change) = recorder();
    assert!(
        port.register("example.b", &spec(MenuKind::Toggle, "fast", 1.0), on_change).is_ok(),
        "another mod may use the name"
    );
}

#[test]
fn set_and_get_belong_to_the_mod_that_registered_the_row() {
    let (_, store, port) = setup();
    let (_, on_change) = recorder();
    let handle = port.register("example.a", &spec(slider(), "speed", 4.0), on_change).unwrap();
    port.set("example.a", handle, 6.0).unwrap();
    assert_eq!(port.get("example.a", handle), Ok(6.0));
    assert_eq!(port.get("example.b", handle), Err(BB_ERR_NOT_FOUND));
    assert_eq!(port.set("example.b", handle, 1.0), Err(BB_ERR_NOT_FOUND));
    assert_eq!(store.saved("example.a", "speed"), Some(6.0), "a set value is saved");
}

#[test]
fn a_toggle_flips_saves_and_calls_back() {
    let (bridge, store, port) = setup();
    let (seen, on_change) = recorder();
    let handle = port.register("example.a", &spec(MenuKind::Toggle, "fast", 0.0), on_change).unwrap();
    change(&bridge, handle, true);
    change(&bridge, handle, true);
    assert_eq!(*seen.borrow(), [1.0, 0.0]);
    assert_eq!(store.saved("example.a", "fast"), Some(0.0));
    assert_eq!(store.values.borrow().last().map(|(_, _, value)| value.clone()), Some(Value::Boolean(false)));
}

#[test]
fn a_slider_steps_and_stops_at_its_end_without_a_callback() {
    let (bridge, _, port) = setup();
    let (seen, on_change) = recorder();
    let handle = port.register("example.a", &spec(slider(), "speed", 8.0), on_change).unwrap();
    change(&bridge, handle, true);
    change(&bridge, handle, false);
    assert_eq!(*seen.borrow(), [10.0, 8.0]);
    port.set("example.a", handle, 10.0).unwrap();
    change(&bridge, handle, true);
    assert_eq!(seen.borrow().len(), 2, "at the top a forward step does nothing");
}

#[test]
fn a_list_wraps_through_its_choices() {
    let (bridge, _, port) = setup();
    let (seen, on_change) = recorder();
    let handle = port.register("example.a", &spec(list(), "pick", 0.0), on_change).unwrap();
    for forward in [true, true, true, false] {
        change(&bridge, handle, forward);
    }
    assert_eq!(*seen.borrow(), [1.0, 2.0, 0.0, 2.0]);
}

#[test]
fn a_button_calls_back_with_zero_and_saves_nothing() {
    let (bridge, store, port) = setup();
    let (seen, on_change) = recorder();
    let handle = port.register("example.a", &spec(MenuKind::Button, "run", 0.0), on_change).unwrap();
    change(&bridge, handle, true);
    assert_eq!(*seen.borrow(), [0.0]);
    assert_eq!(store.saved("example.a", "run"), None, "a button has no value to save");
}

#[test]
fn unregister_removes_only_the_owners_row() {
    let (_, _, port) = setup();
    let (_, on_change) = recorder();
    let handle = port.register("example.a", &spec(MenuKind::Toggle, "fast", 0.0), on_change).unwrap();
    assert_eq!(port.unregister("example.b", handle), Err(BB_ERR_NOT_FOUND));
    port.unregister("example.a", handle).unwrap();
    assert!(!any_row(&port.0), "no row left: the Mods page is not shown");
    assert_eq!(port.unregister("example.a", handle), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn the_page_lists_each_row_with_its_label_and_value() {
    let (_, _, port) = setup();
    let (_, on_change) = recorder();
    port.register("example.a", &spec(list(), "pick", 1.0), on_change).unwrap();
    let page = rows(&port.0);
    assert_eq!(page.len(), 1);
    assert_eq!((page[0].label.as_str(), page[0].value), ("Row", 1.0));
    assert_eq!(page[0].kind, list());
}
