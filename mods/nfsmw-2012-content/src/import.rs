//! The conversion run: every car the settings ask for, from the 2012 install into a mod folder, on a pool of threads.
//!
//! Each car is independent (read its bundle, build its solids and textures, write its files), so cars are converted in
//! parallel. The pool leaves one core to the game, which keeps running while this does.

use std::path::Path;

use anyhow::{Context, Result, bail};
use chameleon_bundle::Bundle;
use chameleon_vehicle::{Lod, PaintPackGroup, PoliceMarks, VehiclePaint, driver_setup, read_stats};
use rayon::prelude::*;

use crate::config::{Config, Role};
use crate::emit::{CarEntry, CarFiles, ModFolder};
use crate::fit::Fit;
use crate::police::PoliceRole;
use crate::shared::SharedSet;
use crate::source::{CarFile, Source, SourceCar};
use crate::stats::ImportedStats;
use crate::{brands, cameras, convert, names, nitrous, paints, sounds, two_colour, variants, wheels};

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
    /// What the car's bundle says about it being a police vehicle.
    marks: PoliceMarks,
    /// The bundles of the car's liveries, which are not cars of their own.
    liveries: Vec<&'a CarFile>,
    /// The bundles of the vehicles that are this car under another vehicle id, which lend it their paints.
    also: Vec<&'a CarFile>,
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
    // A livery bundle is a variant of its vehicle's car, not a car: only the vehicles' own bundles are converted. A car the
    // list has under two vehicle ids is one car (a police car is never folded: it has its own marks and names).
    let every = named(source);
    let name_of = |number: u32| {
        let car = every.iter().find(|car| car.file.number == number)?;
        (!car.marks.is_police()).then(|| car.display.clone())
    };
    let plan = variants::plan(source, &name_of);
    let wanted: Vec<Named> = every
        .into_iter()
        .filter_map(|mut car| {
            let planned = plan.iter().find(|planned| planned.file.number == car.file.number)?;
            car.liveries = planned.liveries.clone();
            car.also = planned.also.clone();
            Some(car)
        })
        .filter(|car| config.wants(&car.id, &car.display))
        .collect();
    if wanted.is_empty() {
        bail!("no car of the 2012 install matches the settings");
    }
    let folder = ModFolder::create(out)?;
    let shared = SharedSet::open(source, config);
    // Every car sits one of the install's characters at the seat its bundle names; the figures are read once, when first asked.
    if !source.has_characters() {
        log::warn!("the 2012 install has no character bundle: the cars have no driver");
    }
    let results: Vec<Result<ImportedCar>> =
        wanted.par_iter().map(|car| import_car(source, car, config, &folder, &shared)).collect();
    write_nitrous(source, out);
    crate::ignition::write_shared(source, out);
    crate::tire_smoke::write_variant(source, out);
    crate::spark_assets::write(source, out);
    let makes: Vec<&str> =
        wanted.iter().zip(&results).filter(|(_, done)| done.is_ok()).map(|(car, _)| car.make.as_str()).collect();
    let written = brands::write_all(source, &folder, &makes);
    log::info!("{written} brand logos written");
    shared.write(folder.root())?;
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
    let mut found = nitrous::find_in(&source.game_logic_bundles()).unwrap_or_default();
    found.effects = nitrous::find_effects(source);
    if found.is_empty() {
        log::warn!("the 2012 install has no nitrous sounds or effects that can be read: the nitrous variant has none");
        return;
    }
    if let Err(error) = nitrous::write(out, &found) {
        log::warn!("the nitrous variant is not written: {error:#}");
    }
}

/// Every car with its names. Reading a name touches only the small objects of the bundle, so this pass is quick; ids
/// are made one after another because a repeated name is numbered in file order.
fn named(source: &Source) -> Vec<Named<'_>> {
    let objects: Vec<(Option<String>, PoliceMarks)> =
        source.cars().par_iter().map(|file| source.identify(file)).collect();
    let mut taken = Vec::new();
    let mut cars = Vec::with_capacity(objects.len());
    for (file, (object, marks)) in source.cars().iter().zip(objects) {
        let name = names::car_name(object.as_deref(), file.number);
        let id = names::unique_id(&name.id, &taken);
        taken.push(id.clone());
        cars.push(Named {
            file,
            id,
            display: name.display,
            make: name.make,
            marks,
            liveries: Vec::new(),
            also: Vec::new(),
        });
    }
    cars
}

fn import_car(
    source: &Source,
    named: &Named,
    config: &Config,
    folder: &ModFolder,
    shared: &SharedSet,
) -> Result<ImportedCar> {
    let mut car = source.load(named.file)?;
    // What the car is comes from its own bundle; the user's multipliers, if any, change it below.
    let bytes = source.bundle_bytes(named.file)?;
    let mut car_stats = read_stats(&Bundle::parse(&bytes)?).context("the stats of the car could not be read")?;
    let body = car.vehicle.body.first().context("the car has no body")?;
    let (rear_end, front_end) = body
        .iter()
        .flat_map(|mesh| mesh.vertices.iter().map(|v| v.position[2]))
        .fold((f32::MAX, f32::MIN), |(low, high), z| (low.min(z), high.max(z)));
    let placed = wheels::place(&car.vehicle, (rear_end, front_end), car_stats.axle_z)
        .context("the wheel layout of the car could not be measured")?;
    // The stats' wheelbase and weight distribution are those of the hubs the wheels are drawn on.
    car_stats.axle_z = placed.axles;
    placed.seat_hubs(&mut car.vehicle);
    let fit = Fit::own(placed.layout);
    let driver = seat_driver(source, &mut car, &bytes);
    // A car with two-colour paint packs offers each as a livery (and takes the texture coordinates of their pictures); any
    // other car offers the pictures of its livery bundles, or its own.
    let pictures = two_colour::apply(source, named.file, &mut car, &named.id).unwrap_or_else(|| {
        let livery_names = variants::livery_names(source, named.file, &named.liveries);
        variants::pictures(source, &car, &named.id, &named.liveries, &livery_names)
    });
    let options = convert::Options {
        max_texture_size: config.max_texture_size,
        liveries: pictures.iter().map(|(choice, picture)| (choice.texture.clone(), picture.clone())).collect(),
        driver,
    };
    let built = convert::build(&car, &fit, &named.id, &options)?;
    let mut stats = ImportedStats::of(&car_stats);
    config.tuning().for_car(&named.id, &named.display).apply(&mut stats);
    let rpm = Some((stats.idle_rpm, stats.redline_rpm));
    let sounds = sounds::of(source, named.file, &named.id, config, rpm);
    let police = config
        .police
        .is_police(&named.id, &named.display, &named.marks)
        .then(|| config.police.plan(&named.id, &named.display, &named.marks));
    // A car with liveries wears the first one from the start, as its stock look: its own bundle's, which the 2012 game dresses
    // it in. The shop can take it off (a plain colour) or change it.
    let dressed = (!pictures.is_empty()).then_some(0);
    let colours = offered_paints(source, named, &car.paints);
    let entry = CarEntry {
        id: named.id.clone(),
        name: named.display.clone(),
        manufacturer: named.make.clone(),
        role: police.as_ref().map_or(config.role, |plan| match plan.role {
            PoliceRole::Cop => Role::Cop,
            PoliceRole::Both => Role::Both,
        }),
        police,
        stats,
        customization: paints::manifest_table(
            &paints::choices(&colours),
            &pictures.iter().map(|(choice, _)| choice.clone()).collect::<Vec<_>>(),
            dressed,
        ) + &plate_table(source, named, &built, !colours.is_empty())
            + &built.damage_parts
            + &placed.manifest_table(),
    };
    let files = CarFiles {
        geometry: &built.geometry,
        textures: &built.textures,
        bounds: &built.bounds,
        engine: sounds.engine.as_ref(),
        effects: sounds.effects.as_ref(),
    };
    folder.add_car(&entry, &files)?;
    // The shared sounds are named after the car is written; a car that cannot name them is still a car.
    if let Err(error) = shared.add_to_car(folder, &named.id, &bytes) {
        log::warn!("{}: no shared sounds: {error:#}", named.id);
    }
    // The car's own 2012 cameras, measured from the converted car's origin (the middle of its axles); the bumper camera
    // starts at the converted body's nose.
    let middle = (placed.axles[0] + placed.axles[1]) / 2.0;
    let nose = front_end - middle;
    if let Err(error) = cameras::write(folder, &named.id, &bytes, nose, middle) {
        log::warn!("{}: no 2012 cameras: {error:#}", named.id);
    }
    Ok(ImportedCar {
        id: named.id.clone(),
        name: named.display.clone(),
        stretch: fit.distortion().exp(),
        triangles: built.triangles,
        liveries: pictures.len(),
    })
}

/// The `[customization.plate]` table of `named`: its glyph sheet and the text of its game pack (`plate.rs`). Empty for a car
/// with no plate, and for one with no `[customization]` table to hold it (`has_table`: a table needs a paint).
fn plate_table(source: &Source, named: &Named, built: &convert::Built, has_table: bool) -> String {
    if !built.plate || !has_table {
        return String::new();
    }
    crate::plate::manifest_table(source.plate_text(named.file).as_deref(), &crate::plate::picture_name(&named.id))
}

/// The paints the shop offers `named`: those of its own bundle (`own`), in the order of its vehicle's paint pack (the first is
/// the default paint), then those of the bundles of the other vehicles that are the same car, each in the order of its own.
fn offered_paints(source: &Source, named: &Named, own: &[VehiclePaint]) -> Vec<VehiclePaint> {
    let in_order = |file: &CarFile, list: &[VehiclePaint]| {
        let order = source.pack_group(file).map(PaintPackGroup::paint_order).unwrap_or_default();
        paints::in_pack_order(list, &order)
    };
    let mut all = in_order(named.file, own);
    for &file in &named.also {
        all.extend(in_order(file, &source.paints_of(file)));
    }
    all
}

/// The driver of `car` (the bundle `bytes`): the character its driver setup names, seated where the setup says, at every level
/// of detail, with the figure's textures added to the car's. `None` when the car has no driver (the BMW 1M, whose setup
/// marks no seat), or the install has no figure for its character.
fn seat_driver(source: &Source, car: &mut SourceCar, bytes: &[u8]) -> Option<Vec<Lod>> {
    let setup = driver_setup(&Bundle::parse(bytes).ok()?)?;
    let seat = setup.seat?;
    let figure = source.driver_figure(setup.character)?;
    car.textures.extend(figure.textures.iter().map(|(id, texture)| (*id, texture.clone())));
    Some(figure.driver.seated(seat))
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
