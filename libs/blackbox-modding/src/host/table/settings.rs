//! `bb.settings` on the host side: the backend the game implements, and the trampolines behind `BbSettings`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, emit_text, guard, name_of, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BB_SETTING_LIST, BB_SETTING_MENU_GAMEPLAY, BB_SETTING_TOGGLE, BbIface, BbSettingCallbackFn,
    BbSettingOption, BbSettingValue, BbSettings, BbSinkFn, BbStatus, BbStr, IFACE_SETTINGS, IFACE_SETTINGS_VERSION,
    MAX_NAME_BYTES,
};
use crate::host::options::{MAX_OPTION_VALUES, OptionKind, OptionSpec, ValueSpec};

/// What the mod's callback is called with: the option's new value, a value id (`"on"` or `"off"` for a toggle).
pub type SettingHandler = Box<dyn Fn(&str)>;

/// What the game implements for `bb.settings`. `owner` is the id of the mod that made the call.
pub trait SettingsBackend {
    /// Add the option or merge its values into the game's (see [`crate::host::OptionsModel::add_option`]). `on_change`
    /// is called with the new value when the player changes the option. Returns the registration's handle. A kind that
    /// clashes with the game's answers `BB_ERR_EXISTS`, and the game logs a warning.
    fn add_option(&self, owner: &str, spec: &OptionSpec<'_>, on_change: Option<SettingHandler>) -> BackendResult<u64>;
    /// Take back one registration.
    fn remove_option(&self, owner: &str, handle: u64) -> BackendResult<()>;
    /// The chosen value of option `option`: a value id, or `"on"` or `"off"`.
    fn get(&self, owner: &str, option: &str) -> BackendResult<String>;
}

/// Build the `bb.settings` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn SettingsBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbSettings {
        head: BbIface {
            size: size_of::<BbSettings>() as u32,
            major: IFACE_SETTINGS_VERSION.0,
            minor: IFACE_SETTINGS_VERSION.1,
        },
        ctx: Slot::ctx(&slot),
        add_option: Some(add_option),
        remove_option: Some(remove_option),
        get: Some(get),
    });
    let kept = slot.clone();
    let release = Box::new(move || {
        kept.release_with(|backend, owner, handle| {
            let _ = backend.remove_option(owner, handle);
        })
    });
    Attached::new(IFACE_SETTINGS, IFACE_SETTINGS_VERSION, table, release)
}

/// The values the mod wrote: `count` pairs at `values`, each string valid for the call.
///
/// # Safety
/// `values` must be valid for reads of `count` `BbSettingValue`s, and each string valid for the call (ADR 0005 section 4).
unsafe fn read_values<'a>(values: *const BbSettingValue, count: u64) -> Result<Vec<ValueSpec<'a>>, BbStatus> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if values.is_null() || count > MAX_OPTION_VALUES as u64 {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the caller guarantees `values` holds `count` entries, and `count` is at least 1 (checked above).
    let items = unsafe { core::slice::from_raw_parts(values, count as usize) };
    items
        .iter()
        .map(|item| {
            // SAFETY: each string is valid for the call, as the caller guarantees.
            let id = unsafe { name_of(item.id) }?;
            // SAFETY: as above.
            let label = unsafe { text_within(item.label, MAX_NAME_BYTES) }?;
            Ok(ValueSpec { id, label })
        })
        .collect()
}

/// Read the option the mod wrote. Its `size` must cover the whole struct.
///
/// # Safety
/// `option` must be valid for reads of the bytes its own `size` field declares, and its strings valid for the call.
unsafe fn read_option<'a>(option: *const BbSettingOption) -> Result<OptionSpec<'a>, BbStatus> {
    if option.is_null() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the option's size word is its first field (see `# Safety`).
    let size = unsafe { option.cast::<u32>().read() };
    if (size as usize) < size_of::<BbSettingOption>() {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the size covers the struct, so the read is in bounds (see `# Safety`).
    let option = unsafe { option.read() };
    if option.menu != BB_SETTING_MENU_GAMEPLAY {
        return Err(BB_ERR_ARG);
    }
    let kind = match option.kind {
        BB_SETTING_TOGGLE => OptionKind::Toggle,
        BB_SETTING_LIST => OptionKind::List,
        _ => return Err(BB_ERR_ARG),
    };
    // SAFETY: the mod passes strings valid for the call.
    let id = unsafe { name_of(option.id) }?;
    // SAFETY: as above.
    let label = unsafe { text_within(option.label, MAX_NAME_BYTES) }?;
    // SAFETY: the array and its strings are the mod's, valid for the call (see `# Safety`).
    let values = unsafe { read_values(option.values, option.value_count) }?;
    Ok(OptionSpec { id, kind, label, values, default_on: option.default_on != 0 })
}

/// `add_option`: the option described by `option`; the registration's handle goes to `out_id`.
unsafe extern "C" fn add_option(
    ctx: *mut c_void,
    option: *const BbSettingOption,
    user: *mut c_void,
    callback: Option<BbSettingCallbackFn>,
    out_id: *mut u64,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn SettingsBackend>(ctx)? };
        if out_id.is_null() {
            return Err(BB_ERR_ARG);
        }
        // SAFETY: the mod's option is valid for the call (see `read_option`).
        let spec = unsafe { read_option(option) }?;
        let on_change: Option<SettingHandler> = callback.map(|callback| {
            let handler: SettingHandler = Box::new(move |value: &str| {
                // SAFETY: the mod keeps `user` valid until the registration is removed (ADR 0005 section 4), and the
                // string lives for this call.
                unsafe { callback(user, BbStr::new(value)) }
            });
            handler
        });
        let handle = slot.backend.add_option(&slot.owner, &spec, on_change)?;
        slot.remember(handle);
        // SAFETY: `out_id` is non-null (checked above) and valid for one write.
        unsafe { out_id.write(handle) };
        Ok(())
    })
}

/// `remove_option`: take back one registration.
unsafe extern "C" fn remove_option(ctx: *mut c_void, handle: u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn SettingsBackend>(ctx)? };
        slot.backend.remove_option(&slot.owner, handle)?;
        slot.forget(handle);
        Ok(())
    })
}

/// `get`: the chosen value of an option, through the sink.
unsafe extern "C" fn get(ctx: *mut c_void, option: BbStr, sink: Option<BbSinkFn>, user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn SettingsBackend>(ctx)? };
        // SAFETY: the option id is valid for the call.
        let option = unsafe { name_of(option) }?;
        let value = slot.backend.get(&slot.owner, option)?;
        // SAFETY: the sink and `user` are the mod's, valid for the call.
        unsafe { emit_text(sink, user, &value) }
    })
}
