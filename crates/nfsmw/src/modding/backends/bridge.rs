//! The per-frame context the mods' calls read and write.
//!
//! The ports in this folder read it and queue their writes in it, all on the main thread, inside a mod callback. The frame
//! systems in `plugin.rs` do the rest: [`Bridge::sync`] copies the scene's clock in before the mods run, [`Bridge::press`]
//! fires the hotkeys that were pressed, and [`Bridge::apply`] makes the scene and the console take what the mods asked
//! for. Nothing here is a global: the bridge belongs to the `Mods` resource, and the ports hold an `Rc` to it.

use std::cell::{Cell, RefCell};
use std::rc::Rc;

use bevy_input::keyboard::KeyCode;
use blackbox_modding::host::HotkeyHandler;

use super::commands::ModCommand;
use super::input::Hotkey;
use super::map::{self, ModBlip};
use super::time::{self, Clock, ClockRequest};
use crate::devtools::Console;
use crate::viewer::Scene;

#[derive(Default)]
pub struct Bridge {
    /// The last handle given out. Handles start at 1; 0 means none.
    last_handle: Cell<u64>,
    /// Console commands the mods registered, in registration order.
    pub(super) commands: RefCell<Vec<ModCommand>>,
    /// Console lines the mods asked to run (`bb.commands.run`), handed to the console by [`Bridge::apply`].
    pub(super) queued_lines: RefCell<Vec<String>>,
    /// Hotkeys the mods registered.
    pub(super) hotkeys: RefCell<Vec<Hotkey>>,
    /// The day clock as the mods see it this frame, or `None` when the scene has no clock (the menus).
    pub(super) clock: Cell<Option<Clock>>,
    /// Clock changes the mods asked for, applied to the scene in order by [`Bridge::apply`].
    pub(super) clock_requests: RefCell<Vec<ClockRequest>>,
    /// The minimap blips the mods placed.
    pub(super) blips: RefCell<Vec<ModBlip>>,
}

impl Bridge {
    /// A handle nobody has been given yet.
    pub(super) fn next_handle(&self) -> u64 {
        let handle = self.last_handle.get() + 1;
        self.last_handle.set(handle);
        handle
    }

    /// Copy the scene's clock in, so the mods read this frame's time. Called before the mods run.
    pub fn sync(&self, scene: &dyn Scene) {
        self.clock.set(time::read(scene));
    }

    /// Fire the hotkeys `pressed` reports as pressed this frame. The callbacks run one by one, with no borrow of the
    /// table held, so a callback may register or remove hotkeys.
    pub fn press(&self, pressed: impl Fn(KeyCode) -> bool) {
        let due: Vec<Rc<HotkeyHandler>> = self
            .hotkeys
            .borrow()
            .iter()
            .filter(|hotkey| hotkey.key.is_some_and(&pressed))
            .map(|hotkey| hotkey.handler.clone())
            .collect();
        for handler in due {
            (*handler)();
        }
    }

    /// Make the scene and the console take what the mods asked for this frame: the clock changes, the blips, and the
    /// console lines, which run from the next frame on.
    pub fn apply(&self, scene: &mut dyn Scene, console: &mut Console) {
        let requests = std::mem::take(&mut *self.clock_requests.borrow_mut());
        time::apply(requests, scene);
        scene.set_mod_blips(&map::minimap_blips(self));
        let lines = std::mem::take(&mut *self.queued_lines.borrow_mut());
        console.queue_lines(lines);
    }
}
