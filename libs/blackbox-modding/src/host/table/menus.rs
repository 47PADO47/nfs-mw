//! `bb.menus` on the host side: the backend the game implements, and the trampolines behind `BbMenus`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, guard, name_of, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BB_MENU_BUTTON, BB_MENU_LIST, BB_MENU_SLIDER, BB_MENU_TOGGLE, BbIface, BbMenuCallbackFn, BbMenuRow,
    BbMenus, BbStatus, BbStr, IFACE_MENUS, IFACE_MENUS_VERSION, MAX_NAME_BYTES,
};

/// The most choices a list row may have. A longer list is refused.
pub const MAX_CHOICES: usize = 64;

/// What the mod's callback is called with: the row's new value.
pub type MenuHandler = Box<dyn Fn(f32)>;

/// The kind of a row, with what it needs.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuKind {
    /// Off (0) or on (1).
    Toggle,
    /// From `min` to `max` in steps of `step`. A step of zero is a tenth of the range.
    Slider { min: f32, max: f32, step: f32 },
    /// The index of one of `choices`.
    List { choices: Vec<String> },
    /// Pressed, not set: its callback runs with 0.
    Button,
}

/// A row as the mod describes it. The strings are valid for the call that registers the row.
#[derive(Debug, Clone, PartialEq)]
pub struct MenuRowSpec<'a> {
    pub kind: MenuKind,
    /// The row's name, unique among the mod's rows; its saved value is kept under it.
    pub name: &'a str,
    /// The text the row shows.
    pub label: &'a str,
    /// The value before anything changes it.
    pub default: f32,
}

/// What the game implements for `bb.menus`. `owner` is the id of the mod that made the call.
pub trait MenusBackend {
    /// Add a row. `on_change` is called with the row's new value, and with 0 for a button. Returns the row's handle.
    fn register(&self, owner: &str, row: &MenuRowSpec<'_>, on_change: MenuHandler) -> BackendResult<u64>;
    /// Remove a row by handle.
    fn unregister(&self, owner: &str, handle: u64) -> BackendResult<()>;
    /// The value of a row.
    fn get(&self, owner: &str, handle: u64) -> BackendResult<f32>;
    /// Set a row's value, as a change by the player would (saved, callback not called).
    fn set(&self, owner: &str, handle: u64, value: f32) -> BackendResult<()>;
}

/// Build the `bb.menus` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn MenusBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbMenus {
        head: BbIface { size: size_of::<BbMenus>() as u32, major: IFACE_MENUS_VERSION.0, minor: IFACE_MENUS_VERSION.1 },
        ctx: Slot::ctx(&slot),
        register: Some(register),
        unregister: Some(unregister),
        get: Some(get),
        set: Some(set),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.unregister(owner, handle);
        })
    });
    Attached::new(IFACE_MENUS, IFACE_MENUS_VERSION, table, release)
}

/// A value the mod sends must be a finite number.
fn finite(value: f32) -> Result<f32, BbStatus> {
    if !value.is_finite() {
        return Err(BB_ERR_ARG);
    }
    Ok(value)
}

/// Read the list's texts the mod wrote: `count` strings at `choices`, each valid for the call.
///
/// # Safety
/// `choices` must be valid for reads of `count` `BbStr`s, and each string valid for the call (ADR 0005 section 4).
unsafe fn read_choices(choices: *const BbStr, count: u64) -> Result<Vec<String>, BbStatus> {
    if choices.is_null() || count == 0 || count > MAX_CHOICES as u64 {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the caller guarantees `choices` holds `count` strings, and `count` is at least 1 (checked above).
    let items = unsafe { core::slice::from_raw_parts(choices, count as usize) };
    items
        .iter()
        .map(|item| {
            // SAFETY: each string is valid for the call, as the caller guarantees.
            unsafe { text_within(*item, MAX_NAME_BYTES) }.map(str::to_owned)
        })
        .collect()
}

/// Read a row the mod wrote. Its `size` must cover the whole struct; the kind decides which fields count.
///
/// # Safety
/// `row` must be valid for reads of the bytes its own `size` field declares, and its strings valid for the call.
unsafe fn read_row(row: *const BbMenuRow) -> Result<(MenuKind, String, String, f32), BbStatus> {
    if row.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the row's size word is its first field (see `# Safety`).
    let size = unsafe { row.cast::<u32>().read() };
    if (size as usize) < size_of::<BbMenuRow>() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the size covers the struct, so the read is in bounds (see `# Safety`).
    let row = unsafe { row.read() };
    // SAFETY: the mod passes a name valid for the call.
    let name = unsafe { name_of(row.name) }?.to_owned();
    // SAFETY: as above.
    let label = unsafe { text_within(row.label, MAX_NAME_BYTES) }?.to_owned();
    let default = finite(row.default)?;
    let kind = match row.kind {
        BB_MENU_TOGGLE => MenuKind::Toggle,
        BB_MENU_BUTTON => MenuKind::Button,
        BB_MENU_SLIDER => {
            let (min, max, step) = (finite(row.min)?, finite(row.max)?, finite(row.step)?);
            if min > max || step < 0.0 {
                return Err(BB_ERR_ARG);
            }
            MenuKind::Slider { min, max, step }
        }
        BB_MENU_LIST => {
            // SAFETY: the list's pointer and count are the mod's, valid for the call (see `# Safety`).
            let choices = unsafe { read_choices(row.choices, row.choice_count) }?;
            MenuKind::List { choices }
        }
        _ => return Err(BB_ERR_ARG),
    };
    Ok((kind, name, label, default))
}

/// `register`: a row described by `row`; its handle goes to `out_id`.
unsafe extern "C" fn register(
    ctx: *mut c_void,
    row: *const BbMenuRow,
    user: *mut c_void,
    callback: Option<BbMenuCallbackFn>,
    out_id: *mut u64,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn MenusBackend>(ctx)? };
        let callback = callback.ok_or(BB_ERR_ARG)?;
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's row is valid for the call (see `read_row`).
        let (kind, name, label, default) = unsafe { read_row(row) }?;
        let spec = MenuRowSpec { kind, name: &name, label: &label, default };
        let on_change: MenuHandler = Box::new(move |value: f32| {
            // SAFETY: the mod keeps `user` valid until the row is unregistered (ADR 0005 section 4).
            unsafe { callback(user, value) }
        });
        let handle = slot.backend.register(&slot.owner, &spec, on_change)?;
        slot.remember(handle);
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `unregister`: remove one row.
unsafe extern "C" fn unregister(ctx: *mut c_void, handle: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn MenusBackend>(ctx)? };
        slot.backend.unregister(&slot.owner, handle)?;
        slot.forget(handle);
        Ok(())
    })
}

/// `get`: write a row's value to `out`.
unsafe extern "C" fn get(ctx: *mut c_void, handle: u64, out: *mut f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn MenusBackend>(ctx)? };
        if out.is_null() {
            return Err(BB_ERR_ARG);
        }
        let value = slot.backend.get(&slot.owner, handle)?;
        // SAFETY: `out` is non-null (checked above) and valid for one write.
        unsafe { out.write(value) };
        Ok(())
    })
}

/// `set`: a new value for a row.
unsafe extern "C" fn set(ctx: *mut c_void, handle: u64, value: f32) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn MenusBackend>(ctx)? };
        slot.backend.set(&slot.owner, handle, finite(value)?)
    })
}
