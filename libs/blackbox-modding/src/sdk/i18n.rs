//! `I18n`: the mod's own messages in the player's language (catalogue: docs/modding/translations.md).
//!
//! Text a mod hands the host to keep (a menu row's label, a theme's name, a content file's text) can simply be one of the
//! mod's message ids: the host shows that message, in the current language, every time it draws it. `I18n` is for text
//! the mod builds itself, such as a HUD message with numbers in it.

use super::ModResult;
use super::ffi::i18n::{self as host_i18n, I18nFns, Value};

/// The `bb.i18n` interface of the host, as a mod uses it. Obtained from `Ctx::i18n`.
#[derive(Clone, Copy)]
pub struct I18n {
    fns: I18nFns,
}

/// A message variable's value: a whole number (it selects plural forms) or text.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Arg<'a> {
    Int(i64),
    Text(&'a str),
}

impl I18n {
    pub(crate) fn new(fns: I18nFns) -> Self {
        I18n { fns }
    }

    /// Message `id` of the mod's `lang/<locale>/*.ftl` in the current language; its id when no language has it.
    pub fn t(&self, id: &str) -> ModResult<String> {
        host_i18n::text(&self.fns, id, &[])
    }

    /// Message `id` with its variables: `i18n.t_args("laps-left", &[("n", Arg::Int(2))])`.
    pub fn t_args(&self, id: &str, args: &[(&str, Arg<'_>)]) -> ModResult<String> {
        let args: Vec<(&str, Value<'_>)> = args
            .iter()
            .map(|(name, arg)| {
                let value = match arg {
                    Arg::Int(n) => Value::Int(*n),
                    Arg::Text(t) => Value::Text(t),
                };
                (*name, value)
            })
            .collect();
        host_i18n::text(&self.fns, id, &args)
    }

    /// The tag of the language in use (`en`, `de`, `pt-BR`).
    pub fn locale(&self) -> ModResult<String> {
        host_i18n::locale(&self.fns)
    }

    /// How many times the language has changed. Text kept from an earlier count is in the old language.
    pub fn generation(&self) -> ModResult<u64> {
        host_i18n::generation(&self.fns)
    }
}
