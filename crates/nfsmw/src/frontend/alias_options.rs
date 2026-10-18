//! The alias manager's rows, on the list the main menu's Alias Manager icon opens (docs/specs/save-alias-manager.md).
//! Save and Load act on the settings alias. The three career rows open the career profile list (`MC_List.fng`, load or
//! delete) or make a new career. The rows change no setting: `commands` gives what a press sends, in order.

use crate::frontend::career_screens::save::{ListMode, list_aliases, next_alias, set_list_mode};
use crate::frontend::ids::screen;
use crate::frontend::logic::{Args, Command};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AliasSetting {
    /// Saves the current settings as a new alias (`Command::SaveAlias`, `flow.rs`).
    Save,
    /// Loads the latest alias (`Command::LoadAlias`, `flow.rs`).
    Load,
    /// Opens the career profile list in load mode; a pick plays that career (`save.rs`, `Command::CareerLoad`).
    LoadCareer,
    /// Opens the career profile list in delete mode; a pick asks to delete that career (`Command::CareerDelete`).
    DeleteCareer,
    /// Makes a career under the first free name and plays it (`Command::CareerNew`, `flow_save.rs`).
    NewCareer,
}

impl AliasSetting {
    pub const ALL: [Self; 5] = [Self::Save, Self::Load, Self::LoadCareer, Self::DeleteCareer, Self::NewCareer];

    /// The row's title. The career rows' titles are plain text: the spec gives no language label for them.
    pub fn title(self) -> &'static str {
        match self {
            Self::Save => "Save Alias",
            Self::Load => "Load Alias",
            Self::LoadCareer => "Load Career",
            Self::DeleteCareer => "Delete Career",
            Self::NewCareer => "New Career",
        }
    }

    /// The word the row's data shows: what a press does.
    pub fn data(self) -> &'static str {
        match self {
            Self::Save => "Save",
            Self::Load => "Load",
            Self::LoadCareer => "Load",
            Self::DeleteCareer => "Delete",
            Self::NewCareer => "New",
        }
    }

    /// The commands a press sends, in order. Opening a list sets its mode first, since a screen gets no data.
    pub fn commands(self) -> Vec<Command> {
        match self {
            Self::Save => vec![Command::SaveAlias],
            Self::Load => vec![Command::LoadAlias],
            Self::LoadCareer => open_list(ListMode::Load),
            Self::DeleteCareer => open_list(ListMode::Delete),
            Self::NewCareer => vec![new_career(&list_aliases())],
        }
    }
}

/// Sets the list's mode and pushes the profile list over this screen.
fn open_list(mode: ListMode) -> Vec<Command> {
    set_list_mode(mode);
    vec![Command::Push(screen::MC_LIST.into(), Args::default())]
}

/// The new-career command for the careers already in the folder: the first free name (`save.rs`, `next_alias`).
fn new_career(taken: &[String]) -> Command {
    Command::CareerNew(next_alias(taken))
}

#[cfg(test)]
#[path = "alias_options_tests.rs"]
mod tests;
