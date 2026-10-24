//! `bb.i18n`: a mod's own messages in the current language (`crate::i18n`, the mod's `lang/` folder).

use blackbox_modding::host::{BackendResult, I18nArg, I18nBackend};

use crate::i18n::{self, Args};

/// The `bb.i18n` port of the game. It needs no state of its own: the language is process-wide.
pub struct I18nPort;

impl I18nBackend for I18nPort {
    fn text(&self, owner: &str, id: &str, args: &[(String, I18nArg)]) -> BackendResult<String> {
        let mut values = Args::new();
        for (name, value) in args {
            match value {
                I18nArg::Int(n) => values.set(name, *n),
                I18nArg::Text(text) => values.set(name, text.as_str()),
            }
        }
        Ok(i18n::mod_text(owner, id, &values))
    }

    fn locale(&self, _owner: &str) -> BackendResult<String> {
        Ok(i18n::current().locale.to_string())
    }

    fn generation(&self, _owner: &str) -> BackendResult<u64> {
        Ok(i18n::generation())
    }
}
