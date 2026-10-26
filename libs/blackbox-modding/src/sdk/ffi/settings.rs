//! `bb.settings` on the mod side: read the table, add an option with an optional Rust closure, read its value, remove it.

use core::cell::RefCell;
use core::ffi::c_void;
use core::mem::{offset_of, size_of};
use core::ptr::null_mut;

use super::{HostTable, finish, read_or, status_result};
use crate::abi::{
    BB_SETTING_LIST, BB_SETTING_MENU_GAMEPLAY, BB_SETTING_TOGGLE, BbSettingCallbackFn, BbSettingOption, BbSettingValue,
    BbSettings, BbSettingsAddOptionFn, BbSettingsGetFn, BbSettingsRemoveOptionFn, BbStr,
};
use crate::sdk::ctx::Keep;
use crate::sdk::{Ctx, Error, Interface, ModResult, SettingId, SettingValue, guard};

/// The function pointers of one `bb.settings` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct SettingsFns {
    ctx: *mut c_void,
    add_option: Option<BbSettingsAddOptionFn>,
    remove_option: Option<BbSettingsRemoveOptionFn>,
    get: Option<BbSettingsGetFn>,
}

/// Read the `bb.settings` table the host offered.
pub(crate) fn read(iface: Interface) -> SettingsFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        SettingsFns {
            ctx: read_or(&iface, offset_of!(BbSettings, ctx), null_mut()),
            add_option: read_or(&iface, offset_of!(BbSettings, add_option), None),
            remove_option: read_or(&iface, offset_of!(BbSettings, remove_option), None),
            get: read_or(&iface, offset_of!(BbSettings, get), None),
        }
    }
}

/// The closure an option runs, with the option's new value.
pub(crate) type ChangeHandler = Box<dyn FnMut(&mut Ctx, &str)>;

/// A registered option: the closure, and what its callback needs to call it.
struct Entry {
    host: HostTable,
    keep: Keep,
    handler: RefCell<ChangeHandler>,
}

/// What `add_option` sends: the option's id and text, its kind and what a kind uses.
pub(crate) struct OptionSpec<'a> {
    pub(crate) toggle: bool,
    pub(crate) id: &'a str,
    pub(crate) label: &'a str,
    pub(crate) values: &'a [SettingValue<'a>],
    pub(crate) default_on: bool,
}

/// Add an option. The closure, if any, is kept in `keep` until the mod unloads.
pub(crate) fn add_option(
    fns: &SettingsFns,
    host: HostTable,
    keep: &Keep,
    spec: &OptionSpec<'_>,
    handler: Option<ChangeHandler>,
) -> ModResult<SettingId> {
    let add = fns.add_option.ok_or(Error::Unsupported)?;
    let entry = handler.map(|handler| Box::new(Entry { host, keep: keep.clone(), handler: RefCell::new(handler) }));
    let user: *mut c_void = entry.as_ref().map_or(null_mut(), |entry| (&**entry as *const Entry).cast_mut().cast());
    let values: Vec<BbSettingValue> = spec
        .values
        .iter()
        .map(|value| BbSettingValue { id: BbStr::new(value.id), label: BbStr::new(value.label) })
        .collect();
    let option = BbSettingOption {
        size: size_of::<BbSettingOption>() as u32,
        kind: if spec.toggle { BB_SETTING_TOGGLE } else { BB_SETTING_LIST },
        menu: BB_SETTING_MENU_GAMEPLAY,
        reserved: 0,
        id: BbStr::new(spec.id),
        label: BbStr::new(spec.label),
        values: if values.is_empty() { core::ptr::null() } else { values.as_ptr() },
        value_count: values.len() as u64,
        default_on: u32::from(spec.default_on),
        pad: 0,
    };
    let callback: Option<BbSettingCallbackFn> = entry.as_ref().map(|_| option_trampoline as BbSettingCallbackFn);
    let mut id = 0_u64;
    // SAFETY: the host calls `option_trampoline` with `user` only while the registration exists. `entry` moves into
    // `keep` below, which lives until the mod's `destroy`; the host removes the registration when the mod unloads.
    // `option`, its strings and `values` are valid for this call.
    let status = unsafe { add(fns.ctx, &option, user, callback, &mut id) };
    status_result(status)?;
    if let Some(entry) = entry {
        keep.borrow_mut().push(entry);
    }
    Ok(SettingId::from_raw(id))
}

/// Take back one `add_option` of this mod.
pub(crate) fn remove_option(fns: &SettingsFns, id: SettingId) -> ModResult<()> {
    let remove = fns.remove_option.ok_or(Error::Unsupported)?;
    // SAFETY: the table is valid while the mod is loaded.
    status_result(unsafe { remove(fns.ctx, id.raw()) })
}

/// The chosen value of option `id`.
pub(crate) fn get(fns: &SettingsFns, id: &str) -> ModResult<String> {
    let get = fns.get.ok_or(Error::Unsupported)?;
    let mut value = String::new();
    let user: *mut c_void = (&mut value as *mut String).cast();
    // SAFETY: the host calls `capture` only during this call, with `user` naming `value`, which outlives it.
    let status = unsafe { get(fns.ctx, BbStr::new(id), Some(capture), user) };
    status_result(status)?;
    Ok(value)
}

/// The sink of [`get`]: appends the host's text to the `String` it names.
unsafe extern "C" fn capture(user: *mut c_void, text: BbStr) {
    // SAFETY: `user` is the `String` that `get` passed in, alive for the call.
    let Some(output) = (unsafe { user.cast::<String>().as_mut() }) else {
        return;
    };
    // SAFETY: the host passes a string valid for this call.
    let Some(text) = (unsafe { text.as_str() }) else {
        return;
    };
    output.push_str(text);
}

/// The callback the host calls when the player changes an option the SDK added: runs the closure in `user` with the value.
unsafe extern "C" fn option_trampoline(user: *mut c_void, value: BbStr) {
    if user.is_null() {
        return;
    }
    // SAFETY: `user` is the `Entry` that `add_option` kept alive (see there).
    let entry = unsafe { &*user.cast::<Entry>() };
    // SAFETY: the host passes a string valid for this call.
    let Some(value) = (unsafe { value.as_str() }) else {
        return;
    };
    // An option that changes while its own closure runs is dropped rather than aliased.
    let Ok(mut handler) = entry.handler.try_borrow_mut() else {
        return;
    };
    let mut ctx = Ctx::new(entry.host, entry.keep.clone());
    let result = guard::run(|| {
        (*handler)(&mut ctx, value);
        Ok(())
    });
    let _ = finish(entry.host, "setting option", result);
}
