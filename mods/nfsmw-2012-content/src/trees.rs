//! The trees of the 2012 game as a variant of the world's trees (`TREES/MW2012/` in the mod's `files/` and the value **NFS Most
//! Wanted 2012** of the gameplay setting "Trees").
//!
//! The 2012 game's trees are models of its world bundles: a trunk and branches, and cards of leaves with a cut-out texture,
//! at four levels of detail. This reads eleven of them ([`species::SPECIES`]) and writes them as solids and textures of the
//! 2005 formats, with a profile that lists which solids of the 2005 game's tree scenery each replaces (the list is
//! [`stock::FAMILIES`]). The game swaps them in as it streams the world in, fitted into the bounds of the solid each replaces
//! (`docs/modding/effect-variants.md`, "Profile format of the trees"). What the 2012 files hold, and how the trees were found:
//! `docs/formats/chameleon-vegetation.md`.
//!
//! The setting `trees` (default `true`) turns the conversion, and the value in the gameplay setting, off.

mod build;
pub mod bushes;
#[cfg(test)]
mod bushes_tests;
pub(crate) mod light;
mod manifest;
pub(crate) mod ranges;
pub(crate) mod read;
#[cfg(test)]
mod real_tests;
pub mod species;
pub mod stock;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Pool, Ranges};

use crate::config::Config;
use crate::source::Source;
use read::TreeModel;
use species::{GLOBAL_BUNDLE, SPECIES, Species};

/// The id of the variant in the `trees_variant` option, and what the menu shows for it.
pub const VARIANT: &str = "mw2012";
pub const LABEL: &str = "NFS Most Wanted 2012";
/// The folder of the variant under the mod's `files/`.
pub const FOLDER: &str = build::FOLDER;

/// Read the 2012 trees of the install and write the variant under `out/files/TREES/MW2012/`. A failure only costs the variant:
/// the game's own trees stay the only choice. Does nothing when the setting `trees` is off.
pub fn write(source: &Source, config: &Config, out: &Path) {
    if !config.trees {
        return;
    }
    if let Err(error) = try_write(source, config, out) {
        log::warn!("the trees variant is not written: {error:#}");
    }
}

fn try_write(source: &Source, config: &Config, out: &Path) -> Result<()> {
    let started = Instant::now();
    let (models, taken) = read_models(source);
    if models.is_empty() {
        bail!("none of the trees could be read");
    }
    let pack = build::build(&models, config.max_texture_size)?;
    let profile = manifest::profile_text(&sets(config)[0].1, &pack.species);
    let folder = out.join("files").join(FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    let mut written = 0;
    for (file, bytes) in [
        (build::GEOMETRY_FILE, pack.geometry.as_slice()),
        (build::TEXTURES_FILE, pack.textures.as_slice()),
        ("profile.toml", profile.as_bytes()),
    ] {
        std::fs::write(folder.join(file), bytes).with_context(|| format!("writing {file}"))?;
        written += bytes.len();
    }
    // The sets declared through the one replacement API, written last so that a host that reads them finds the files in place: the
    // trees, and the bushes when they are a set of their own (`bushes.rs`).
    let replace_folder = out.join(blackbox_modding::content::REPLACE_FOLDER);
    std::fs::create_dir_all(&replace_folder).with_context(|| format!("creating {}", replace_folder.display()))?;
    for (file, set) in sets(config) {
        let text = manifest::replace_manifest(&set, &pack.species);
        std::fs::write(replace_folder.join(file), text.as_bytes()).with_context(|| format!("writing {file}"))?;
    }
    let names: Vec<String> = pack.species.iter().map(|(key, levels)| format!("{key} ({levels})")).collect();
    log::info!("trees variant: {}", names.join(", "));
    log::info!(
        "trees variant: read {} KiB of {} KiB of the bundles, wrote {} KiB ({} KiB of textures, {} KiB of models), {} ms",
        taken.read / 1024,
        taken.total / 1024,
        written / 1024,
        pack.textures.len() / 1024,
        pack.geometry.len() / 1024,
        started.elapsed().as_millis()
    );
    Ok(())
}

/// The sets the conversion declares, each with the file of its manifest in `replace/`: the trees, which are also the profile the
/// older hosts read, and the bushes when the setting `bushes` is on (`bushes.rs`). The trees come first.
fn sets(config: &Config) -> Vec<(&'static str, manifest::Set<'static>)> {
    let mut sets = vec![("trees-mw2012.toml", trees_set(config.bushes))];
    if config.bushes {
        sets.push(("bushes-mw2012.toml", bushes::set()));
    }
    sets
}

/// The trees set: the families of `stock::tree_families` (the old round bushes among them unless `bushes_apart`).
pub fn trees_set(bushes_apart: bool) -> manifest::Set<'static> {
    manifest::Set {
        kind: "trees",
        kind_label: "Trees",
        value: VARIANT,
        value_label: LABEL,
        limits: manifest::TREE_LIMITS,
        families: stock::tree_families(bushes_apart).collect(),
    }
}

/// What reading the species cost: the bytes asked of the bundles' files and the size of those files.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Taken {
    pub read: u64,
    pub total: u64,
}

/// The bundle at `rel` under the install, read by ranges; its reader is added to `readers`, which count what is read. A bundle that
/// cannot be opened is a warning and `None`.
fn open_bundle(source: &Source, rel: &str, readers: &mut Vec<Arc<ranges::FileRanges>>) -> Option<Bundle<'static>> {
    match ranges::open(&source.root().join(rel)) {
        Ok((bundle, reader)) => {
            readers.push(reader);
            Some(bundle)
        }
        Err(error) => {
            log::warn!("trees: {error:#}");
            None
        }
    }
}

/// The species the install holds, read bundle by bundle (each bundle once, by ranges: only the resources a tree needs are read).
/// A species that cannot be read is left out with a warning. Also what the reading cost.
fn read_models(source: &Source) -> (Vec<(&'static Species, TreeModel)>, Taken) {
    let shaders = source.shared_bundles().1;
    let mut readers = Vec::new();
    let global = open_bundle(source, GLOBAL_BUNDLE, &mut readers);
    let mut bundles: Vec<&str> = SPECIES.iter().map(|species| species.bundle).collect();
    bundles.sort_unstable();
    bundles.dedup();
    let mut found = Vec::new();
    for bundle in bundles {
        let is_global = bundle == GLOBAL_BUNDLE;
        let first = match is_global {
            true => global.clone(),
            false => open_bundle(source, bundle, &mut readers),
        };
        let Some(first) = first else { continue };
        let behind = match is_global {
            true => None,
            false => global.clone(),
        };
        let wanted: Vec<&'static Species> = SPECIES.iter().filter(|species| species.bundle == bundle).collect();
        match read_from(first, behind, shaders, &wanted) {
            Ok(models) => found.extend(models),
            Err(error) => log::warn!("trees: {bundle} cannot be read: {error:#}"),
        }
    }
    found.sort_by_key(|(species, _)| SPECIES.iter().position(|known| known.key == species.key));
    let taken = Taken {
        read: readers.iter().map(|reader| reader.bytes_read()).sum(),
        total: readers.iter().map(|reader| reader.size()).sum(),
    };
    (found, taken)
}

/// The models of `wanted` out of the bundle `first`, with the shared bundle `behind` it (when there is one) and the shaders.
fn read_from(
    first: Bundle<'_>,
    behind: Option<Bundle<'_>>,
    shaders: &[u8],
    wanted: &[&'static Species],
) -> Result<Vec<(&'static Species, TreeModel)>> {
    let mut pool = Pool::new();
    pool.add(first);
    if let Some(behind) = behind {
        pool.add(behind);
    }
    pool.add(Bundle::parse(shaders)?);
    let mut models = Vec::new();
    for &species in wanted {
        match read::read_model(&pool, species.model) {
            Ok(model) => models.push((species, model)),
            Err(error) => log::warn!("trees: the {} ({}) cannot be read: {error:#}", species.key, species.label),
        }
    }
    Ok(models)
}
