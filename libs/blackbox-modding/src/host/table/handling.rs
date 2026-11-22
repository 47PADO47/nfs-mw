//! `bb.handling` on the host side: the backend the game implements, and the trampolines behind `BbHandling`.
//!
//! The backend takes the registered hooks ([`TyreHook`], [`SuspensionHook`], [`ControlsHook`]) and decides where each applies and
//! when it is refused (the game refuses them online). The adapters the vehicle calls are in `handling_model.rs`,
//! `handling_suspension_model.rs` and `handling_controls_model.rs`.
//!
//! One hook of each kind per mod. The three kinds share one table and one slot, so the slot remembers the handle of each kind
//! apart: a mod may register any subset, and registering one kind never answers `BB_ERR_EXISTS` because of another.

use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use std::cell::RefCell;
use std::rc::Rc;

use super::BackendResult;
use super::handling_model::{ControlsHook, ModHook, SuspensionHook, TyreHook, WireFn, WireQuery};
use super::slot::{Attached, Slot, guard, read_prefixed, slot_of};
use crate::abi::{
    BB_ERR_ARG, BB_ERR_EXISTS, BB_ERR_UNSUPPORTED, BbControlsHook, BbHandling, BbIface, BbStatus, BbSuspensionHook,
    BbTyreHook, IFACE_HANDLING, IFACE_HANDLING_VERSION,
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

    /// Take the suspension hook of `owner` (1.1), under the same rules as [`HandlingBackend::register_tyre`]. A game that does
    /// not implement the hook keeps the default, which answers `BB_ERR_UNSUPPORTED`.
    fn register_suspension(&self, _owner: &str, _hook: SuspensionHook) -> BackendResult<u64> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Drop the suspension hook with this handle, and retire it. The built-in springs run from the next tick.
    fn unregister_suspension(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }

    /// Take the controls hook of `owner` (1.2), under the same rules as [`HandlingBackend::register_tyre`]. A game that does
    /// not implement the hook keeps the default, which answers `BB_ERR_UNSUPPORTED`.
    fn register_controls(&self, _owner: &str, _hook: ControlsHook) -> BackendResult<u64> {
        Err(BB_ERR_UNSUPPORTED)
    }
    /// Drop the controls hook with this handle, and retire it. The controls stay as the game shapes them from the next tick.
    fn unregister_controls(&self, _owner: &str, _handle: u64) -> BackendResult<()> {
        Err(BB_ERR_UNSUPPORTED)
    }
}

/// Index of each hook kind in the handles a table keeps.
const TYRE: usize = 0;
const SUSPENSION: usize = 1;
const CONTROLS: usize = 2;

/// What a table keeps for its mod: the game's backend and the handle of each hook kind the mod has registered.
struct Hooks {
    backend: Rc<dyn HandlingBackend>,
    handles: RefCell<[Option<u64>; 3]>,
}

/// Build the `bb.handling` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn HandlingBackend>, owner: &str) -> Attached {
    let slot = Slot::new(Rc::new(Hooks { backend, handles: RefCell::new([None; 3]) }), owner);
    let table = Box::new(BbHandling {
        head: BbIface {
            size: size_of::<BbHandling>() as u32,
            major: IFACE_HANDLING_VERSION.0,
            minor: IFACE_HANDLING_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        register_tyre: Some(register_tyre),
        unregister_tyre: Some(unregister_tyre),
        register_suspension: Some(register_suspension),
        unregister_suspension: Some(unregister_suspension),
        register_controls: Some(register_controls),
        unregister_controls: Some(unregister_controls),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        let handles = kept.backend.handles.take();
        let backend = &kept.backend.backend;
        if let Some(handle) = handles[TYRE] {
            let _ = backend.unregister_tyre(&kept.owner, handle);
        }
        if let Some(handle) = handles[SUSPENSION] {
            let _ = backend.unregister_suspension(&kept.owner, handle);
        }
        if let Some(handle) = handles[CONTROLS] {
            let _ = backend.unregister_controls(&kept.owner, handle);
        }
        kept.release_with(|_, _, _| {});
    });
    Attached::new(IFACE_HANDLING, IFACE_HANDLING_VERSION, table, release)
}

/// The bytes of a registration the host cannot do without: up to and including the callback. The three registration structs
/// share one layout apart from the callback's type.
const HOOK_MIN_SIZE: usize = offset_of!(BbTyreHook, callback) + size_of::<usize>();
const _: () = assert!(
    offset_of!(BbSuspensionHook, callback) + size_of::<usize>() == HOOK_MIN_SIZE
        && offset_of!(BbControlsHook, callback) + size_of::<usize>() == HOOK_MIN_SIZE
);

/// The registration of one hook kind. The hook is retired when the mod unloads, whatever the backend does.
///
/// `parts` takes the callback and the user pointer out of the mod's registration, and `submit` hands the hook to the game.
fn register_hook<Q, R, W>(
    ctx: *mut c_void,
    hook: *const W,
    out_id: *mut u64,
    kind: usize,
    parts: impl FnOnce(&W) -> (Option<WireFn<Q, R>>, *mut c_void),
    submit: impl FnOnce(&dyn HandlingBackend, &str, ModHook<Q, R>) -> BackendResult<u64>,
) -> BbStatus
where
    Q: WireQuery + 'static,
    R: 'static,
    W: Copy + Default,
{
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<Hooks>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's registration is valid for the bytes its size word declares.
        let wire = unsafe { read_prefixed(hook, HOOK_MIN_SIZE) }?;
        let (callback, user) = parts(&wire);
        let callback = callback.ok_or(BB_ERR_ARG)?;
        if slot.backend.handles.borrow()[kind].is_some() {
            return Err(BB_ERR_EXISTS);
        }
        // SAFETY: the mod keeps `user` and the callback valid until the hook is unregistered or the mod unloads, and the hook is
        // retired by then (see the release in `attach` and `HandlingBackend::unregister_*`).
        let hook = unsafe { ModHook::<Q, R>::new(&slot.owner, callback, user) };
        let handle = submit(&*slot.backend.backend, &slot.owner, hook.clone())?;
        slot.backend.handles.borrow_mut()[kind] = Some(handle);
        slot.on_release(Box::new(move || hook.retire()));
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// The removal of one hook kind by handle.
fn unregister_hook(
    ctx: *mut c_void,
    handle: u64,
    kind: usize,
    remove: impl FnOnce(&dyn HandlingBackend, &str, u64) -> BackendResult<()>,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<Hooks>(ctx)? };
        remove(&*slot.backend.backend, &slot.owner, handle)?;
        let mut handles = slot.backend.handles.borrow_mut();
        if handles[kind] == Some(handle) {
            handles[kind] = None;
        }
        Ok(())
    })
}

/// `register_tyre`: one hook per mod.
unsafe extern "C" fn register_tyre(ctx: *mut c_void, hook: *const BbTyreHook, out_id: *mut u64) -> BbStatus {
    register_hook(
        ctx,
        hook,
        out_id,
        TYRE,
        |wire: &BbTyreHook| (wire.callback, wire.user),
        |backend, owner, hook| backend.register_tyre(owner, hook),
    )
}

/// `unregister_tyre`: drop the hook.
unsafe extern "C" fn unregister_tyre(ctx: *mut c_void, handle: u64) -> BbStatus {
    unregister_hook(ctx, handle, TYRE, |backend, owner, handle| backend.unregister_tyre(owner, handle))
}

/// `register_suspension`: one hook per mod (1.1).
unsafe extern "C" fn register_suspension(
    ctx: *mut c_void,
    hook: *const BbSuspensionHook,
    out_id: *mut u64,
) -> BbStatus {
    register_hook(
        ctx,
        hook,
        out_id,
        SUSPENSION,
        |wire: &BbSuspensionHook| (wire.callback, wire.user),
        |backend, owner, hook| backend.register_suspension(owner, hook),
    )
}

/// `unregister_suspension`: drop the hook (1.1).
unsafe extern "C" fn unregister_suspension(ctx: *mut c_void, handle: u64) -> BbStatus {
    unregister_hook(ctx, handle, SUSPENSION, |backend, owner, handle| backend.unregister_suspension(owner, handle))
}

/// `register_controls`: one hook per mod (1.2).
unsafe extern "C" fn register_controls(ctx: *mut c_void, hook: *const BbControlsHook, out_id: *mut u64) -> BbStatus {
    register_hook(
        ctx,
        hook,
        out_id,
        CONTROLS,
        |wire: &BbControlsHook| (wire.callback, wire.user),
        |backend, owner, hook| backend.register_controls(owner, hook),
    )
}

/// `unregister_controls`: drop the hook (1.2).
unsafe extern "C" fn unregister_controls(ctx: *mut c_void, handle: u64) -> BbStatus {
    unregister_hook(ctx, handle, CONTROLS, |backend, owner, handle| backend.unregister_controls(owner, handle))
}
