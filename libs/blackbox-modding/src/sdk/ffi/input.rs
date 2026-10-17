//! `bb.input` on the mod side: read the table, and register a Rust closure as a hotkey callback.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{HostTable, finish, read_or, status_result};
use crate::abi::{BbInput, BbInputRegisterFn, BbInputUnregisterFn, BbStr};
use crate::sdk::ctx::Keep;
use crate::sdk::{ActionId, Ctx, Error, Interface, ModResult, guard};

/// The function pointers of one `bb.input` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct InputFns {
    ctx: *mut c_void,
    register_hotkey: Option<BbInputRegisterFn>,
    unregister: Option<BbInputUnregisterFn>,
}

/// Read the `bb.input` table the host offered.
pub(crate) fn read(iface: Interface) -> InputFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        InputFns {
            ctx: read_or(&iface, offset_of!(BbInput, ctx), null_mut()),
            register_hotkey: read_or(&iface, offset_of!(BbInput, register_hotkey), None),
            unregister: read_or(&iface, offset_of!(BbInput, unregister), None),
        }
    }
}

/// A registered hotkey: the closure, and what its trampoline needs to call it.
struct HotkeyEntry<F> {
    host: HostTable,
    keep: Keep,
    handler: RefCell<F>,
}

/// Register a closure to run when a hotkey is pressed. The closure is kept in `keep` until the mod unloads.
pub(crate) fn register<F>(
    fns: &InputFns,
    host: HostTable,
    keep: &Keep,
    name: &str,
    label: &str,
    default_binding: &str,
    handler: F,
) -> ModResult<ActionId>
where
    F: FnMut(&mut Ctx) + 'static,
{
    let register = fns.register_hotkey.ok_or(Error::Unsupported)?;
    let entry = Box::new(HotkeyEntry { host, keep: keep.clone(), handler: RefCell::new(handler) });
    let user: *mut c_void = (&*entry as *const HotkeyEntry<F>).cast_mut().cast();
    let mut id = 0_u64;
    // SAFETY: the host calls `hotkey_trampoline::<F>` with `user` only while the hotkey is registered. `entry` moves into
    // `keep` below, which lives until the mod's `destroy`; the host removes the hotkey when the mod unloads.
    let status = unsafe {
        register(
            fns.ctx,
            BbStr::new(name),
            BbStr::new(label),
            BbStr::new(default_binding),
            user,
            Some(hotkey_trampoline::<F>),
            &mut id,
        )
    };
    status_result(status)?;
    keep.borrow_mut().push(entry);
    Ok(ActionId::from_raw(id))
}

/// Remove a hotkey this mod registered.
pub(crate) fn unregister(fns: &InputFns, id: ActionId) -> ModResult<()> {
    let unregister = fns.unregister.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, id.raw()) })
}

/// The callback the host calls when a registered hotkey is pressed: runs the closure in `user`.
unsafe extern "C" fn hotkey_trampoline<F>(user: *mut c_void)
where
    F: FnMut(&mut Ctx) + 'static,
{
    if user.is_null() {
        return;
    }
    // SAFETY: `user` is the `HotkeyEntry<F>` that `register` kept alive (see there).
    let entry = unsafe { &*user.cast::<HotkeyEntry<F>>() };
    // A hotkey that fires while its own closure runs is dropped rather than aliased.
    let Ok(mut handler) = entry.handler.try_borrow_mut() else {
        return;
    };
    let mut ctx = Ctx::new(entry.host, entry.keep.clone());
    let result = guard::run(|| {
        (*handler)(&mut ctx);
        Ok(())
    });
    let _ = finish(entry.host, "hotkey", result);
}
