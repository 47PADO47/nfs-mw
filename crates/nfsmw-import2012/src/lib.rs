//! Converts the cars of NFS Most Wanted (2012) into cars of NFS: Most Wanted (2005). The `mw2012-cars` mod runs it.
//!
//! The 2012 game runs on the Chameleon engine, not on the 2005 Black Box engine, so its car models and textures are
//! converted: `chameleon-vehicle` reads a car out of its bundles, this crate fits it to the wheel layout of a stock 2005
//! car, writes it as that car's `GEOMETRY.BIN` and `TEXTURES.BIN`, and lists it in the mod's `cars/` and `files/`
//! folders, which the game loads like any other mod content (`docs/modding/api-content.md`). Nothing from either game is
//! stored in the repository: the cars are made on the user's machine from the user's own installs, and without a 2012
//! install ([`config::Config::source`]) there is nothing to do.

pub mod config;
mod convert;
mod emit;
mod fit;
mod names;
mod source;
mod stamp;
mod textures;

use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use config::Config;
use emit::{CarEntry, CarFiles, ModFolder};
use fit::{StockCar, best_fit, measure};
pub use source::{CarFile, Source};

/// A commented settings file with the defaults: what the mod writes into its folder when it finds none.
pub const DEFAULT_CONFIG: &str = "\
# Settings of the NFS: Most Wanted (2012) cars mod (docs/modding/mw2012-cars.md). Restart the game after a change.

# Folder of your own NFS Most Wanted (2012) install. Without it no car is added.
# source = 'D:\\Need For Speed Most Wanted 2012'

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

/// What a start of the mod did.
#[derive(Debug)]
pub enum Outcome {
    /// No 2012 install is set. `created` says the settings file was missing and has just been written.
    NotConfigured { config: PathBuf, created: bool },
    /// The cars of the last run are still right for the settings and the installs.
    UpToDate { cars: usize },
    /// The cars were converted.
    Imported(Report),
}

/// What a conversion run did.
#[derive(Debug, Default)]
pub struct Report {
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

/// A car the 2012 install holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    pub number: u32,
    pub id: String,
    pub name: String,
}

/// What the mod does at load: read its settings from `mod_dir/config.toml`, and, unless the last run's cars are still
/// right, convert the cars into `mod_dir/files/` and `mod_dir/cars/`. `game_dir` is the 2005 install.
pub fn prepare(mod_dir: &Path, game_dir: &Path) -> Result<Outcome> {
    let config_path = mod_dir.join(config::FILE_NAME);
    let created = !config_path.exists();
    if created {
        std::fs::write(&config_path, DEFAULT_CONFIG).with_context(|| format!("writing {}", config_path.display()))?;
    }
    let config = Config::load(&config_path)?;
    let Some(source) = config.source.clone() else {
        return Ok(Outcome::NotConfigured { config: config_path, created });
    };
    let stamp = stamp::of(&config, &source, game_dir);
    if stamp::is_current(mod_dir, &stamp) {
        return Ok(Outcome::UpToDate { cars: count_cars(mod_dir) });
    }
    stamp::clear(mod_dir);
    let report = run(&config, &source, game_dir, mod_dir)?;
    stamp::save(mod_dir, &stamp).context("saving the import stamp")?;
    Ok(Outcome::Imported(report))
}

fn count_cars(mod_dir: &Path) -> usize {
    let cars = std::fs::read_dir(mod_dir.join("cars")).into_iter().flatten().flatten();
    cars.filter(|entry| entry.path().extension().is_some_and(|ext| ext == "toml")).count()
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

/// Convert the cars `config` asks for, from the 2012 install at `source_root` and with the stock cars of the 2005
/// install at `game_dir`, into the mod folder `out`.
pub fn run(config: &Config, source_root: &Path, game_dir: &Path, out: &Path) -> Result<Report> {
    config.check()?;
    let source = Source::open(source_root)?;
    let game = game_install::GameDir::open(game_dir)
        .with_context(|| format!("opening the NFS: Most Wanted install {}", game_dir.display()))?;
    let stock = fit::stock_cars(&game).context("reading the stock cars' wheel layouts")?;
    let folder = ModFolder::create(out)?;
    let mut report = Report::default();
    let mut taken = Vec::new();
    for file in source.cars() {
        let (id, display) = identify(&source, file, &mut taken);
        if !config.wants(&id, &display) {
            continue;
        }
        match import_car(&source, file, &id, &display, config, &stock, &folder) {
            Ok(car) => report.imported.push(car),
            Err(error) => report.skipped.push((format!("{id} {display}"), format!("{error:#}"))),
        }
    }
    if report.imported.is_empty() && report.skipped.is_empty() {
        bail!("no car of {} matches the settings", source_root.display());
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_default_settings_file_parses_to_the_defaults() {
        let config = Config::parse(DEFAULT_CONFIG).unwrap();
        assert_eq!(config, Config::default());
    }

    #[test]
    fn a_first_start_writes_the_settings_and_adds_no_cars() {
        let dir = std::env::temp_dir().join(format!("nfsmw-import2012-prepare-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let outcome = prepare(&dir, Path::new("no/game")).unwrap();
        assert!(matches!(outcome, Outcome::NotConfigured { created: true, .. }));
        assert!(dir.join("config.toml").is_file());
        assert!(!dir.join("files").exists() && !dir.join("cars").exists());
        let again = prepare(&dir, Path::new("no/game")).unwrap();
        assert!(matches!(again, Outcome::NotConfigured { created: false, .. }));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
