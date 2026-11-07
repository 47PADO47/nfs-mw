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
mod light;
mod read;
#[cfg(test)]
mod real_tests;
pub mod species;
pub mod stock;
#[cfg(test)]
mod tests;

use std::path::Path;

use anyhow::{Context, Result, bail};
use chameleon_bundle::{Bundle, Pool};

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
    let models = read_models(source);
    if models.is_empty() {
        bail!("none of the trees could be read");
    }
    let pack = build::build(&models, config.max_texture_size)?;
    let folder = out.join("files").join(FOLDER);
    std::fs::create_dir_all(&folder).with_context(|| format!("creating {}", folder.display()))?;
    for (file, bytes) in [
        (build::GEOMETRY_FILE, pack.geometry.as_slice()),
        (build::TEXTURES_FILE, pack.textures.as_slice()),
        ("profile.toml", pack.profile.as_bytes()),
    ] {
        std::fs::write(folder.join(file), bytes).with_context(|| format!("writing {file}"))?;
    }
    let names: Vec<String> = pack.species.iter().map(|(key, levels)| format!("{key} ({levels})")).collect();
    log::info!("trees variant: {}", names.join(", "));
    Ok(())
}

/// The species the install holds, read bundle by bundle (each bundle once). A species that cannot be read is left out with a
/// warning.
fn read_models(source: &Source) -> Vec<(&'static Species, TreeModel)> {
    let shaders = source.shared_bundles().1;
    let global = source.read_file(GLOBAL_BUNDLE).map_err(|error| log::warn!("trees: {error:#}")).ok();
    let mut bundles: Vec<&str> = SPECIES.iter().map(|species| species.bundle).collect();
    bundles.sort_unstable();
    bundles.dedup();
    let mut found = Vec::new();
    for bundle in bundles {
        let is_global = bundle == GLOBAL_BUNDLE;
        let own = match is_global {
            true => None,
            false => source.read_file(bundle).map_err(|error| log::warn!("trees: {error:#}")).ok(),
        };
        let first = match is_global {
            true => global.as_deref(),
            false => own.as_deref(),
        };
        let Some(first) = first else { continue };
        let behind = match is_global {
            true => None,
            false => global.as_deref(),
        };
        let wanted: Vec<&'static Species> = SPECIES.iter().filter(|species| species.bundle == bundle).collect();
        match read_from(first, behind, shaders, &wanted) {
            Ok(models) => found.extend(models),
            Err(error) => log::warn!("trees: {bundle} cannot be read: {error:#}"),
        }
    }
    found.sort_by_key(|(species, _)| SPECIES.iter().position(|known| known.key == species.key));
    found
}

/// The models of `wanted` out of the bundle `first`, with the shared bundle `behind` it (when there is one) and the shaders.
fn read_from(
    first: &[u8],
    behind: Option<&[u8]>,
    shaders: &[u8],
    wanted: &[&'static Species],
) -> Result<Vec<(&'static Species, TreeModel)>> {
    let mut pool = Pool::new();
    pool.add(Bundle::parse(first)?);
    if let Some(behind) = behind {
        pool.add(Bundle::parse(behind)?);
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
