//! The licence plate texts the player typed, kept beside the save of the profile.
//!
//! A mod car's plate (`[customization.plate]`) shows the text of its manifest until the player types another. The text is
//! the player's, not a part the shop sells, and the original game's save has no place for it, so it is kept in the
//! profile's own mod data folder (`plates.toml`, next to `my-cars.toml`; a guest has `plates-guest.toml` in the settings
//! folder), keyed by the car's folder. The original save is untouched: a profile without the file wears the manifest texts.
//! One text belongs to a car model, not to each copy of it in the garage.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};

/// The file's table: car folder in upper case, to the text.
const TABLE: &str = "plates";

/// A plates file read: where, when it was written, and what it holds.
type Cached = (PathBuf, Option<SystemTime>, BTreeMap<String, String>);

/// The plates file read last.
static CACHE: Mutex<Option<Cached>> = Mutex::new(None);

/// The plates file of the current profile, beside its My Cars slots.
fn path() -> Option<PathBuf> {
    let slots = super::my_cars::path().ok()?;
    let name = match slots.file_name()?.to_str()? {
        "my-cars.toml" => "plates.toml",
        _ => "plates-guest.toml",
    };
    Some(slots.with_file_name(name))
}

/// The texts in the file at `path` (none when it is missing or does not read).
fn read(path: &Path) -> BTreeMap<String, String> {
    let Ok(text) = std::fs::read_to_string(path) else { return BTreeMap::new() };
    let Ok(file) = text.parse::<toml::Table>() else {
        log::warn!("{}: the plate texts do not read", path.display());
        return BTreeMap::new();
    };
    let Some(table) = file.get(TABLE).and_then(toml::Value::as_table) else { return BTreeMap::new() };
    table.iter().filter_map(|(folder, text)| Some((folder.clone(), text.as_str()?.to_owned()))).collect()
}

fn written(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

/// The text the player kept for the car in `folder`; `None` when there is none (the car wears its manifest's text).
pub(crate) fn text_for(folder: &str) -> Option<String> {
    let path = path()?;
    let stamp = written(&path);
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let fresh = cache.as_ref().is_some_and(|(known, when, _)| *known == path && *when == stamp);
    if !fresh {
        *cache = Some((path.clone(), stamp, read(&path)));
    }
    cache.as_ref()?.2.get(&folder.to_ascii_uppercase()).cloned()
}

/// Keep `text` for the car in `folder`, or forget it (`None`: the car wears its manifest's text again).
pub(crate) fn set(folder: &str, text: Option<&str>) -> Result<()> {
    let path = path().context("No profile folder for the plate texts")?;
    let mut texts = read(&path);
    let key = folder.to_ascii_uppercase();
    match text {
        Some(text) => texts.insert(key, text.to_owned()),
        None => texts.remove(&key),
    };
    write(&path, &texts)?;
    // The next read sees the file as written now.
    *CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    Ok(())
}

/// The file replaced atomically, as the My Cars slots are.
fn write(path: &Path, texts: &BTreeMap<String, String>) -> Result<()> {
    let table: toml::Table =
        texts.iter().map(|(folder, text)| (folder.clone(), toml::Value::String(text.clone()))).collect();
    let mut file = toml::Table::new();
    file.insert(TABLE.into(), toml::Value::Table(table));
    let text = toml::to_string(&file).context("Serializing the plate texts")?;
    let parent = path.parent().context("Missing plate text folder")?;
    std::fs::create_dir_all(parent).context("Creating the plate text folder")?;
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, text).context("Staging the plate texts")?;
    std::fs::rename(temporary, path).context("Replacing the plate texts")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("nfsmw-plates-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("plates.toml")
    }

    #[test]
    fn texts_written_are_read_back_by_folder() {
        let file = temp("round-trip");
        let mut texts = BTreeMap::new();
        texts.insert("POR911".to_owned(), "MY PLATE".to_owned());
        texts.insert("BMWM3".to_owned(), "NFS HERO".to_owned());
        write(&file, &texts).unwrap();
        assert_eq!(read(&file), texts);
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }

    #[test]
    fn a_missing_or_broken_file_holds_no_text() {
        let file = temp("broken");
        assert!(read(&file).is_empty());
        std::fs::write(&file, "plates = [").unwrap();
        assert!(read(&file).is_empty());
        std::fs::write(&file, "[plates]\nA = 5\nB = \"X\"\n").unwrap();
        assert_eq!(read(&file).get("B").map(String::as_str), Some("X"), "a value that is no text is skipped");
        assert_eq!(read(&file).len(), 1);
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }

    #[test]
    fn the_file_is_replaced_whole() {
        let file = temp("replace");
        write(&file, &BTreeMap::from([("A".to_owned(), "ONE".to_owned())])).unwrap();
        write(&file, &BTreeMap::from([("B".to_owned(), "TWO".to_owned())])).unwrap();
        assert_eq!(read(&file), BTreeMap::from([("B".to_owned(), "TWO".to_owned())]));
        assert!(!file.with_extension("toml.tmp").exists());
        std::fs::remove_dir_all(file.parent().unwrap()).unwrap();
    }
}
