//! The replacement set that offers the textures to the game (`docs/modding/api-replace.md`) and the stubs of the files it names.
//!
//! The set is one file, `replace/road_textures-mw2012.toml`, of kind `road_textures`: the game adds the option **Road textures**
//! to the Gameplay options with the values *Original* and *NFS Most Wanted 2012*. Each `[[texture]]` names a texture of the 2005
//! world by its hash and a `.dds` under `files/REPLACE/ROADTEX/`. Those files are lazy files ([`crate::lazy`]): the mod writes a
//! stub of a few dozen bytes and the game has [`super::Maker`] make the picture from the 2012 install when it first loads a
//! section that uses the texture.

use std::fmt::Write;
use std::fs;
use std::path::Path;

use anyhow::{Context, Result};

use super::table;

/// The folder of the sets under the mod, and the file this one is.
pub const REPLACE_FOLDER: &str = "replace";
pub const MANIFEST: &str = "road_textures-mw2012.toml";
/// Where the pictures are, under `files/`.
pub const FOLDER: &str = "REPLACE/ROADTEX";
/// The kind of the set, the value that chooses it, and the texts of the option (message ids of the mod's catalogue).
pub const KIND: &str = "road_textures";
pub const VALUE: &str = "mw2012";
pub const KIND_LABEL: &str = "nfsmw2012-road-textures";
pub const VALUE_LABEL: &str = "nfsmw2012-road-textures-value";
/// What a lazy file's key starts with.
pub const KEY_PREFIX: &str = "road/";

/// The key of the picture of the texture with name hash `hash`.
pub fn key(hash: u32) -> String {
    format!("{KEY_PREFIX}{hash:08X}")
}

/// The hash a key names, when it is a key of this module.
pub fn parse_key(key: &str) -> Option<u32> {
    let digits = key.strip_prefix(KEY_PREFIX)?;
    if digits.len() != 8 {
        return None;
    }
    u32::from_str_radix(digits, 16).ok()
}

/// The path of a picture under `files/`.
pub fn file_path(hash: u32) -> String {
    format!("{FOLDER}/{hash:08X}.dds")
}

/// The text of the set's manifest.
pub fn text() -> String {
    let mut out = format!(
        "# The textures of NFS Most Wanted (2012) for the 2005 world (nfsmw-2012-content, road_textures.rs).\n\
         format = 1\n\n[set]\nkind = \"{KIND}\"\nkind_label = \"{KIND_LABEL}\"\nvalue = \"{VALUE}\"\nvalue_label = \"{VALUE_LABEL}\"\n"
    );
    for entry in table::entries() {
        let _ = write!(out, "\n[[texture]]\ntarget_hash = {}\nfile = \"{}\"\n", entry.hash, file_path(entry.hash));
    }
    out
}

/// Write `path` unless it already holds exactly `bytes` (a start that changes nothing writes nothing).
fn write_if_changed(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::read(path).is_ok_and(|have| have == bytes) {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
    }
    fs::write(path, bytes).with_context(|| format!("writing {}", path.display()))
}

/// Write the set under the mod folder `dir` (or the staging folder that becomes it): the pictures' stubs first, the manifest last,
/// so a game that reads it finds the files in place.
pub fn write(dir: &Path) -> Result<()> {
    let folder = dir.join("files").join(FOLDER);
    let stale = stale_files(&folder);
    for entry in table::entries() {
        let name = format!("{:08X}.dds", entry.hash);
        write_if_changed(&folder.join(&name), &crate::lazy::stub(&key(entry.hash)))?;
    }
    for path in stale {
        let _ = fs::remove_file(path);
    }
    write_if_changed(&dir.join(REPLACE_FOLDER).join(MANIFEST), text().as_bytes())
}

/// The files of the pictures' folder that no row names any more.
fn stale_files(folder: &Path) -> Vec<std::path::PathBuf> {
    let known: Vec<String> = table::entries().map(|entry| format!("{:08X}.DDS", entry.hash)).collect();
    let Ok(listing) = fs::read_dir(folder) else { return Vec::new() };
    listing
        .flatten()
        .filter(|item| !known.contains(&item.file_name().to_string_lossy().to_ascii_uppercase()))
        .map(|item| item.path())
        .collect()
}

/// Take the set away: the manifest first (the option goes), then the stubs.
pub fn remove(dir: &Path) {
    let _ = fs::remove_file(dir.join(REPLACE_FOLDER).join(MANIFEST));
    let _ = fs::remove_dir_all(dir.join("files").join(FOLDER));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_key_names_its_hash_and_only_its_own_keys_parse() {
        assert_eq!(key(0x1A2B_3C4D), "road/1A2B3C4D");
        assert_eq!(parse_key("road/1A2B3C4D"), Some(0x1A2B_3C4D));
        assert_eq!(parse_key("car/POR911/GEOMETRY.BIN"), None);
        assert_eq!(parse_key("road/1234"), None);
        assert_eq!(parse_key("road/ZZZZZZZZ"), None);
    }

    #[test]
    fn the_manifest_is_a_set_the_game_accepts() {
        let set = blackbox_modding::content::parse_replace_set("road_textures-mw2012", &text()).unwrap();
        assert_eq!(set.kind, KIND);
        assert_eq!(set.value, VALUE);
        assert_eq!(set.texture_entries.len(), table::entries().count());
        assert!(set.models.is_empty());
    }

    #[test]
    fn the_stubs_are_written_once_and_taken_away_with_the_set() {
        let dir = std::env::temp_dir().join(format!("nfsmw-roadtex-manifest-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        write(&dir).unwrap();
        let first = table::entries().next().unwrap().hash;
        let stub = fs::read(dir.join("files").join(file_path(first))).unwrap();
        assert!(stub.starts_with(b"BBLAZY1"), "a lazy file's stub");
        assert!(dir.join(REPLACE_FOLDER).join(MANIFEST).is_file());
        fs::write(dir.join("files").join(FOLDER).join("OLD.dds"), b"x").unwrap();
        write(&dir).unwrap();
        assert!(!dir.join("files").join(FOLDER).join("OLD.dds").exists(), "a file no row names is removed");
        assert!(dir.join("files").join(file_path(first)).is_file(), "a file a row names stays");
        remove(&dir);
        assert!(!dir.join(REPLACE_FOLDER).join(MANIFEST).exists());
        assert!(!dir.join("files").join(FOLDER).exists());
        let _ = fs::remove_dir_all(&dir);
    }
}
