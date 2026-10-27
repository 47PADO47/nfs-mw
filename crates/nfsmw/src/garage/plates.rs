//! The licence plate texts and designs the player picked, kept beside the save of the profile.
//!
//! A mod car's plate (`[customization.plate]`) shows the text of its manifest until the player types another, and the design
//! of its manifest (if it names one) until the player picks another. Both are the player's, not parts the shop sells, and the
//! original game's save has no place for them, so they are kept in the profile's own mod data folder (`plates.toml`, next to
//! `my-cars.toml`; a guest has `plates-guest.toml` in the settings folder), keyed by the car's folder:
//!
//! ```toml
//! [plates]
//! POR911CARR12 = "MY PLATE"
//!
//! [plate_designs]
//! POR911CARR12 = "1863272"   # the empty id is the plain plate, picked on purpose
//! ```
//!
//! The original save is untouched: a profile without the file wears the manifest texts and designs, and a file without the
//! second table (written before designs existed) reads as before. One text and one design belong to a car model, not to each
//! copy of it in the garage.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::SystemTime;

use anyhow::{Context, Result};

/// The file's tables: car folder in upper case, to the text, and to the design id.
const TEXTS: &str = "plates";
const DESIGNS: &str = "plate_designs";

/// What the plates file holds.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
struct Stored {
    texts: BTreeMap<String, String>,
    designs: BTreeMap<String, String>,
}

/// A plates file read: where, when it was written, and what it holds.
type Cached = (PathBuf, Option<SystemTime>, Stored);

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

/// The strings of the table `name` of `file` (a value that is no string is skipped).
fn strings(file: &toml::Table, name: &str) -> BTreeMap<String, String> {
    let Some(table) = file.get(name).and_then(toml::Value::as_table) else { return BTreeMap::new() };
    table.iter().filter_map(|(folder, text)| Some((folder.clone(), text.as_str()?.to_owned()))).collect()
}

/// What the file at `path` holds (nothing when it is missing or does not read).
fn read(path: &Path) -> Stored {
    let Ok(text) = std::fs::read_to_string(path) else { return Stored::default() };
    let Ok(file) = text.parse::<toml::Table>() else {
        log::warn!("{}: the plate texts do not read", path.display());
        return Stored::default();
    };
    Stored { texts: strings(&file, TEXTS), designs: strings(&file, DESIGNS) }
}

fn written(path: &Path) -> Option<SystemTime> {
    std::fs::metadata(path).and_then(|meta| meta.modified()).ok()
}

/// The entry for `folder` of the table `pick` chooses from what the file holds, read afresh when the file changed.
fn lookup(folder: &str, pick: impl Fn(&Stored) -> &BTreeMap<String, String>) -> Option<String> {
    let path = path()?;
    let stamp = written(&path);
    let mut cache = CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let fresh = cache.as_ref().is_some_and(|(known, when, _)| *known == path && *when == stamp);
    if !fresh {
        *cache = Some((path.clone(), stamp, read(&path)));
    }
    pick(&cache.as_ref()?.2).get(&folder.to_ascii_uppercase()).cloned()
}

/// The text the player kept for the car in `folder`; `None` when there is none (the car wears its manifest's text).
pub(crate) fn text_for(folder: &str) -> Option<String> {
    lookup(folder, |stored| &stored.texts)
}

/// The design the player kept for the car in `folder` (the empty id is the plain plate); `None` when there is none (the car
/// wears its manifest's default design).
pub(crate) fn design_for(folder: &str) -> Option<String> {
    lookup(folder, |stored| &stored.designs)
}

/// Keep `text` for the car in `folder`, or forget it (`None`: the car wears its manifest's text again).
pub(crate) fn set(folder: &str, text: Option<&str>) -> Result<()> {
    change(folder, |stored, key| {
        match text {
            Some(text) => stored.texts.insert(key, text.to_owned()),
            None => stored.texts.remove(&key),
        };
    })
}

/// Keep the design `id` for the car in `folder` (the empty id is the plain plate), or forget it (`None`: the car wears its
/// manifest's default design again).
pub(crate) fn set_design(folder: &str, id: Option<&str>) -> Result<()> {
    change(folder, |stored, key| {
        match id {
            Some(id) => stored.designs.insert(key, id.to_owned()),
            None => stored.designs.remove(&key),
        };
    })
}

/// Read the file, let `edit` change it (given the car's key), and write it back.
fn change(folder: &str, edit: impl FnOnce(&mut Stored, String)) -> Result<()> {
    let path = path().context("No profile folder for the plate texts")?;
    let mut stored = read(&path);
    edit(&mut stored, folder.to_ascii_uppercase());
    write(&path, &stored)?;
    // The next read sees the file as written now.
    *CACHE.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
    Ok(())
}

fn table(entries: &BTreeMap<String, String>) -> toml::Value {
    toml::Value::Table(
        entries.iter().map(|(folder, text)| (folder.clone(), toml::Value::String(text.clone()))).collect(),
    )
}

/// The file replaced atomically, as the My Cars slots are.
fn write(path: &Path, stored: &Stored) -> Result<()> {
    let mut file = toml::Table::new();
    file.insert(TEXTS.into(), table(&stored.texts));
    if !stored.designs.is_empty() {
        file.insert(DESIGNS.into(), table(&stored.designs));
    }
    let text = toml::to_string(&file).context("Serializing the plate texts")?;
    let parent = path.parent().context("Missing plate text folder")?;
    std::fs::create_dir_all(parent).context("Creating the plate text folder")?;
    let temporary = path.with_extension("toml.tmp");
    std::fs::write(&temporary, text).context("Staging the plate texts")?;
    std::fs::rename(temporary, path).context("Replacing the plate texts")
}

#[cfg(test)]
#[path = "plates_tests.rs"]
mod tests;
