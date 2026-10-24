//! `bb.i18n` 1.0: the mod's own messages in the player's language (catalogue: docs/modding/translations.md).
//!
//! A mod ships `lang/<locale>/*.ftl` files; the host loads them as the mod's catalogue. `text` formats one message in the
//! current language, `locale` names the language, and `generation` counts the language changes so a mod that keeps text
//! knows when to format it again.

use core::ffi::c_void;

use super::BbIface;
use crate::abi::{BbSinkFn, BbStatus, BbStr};

/// A whole number: it selects plural forms.
pub const BB_I18N_INT: u32 = 0;
/// Text shown as it is.
pub const BB_I18N_TEXT: u32 = 1;

/// One variable of a message, read by `text`. Its strings are valid for that call.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct BbI18nArg {
    /// The variable's name, as the message writes it after `$`.
    pub name: BbStr,
    /// `BB_I18N_INT` or `BB_I18N_TEXT`.
    pub kind: u32,
    /// Padding, zero.
    pub _pad: u32,
    /// `BB_I18N_INT`: the number.
    pub int: i64,
    /// `BB_I18N_TEXT`: the text.
    pub text: BbStr,
}

/// `text`: message `id` of the calling mod's catalogue, formatted with `arg_count` variables at `args`, written to `sink`
/// during the call. A message no language has is written as its id. `args` may be null when `arg_count` is 0.
pub type BbI18nTextFn = unsafe extern "C" fn(
    ctx: *mut c_void,
    id: BbStr,
    args: *const BbI18nArg,
    arg_count: u64,
    sink: Option<BbSinkFn>,
    sink_user: *mut c_void,
) -> BbStatus;
/// `locale`: the tag of the language in use (`en`, `de`, `pt-BR`), written to `sink` during the call.
pub type BbI18nLocaleFn =
    unsafe extern "C" fn(ctx: *mut c_void, sink: Option<BbSinkFn>, sink_user: *mut c_void) -> BbStatus;
/// `generation`: how many times the language has changed, written to `out`.
pub type BbI18nGenerationFn = unsafe extern "C" fn(ctx: *mut c_void, out: *mut u64) -> BbStatus;

/// The `bb.i18n` table. Field order is part of the ABI; new fields are appended.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct BbI18n {
    /// Size, major and minor version.
    pub head: BbIface,
    /// The host's pointer for this table, first argument of every function.
    pub ctx: *mut c_void,
    /// Format one of the mod's messages.
    pub text: Option<BbI18nTextFn>,
    /// The language in use.
    pub locale: Option<BbI18nLocaleFn>,
    /// The language change counter.
    pub generation: Option<BbI18nGenerationFn>,
}
