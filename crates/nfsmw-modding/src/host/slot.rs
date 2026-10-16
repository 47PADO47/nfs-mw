//! What every `nfsmw.*` table shares on the host side: the backend, the owning mod's id, the text arguments and the guard that
//! turns a panic into a status (ADR 0005 section 3, "panic containment").

#![allow(unsafe_code)] // ADR 0005 section 8: unsafe confined to host table, io and slot modules.

use core::ffi::c_void;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;

use blackbox_modding::abi::{BB_ERR_ARG, BB_ERR_PANIC, BB_OK, BbStatus, BbStr};

/// One table's state. The table's `ctx` is `Rc::as_ptr` of its slot, so every trampoline reads the backend and the owner
/// from it. The table's owner keeps the `Rc` alive as long as the table is in use.
pub(crate) struct Slot<T: ?Sized> {
    /// The game's implementation of the interface.
    pub(crate) backend: Rc<T>,
    /// Id of the mod this table was built for.
    pub(crate) owner: String,
}

impl<T: ?Sized> Slot<T> {
    /// A new slot for `owner`.
    pub(crate) fn new(backend: Rc<T>, owner: &str) -> Rc<Self> {
        Rc::new(Slot { backend, owner: owner.to_owned() })
    }

    /// The `ctx` value a table carries for this slot.
    pub(crate) fn ctx(slot: &Rc<Self>) -> *mut c_void {
        Rc::as_ptr(slot).cast_mut().cast::<c_void>()
    }
}

/// The slot a table's `ctx` names.
///
/// # Safety
/// `ctx` must be null, or the value [`Slot::ctx`] gave for a slot of the same `T` that is still alive. The table that
/// carries it is alive while its owner is, and the host calls the table only while the mod is loaded.
pub(crate) unsafe fn slot_of<'a, T: ?Sized>(ctx: *mut c_void) -> Result<&'a Slot<T>, BbStatus> {
    if ctx.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the caller guarantees `ctx` is a live slot of this type (see `# Safety`).
    Ok(unsafe { &*ctx.cast::<Slot<T>>() })
}

/// Read a text argument the mod passed, at most `max` bytes, as UTF-8.
///
/// # Safety
/// `text` must be valid for the call that carries it (ADR 0005 section 4).
pub(crate) unsafe fn text_arg<'a>(text: BbStr, max: usize) -> Result<&'a str, BbStatus> {
    // SAFETY: the caller guarantees the string is valid for this call.
    let text = unsafe { text.as_str() }.ok_or(BB_ERR_ARG)?;
    if text.len() > max {
        return Err(BB_ERR_ARG);
    }
    Ok(text)
}

/// Run a trampoline body that returns nothing. A panic becomes `BB_ERR_PANIC`; an error becomes its status; success `BB_OK`.
pub(crate) fn guard(body: impl FnOnce() -> Result<(), BbStatus>) -> BbStatus {
    match caught(body) {
        Ok(()) => BB_OK,
        Err(status) => status,
    }
}

/// Run a body that yields a value. A panic becomes `Err(BB_ERR_PANIC)`, so it never unwinds past the boundary.
pub(crate) fn caught<T>(body: impl FnOnce() -> Result<T, BbStatus>) -> Result<T, BbStatus> {
    catch_unwind(AssertUnwindSafe(body)).unwrap_or(Err(BB_ERR_PANIC))
}
