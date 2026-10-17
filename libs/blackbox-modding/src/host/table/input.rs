//! `bb.input` on the host side: the backend the game implements, and the trampolines behind `BbInput`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, name_of, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BbHotkeyFn, BbIface, BbInput, BbStatus, BbStr, IFACE_INPUT, IFACE_INPUT_VERSION, MAX_NAME_BYTES,
};

/// The Rust side of a hotkey: called each time the hotkey is pressed.
pub type HotkeyHandler = Box<dyn Fn()>;

/// What the game implements for `bb.input`. `owner` is the id of the mod that made the call.
pub trait InputBackend {
    /// Register a hotkey. `default_binding` may be empty. `on_press` runs when the key is pressed. Returns the action's
    /// handle.
    fn register_hotkey(
        &self,
        owner: &str,
        name: &str,
        label: &str,
        default_binding: &str,
        on_press: HotkeyHandler,
    ) -> BackendResult<u64>;
    /// Remove a hotkey by handle.
    fn unregister(&self, owner: &str, handle: u64) -> BackendResult<()>;
}

/// Build the `bb.input` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn InputBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbInput {
        head: BbIface { size: size_of::<BbInput>() as u32, major: IFACE_INPUT_VERSION.0, minor: IFACE_INPUT_VERSION.1 },
        ctx: Slot::ctx(&slot),
        register_hotkey: Some(register_hotkey),
        unregister: Some(unregister),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.unregister(owner, handle);
        })
    });
    Attached::new(IFACE_INPUT, IFACE_INPUT_VERSION, table, release)
}

/// `register_hotkey`: a named action with a default binding and the mod's callback.
unsafe extern "C" fn register_hotkey(
    ctx: *mut c_void,
    name: BbStr,
    label: BbStr,
    default_binding: BbStr,
    user: *mut c_void,
    callback: Option<BbHotkeyFn>,
    out_id: *mut u64,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn InputBackend>(ctx)? };
        let callback = callback.ok_or(BB_ERR_ARG)?;
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the caller's strings are valid for this call.
        let name = unsafe { name_of(name) }?;
        // SAFETY: as above.
        let label = unsafe { text_within(label, MAX_NAME_BYTES) }?;
        // SAFETY: as above.
        let binding = unsafe { text_within(default_binding, MAX_NAME_BYTES) }?;
        let on_press: HotkeyHandler = Box::new(move || {
            // SAFETY: the mod keeps `user` valid until the hotkey is unregistered (ADR 0005 section 4).
            unsafe { callback(user) }
        });
        let handle = slot.backend.register_hotkey(&slot.owner, name, label, binding, on_press)?;
        slot.remember(handle);
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `unregister`: remove one hotkey.
unsafe extern "C" fn unregister(ctx: *mut c_void, handle: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn InputBackend>(ctx)? };
        slot.backend.unregister(&slot.owner, handle)?;
        slot.forget(handle);
        Ok(())
    })
}
