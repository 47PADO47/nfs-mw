//! `bb.input`: hotkeys a mod registers. A hotkey's callback runs in the frame's `ModsPre` step, on the frame its key goes
//! down. Only keyboard keys are hotkeys, named the way the `bind` command names them (`key:F5`).

use std::rc::Rc;

use bevy_input::keyboard::KeyCode;
use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_NOT_FOUND};
use blackbox_modding::host::{BackendResult, HotkeyHandler, InputBackend};

use super::bridge::Bridge;
use super::is_plain_name;
use crate::input::Bindings;

/// The `bb.input` port of the game: one per mod table, sharing the bridge.
pub struct InputPort(pub(super) Rc<Bridge>);

/// A hotkey a mod registered. `key` is `None` when the mod gave no default binding: the hotkey exists but nothing presses it.
pub struct Hotkey {
    pub handle: u64,
    pub owner: String,
    pub name: String,
    pub key: Option<KeyCode>,
    pub handler: Rc<HotkeyHandler>,
}

impl InputBackend for InputPort {
    fn register_hotkey(
        &self,
        owner: &str,
        name: &str,
        label: &str,
        default_binding: &str,
        on_press: HotkeyHandler,
    ) -> BackendResult<u64> {
        if !is_plain_name(name) {
            return Err(BB_ERR_ARG);
        }
        let key = match default_binding {
            "" => None,
            text => Some(Bindings::hotkey_key(text).map_err(|_| BB_ERR_ARG)?),
        };
        let mut hotkeys = self.0.hotkeys.borrow_mut();
        if hotkeys.iter().any(|hotkey| hotkey.owner == owner && hotkey.name == name) {
            return Err(BB_ERR_EXISTS);
        }
        let handle = self.0.next_handle();
        log::info!("mod {owner}: hotkey {name:?} ({label}) on {default_binding:?}");
        hotkeys.push(Hotkey {
            handle,
            owner: owner.to_owned(),
            name: name.to_owned(),
            key,
            handler: Rc::new(on_press),
        });
        Ok(handle)
    }

    fn unregister(&self, owner: &str, handle: u64) -> BackendResult<()> {
        let mut hotkeys = self.0.hotkeys.borrow_mut();
        let before = hotkeys.len();
        hotkeys.retain(|hotkey| !(hotkey.handle == handle && hotkey.owner == owner));
        if hotkeys.len() == before {
            return Err(BB_ERR_NOT_FOUND);
        }
        Ok(())
    }
}
