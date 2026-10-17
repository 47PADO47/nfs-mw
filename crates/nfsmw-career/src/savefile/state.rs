//! What saving changes, the alias rules and the manual save prompts
//! (`docs/specs/career-save-screens.md` sections 2, 5 and 6, `docs/specs/career-save-screens-2.md` section 12).

use super::autosave::AutosaveManager;

/// Longest alias kept by the library: 31 characters (`strncpy` into 32 bytes).
pub const ALIAS_MAX_CHARS: usize = 31;
/// Aliases listed by the screens are shorter than this many characters.
pub const LISTABLE_ALIAS_LIMIT: usize = 8;
/// The label of the default name of player 1 when the alias is empty.
pub const DEFAULT_NAME_PLAYER_1: &str = "COMMON_PLAYER_1";
/// The label of the default name of player 2.
pub const DEFAULT_NAME_PLAYER_2: &str = "COMMON_PLAYER_2";

/// The alias as the library keeps it: cut to 31 characters.
pub fn truncate_alias(alias: &str) -> String {
    alias.chars().take(ALIAS_MAX_CHARS).collect()
}

/// The file path of an alias under a root: `<root>/<alias>/<alias>` with forward slashes. The alias is cut to 31
/// characters. The root is the documents folder plus the game folder name (`NFS Most Wanted`, which a mod can
/// change); the caller builds it.
pub fn alias_file_path(root: &str, alias: &str) -> String {
    let alias = truncate_alias(alias);
    format!("{root}/{alias}/{alias}")
}

/// The folder of an alias: `<root>/<alias>`.
pub fn alias_dir_path(root: &str, alias: &str) -> String {
    format!("{root}/{}", truncate_alias(alias))
}

/// The listing rule: of every entry of the root (folders and files, not `.` and `..`) the game keeps only names
/// shorter than 8 characters and without a space. This hides `Settings.ini`, and an alias of 8 or more characters or
/// with a space can never be loaded or deleted from the screens.
pub fn is_listable_alias(name: &str) -> bool {
    name.chars().count() < LISTABLE_ALIAS_LIMIT && !name.contains(' ')
}

/// Filters a directory listing with [`is_listable_alias`], keeping the order.
pub fn list_aliases<'a>(entries: impl IntoIterator<Item = &'a str>) -> Vec<&'a str> {
    entries.into_iter().filter(|name| is_listable_alias(name)).collect()
}

/// The profile database bytes that a save touches (`FEDatabase +3`, `+4` and the name fields of the profile).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileState {
    /// The profile name (`+0x00`).
    pub name: String,
    /// The custom-name byte (`+0x20`): 1 for a typed name, 0 for a default label.
    pub custom_name: bool,
    /// `FEDatabase +3`: an alias is active.
    pub loaded: bool,
    /// `FEDatabase +4`: unsaved changes (set by the option screens, cleared by a save).
    pub unsaved_changes: bool,
}

impl ProfileState {
    /// A fresh profile with no alias.
    pub fn new() -> ProfileState {
        ProfileState { name: String::new(), custom_name: false, loaded: false, unsaved_changes: false }
    }

    /// Stores the alias as the profile name (manual save and create only). An empty name becomes the label
    /// `COMMON_PLAYER_1` (or `_2` for the second player) with the custom-name byte 0; otherwise the byte is 1.
    pub fn store_name(&mut self, alias: &str, second_player: bool) {
        if alias.is_empty() {
            self.name = match second_player {
                true => DEFAULT_NAME_PLAYER_2.to_string(),
                false => DEFAULT_NAME_PLAYER_1.to_string(),
            };
            self.custom_name = false;
            return;
        }
        self.name = truncate_alias(alias);
        self.custom_name = true;
    }

    /// The completion callback shared by the manual and the background save: profile loaded becomes 1, unsaved
    /// changes 0 and the manager clears "switched off by error". Nothing else is changed: the autosave option is
    /// left alone, and no counter, flag, respawn key or stage is written.
    pub fn save_completed(&mut self, manager: &mut AutosaveManager) {
        self.loaded = true;
        self.unsaved_changes = false;
        manager.write_completed();
    }

    /// A successful first-player load: profile loaded is 1 and "switched off by error" is cleared.
    pub fn load_succeeded(&mut self, manager: &mut AutosaveManager) {
        self.loaded = true;
        manager.off_by_error = false;
    }

    /// Create: the typed name is stored and *profile loaded* is set **before** anything is written. A cancelled
    /// create after this leaves *loaded* = 1 with an unsaved default profile.
    pub fn create_named(&mut self, alias: &str) {
        self.store_name(alias, false);
        self.loaded = true;
    }

    /// Delete finished for `deleted`: when it equals the loaded profile name (exact, case-sensitive) the profile is
    /// reset to a fresh default and *loaded* becomes 0. Returns whether the profile was reset.
    pub fn alias_deleted(&mut self, deleted: &str) -> bool {
        if self.name != deleted {
            return false;
        }
        *self = ProfileState::new();
        true
    }
}

impl Default for ProfileState {
    fn default() -> Self {
        ProfileState::new()
    }
}

/// The panels of a manual save, in the order the library raises them. **Reconstructed** from the library state
/// machine; the order was not seen in game (the spec says so). A rewrite can show them in any equivalent way.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SavePanel {
    /// "Are you sure you want to save the alias named X?" (`MC_CONFIRM_SAVE_THE_PROFILE`, No focused).
    Confirm,
    /// The overwrite question (library prompt 7, No / Yes), when the alias folder already exists.
    Overwrite,
    /// "Save complete" (library prompt 8, Continue).
    Complete,
    /// "Unable to save X" (library prompt 9, Continue), after an open failure.
    UnableToSave,
}

/// The panels a manual save shows: the confirmation, the overwrite question for an existing alias, then the
/// result. An overwrite answered No ends the operation as cancelled with no result panel.
pub fn manual_save_panels(alias_exists: bool, overwrite_confirmed: bool, open_failed: bool) -> Vec<SavePanel> {
    let mut panels = vec![SavePanel::Confirm];
    if alias_exists {
        panels.push(SavePanel::Overwrite);
        if !overwrite_confirmed {
            return panels;
        }
    }
    panels.push(match open_failed {
        true => SavePanel::UnableToSave,
        false => SavePanel::Complete,
    });
    panels
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths_use_forward_slashes_and_the_alias_twice() {
        assert_eq!(
            alias_file_path("C:/Users/p/Documents/NFS Most Wanted", "mwo"),
            "C:/Users/p/Documents/NFS Most Wanted/mwo/mwo"
        );
        assert_eq!(alias_dir_path("root", "stock"), "root/stock");
    }

    #[test]
    fn the_alias_is_cut_to_31_characters() {
        let long = "a".repeat(40);
        assert_eq!(truncate_alias(&long).chars().count(), 31);
        assert_eq!(alias_file_path("r", &long), format!("r/{0}/{0}", "a".repeat(31)));
        assert_eq!(truncate_alias("short"), "short");
    }

    #[test]
    fn only_short_names_without_a_space_are_listed() {
        assert!(is_listable_alias("NAME"));
        assert!(is_listable_alias("stock"));
        assert!(is_listable_alias("1234567"), "seven characters");
        assert!(!is_listable_alias("12345678"), "eight characters");
        assert!(!is_listable_alias("my name"));
        assert!(!is_listable_alias("Settings.ini"));
        assert!(is_listable_alias(""));
        assert_eq!(list_aliases(["NAME", "Settings.ini", "a b", "mwo"]), ["NAME", "mwo"]);
    }

    #[test]
    fn an_empty_name_gets_the_default_label() {
        let mut p = ProfileState::new();
        p.store_name("", false);
        assert_eq!((p.name.as_str(), p.custom_name), ("COMMON_PLAYER_1", false));
        p.store_name("", true);
        assert_eq!(p.name, "COMMON_PLAYER_2");
        p.store_name("Pietro", false);
        assert_eq!((p.name.as_str(), p.custom_name), ("Pietro", true));
    }

    #[test]
    fn saving_changes_exactly_the_documented_bytes() {
        let mut p = ProfileState { name: "x".into(), custom_name: true, loaded: false, unsaved_changes: true };
        let mut m = AutosaveManager { off_by_error: true, option_on: false, ..AutosaveManager::default() };
        p.save_completed(&mut m);
        assert!(p.loaded && !p.unsaved_changes);
        assert!(!m.off_by_error);
        assert!(!m.option_on, "the autosave option is unchanged by a successful save");
        assert_eq!(p.name, "x");
    }

    #[test]
    fn a_load_sets_loaded_and_clears_the_error_byte() {
        let mut p = ProfileState::new();
        let mut m = AutosaveManager { off_by_error: true, ..AutosaveManager::default() };
        p.load_succeeded(&mut m);
        assert!(p.loaded && !m.off_by_error);
    }

    #[test]
    fn a_cancelled_create_leaves_loaded_set() {
        let mut p = ProfileState::new();
        p.create_named("fresh");
        assert!(p.loaded);
        assert_eq!(p.name, "fresh");
    }

    #[test]
    fn deleting_the_loaded_alias_resets_the_profile_exactly_by_name() {
        let mut p = ProfileState { name: "Abc".into(), custom_name: true, loaded: true, unsaved_changes: true };
        assert!(!p.alias_deleted("abc"), "case sensitive");
        assert!(p.loaded);
        assert!(p.alias_deleted("Abc"));
        assert_eq!(p, ProfileState::new());
    }

    #[test]
    fn the_manual_panels_follow_the_reconstructed_order() {
        use SavePanel::*;
        assert_eq!(manual_save_panels(false, false, false), [Confirm, Complete]);
        assert_eq!(manual_save_panels(true, true, false), [Confirm, Overwrite, Complete]);
        assert_eq!(manual_save_panels(true, false, false), [Confirm, Overwrite], "No cancels");
        assert_eq!(manual_save_panels(false, false, true), [Confirm, UnableToSave]);
        assert_eq!(manual_save_panels(true, true, true), [Confirm, Overwrite, UnableToSave]);
    }
}
