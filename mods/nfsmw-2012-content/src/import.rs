//! The conversion run: every car the settings ask for, from the 2012 install into a mod folder, on a pool of threads.
//!
//! Each car is independent (read its bundle, build its solids and textures, write its files), so cars are converted in
//! parallel. The pool leaves one core to the game, which keeps running while this does.

use std::path::Path;

use anyhow::{Context, Result, bail};
use rayon::prelude::*;

use crate::config::Config;
use crate::donor::Donors;
use crate::emit::{CarEntry, CarFiles, EngineFiles, ModFolder};
use crate::fit::{StockCar, best_fit, measure, stock_cars};
use crate::source::{CarFile, Source};
use crate::{convert, engine, names};

/// Threads the conversion may use at most: more would only add memory (each car in flight holds its meshes).
const MAX_THREADS: usize = 6;

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

/// A car of the install with the names the mod gives it.
struct Named<'a> {
    file: &'a CarFile,
    id: String,
    display: String,
    make: String,
}

/// How many threads to convert on: all the cores but one, between one and [`MAX_THREADS`].
pub fn default_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(2, usize::from);
    cores.saturating_sub(1).clamp(1, MAX_THREADS)
}

/// Convert the cars `config` asks for, from the 2012 install at `source_root` and with the stock cars of the 2005
/// install at `game_dir`, into the mod folder `out`, on `threads` threads.
pub fn run(config: &Config, source_root: &Path, game_dir: &Path, out: &Path, threads: usize) -> Result<Report> {
    config.check()?;
    let source = Source::open(source_root)?;
    let game = game_install::GameDir::open(game_dir)
        .with_context(|| format!("opening the NFS: Most Wanted install {}", game_dir.display()))?;
    let stock = stock_cars(&game).context("reading the stock cars' wheel layouts")?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .thread_name(|n| format!("nfsmw-2012-content-{n}"))
        .build()
        .context("starting the conversion threads")?;
    let donors = Donors::new(&game);
    pool.install(|| convert_all(config, &source, &stock, &donors, out))
}

fn convert_all(config: &Config, source: &Source, stock: &[StockCar], donors: &Donors, out: &Path) -> Result<Report> {
    let wanted: Vec<Named> = named(source).into_iter().filter(|car| config.wants(&car.id, &car.display)).collect();
    if wanted.is_empty() {
        bail!("no car of the 2012 install matches the settings");
    }
    let folder = ModFolder::create(out)?;
    let results: Vec<Result<ImportedCar>> =
        wanted.par_iter().map(|car| import_car(source, car, config, stock, donors, &folder)).collect();
    let mut report = Report::default();
    for (car, result) in wanted.iter().zip(results) {
        match result {
            Ok(done) => report.imported.push(done),
            Err(error) => report.skipped.push((format!("{} {}", car.id, car.display), format!("{error:#}"))),
        }
    }
    Ok(report)
}

/// Every car with its names. Reading a name touches only the small objects of the bundle, so this pass is quick; ids
/// are made one after another because a repeated name is numbered in file order.
fn named(source: &Source) -> Vec<Named<'_>> {
    let objects: Vec<Option<String>> = source.cars().par_iter().map(|file| source.object_name(file)).collect();
    let mut taken = Vec::new();
    let mut cars = Vec::with_capacity(objects.len());
    for (file, object) in source.cars().iter().zip(objects) {
        let name = names::car_name(object.as_deref(), file.number);
        let id = names::unique_id(&name.id, &taken);
        taken.push(id.clone());
        cars.push(Named { file, id, display: name.display, make: name.make });
    }
    cars
}

fn import_car(
    source: &Source,
    named: &Named,
    config: &Config,
    stock: &[StockCar],
    donors: &Donors,
    folder: &ModFolder,
) -> Result<ImportedCar> {
    let car = source.load(named.file)?;
    let layout = measure(&car.vehicle).context("the wheel layout of the car could not be measured")?;
    let (clone, fit) = best_fit(layout, stock).context("no stock car to clone")?;
    log::debug!("{}: wheel layout {:?} fitted onto {} {:?}", named.id, fit.from, clone.type_name, fit.to);
    let options = convert::Options { max_texture_size: config.max_texture_size };
    let base = clone.base_model_name.to_ascii_uppercase();
    let donor = donors.of(clone);
    let built = convert::build(&car, &fit, &donor, &base, &named.id, &options)?;
    let engine = engine_of(source, named, config, clone);
    let entry = CarEntry {
        id: named.id.clone(),
        clone: clone.type_name.clone(),
        name: named.display.clone(),
        manufacturer: named.make.clone(),
        role: config.role,
        performance: config.tuning().for_car(&named.id, &named.display),
    };
    folder.add_car(
        &entry,
        &CarFiles {
            geometry: &built.geometry,
            textures: &built.textures,
            bounds: &built.bounds,
            engine: engine.as_ref(),
        },
    )?;
    Ok(ImportedCar {
        id: named.id.clone(),
        name: named.display.clone(),
        clone: clone.type_name.clone(),
        stretch: fit.distortion().exp(),
        triangles: built.triangles,
    })
}

/// The car's own engine loops, when the settings ask for them and its bundle has pitched loops. A car whose loops cannot
/// be read still converts, with its clone's engine sound.
fn engine_of(source: &Source, named: &Named, config: &Config, clone: &StockCar) -> Option<EngineFiles> {
    if !config.own_engine_sound || clone.idle_rpm <= 0.0 || clone.redline_rpm <= clone.idle_rpm {
        return None;
    }
    let loops = source.bundle_bytes(named.file).and_then(|bytes| engine::loops(&bytes));
    let loops = loops.map_err(|error| log::warn!("{}: no engine loops: {error:#}", named.id)).ok()?;
    let layers = engine::layers(loops, clone.idle_rpm, clone.redline_rpm);
    (!layers.is_empty()).then_some(EngineFiles { idle_rpm: clone.idle_rpm, redline_rpm: clone.redline_rpm, layers })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_pool_is_never_empty_and_never_huge() {
        let threads = default_threads();
        assert!((1..=MAX_THREADS).contains(&threads));
    }
}
