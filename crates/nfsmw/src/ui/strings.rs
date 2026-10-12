//! Translated game strings for any screen or dialog: a label hash in, the text of the loaded language out.
//! The original looks labels up the same way (`0x0056BB80` in `speed.exe`, see docs/specs/confirmation-dialogs.md). // leak-check: allow

use super::UiAssets;

/// The text of a label in the loaded language table; `None` when the table is missing or has no such label.
pub fn game_string(assets: &UiAssets, label: u32) -> Option<String> {
    assets.strings.as_ref()?.get(label)
}

/// Text a screen can show: either written by the caller or looked up from the game's language table by label.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Text {
    /// Shown exactly as given. Callers that bring their own wording use it; the game's own dialogs do not.
    #[allow(dead_code)]
    Custom(String),
    /// The label hash (`blackbox_text::label_hash` of the English key) of a game string.
    Game(u32),
}

impl Text {
    /// The text to show; `None` only for a game label the table does not have.
    pub fn resolve(&self, assets: &UiAssets) -> Option<String> {
        match self {
            Self::Custom(text) => Some(text.clone()),
            Self::Game(label) => game_string(assets, *label),
        }
    }
}
