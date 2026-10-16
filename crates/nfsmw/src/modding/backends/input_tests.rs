use std::cell::Cell;
use std::rc::Rc;

use bevy_input::keyboard::KeyCode;
use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{HotkeyHandler, InputBackend};

use super::bridge::Bridge;
use super::input::InputPort;

/// A hotkey callback that counts its presses, and the count.
fn counter() -> (Rc<Cell<u32>>, HotkeyHandler) {
    let count = Rc::new(Cell::new(0));
    let inside = count.clone();
    (count, Box::new(move || inside.set(inside.get() + 1)))
}

fn port() -> (Rc<Bridge>, InputPort) {
    let bridge = Rc::new(Bridge::default());
    (bridge.clone(), InputPort(bridge))
}

#[test]
fn a_hotkey_fires_when_its_key_goes_down() {
    let (bridge, port) = port();
    let (count, on_press) = counter();
    let handle = port.register_hotkey("example.a", "bump", "Bump the day", "key:F5", on_press).unwrap();
    assert_ne!(handle, 0);
    bridge.press(|key| key == KeyCode::F6);
    assert_eq!(count.get(), 0, "another key does nothing");
    bridge.press(|key| key == KeyCode::F5);
    assert_eq!(count.get(), 1);
}

#[test]
fn a_hotkey_without_a_default_binding_never_fires() {
    let (bridge, port) = port();
    let (count, on_press) = counter();
    port.register_hotkey("example.a", "unbound", "Not bound yet", "", on_press).unwrap();
    bridge.press(|_| true);
    assert_eq!(count.get(), 0);
}

#[test]
fn a_binding_must_name_a_keyboard_key() {
    let (_, port) = port();
    let (_, on_press) = counter();
    assert_eq!(port.register_hotkey("example.a", "pad", "", "button:South", on_press), Err(BB_ERR_ARG));
    let (_, on_press) = counter();
    assert_eq!(port.register_hotkey("example.a", "nokey", "", "key:NotAKey", on_press), Err(BB_ERR_ARG));
    let (_, on_press) = counter();
    assert_eq!(port.register_hotkey("example.a", "", "", "key:F5", on_press), Err(BB_ERR_ARG));
}

#[test]
fn names_are_unique_per_mod() {
    let (_, port) = port();
    let (_, on_press) = counter();
    port.register_hotkey("example.a", "bump", "", "key:F5", on_press).unwrap();
    let (_, on_press) = counter();
    assert_eq!(port.register_hotkey("example.a", "bump", "", "key:F6", on_press), Err(BB_ERR_EXISTS));
    let (_, on_press) = counter();
    assert!(port.register_hotkey("example.b", "bump", "", "key:F6", on_press).is_ok());
}

#[test]
fn unregister_stops_the_hotkey_and_only_its_owner_can_remove_it() {
    let (bridge, port) = port();
    let (count, on_press) = counter();
    let handle = port.register_hotkey("example.a", "bump", "", "key:F5", on_press).unwrap();
    assert_eq!(port.unregister("example.b", handle), Err(BB_ERR_NOT_FOUND));
    port.unregister("example.a", handle).unwrap();
    bridge.press(|key| key == KeyCode::F5);
    assert_eq!(count.get(), 0);
    assert_eq!(port.unregister("example.a", handle), Err(BB_ERR_NOT_FOUND));
}

#[test]
fn a_callback_may_register_a_hotkey_while_presses_are_fired() {
    let (bridge, port) = port();
    let inner = bridge.clone();
    let on_press: HotkeyHandler = Box::new(move || {
        let (_, later) = counter();
        let _ = InputPort(inner.clone()).register_hotkey("example.a", "later", "", "key:F6", later);
    });
    port.register_hotkey("example.a", "first", "", "key:F5", on_press).unwrap();
    bridge.press(|key| key == KeyCode::F5);
    assert_eq!(bridge.hotkeys.borrow().len(), 2, "the table was free during the press");
}
