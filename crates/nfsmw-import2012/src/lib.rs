//! Adds the cars of NFS Most Wanted (2012) to NFS: Most Wanted (2005) as a content mod.
//!
//! The 2012 game runs on the Chameleon engine, not on the 2005 Black Box engine, so its car models and textures are
//! converted: `chameleon-vehicle` reads a car out of its bundles, this crate fits it to the wheel layout of a stock 2005
//! car, writes it as that car's `GEOMETRY.BIN` and `TEXTURES.BIN`, and lists it in a mod folder the game loads like any
//! other (`docs/modding/api-content.md`). Nothing from either game is stored in the repository: the mod is made on the
//! user's machine from the user's own installs, and without a 2012 install there is nothing to do.

pub mod config;
mod convert;
mod emit;
mod fit;
mod names;
mod source;
mod textures;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use config::Config;
use emit::{CarEntry, CarFiles, ModFolder};
use fit::{StockCar, best_fit, measure};
pub use source::{CarFile, Source};

/// The environment variable that names the 2012 install when the settings do not.
pub const SOURCE_ENV: &str = "MW2012_GAME_DIR";

/// A car the importer found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub number: u32,
    pub id: String,
    pub name: String,
}

/// What a run did.
#[derive(Debug, Default)]
pub struct Report {
    /// The mod folder written.
    pub output: PathBuf,
    pub imported: Vec<ImportedCar>,
    /// Cars left out and why.
    pub skipped: Vec<(String, String)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportedCar {
    pub id: String,
    pub name: String,
    pub clone: String,
    /// Largest stretch the fit applied along any axis (1.0 is none).
    pub stretch: f32,
    /// Triangles of the finest body level.
    pub triangles: usize,
}

/// The 2012 install folder: the settings, then the environment.
pub fn source_dir(config: &Config) -> Option<PathBuf> {
    config.source.clone().or_else(|| std::env::var_os(SOURCE_ENV).map(PathBuf::from))
}

/// Where the settings file lives: next to the game's own `config.toml`.
pub fn default_config_path() -> Option<PathBuf> {
    let game = game_install::config_file_path(&nfsmw_data::game::SPEC)?;
    Some(game.parent()?.join(config::FILE_NAME))
}

/// The folder the mod goes to when the settings name none: `<config dir>/mods/mw2012.cars`.
pub fn default_output() -> Option<PathBuf> {
    let game = game_install::config_file_path(&nfsmw_data::game::SPEC)?;
    Some(game.parent()?.join("mods").join(emit::MOD_ID))
}

/// The cars in the 2012 install, named, in file order.
pub fn list(source: &Source) -> Vec<Listing> {
    let mut taken = Vec::new();
    let mut listings = Vec::new();
    for file in source.cars() {
        let (id, name) = identify(source, file, &mut taken);
        listings.push(Listing { number: file.number, id, name });
    }
    listings
}

fn identify(source: &Source, file: &CarFile, taken: &mut Vec<String>) -> (String, String) {
    let object = source.object_name(file);
    let name = names::car_name(object.as_deref(), file.number);
    let id = names::unique_id(&name.id, taken);
    taken.push(id.clone());
    (id, name.display)
}

/// Import the cars the settings ask for.
pub fn run(config: &Config) -> Result<Report> {
    config.check()?;
    let Some(root) = source_dir(config) else {
        bail!(
            "no NFS Most Wanted (2012) install is set: put `source = '<folder>'` in the settings or set {SOURCE_ENV}"
        );
    };
    let source = Source::open(&root)?;
    let game = nfsmw_data::game::open_install(config.game_dir.as_deref())
        .context("the NFS: Most Wanted (2005) install is needed for the stock cars' wheel layouts")?;
    let stock = fit::stock_cars(&game)?;
    let output =
        config.output.clone().or_else(default_output).context("no output folder: set `output` in the settings")?;
    let folder = ModFolder::create(&output)?;
    let mut report = Report { output, ..Report::default() };
    let mut taken = Vec::new();
    for file in source.cars() {
        let (id, display) = identify(&source, file, &mut taken);
        if !config.wants(&id, &display) {
            continue;
        }
        match import_car(&source, file, &id, &display, config, &stock, &folder) {
            Ok(car) => {
                log::info!(
                    "{id}: {display} on {} (stretch {:.2}, {} triangles)",
                    car.clone,
                    car.stretch,
                    car.triangles
                );
                report.imported.push(car);
            }
            Err(error) => {
                log::warn!("{id}: {display}: {error:#}");
                report.skipped.push((format!("{id} {display}"), format!("{error:#}")));
            }
        }
    }
    folder.finish(report.imported.len())?;
    Ok(report)
}

fn import_car(
    source: &Source,
    file: &CarFile,
    id: &str,
    display: &str,
    config: &Config,
    stock: &[StockCar],
    folder: &ModFolder,
) -> Result<ImportedCar> {
    let car = source.load(file)?;
    let layout = measure(&car.vehicle).context("the wheel layout of the car could not be measured")?;
    let (clone, fit) = best_fit(layout, stock).context("no stock car to clone")?;
    log::debug!("{id}: wheel layout {:?} fitted onto {} {:?}", fit.from, clone.type_name, fit.to);
    let options = convert::Options { max_texture_size: config.max_texture_size };
    let built = convert::build(&car, &fit, &clone.base_model_name.to_ascii_uppercase(), id, &options)?;
    let name = names::car_name(car.object_name.as_deref(), file.number);
    let entry = CarEntry {
        id: id.to_owned(),
        clone: clone.type_name.clone(),
        name: display.to_owned(),
        manufacturer: name.make,
        role: config.role,
    };
    folder.add_car(&entry, &CarFiles { geometry: &built.geometry, textures: &built.textures })?;
    Ok(ImportedCar {
        id: id.to_owned(),
        name: display.to_owned(),
        clone: clone.type_name.clone(),
        stretch: fit.distortion().exp(),
        triangles: built.triangles,
    })
}

/// Write a commented settings file with the defaults to `path`, unless one is there.
pub fn write_default_config(path: &Path) -> Result<bool> {
    if path.exists() {
        return Ok(false);
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, DEFAULT_CONFIG).with_context(|| format!("writing {}", path.display()))?;
    Ok(true)
}

const DEFAULT_CONFIG: &str = "\
# Settings of nfsmw-import2012 (docs/modding/mw2012-cars.md). Every line is optional.

# Folder of your own NFS Most Wanted (2012) install. Without it nothing is imported.
# source = 'D:\\Need For Speed Most Wanted 2012'

# Folder of your NFS: Most Wanted (2005) install, if the game cannot find it itself.
# game_dir = 'D:\\Need For Speed Most Wanted Black Edition'

# Where the mod folder goes. Default: <config folder>/mods/mw2012.cars
# output = 'D:\\mods\\mw2012.cars'

# Which cars: \"all\", or a list of ids or name fragments.
cars = \"all\"
# cars = [\"Porsche\", \"Lamborghini\"]

# Cars to leave out, by id or name fragment.
exclude = []

# Longest texture side in pixels (a power of two, 16 to 4096). Bigger textures lose their top mip levels.
max_texture_size = 1024

# Who can use the cars: \"player\", \"cop\" or \"both\".
role = \"player\"
";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_file_parses_to_the_defaults() {
        let config: Config = toml::from_str(DEFAULT_CONFIG).unwrap();
        assert_eq!(config, Config::default());
    }
}
