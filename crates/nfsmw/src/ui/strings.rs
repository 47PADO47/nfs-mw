//! Text a screen can show, in the current language (`crate::i18n`): a game label, a catalogue message, or text that is
//! not language at all (a name, a chat line).
//! The original looks labels up the same way (`0x0056BB80` in `speed.exe`, see docs/specs/confirmation-dialogs.md). // leak-check: allow

use crate::i18n::{self, Args};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Text {
    /// Shown exactly as given: names, chat lines, room names, file names. Never words of the interface.
    Literal(String),
    /// The label hash (`blackbox_text::label_hash` of the English key) of a game string.
    Game(u32),
    /// A message of the rewrite's catalogue, with its variables.
    Key(&'static str, Args),
}

impl Text {
    /// A catalogue message without variables.
    pub fn key(id: &'static str) -> Self {
        Self::Key(id, Args::new())
    }

    /// The text to show; `None` only for a game label no language has.
    pub fn resolve(&self) -> Option<String> {
        match self {
            Self::Literal(text) => Some(text.clone()),
            Self::Game(label) => i18n::game(*label),
            Self::Key(id, args) => Some(i18n::text(id, args)),
        }
    }

    /// The text to show; for a game label no language has, the original's default-string error text.
    pub fn show(&self) -> String {
        match self {
            Self::Game(label) => i18n::game_or_error(*label),
            other => other.resolve().unwrap_or_default(),
        }
    }
}
