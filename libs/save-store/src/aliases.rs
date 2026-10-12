//! The alias index: friendly names for slots, kept in `aliases.toml` next to the saves.

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use toml::{Table, Value};

use super::store::{SaveStore, StoreError, check_slot_name};

const FILE: &str = "aliases.toml";

/// Alias to slot. An alias is any text; the slot it points at must be a valid slot name.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Aliases {
    map: BTreeMap<String, String>,
}

impl Aliases {
    /// The index in `store`'s folder. A missing file is an empty index; a file that does not parse is an error, so
    /// it is never overwritten by accident.
    pub fn load(store: &SaveStore) -> Result<Self, AliasError> {
        let path = store.dir().join(FILE);
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(source) => return Err(StoreError::Io { path, source }.into()),
        };
        let table: Table = text.parse().map_err(|_| AliasError::Corrupt)?;
        let mut map = BTreeMap::new();
        for (alias, value) in table {
            let Value::String(slot) = value else { return Err(AliasError::Corrupt) };
            map.insert(alias, slot);
        }
        Ok(Self { map })
    }

    /// Writes the index to the store's folder. The file is replaced in one step, as slots are.
    pub fn save(&self, store: &SaveStore) -> Result<(), AliasError> {
        let table: Table = self.map.iter().map(|(a, s)| (a.clone(), Value::String(s.clone()))).collect();
        let text = toml::to_string(&table).map_err(|_| AliasError::Corrupt)?;
        let dir = store.dir();
        let path: PathBuf = dir.join(FILE);
        let temp = dir.join(format!("{FILE}.tmp"));
        fs::create_dir_all(dir).map_err(|source| StoreError::Io { path: dir.to_owned(), source })?;
        fs::write(&temp, text).map_err(|source| StoreError::Io { path: temp.clone(), source })?;
        fs::rename(&temp, &path).map_err(|source| StoreError::Io { path, source })?;
        Ok(())
    }

    /// The slot an alias points at.
    pub fn slot(&self, alias: &str) -> Option<&str> {
        self.map.get(alias).map(String::as_str)
    }

    /// Points an alias at a slot, replacing an earlier target. The slot name is checked.
    pub fn assign(&mut self, alias: &str, slot: &str) -> Result<(), StoreError> {
        check_slot_name(slot)?;
        self.map.insert(alias.to_owned(), slot.to_owned());
        Ok(())
    }

    /// Removes an alias (the slot itself stays). Returns the slot it pointed at.
    pub fn unassign(&mut self, alias: &str) -> Option<String> {
        self.map.remove(alias)
    }

    /// Every alias and its slot, sorted by alias.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.map.iter().map(|(a, s)| (a.as_str(), s.as_str()))
    }
}

/// Why the alias index could not be read or written.
#[derive(Debug, thiserror::Error)]
pub enum AliasError {
    #[error("the alias index is not a readable table of alias = slot pairs")]
    Corrupt,
    #[error(transparent)]
    Store(#[from] StoreError),
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(name: &str) -> PathBuf {
        std::env::temp_dir().join(format!("save-store-alias-{name}-{}", std::process::id()))
    }

    #[test]
    fn aliases_round_trip_through_the_folder() {
        let dir = scratch("round");
        let _ = fs::remove_dir_all(&dir);
        let store = SaveStore::new(&dir);
        let mut aliases = Aliases::load(&store).unwrap();
        assert_eq!(aliases.iter().count(), 0);
        aliases.assign("My Career", "career_1").unwrap();
        aliases.assign("Test run", "slot-2").unwrap();
        aliases.save(&store).unwrap();
        let back = Aliases::load(&store).unwrap();
        assert_eq!(back.slot("My Career"), Some("career_1"));
        assert_eq!(back.iter().count(), 2);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_invalid_slot_is_refused_and_unassign_returns_the_old_one() {
        let mut aliases = Aliases::default();
        assert!(aliases.assign("x", "../bad").is_err());
        aliases.assign("x", "good").unwrap();
        assert_eq!(aliases.unassign("x").as_deref(), Some("good"));
        assert_eq!(aliases.slot("x"), None);
    }

    #[test]
    fn a_corrupt_index_is_an_error_not_an_empty_one() {
        let dir = scratch("corrupt");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(FILE), "not = [valid").unwrap();
        assert!(matches!(Aliases::load(&SaveStore::new(&dir)), Err(AliasError::Corrupt)));
        let _ = fs::remove_dir_all(&dir);
    }
}
