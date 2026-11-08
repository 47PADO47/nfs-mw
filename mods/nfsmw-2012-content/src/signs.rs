//! The 2012 game's road signs and traffic lights as variants of the 2005 world's own (`SIGNS/MW2012/` and `LIGHTS/MW2012/` in the
//! mod's `files/`, and the values **NFS Most Wanted 2012** of the gameplay settings "Road signs" and "Traffic light models").
//!
//! The 2012 world is made of models whose names are not kept (they are hashes), so every model was found by looking at it
//! ([`table`]): the stop sign, the speed limit signs, the do-not-enter and tow-zone signs, the traffic light's mast arm and its signal
//! head. Each is read out of the install by ranges (only the model's resources and its texture are read), turned and fitted into the box
//! of the solid of the 2005 world it takes the place of ([`geometry`]), and written with the pictures it uses ([`build`]). The sets
//! are declared through the replacement API (`docs/modding/api-replace.md`): two manifests in the mod's `replace/` folder, and no code
//! in the game. What the 2012 files hold and how the models were told apart: `docs/formats/chameleon-signs.md`.
//!
//! The setting `signs` (default `true`) turns the conversion, and the values, off.

mod build;
mod geometry;
#[cfg(test)]
mod real_tests;
mod table;
#[cfg(test)]
mod tests;

use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Pool};

use crate::config::Config;
use crate::source::Source;
use crate::trees::ranges::{self, FileRanges};
use crate::trees::read::{self, TreeModel};
use chameleon_bundle::Ranges;
use table::{GLOBAL, SIGNS};

/// The largest side of a picture of these models: a sign or a signal head is seen from a few metres, and the 2012 pictures run up to
/// 1,024 pixels for a plate of half a metre (the `max_texture_size` key may lower it further).
const TEXTURE_LIMIT: u32 = 512;

/// The id of the value in both options.
const VALUE: &str = table::VALUE;

/// Read the 2012 signs and traffic lights of the install and write the two sets under `out`. A failure only costs the set that failed:
/// the game's own signs and lights stay the only choice. Does nothing when the setting `signs` is off.
pub fn write(source: &Source, config: &Config, out: &Path) {
    if !config.signs {
        return;
    }
    let started = Instant::now();
    let size = config.max_texture_size.min(TEXTURE_LIMIT);
    let mut readers = Vec::new();
    let signs = read_signs(source, &mut readers);
    let signs_pack = build::signs(&signs, size);
    let lights_pack = match read_lights(source, &mut readers) {
        Ok((arm, head)) => build::lights(&arm, &head, size),
        Err(error) => Err(error),
    };
    let taken: u64 = readers.iter().map(|reader| reader.bytes_read()).sum();
    let total: u64 = readers.iter().map(|reader| reader.size()).sum();
    for (pack, folder, what) in
        [(signs_pack, table::SIGNS_FOLDER, "road signs"), (lights_pack, table::LIGHTS_FOLDER, "traffic lights")]
    {
        if let Err(error) = pack.and_then(|pack| save(&pack, folder, out, what)) {
            log::warn!("{what}: the variant is not written: {error:#}");
        }
    }
    log::info!(
        "signs: read {} KiB of {} KiB of the bundles, {} ms",
        taken / 1024,
        total / 1024,
        started.elapsed().as_millis()
    );
}

/// Write one set: its files first, then the manifest, so a host that reads the manifest finds the files in place.
fn save(pack: &build::Pack, folder: &str, out: &Path, what: &str) -> Result<()> {
    let files = out.join("files").join(folder);
    std::fs::create_dir_all(&files).with_context(|| format!("creating {}", files.display()))?;
    for (name, bytes) in [(table::GEOMETRY_FILE, &pack.geometry), (table::TEXTURES_FILE, &pack.textures)] {
        std::fs::write(files.join(name), bytes).with_context(|| format!("writing {name}"))?;
    }
    let replace = out.join(blackbox_modding::content::REPLACE_FOLDER);
    std::fs::create_dir_all(&replace).with_context(|| format!("creating {}", replace.display()))?;
    let name = format!("{}-{}.toml", folder.split('/').next().unwrap_or("set").to_ascii_lowercase(), VALUE);
    std::fs::write(replace.join(name), pack.manifest.as_bytes()).context("writing the replacement set")?;
    log::info!(
        "{what}: {} solids replaced, {} KiB of models, {} KiB of textures",
        pack.swaps,
        pack.geometry.len() / 1024,
        pack.textures.len() / 1024
    );
    Ok(())
}

/// The bundle at `rel`, read by ranges; its reader is added to `readers`, which count what is read.
fn open(source: &Source, rel: &str, readers: &mut Vec<Arc<FileRanges>>) -> Result<Bundle<'static>> {
    let (bundle, reader) = ranges::open(&source.root().join(rel))?;
    readers.push(reader);
    Ok(bundle)
}

/// The model of `spec`, with the global resources and the shaders behind its bundle.
fn read_model(source: &Source, spec: table::Source, readers: &mut Vec<Arc<FileRanges>>) -> Result<TreeModel> {
    let mut pool = Pool::new();
    pool.add(open(source, spec.bundle, readers)?);
    if spec.bundle != GLOBAL {
        pool.add(open(source, GLOBAL, readers)?);
    }
    pool.add(Bundle::parse(source.shared_bundles().1)?);
    read::read_model(&pool, spec.model).with_context(|| format!("the model {:#x} of {}", spec.model, spec.bundle))
}

fn read_signs(source: &Source, readers: &mut Vec<Arc<FileRanges>>) -> HashMap<&'static str, TreeModel> {
    let mut found = HashMap::new();
    for sign in &SIGNS {
        match read_model(source, sign.source, readers) {
            Ok(model) => {
                found.insert(sign.key, model);
            }
            Err(error) => log::warn!("signs: the {} cannot be read: {error:#}", sign.label),
        }
    }
    found
}

fn read_lights(source: &Source, readers: &mut Vec<Arc<FileRanges>>) -> Result<(TreeModel, TreeModel)> {
    let arm = read_model(source, table::ARM, readers).context("the traffic light's mast arm")?;
    let head = read_model(source, table::HEAD, readers).context("the traffic light's signal head")?;
    if arm.levels.is_empty() || head.levels.is_empty() {
        bail!("a traffic light model has no level of detail");
    }
    Ok((arm, head))
}
