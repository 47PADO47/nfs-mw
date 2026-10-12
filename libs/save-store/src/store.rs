//! Reading, writing and listing save slots.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The extension of a slot file.
pub const EXTENSION: &str = "sav";
const MAX_SLOT_LEN: usize = 64;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("{0:?} is not a valid slot name (letters, digits, '-' and '_', 1 to 64 characters)")]
    InvalidSlot(String),
    #[error("{}: {source}", path.display())]
    Io { path: PathBuf, source: io::Error },
}

/// A slot name is letters, digits, `-` and `_`, up to 64 characters.
pub fn check_slot_name(slot: &str) -> Result<(), StoreError> {
    let valid = !slot.is_empty()
        && slot.len() <= MAX_SLOT_LEN
        && slot.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_');
    match valid {
        true => Ok(()),
        false => Err(StoreError::InvalidSlot(slot.to_owned())),
    }
}

/// A folder of save slots.
#[derive(Clone, Debug)]
pub struct SaveStore {
    dir: PathBuf,
}

impl SaveStore {
    /// A store in `dir`. The folder is created on the first write.
    pub fn new(dir: impl Into<PathBuf>) -> Self {
        Self { dir: dir.into() }
    }

    pub fn dir(&self) -> &Path {
        &self.dir
    }

    fn path(&self, slot: &str) -> Result<PathBuf, StoreError> {
        check_slot_name(slot)?;
        Ok(self.dir.join(format!("{slot}.{EXTENSION}")))
    }

    /// The bytes of a slot, or `None` when it has not been written.
    pub fn read(&self, slot: &str) -> Result<Option<Vec<u8>>, StoreError> {
        let path = self.path(slot)?;
        match fs::read(&path) {
            Ok(bytes) => Ok(Some(bytes)),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }

    /// Replaces a slot with `bytes`. The new bytes land in a temporary file first and then take the slot's name, so
    /// the old save survives a failed write.
    pub fn write(&self, slot: &str, bytes: &[u8]) -> Result<(), StoreError> {
        let path = self.path(slot)?;
        let temp = path.with_extension(format!("{EXTENSION}.tmp"));
        fs::create_dir_all(&self.dir).map_err(|source| StoreError::Io { path: self.dir.clone(), source })?;
        fs::write(&temp, bytes).map_err(|source| StoreError::Io { path: temp.clone(), source })?;
        fs::rename(&temp, &path).map_err(|source| StoreError::Io { path, source })
    }

    /// Whether the slot has been written.
    pub fn exists(&self, slot: &str) -> Result<bool, StoreError> {
        Ok(self.path(slot)?.is_file())
    }

    /// The names of the written slots, sorted. A folder that does not exist yet has none.
    pub fn list(&self) -> Result<Vec<String>, StoreError> {
        let entries = match fs::read_dir(&self.dir) {
            Ok(entries) => entries,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(source) => return Err(StoreError::Io { path: self.dir.clone(), source }),
        };
        let mut slots = Vec::new();
        for entry in entries {
            let entry = entry.map_err(|source| StoreError::Io { path: self.dir.clone(), source })?;
            let path = entry.path();
            let is_slot = path.extension().is_some_and(|e| e == EXTENSION);
            let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else { continue };
            if is_slot && check_slot_name(stem).is_ok() {
                slots.push(stem.to_owned());
            }
        }
        slots.sort();
        Ok(slots)
    }

    /// Deletes a slot. A slot that is not there is not an error.
    pub fn remove(&self, slot: &str) -> Result<(), StoreError> {
        let path = self.path(slot)?;
        match fs::remove_file(&path) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
            Err(source) => Err(StoreError::Io { path, source }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A fresh folder under the system temp directory, unique to this test run.
    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("save-store-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn slot_names_are_checked() {
        assert!(check_slot_name("career_1").is_ok());
        assert!(check_slot_name("slot-2").is_ok());
        for bad in ["", "../x", "a/b", "a.b", "con x", &"a".repeat(65)] {
            assert!(check_slot_name(bad).is_err(), "{bad:?} should be refused");
        }
    }

    #[test]
    fn a_write_reads_back_and_lists() {
        let dir = scratch("roundtrip");
        let store = SaveStore::new(&dir);
        assert_eq!(store.read("career").unwrap(), None);
        assert!(store.list().unwrap().is_empty());
        store.write("career", b"one").unwrap();
        store.write("b", b"two").unwrap();
        store.write("career", b"three").unwrap();
        assert_eq!(store.read("career").unwrap().as_deref(), Some(&b"three"[..]));
        assert_eq!(store.list().unwrap(), vec!["b".to_owned(), "career".to_owned()]);
        assert!(store.exists("b").unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_deletes_and_is_idempotent() {
        let dir = scratch("remove");
        let store = SaveStore::new(&dir);
        store.write("a", b"x").unwrap();
        store.remove("a").unwrap();
        store.remove("a").unwrap();
        assert!(!store.exists("a").unwrap());
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_bad_name_never_touches_the_disk() {
        let dir = scratch("bad-name");
        let store = SaveStore::new(&dir);
        assert!(matches!(store.write("../escape", b"x"), Err(StoreError::InvalidSlot(_))));
        assert!(!dir.exists());
    }
}
