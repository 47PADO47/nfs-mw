//! `bb.menus` on the mod side: read the table, add a row with a Rust closure, and read, set or remove a row.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{HostTable, finish, read_or, status_result};
use crate::abi::{BbMenuRow, BbMenus, BbMenusGetFn, BbMenusRegisterFn, BbMenusSetFn, BbMenusUnregisterFn, BbStr};
use crate::sdk::ctx::Keep;
use crate::sdk::{Ctx, Error, Interface, MenuId, ModResult, guard};

/// The function pointers of one `bb.menus` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct MenusFns {
    ctx: *mut c_void,
    register: Option<BbMenusRegisterFn>,
    unregister: Option<BbMenusUnregisterFn>,
    get: Option<BbMenusGetFn>,
    set: Option<BbMenusSetFn>,
}

/// Read the `bb.menus` table the host offered.
pub(crate) fn read(iface: Interface) -> MenusFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        MenusFns {
            ctx: read_or(&iface, offset_of!(BbMenus, ctx), null_mut()),
            register: read_or(&iface, offset_of!(BbMenus, register), None),
            unregister: read_or(&iface, offset_of!(BbMenus, unregister), None),
            get: read_or(&iface, offset_of!(BbMenus, get), None),
            set: read_or(&iface, offset_of!(BbMenus, set), None),
        }
    }
}

/// A row as the SDK describes it before it goes to the host. `kind` is one of the `BB_MENU_*` kinds; the numbers a
/// kind does not use are zero, and `choices` is empty for every kind but a list.
pub(crate) struct RowSpec<'a> {
    pub(crate) kind: u32,
    pub(crate) name: &'a str,
    pub(crate) label: &'a str,
    pub(crate) min: f32,
    pub(crate) max: f32,
    pub(crate) step: f32,
    pub(crate) default: f32,
    pub(crate) choices: &'a [&'a str],
}

/// The closure a row runs, with the row's new value (a button passes 0).
pub(crate) type RowHandler = Box<dyn FnMut(&mut Ctx, f32)>;

/// A registered row: the closure, and what its callback needs to call it.
struct RowEntry {
    host: HostTable,
    keep: Keep,
    handler: RefCell<RowHandler>,
}

/// Add a row with `handler`. The closure is kept in `keep` until the mod unloads.
pub(crate) fn register(
    fns: &MenusFns,
    host: HostTable,
    keep: &Keep,
    spec: &RowSpec<'_>,
    handler: RowHandler,
) -> ModResult<MenuId> {
    let register = fns.register.ok_or(Error::Unsupported)?;
    let entry = Box::new(RowEntry { host, keep: keep.clone(), handler: RefCell::new(handler) });
    let user: *mut c_void = (&*entry as *const RowEntry).cast_mut().cast();
    let choices: Vec<BbStr> = spec.choices.iter().map(|choice| BbStr::new(choice)).collect();
    let row = BbMenuRow {
        size: size_of::<BbMenuRow>() as u32,
        kind: spec.kind,
        name: BbStr::new(spec.name),
        label: BbStr::new(spec.label),
        min: spec.min,
        max: spec.max,
        step: spec.step,
        default: spec.default,
        choices: choices.as_ptr(),
        choice_count: choices.len() as u64,
    };
    let mut id = 0_u64;
    // SAFETY: the host calls `row_trampoline` with `user` only while the row is registered. `entry` moves into `keep`
    // below, which lives until the mod's `destroy`; the host removes the row when the mod unloads. `row`, its strings
    // and `choices` are valid for this call.
    let status = unsafe { register(fns.ctx, &row, user, Some(row_trampoline), &mut id) };
    status_result(status)?;
    keep.borrow_mut().push(entry);
    Ok(MenuId::from_raw(id))
}

/// Remove a row this mod added.
pub(crate) fn unregister(fns: &MenusFns, id: MenuId) -> ModResult<()> {
    let unregister = fns.unregister.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { unregister(fns.ctx, id.raw()) })
}

/// The value of a row.
pub(crate) fn get(fns: &MenusFns, id: MenuId) -> ModResult<f32> {
    let get = fns.get.ok_or(Error::Unsupported)?;
    let mut value = 0.0_f32;
    // SAFETY: `value` is valid for one write, and the table is valid while the mod is loaded.
    status_result(unsafe { get(fns.ctx, id.raw(), &mut value) })?;
    Ok(value)
}

/// Set the value of a row, as the player's change would (saved; the row's closure does not run).
pub(crate) fn set(fns: &MenusFns, id: MenuId, value: f32) -> ModResult<()> {
    let set = fns.set.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { set(fns.ctx, id.raw(), value) })
}

/// The callback the host calls when the player changes a row the SDK added: runs the closure in `user` with the value.
unsafe extern "C" fn row_trampoline(user: *mut c_void, value: f32) {
    if user.is_null() {
        return;
    }
    // SAFETY: `user` is the `RowEntry` that `register` kept alive (see there).
    let entry = unsafe { &*user.cast::<RowEntry>() };
    // A row that changes while its own closure runs is dropped rather than aliased.
    let Ok(mut handler) = entry.handler.try_borrow_mut() else {
        return;
    };
    let mut ctx = Ctx::new(entry.host, entry.keep.clone());
    let result = guard::run(|| {
        (*handler)(&mut ctx, value);
        Ok(())
    });
    let _ = finish(entry.host, "menu row", result);
}
