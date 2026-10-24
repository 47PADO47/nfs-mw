//! `bb.i18n` on the mod side: read the table, format a message, read the language and its change counter.

use core::ffi::c_void;
use core::mem::offset_of;
use core::ptr::null_mut;

use super::{read_or, status_result};
use crate::abi::{
    BB_I18N_INT, BB_I18N_TEXT, BbI18n, BbI18nArg, BbI18nGenerationFn, BbI18nLocaleFn, BbI18nTextFn, BbSinkFn, BbStr,
};
use crate::sdk::{Error, Interface, ModResult};

/// The function pointers of one `bb.i18n` table, read once.
#[derive(Clone, Copy)]
pub(crate) struct I18nFns {
    ctx: *mut c_void,
    text: Option<BbI18nTextFn>,
    locale: Option<BbI18nLocaleFn>,
    generation: Option<BbI18nGenerationFn>,
}

/// One variable of a message, as the SDK passes it on.
pub(crate) enum Value<'a> {
    Int(i64),
    Text(&'a str),
}

/// Read the `bb.i18n` table the host offered.
pub(crate) fn read(iface: Interface) -> I18nFns {
    // SAFETY: each field is read only when the table's size covers it (see `read_or`).
    unsafe {
        I18nFns {
            ctx: read_or(&iface, offset_of!(BbI18n, ctx), null_mut()),
            text: read_or(&iface, offset_of!(BbI18n, text), None),
            locale: read_or(&iface, offset_of!(BbI18n, locale), None),
            generation: read_or(&iface, offset_of!(BbI18n, generation), None),
        }
    }
}

/// A text the host writes to the sink during the call: the sink appends it to the `String` the user pointer names.
unsafe extern "C" fn capture(user: *mut c_void, text: BbStr) {
    // SAFETY: `user` is the `String` the reader passed in, alive for the call.
    let Some(output) = (unsafe { user.cast::<String>().as_mut() }) else {
        return;
    };
    // SAFETY: the host passes a string valid for this call.
    let Some(text) = (unsafe { text.as_str() }) else {
        return;
    };
    output.push_str(text);
}

/// Message `id` of the mod's catalogue with `args`.
pub(crate) fn text(fns: &I18nFns, id: &str, args: &[(&str, Value<'_>)]) -> ModResult<String> {
    let text = fns.text.ok_or(Error::Unsupported)?;
    let raw: Vec<BbI18nArg> = args
        .iter()
        .map(|(name, value)| match value {
            Value::Int(n) => BbI18nArg { name: BbStr::new(name), kind: BB_I18N_INT, _pad: 0, int: *n, text: BbStr::EMPTY },
            Value::Text(t) => BbI18nArg { name: BbStr::new(name), kind: BB_I18N_TEXT, _pad: 0, int: 0, text: BbStr::new(t) },
        })
        .collect();
    let mut output = String::new();
    let sink: BbSinkFn = capture;
    let user = (&mut output as *mut String).cast::<c_void>();
    // SAFETY: the id, the variables and `output` live for the call, and the sink takes `output` as `user`.
    status_result(unsafe { text(fns.ctx, BbStr::new(id), raw.as_ptr(), raw.len() as u64, Some(sink), user) })?;
    Ok(output)
}

/// The tag of the language in use.
pub(crate) fn locale(fns: &I18nFns) -> ModResult<String> {
    let locale = fns.locale.ok_or(Error::Unsupported)?;
    let mut output = String::new();
    let sink: BbSinkFn = capture;
    // SAFETY: `output` lives for the call, and the sink takes it as `user`.
    status_result(unsafe { locale(fns.ctx, Some(sink), (&mut output as *mut String).cast::<c_void>()) })?;
    Ok(output)
}

/// How many times the language has changed.
pub(crate) fn generation(fns: &I18nFns) -> ModResult<u64> {
    let generation = fns.generation.ok_or(Error::Unsupported)?;
    let mut out = 0;
    // SAFETY: `out` is a live local, valid for one write.
    status_result(unsafe { generation(fns.ctx, &mut out) })?;
    Ok(out)
}
