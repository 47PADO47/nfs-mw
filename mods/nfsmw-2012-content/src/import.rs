//! The conversion run: every car the settings ask for, from the 2012 install into a mod folder, on a pool of threads.
//!
//! Each car is independent (read its bundle, build its solids and textures, write its files), so cars are converted in
//! parallel. The pool leaves one core to the game, which keeps running while this does.

use std::path::Path;

use anyhow::{Context, Result, bail};
use chameleon_bundle::Bundle;
use chameleon_vehicle::read_stats;
use rayon::prelude::*;

use crate::config::Config;
use crate::emit::{CarEntry, CarFiles, ModFolder};
use crate::fit::{Fit, measure};
use crate::source::{CarFile, Source};
use crate::stats::ImportedStats;
use crate::{convert, names, nitrous, paints, sounds, variants};

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
    /// Largest stretch the fit applied along any axis (1.0 is none).
    pub stretch: f32,
    /// Triangles of the finest body level.
    pub triangles: usize,
    /// Liveries the car offers in the shop (bundles of the same vehicle that were folded into it).
    pub liveries: usize,
}

/// A car of the install with the names the mod gives it.
struct Named<'a> {
    file: &'a CarFile,
    id: String,
    display: String,
    make: String,
    /// The bundles of the car's liveries, which are not cars of their own.
    liveries: Vec<&'a CarFile>,
}

/// How many threads to convert on: all the cores but one, between one and [`MAX_THREADS`].
pub fn default_threads() -> usize {
    let cores = std::thread::available_parallelism().map_or(2, usize::from);
    cores.saturating_sub(1).clamp(1, MAX_THREADS)
}

/// Convert the cars `config` asks for, from the 2012 install at `source_root` into the mod folder `out`, on `threads`
/// threads.
pub fn run(config: &Config, source_root: &Path, out: &Path, threads: usize) -> Result<Report> {
    config.check()?;
    let source = Source::open(source_root)?;
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(threads.max(1))
        .thread_name(|n| format!("nfsmw-2012-content-{n}"))
        .build()
        .context("starting the conversion threads")?;
    pool.install(|| convert_all(config, &source, out))
}

fn convert_all(config: &Config, source: &Source, out: &Path) -> Result<Report> {
    // A livery bundle is a variant of its vehicle's car, not a car: only the vehicles' own bundles are converted.
    let plan = variants::plan(source);
    let wanted: Vec<Named> = named(source)
        .into_iter()
        .filter_map(|mut car| {
            car.liveries = plan.iter().find(|planned| planned.file.number == car.file.number)?.liveries.clone();
            Some(car)
        })
        .filter(|car| config.wants(&car.id, &car.display))
        .collect();
    if wanted.is_empty() {
        bail!("no car of the 2012 install matches the settings");
    }
    let folder = ModFolder::create(out)?;
    let results: Vec<Result<ImportedCar>> =
        wanted.par_iter().map(|car| import_car(source, car, config, &folder)).collect();
    write_nitrous(source, out);
    let mut report = Report::default();
    for (car, result) in wanted.iter().zip(results) {
        match result {
            Ok(done) => report.imported.push(done),
            Err(error) => report.skipped.push((format!("{} {}", car.id, car.display), format!("{error:#}"))),
        }
    }
    Ok(report)
}

/// The nitrous sounds and profile of the 2012 game, as the variant the 2005 game's nitrous option offers. A failure only costs
/// the variant: the cars are converted, and the game's own nitrous stays the only choice.
fn write_nitrous(source: &Source, out: &Path) {
    let Some(found) = nitrous::find_in(&source.game_logic_bundles()) else {
        log::warn!("the 2012 install has no nitrous sounds that can be read: the nitrous variant has no effects");
        return;
    };
    if let Err(error) = nitrous::write(out, &found) {
        log::warn!("the nitrous variant is not written: {error:#}");
    }
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
        cars.push(Named { file, id, display: name.display, make: name.make, liveries: Vec::new() });
    }
    cars
}

fn import_car(source: &Source, named: &Named, config: &Config, folder: &ModFolder) -> Result<ImportedCar> {
    let car = source.load(named.file)?;
    let layout = measure(&car.vehicle).context("the wheel layout of the car could not be measured")?;
    let fit = Fit::own(layout);
    let pictures = variants::pictures(source, &car, &named.id, &named.liveries);
    let options = convert::Options {
        max_texture_size: config.max_texture_size,
        liveries: pictures.iter().map(|(choice, picture)| (choice.texture.clone(), picture.clone())).collect(),
    };
    let built = convert::build(&car, &fit, &named.id, &options)?;
    // What the car is comes from its own bundle; the user's multipliers, if any, change it here.
    let bytes = source.bundle_bytes(named.file)?;
    let car_stats = read_stats(&Bundle::parse(&bytes)?).context("the stats of the car could not be read")?;
    let mut stats = ImportedStats::of(&car_stats);
    config.tuning().for_car(&named.id, &named.display).apply(&mut stats);
    let rpm = Some((stats.idle_rpm, stats.redline_rpm));
    let sounds = sounds::of(source, named.file, &named.id, config, rpm);
    let entry = CarEntry {
        id: named.id.clone(),
        name: named.display.clone(),
        manufacturer: named.make.clone(),
        role: config.role,
        stats,
        customization: paints::manifest_table(
            &paints::choices(&car.paints),
            &pictures.iter().map(|(choice, _)| choice.clone()).collect::<Vec<_>>(),
        ) + &built.damage_parts,
    };
    let files = CarFiles {
        geometry: &built.geometry,
        textures: &built.textures,
        bounds: &built.bounds,
        engine: sounds.engine.as_ref(),
        effects: sounds.effects.as_ref(),
    };
    folder.add_car(&entry, &files)?;
    Ok(ImportedCar {
        id: named.id.clone(),
        name: named.display.clone(),
        stretch: fit.distortion().exp(),
        triangles: built.triangles,
        liveries: pictures.len(),
    })
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
