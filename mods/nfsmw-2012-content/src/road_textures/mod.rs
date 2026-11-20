//! The road, ground and sign textures of NFS: Most Wanted (2012) as a replacement of the 2005 world's (`docs/modding/road-textures.md`).
//!
//! The 2005 world is textured by name: every texture is a record of a streamed section, found by the hash of its name. The 2012
//! game keeps its shared world textures in `HAWAII/GLOBALRESOURCES.BNDL`: asphalt, concrete slabs, grass, road paint, the STOP and
//! speed limit signs, the covers and grates. [`table`] says which 2005 texture each look replaces, [`recipe`] and [`layer`] make the
//! picture from the 2012 pictures (a crop, a quarter turn, a resize, a mask), and [`manifest`] hands the game one replacement set
//! of kind `road_textures`, which the game shows as the Gameplay option **Road textures** (`road_textures_variant`).
//!
//! **Streamed, not converted.** The mod writes stubs of a few dozen bytes. When the game first loads a section that holds one of the
//! textures it asks [`Maker`] for the file; the maker reads the 2012 textures it needs from the install by byte range ([`source`]),
//! makes the picture and gives it back. Nothing of the 2012 game is copied to the disk.
//!
//! The setting `road_textures` (default `true`) turns the set, and with it the option, on or off.

mod dds;
pub mod layer;
mod manifest;
pub mod recipe;
mod resample;
mod source;
pub mod table;
#[cfg(test)]
mod tests;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock, PoisonError};
use std::time::Instant;

use anyhow::{Context, Result};
use blackbox_modding::sdk::{Error, ModResult};
use serde::Deserialize;

pub use manifest::{KEY_PREFIX, REPLACE_FOLDER, parse_key};
pub use source::Sources;

/// The file next to the import stamp that tells the maker where the 2012 install is.
pub const RECIPE_FILE: &str = "lazy-roads.toml";
/// The format number of that file.
const FORMAT: u32 = 1;

/// Write the set into the mod folder `dir` (or the staging folder that becomes it).
pub fn write(dir: &Path) -> Result<()> {
    manifest::write(dir)
}

/// Keep the set in line with the settings at every start: written for the install at `source` when `enabled`, taken away when not.
/// A failure is logged and costs only the set.
pub fn sync(mod_dir: &Path, source: Option<&Path>, enabled: bool) {
    let result = match (enabled, source) {
        (true, Some(source)) => write(mod_dir).and_then(|()| write_recipe(mod_dir, source)),
        _ => {
            manifest::remove(mod_dir);
            let _ = std::fs::remove_file(mod_dir.join(RECIPE_FILE));
            Ok(())
        }
    };
    if let Err(error) = result {
        log::warn!("nfsmw-2012-content: the road textures are not set up: {error:#}");
    }
}

#[derive(Deserialize)]
struct Recipe {
    format: u32,
    source: PathBuf,
}

fn write_recipe(dir: &Path, source: &Path) -> Result<()> {
    let source = std::path::absolute(source).unwrap_or_else(|_| source.to_owned());
    let quoted = source.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    let text = format!(
        "# Where the road textures are made from (road_textures/). Written at every start; do not edit.\nformat = {FORMAT}\nsource = \"{quoted}\"\n"
    );
    let path = dir.join(RECIPE_FILE);
    if std::fs::read_to_string(&path).is_ok_and(|have| have == text) {
        return Ok(());
    }
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))
}

/// Makes the pictures of the lazy files of the set.
pub struct Maker {
    source: PathBuf,
    sources: OnceLock<std::result::Result<Sources, String>>,
    /// How long each picture took, in milliseconds, for the log and the measuring tests.
    timings: Mutex<Vec<(u32, f32)>>,
}

impl Maker {
    pub fn new(source: PathBuf) -> Self {
        Self { source, sources: OnceLock::new(), timings: Mutex::new(Vec::new()) }
    }

    /// The maker the mod folder `dir` asks for, when its recipe file is there and reads.
    pub fn read(dir: &Path) -> Option<Self> {
        let text = std::fs::read_to_string(dir.join(RECIPE_FILE)).ok()?;
        let recipe: Recipe = toml::from_str(&text).map_err(|error| log::warn!("{RECIPE_FILE}: {error}")).ok()?;
        (recipe.format == FORMAT).then(|| Self::new(recipe.source))
    }

    /// The 2012 install the pictures are made from.
    pub fn source(&self) -> &Path {
        &self.source
    }

    /// How long each picture took so far, as `(name hash, milliseconds)`.
    pub fn timings(&self) -> Vec<(u32, f32)> {
        self.timings.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    fn pictures(&self) -> ModResult<&Sources> {
        let opened = self.sources.get_or_init(|| Sources::open(&self.source).map_err(|error| format!("{error:#}")));
        opened.as_ref().map_err(|reason| {
            log::warn!("nfsmw-2012-content: road textures: {reason}");
            Error::NotFound
        })
    }

    /// The file of the lazy key `key`: the `.dds` of one replacement.
    pub fn read_key(&self, key: &str) -> ModResult<Vec<u8>> {
        let hash = parse_key(key).ok_or(Error::Arg)?;
        let entry = table::find(hash).ok_or(Error::NotFound)?;
        let pictures = self.pictures()?;
        let started = Instant::now();
        let made = entry.dds(pictures).map_err(|error| {
            log::warn!("nfsmw-2012-content: the road texture {} is not made: {error:#}", entry.name);
            Error::Host
        })?;
        let millis = started.elapsed().as_secs_f32() * 1000.0;
        log::debug!(
            "nfsmw-2012-content: road texture {} ({hash:08X}) made in {millis:.0} ms, {} KiB ({} KiB of the install read so far)",
            entry.name,
            made.len() / 1024,
            pictures.bytes_read() / 1024
        );
        self.timings.lock().unwrap_or_else(PoisonError::into_inner).push((hash, millis));
        Ok(made)
    }
}
