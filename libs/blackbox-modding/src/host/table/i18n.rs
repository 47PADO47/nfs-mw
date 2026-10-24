//! `bb.i18n` on the host side: the backend the game implements, and the trampolines behind `BbI18n`.

use core::ffi::c_void;
use core::mem::size_of;
use std::rc::Rc;

use super::BackendResult;
use super::slot::{Attached, Slot, emit_text, guard, name_of, slot_of, text_within};
use crate::abi::{
    BB_ERR_ARG, BB_I18N_INT, BB_I18N_TEXT, BbI18n, BbI18nArg, BbIface, BbSinkFn, BbStatus, BbStr, IFACE_I18N,
    IFACE_I18N_VERSION, MAX_MESSAGE_BYTES,
};

/// The most variables one message may take.
pub const MAX_ARGS: usize = 16;

/// One variable's value, as the mod passed it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum I18nArg {
    /// A whole number: it selects plural forms.
    Int(i64),
    /// Text shown as it is.
    Text(String),
}

/// What the game implements for `bb.i18n`. `owner` is the id of the mod that made the call, whose catalogue answers.
pub trait I18nBackend {
    /// Message `id` of `owner`'s catalogue in the current language, formatted with `args`; the id when no language has it.
    fn text(&self, owner: &str, id: &str, args: &[(String, I18nArg)]) -> BackendResult<String>;
    /// The tag of the language in use.
    fn locale(&self, owner: &str) -> BackendResult<String>;
    /// How many times the language has changed.
    fn generation(&self, owner: &str) -> BackendResult<u64>;
}

/// Build the `bb.i18n` table for `owner`, backed by `backend`.
pub(crate) fn attach(backend: Rc<dyn I18nBackend>, owner: &str) -> Attached {
    let slot = Slot::new(backend, owner);
    let table = Box::new(BbI18n {
        head: BbIface { size: size_of::<BbI18n>() as u32, major: IFACE_I18N_VERSION.0, minor: IFACE_I18N_VERSION.1 },
        ctx: Slot::ctx(&slot),
        text: Some(text),
        locale: Some(locale),
        generation: Some(generation),
    });
    let kept = slot.clone();
    let release = Box::new(move || kept.release_with(|_, _, _| {}));
    Attached::new(IFACE_I18N, IFACE_I18N_VERSION, table, release)
}

/// Read the variables the mod wrote: `count` of them at `args`, each string valid for the call.
///
/// # Safety
/// `args` must be valid for reads of `count` `BbI18nArg`s when `count` is not 0, and each string valid for the call.
unsafe fn read_args(args: *const BbI18nArg, count: u64) -> Result<Vec<(String, I18nArg)>, BbStatus> {
    if count == 0 {
        return Ok(Vec::new());
    }
    if args.is_null() || count > MAX_ARGS as u64 {
        return Err(BB_ERR_ARG);
    }
    // SAFETY: the caller guarantees `args` holds `count` variables, and `count` is at least 1 (checked above).
    let items = unsafe { core::slice::from_raw_parts(args, count as usize) };
    items
        .iter()
        .map(|item| {
            // SAFETY: each string is valid for the call, as the caller guarantees.
            let name = unsafe { name_of(item.name) }?.to_owned();
            let value = match item.kind {
                BB_I18N_INT => I18nArg::Int(item.int),
                // SAFETY: as above.
                BB_I18N_TEXT => I18nArg::Text(unsafe { text_within(item.text, MAX_MESSAGE_BYTES) }?.to_owned()),
                _ => return Err(BB_ERR_ARG),
            };
            Ok((name, value))
        })
        .collect()
}

/// `text`: the formatted message goes to the sink.
unsafe extern "C" fn text(
    ctx: *mut c_void,
    id: BbStr,
    args: *const BbI18nArg,
    arg_count: u64,
    sink: Option<BbSinkFn>,
    sink_user: *mut c_void,
) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn I18nBackend>(ctx)? };
        // SAFETY: the mod passes an id valid for the call.
        let id = unsafe { name_of(id) }?;
        // SAFETY: the mod's variables are valid for the call (see `read_args`).
        let args = unsafe { read_args(args, arg_count) }?;
        let shown = slot.backend.text(&slot.owner, id, &args)?;
        // SAFETY: the sink and its user pointer are the mod's, valid for this call.
        unsafe { emit_text(sink, sink_user, &shown) }
    })
}

/// `locale`: the language's tag goes to the sink.
unsafe extern "C" fn locale(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn I18nBackend>(ctx)? };
        let tag = slot.backend.locale(&slot.owner)?;
        // SAFETY: the sink and its user pointer are the mod's, valid for this call.
        unsafe { emit_text(sink, sink_user, &tag) }
    })
}

/// `generation`: the change counter goes to `out`.
unsafe extern "C" fn generation(ctx: *mut c_void, out: *mut u64) -> BbStatus {
    guard(|| {
        // SAFETY: `ctx` is this table's slot (see `slot_of`).
        let slot = unsafe { slot_of::<dyn I18nBackend>(ctx)? };
        if out.is_null() {
            return Err(BB_ERR_ARG);
        }
        let count = slot.backend.generation(&slot.owner)?;
        // SAFETY: `out` is non-null (checked above) and valid for one write.
        unsafe { out.write(count) };
        Ok(())
    })
}
