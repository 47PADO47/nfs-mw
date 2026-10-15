//! The alias manager's rows, on the list the main menu's Alias Manager icon opens (docs/specs/save-alias-manager.md).
//! Neither row changes a setting: a press saves or loads an alias, and `widget_menu.rs` sends the command for it.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasSetting {
    /// Saves the current settings as a new alias (`Command::SaveAlias`, `flow.rs`).
    Save,
    /// Loads the latest alias (`Command::LoadAlias`, `flow.rs`).
    Load,
}

impl AliasSetting {
    pub const ALL: [Self; 2] = [Self::Save, Self::Load];

    pub fn title(self) -> &'static str {
        match self {
            Self::Save => "Save Alias",
            Self::Load => "Load Alias",
        }
    }

    /// The word the row's data shows: what a press does.
    pub fn data(self) -> &'static str {
        match self {
            Self::Save => "Save",
            Self::Load => "Load",
        }
    }
}
