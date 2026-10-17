//! The mod manager: the mods the Mods page lists, and the on/off switch of each one (docs/specs/mod-manager.md).
//!
//! A switch is saved in the config file and takes effect at the next start: a normal run loads and unloads mods only at
//! start and exit (ADR 0005 section 3), so a mod that is switched off keeps running until the game restarts. In `all`
//! mode the switch edits the `mods_disabled` list; in `allowlist` mode it edits `mods_allowlist`, because there a mod
//! loads only when it is listed.

use std::cell::RefCell;
use std::path::PathBuf;

use super::policy;
use crate::settings::mods::{ALLOWLIST_KEY, DISABLED_KEY, ModsMode, ModsSettings, write_list};

/// One mod the manager lists: its id, whether it is loaded in this run, and whether the next start will load it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub id: String,
    pub loaded: bool,
    pub enabled: bool,
}

/// The listed mods and the settings their switches edit.
#[derive(Debug, Default)]
pub struct ModManager {
    /// The config file the switches are saved to. `None` when the game has no config folder: the switches then cannot
    /// be saved, and the Mods page says so.
    path: Option<PathBuf>,
    settings: RefCell<ModsSettings>,
    /// The mods of this run: `(id, loaded)`, loaded ones first, then the refused ones that have an id.
    ids: Vec<(String, bool)>,
}

impl ModManager {
    /// The manager for the mods `loaded` and `rejected` (the refused ids). A refused id that is also loaded is listed once.
    pub fn new(settings: ModsSettings, path: Option<PathBuf>, loaded: &[String], rejected: &[String]) -> Self {
        let mut ids: Vec<(String, bool)> = loaded.iter().map(|id| (id.clone(), true)).collect();
        for id in rejected {
            if !ids.iter().any(|(known, _)| known == id) {
                ids.push((id.clone(), false));
            }
        }
        Self { path, settings: RefCell::new(settings), ids }
    }

    /// The listed mods, in the order they were loaded, then the refused ones.
    pub fn rows(&self) -> Vec<Row> {
        let settings = self.settings.borrow();
        let policy = policy::policy(&settings);
        self.ids
            .iter()
            .map(|(id, loaded)| Row { id: id.clone(), loaded: *loaded, enabled: policy.allows(id) })
            .collect()
    }

    /// Whether any mod is listed.
    pub fn is_empty(&self) -> bool {
        self.ids.is_empty()
    }

    /// Flip the switch of the row at `index`, save it, and return the switch's new state. Nothing changes when the
    /// index is out of range, the mods are off, or the config file cannot be written.
    pub fn toggle(&self, index: usize) -> Result<bool, String> {
        let Some((id, _)) = self.ids.get(index) else {
            return Err(format!("no mod at row {index}"));
        };
        let current = self.settings.borrow().clone();
        let enable = !policy::policy(&current).allows(id);
        let Some((key, next)) = switched(&current, id, enable) else {
            return Err("the mods are off in the settings, so there is nothing to switch".to_owned());
        };
        let path = self.path.as_deref().ok_or("the game has no config folder to save the switch to")?;
        let list = match key {
            DISABLED_KEY => &next.disabled,
            _ => &next.allowlist,
        };
        write_list(path, key, list).map_err(|e| format!("{e:#}"))?;
        *self.settings.borrow_mut() = next;
        Ok(enable)
    }
}

/// The settings after the mod `id` is switched on (`enable`) or off, with the config key the switch is saved under.
/// `None` in `off` mode, where a switch means nothing.
pub fn switched(settings: &ModsSettings, id: &str, enable: bool) -> Option<(&'static str, ModsSettings)> {
    let mut next = settings.clone();
    match settings.mode {
        ModsMode::Off => None,
        ModsMode::All => {
            next.disabled.retain(|listed| listed != id);
            if !enable {
                next.disabled.push(id.to_owned());
            }
            Some((DISABLED_KEY, next))
        }
        ModsMode::Allowlist => {
            next.allowlist.retain(|listed| listed != id);
            if enable {
                next.allowlist.push(id.to_owned());
            }
            Some((ALLOWLIST_KEY, next))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings(mode: ModsMode) -> ModsSettings {
        ModsSettings { mode, allowlist: vec!["on.mod".into()], disabled: vec!["off.mod".into()] }
    }

    #[test]
    fn in_all_mode_a_switch_edits_the_disabled_list() {
        let (key, next) = switched(&settings(ModsMode::All), "x.mod", false).expect("a switch");
        assert_eq!(key, DISABLED_KEY);
        assert_eq!(next.disabled, ["off.mod", "x.mod"]);
        let (_, back) = switched(&next, "off.mod", true).expect("a switch");
        assert_eq!(back.disabled, ["x.mod"], "switching on removes the id");
        assert_eq!(back.allowlist, ["on.mod"], "the allowlist is not touched");
    }

    #[test]
    fn in_allowlist_mode_a_switch_edits_the_allowlist() {
        let (key, next) = switched(&settings(ModsMode::Allowlist), "x.mod", true).expect("a switch");
        assert_eq!(key, ALLOWLIST_KEY);
        assert_eq!(next.allowlist, ["on.mod", "x.mod"]);
        let (_, off) = switched(&next, "on.mod", false).expect("a switch");
        assert_eq!(off.allowlist, ["x.mod"]);
    }

    #[test]
    fn off_mode_has_nothing_to_switch() {
        assert!(switched(&settings(ModsMode::Off), "x.mod", true).is_none());
    }

    #[test]
    fn a_switch_is_not_repeated_in_its_list() {
        let (_, once) = switched(&settings(ModsMode::All), "x.mod", false).expect("a switch");
        let (_, twice) = switched(&once, "x.mod", false).expect("a switch");
        assert_eq!(twice.disabled.iter().filter(|id| *id == "x.mod").count(), 1);
    }

    #[test]
    fn the_rows_show_loaded_and_refused_mods_once_each() {
        let loaded = ["a.mod".to_owned(), "b.mod".to_owned()];
        let rejected = ["b.mod".to_owned(), "off.mod".to_owned()];
        let manager = ModManager::new(settings(ModsMode::All), None, &loaded, &rejected);
        let rows = manager.rows();
        let ids: Vec<&str> = rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["a.mod", "b.mod", "off.mod"]);
        assert!(rows[0].loaded && rows[0].enabled);
        assert!(!rows[2].loaded && !rows[2].enabled, "off.mod is in the disabled list");
    }

    #[test]
    fn a_switch_without_a_config_folder_changes_nothing() {
        let manager = ModManager::new(settings(ModsMode::All), None, &["a.mod".to_owned()], &[]);
        assert!(manager.toggle(0).is_err());
        assert!(manager.rows()[0].enabled, "the failed switch leaves the state as it was");
        assert!(manager.toggle(9).is_err(), "an unknown row changes nothing");
    }

    #[test]
    fn a_switch_is_saved_and_the_run_keeps_the_mod_until_the_next_start() {
        let scratch = std::env::temp_dir().join(format!("nfsmw-manager-{}", std::process::id()));
        let path = scratch.join("settings.toml");
        let manager = ModManager::new(ModsSettings::default(), Some(path.clone()), &["a.mod".to_owned()], &[]);
        assert_eq!(manager.toggle(0), Ok(false));
        let saved = crate::settings::mods::read_list(&path, DISABLED_KEY);
        assert_eq!(saved, ["a.mod"], "the file now lists the switched-off mod");
        let row = &manager.rows()[0];
        assert!(!row.enabled && row.loaded, "the run still has the mod loaded");
        let _ = std::fs::remove_dir_all(scratch);
    }
}
