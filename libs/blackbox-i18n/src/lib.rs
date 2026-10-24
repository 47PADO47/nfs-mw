//! Translations for a game rewrite: the player's locale, the rewrite's own Fluent catalogues, and the game's language
//! tables, answered by one [`Strings`] lookup. Spec: `docs/specs/localization.md`, decision: ADR 0012.
//!
//! Two kinds of key reach the screen. A **game label** is the hash of a label of the game's string tables; it is answered by
//! the [`GameLayer`]s. A **message** is a Fluent message id in a namespace (the rewrite's own, or one per mod); it is answered
//! by the [`Catalog`]. Both try each locale of the [`Locale::chain`] in turn, and in each locale the layers in the order the
//! caller gave them, so a language pack placed before the game's table overrides it for that locale only.
//!
//! The crate takes text and bytes and never opens a file.

mod args;
mod catalog;
mod game;
mod locale;
pub mod pseudo;
mod strings;

pub use args::{Arg, Args};
pub use catalog::{Catalog, Problem, Source};
pub use game::GameLayer;
pub use locale::{Locale, negotiate, system_locales};
pub use strings::Strings;
