//! `bb.handling` on the host side: the backend the game implements, and the trampolines behind `BbHandling`.
//!
//! The backend takes the registered hook ([`TyreHook`]) and decides where it applies and when it is refused (the game refuses
//! it online). The tyre hook adapter that the vehicle calls is in `handling_model.rs`.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use std::rc::Rc;

use super::BackendResult;
use super::handling_model::TyreHook;
use super::slot::{Attached, Slot, guard, read_prefixed, slot_of};
use crate::abi::{
    BB_ERR_ARG, BB_ERR_EXISTS, BbHandling, BbIface, BbStatus, BbTyreHook, IFACE_HANDLING, IFACE_HANDLING_VERSION,
};

/// What the game implements for `bb.handling`. `owner` is the id of the mod that made the call.
pub trait HandlingBackend {
    /// Take the tyre hook of `owner` (at most one per mod; the table has already refused a second). The game decides which
    /// vehicles it applies to, and may refuse with `BB_ERR_DENIED` (an online session) or `BB_ERR_NOT_READY`. On success it
    /// returns the hook's handle. The game retires the hook ([`TyreHook::retire`]) whenever it drops it.
    fn register_tyre(&self, owner: &str, hook: TyreHook) -> BackendResult<u64>;
    /// Drop the hook with this handle, and retire it. The built-in tyres run from the next tick. A handle the game does not
    /// know is `BB_ERR_STALE`. Also called for every hook the mod still has when it unloads.
    fn unregister_tyre(&self, owner: &str, handle: u64) -> BackendResult<()>;
}

/// Build the `bb.handling` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn HandlingBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbHandling {
        head: BbIface {
            size: size_of::<BbHandling>() as u32,
            major: IFACE_HANDLING_VERSION.0,
            minor: IFACE_HANDLING_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        register_tyre: Some(register_tyre),
        unregister_tyre: Some(unregister_tyre),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.unregister_tyre(owner, handle);
        })
    });
    Attached::new(IFACE_HANDLING, IFACE_HANDLING_VERSION, table, release)
}

/// The bytes of a registration the host cannot do without: up to and including the callback.
const HOOK_MIN_SIZE: usize = offset_of!(BbTyreHook, callback) + size_of::<usize>();

/// `register_tyre`: one hook per mod. The hook is retired when the mod unloads, whatever the backend does.
unsafe extern "C" fn register_tyre(ctx: *mut c_void, hook: *const BbTyreHook, out_id: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn HandlingBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's registration is valid for the bytes its size word declares.
        let wire = unsafe { read_prefixed(hook, HOOK_MIN_SIZE) }?;
        let callback = wire.callback.ok_or(BB_ERR_ARG)?;
        if slot.has_registered() {
            return Err(BB_ERR_EXISTS);
        }
        // SAFETY: the mod keeps `user` and the callback valid until the hook is unregistered or the mod unloads, and the hook is
        // retired by then (see the release below and `HandlingBackend::unregister_tyre`).
        let hook = unsafe { TyreHook::new(&slot.owner, callback, wire.user) };
        let handle = slot.backend.register_tyre(&slot.owner, hook.clone())?;
        slot.remember(handle);
        slot.on_release(Box::new(move || hook.retire()));
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `unregister_tyre`: drop the hook.
unsafe extern "C" fn unregister_tyre(ctx: *mut c_void, handle: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn HandlingBackend>(ctx)? };
        slot.backend.unregister_tyre(&slot.owner, handle)?;
        slot.forget(handle);
        Ok(())
    })
}
