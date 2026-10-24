//! The game's language: which one runs, and the text of every game label and catalogue message in it. Spec:
//! `docs/specs/localization.md`, decision: ADR 0012.
//!
//! The current language is process-wide, like the custom fonts' state: [`current`] is answered from anywhere, and a change
//! ([`switch`]) bumps [`generation`] so that screens holding resolved text lay it out again. Before the install is opened
//! (tools, tests) it is English from the built-in catalogue, with no game table.
//!
//! - [`t!`] and [`text`]: a message of the rewrite's catalogue (`crates/nfsmw/i18n/<locale>/*.ftl`).
//! - [`game`]: a game label from the language's table (or a language pack).
//! - [`mod_text`]: a message of a mod's own catalogue.

mod build;
mod catalog;
pub mod fmt;
pub mod matching;
mod mods;
mod select;
mod state;

pub use blackbox_i18n::{Arg, Args, Locale};
pub use build::Active;
pub use select::Choice;
pub use state::{choices, current, generation, start, switch};

/// The namespace of the rewrite's own messages.
pub const CORE: &str = "core";

/// The label of the original's "default string error" text, shown for a game label no language has.
const DEFAULT_STRING_ERROR: u32 = 0x9BB9_CCC3;

/// The namespace of a mod's own messages.
pub fn mod_namespace(id: &str) -> String {
    format!("mod:{id}")
}

/// A message of the rewrite's catalogue in the current language; its id when no language has it.
pub fn text(id: &str, args: &Args) -> String {
    current().strings.text(CORE, id, args)
}

/// A message of mod `id`'s catalogue in the current language; the message id when no language has it.
pub fn mod_text(mod_id: &str, id: &str, args: &Args) -> String {
    current().strings.text(&mod_namespace(mod_id), id, args)
}

/// Text a mod supplied (a menu label or choice, a theme's name, a HUD message, a content file's text): the mod's own
/// message of that id when its catalogue has one, else the text as written. A mod names a message where it would write
/// text, and a mod with no catalogue keeps working.
pub fn mod_label(mod_id: &str, text: &str) -> String {
    current().strings.try_text(&mod_namespace(mod_id), text, &Args::new()).unwrap_or_else(|| text.to_owned())
}

/// The text of a game label in the current language; `None` when no language of the chain has it.
pub fn game(label: u32) -> Option<String> {
    current().strings.game(label)
}

/// The text of a game label, or the original's default-string error text when no language has it.
pub fn game_or_error(label: u32) -> String {
    let strings = &current().strings;
    strings.game(label).or_else(|| strings.game(DEFAULT_STRING_ERROR)).unwrap_or_default()
}

/// A message of the rewrite's catalogue: `t!("back")`, `t!("lan-votes", n = 3)`. The id is a literal so that
/// `cargo xtask i18n-check` can find it.
macro_rules! t {
    ($id:literal) => {
        $crate::i18n::text($id, &$crate::i18n::Args::new())
    };
    ($id:literal, $($name:ident = $value:expr),+ $(,)?) => {
        $crate::i18n::text($id, &$crate::i18n::Args::new()$(.with(stringify!($name), $value))+)
    };
}
pub(crate) use t;

#[cfg(test)]
mod tests;
