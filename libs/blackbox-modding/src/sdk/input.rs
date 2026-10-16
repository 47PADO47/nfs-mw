//! `Input`: register a hotkey with a callback (catalogue: docs/modding/api.md#hotkeys-and-abilities).

use super::ctx::Keep;
use super::ffi::HostTable;
use super::ffi::input::{self as host_input, InputFns};
use super::{ActionId, Ctx, ModResult};

/// The `bb.input` interface of the host, as a mod uses it. Obtained from `Ctx::input`.
pub struct Input {
    fns: InputFns,
    host: HostTable,
    keep: Keep,
}

impl Input {
    pub(crate) fn new(fns: InputFns, host: HostTable, keep: Keep) -> Self {
        Input { fns, host, keep }
    }

    /// Register a hotkey. `default_binding` is the key text (`"F5"`, `"Gamepad X"`), or `""` for none. The handler runs
    /// each time the key is pressed. The hotkey is removed when the mod unloads.
    pub fn register<F>(&self, name: &str, label: &str, default_binding: &str, handler: F) -> ModResult<ActionId>
    where
        F: FnMut(&mut Ctx) + 'static,
    {
        host_input::register(&self.fns, self.host, &self.keep, name, label, default_binding, handler)
    }

    /// Remove a hotkey this mod registered.
    pub fn unregister(&self, id: ActionId) -> ModResult<()> {
        host_input::unregister(&self.fns, id)
    }
}
