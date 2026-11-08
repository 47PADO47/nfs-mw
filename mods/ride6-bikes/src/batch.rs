//! Every bike of an exporter output folder, imported in one run: what the native mod's background job calls.
//!
//! The folder (`source`) holds one folder per bike, each with `parts.json`, `glb/` and `tex/` (`bikeparts.py`,
//! `docs/modding/ride6-bikes-batch.md`). The car ids are assigned to the whole set ([`naming::assign`]), so they are unique
//! and do not depend on the order of the run; each bike is imported by [`pipeline::import_bike`] with its physics file from
//! the dump. A bike that cannot be imported is listed with its reason and the others go on: nothing is skipped silently.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;

use anyhow::{Context, Result};
use serde_json::Value;

use crate::bake::Options;
use crate::naming;
use crate::options::Catalogue;
use crate::options_manifest::{Plan, catalog_text};
use crate::options_pack::{Library, write_all};
use crate::pipeline::{Request, import_bike};
use crate::shop::Shop;

/// What to import from where.
pub struct Input<'a> {
    /// The exporter's output: one folder per bike.
    pub source: &'a Path,
    /// The physics dump folder, and its `Tire` folder when it has one.
    pub physics: &'a Path,
    pub tyre_tables: Option<&'a Path>,
    /// The `sound` command's output folder, if any.
    pub sound: Option<&'a Path>,
    pub max_texture: u32,
    /// The exporter's folder of the optional parts (`catalogue.json`, `parts.json`, `glb/`, `tex/`); `None`: the bikes get no
    /// shop options (`docs/modding/ride6-bike-upgrades.md`).
    pub bike_parts: Option<&'a Path>,
    /// Bikes imported at the same time (each holds its meshes in memory).
    pub threads: usize,
    /// Set to stop the run before its next bike (the game is closing, and its library is about to be unloaded).
    pub cancel: &'a AtomicBool,
}

/// What a run did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// `(vehicle id, car id)` of each bike imported.
    pub imported: Vec<(String, String)>,
    /// `(vehicle id or folder, reason)` of each bike left out.
    pub skipped: Vec<Skipped>,
    /// Why the rider was not converted, when it was asked for and failed (the bikes are still imported).
    pub rider_failed: Option<String>,
    /// Why the rider's gear is missing or incomplete, when a catalogue was given (the bikes are still imported).
    pub gear_failed: Option<String>,
    /// What the shop's options came to; `None` when the run had none.
    pub shop: Option<ShopReport>,
}

/// The shop's options of a run.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ShopReport {
    /// Bikes with at least one slot, the components of the catalog file and the part packs written, and their size in bytes.
    pub bikes: usize,
    pub components: usize,
    pub packs: usize,
    pub bytes: u64,
    /// Why the catalogue or a pack could not be used: the bikes that list it do not offer it.
    pub failed: Vec<Skipped>,
}

/// The stack of an import thread: the converter holds a bike's pictures and meshes in frames of its own, and a thread of the
/// game's host gets no more than the default 2 MB from the platform.
pub const STACK_BYTES: usize = 64 * 1024 * 1024;

/// A bike left out: its vehicle id (or folder) and the reason.
pub type Skipped = (String, String);

/// A bike folder found in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Found {
    pub dir: PathBuf,
    pub vehicle: String,
}

/// The folders of `source` that hold a `parts.json`, with the vehicle id each names, in the order of the vehicle ids; and the
/// folders whose `parts.json` cannot be read, with the reason.
pub fn discover(source: &Path) -> Result<(Vec<Found>, Vec<Skipped>)> {
    let entries = std::fs::read_dir(source).with_context(|| format!("reading {}", source.display()))?;
    let (mut found, mut unreadable) = (Vec::new(), Vec::new());
    for entry in entries.flatten() {
        let dir = entry.path();
        let file = dir.join("parts.json");
        if !file.is_file() {
            continue;
        }
        match vehicle_of(&file) {
            Ok(vehicle) => found.push(Found { dir, vehicle }),
            Err(error) => unreadable.push((entry.file_name().to_string_lossy().into_owned(), format!("{error:#}"))),
        }
    }
    found.sort_by(|a, b| a.vehicle.cmp(&b.vehicle));
    unreadable.sort();
    Ok((found, unreadable))
}

fn vehicle_of(file: &Path) -> Result<String> {
    let text = std::fs::read_to_string(file).with_context(|| format!("reading {}", file.display()))?;
    let doc: Value = serde_json::from_str(&text).context("parts.json is not JSON")?;
    let vehicle = doc.get("vehicle").and_then(Value::as_str).context("parts.json names no vehicle")?;
    Ok(vehicle.to_owned())
}

/// Import every bike of `input.source` into `out` (`files/CARS/<ID>/` and `cars/<id>.toml`; no `mod.toml`).
pub fn run(input: &Input<'_>, out: &Path) -> Result<Report> {
    let (found, unreadable) = discover(input.source)?;
    let vehicles: Vec<String> = found.iter().map(|f| f.vehicle.clone()).collect();
    let ids = naming::assign(&vehicles);
    let work: Vec<(&Found, &String)> = found.iter().zip(&ids).collect();
    let report = Mutex::new(Report { skipped: unreadable, ..Report::default() });
    let next = AtomicUsize::new(0);
    let (catalogue, plans) = (read_catalogue(input), Mutex::new(Vec::new()));
    let shop = catalogue.as_ref().map(|catalogue| Shop { catalogue, plans: &plans });
    thread::scope(|scope| -> Result<()> {
        for _ in 0..input.threads.max(1) {
            let worker = thread::Builder::new().stack_size(STACK_BYTES).spawn_scoped(scope, || {
                loop {
                    if input.cancel.load(Ordering::Relaxed) {
                        return;
                    }
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some((bike, car)) = work.get(at) else { return };
                    let result = import_guarded(bike, car, (input, shop.as_ref()), out);
                    let mut report = report.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                    match result {
                        Ok(()) => report.imported.push((bike.vehicle.clone(), (*car).clone())),
                        Err(reason) => report.skipped.push((bike.vehicle.clone(), reason)),
                    }
                }
            });
            worker.context("starting an import thread")?;
        }
        Ok(())
    })?;
    anyhow::ensure!(!input.cancel.load(Ordering::Relaxed), "the import was stopped");
    let mut report = report.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
    report.imported.sort();
    report.skipped.sort();
    if let (Some(catalogue), Some(dir)) = (&catalogue, input.bike_parts) {
        let plans = plans.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner);
        report.shop = Some(write_shop(input, (catalogue, dir), &plans, out));
    }
    Ok(report)
}

/// The catalogue of the optional parts, when the run has a folder of them that reads; else the reason goes to the log.
fn read_catalogue(input: &Input<'_>) -> Option<Catalogue> {
    let dir = input.bike_parts?;
    match Catalogue::read(dir) {
        Ok(catalogue) => Some(catalogue),
        Err(error) => {
            log::warn!("the bikes get no shop options: {error:#}");
            None
        }
    }
}

/// The catalog file of the components of `plans` and the packs of the parts they bind, in `out`.
fn write_shop(input: &Input<'_>, (catalogue, dir): (&Catalogue, &Path), plans: &[Plan], out: &Path) -> ShopReport {
    let mut report = ShopReport { bikes: plans.len(), ..ShopReport::default() };
    if plans.is_empty() {
        return report;
    }
    let text = catalog_text(plans.iter());
    report.components = text.matches("[[component]]").count();
    let file = out.join("components").join("ride6-bikes.toml");
    let saved = std::fs::create_dir_all(out.join("components")).and_then(|()| std::fs::write(&file, text));
    if let Err(error) = saved {
        report.failed.push(("components".to_owned(), format!("{}: {error}", file.display())));
        return report;
    }
    let mut stems: Vec<String> = plans
        .iter()
        .flat_map(|plan| &plan.parts)
        .filter_map(|part| part.pack.as_deref()?.rsplit('/').next().map(str::to_ascii_lowercase))
        .collect();
    stems.sort();
    stems.dedup();
    let kinds: Vec<_> =
        stems.iter().filter_map(|stem| Some((stem.clone(), catalogue.assets.get(stem)?.kind))).collect();
    let options = Options { max_texture: input.max_texture, ..Options::default() };
    let written =
        Library::open(dir).and_then(|library| write_all(&library, out, &kinds, (options, input.threads), input.cancel));
    match written {
        Ok(written) => {
            report.packs = written.packs;
            report.bytes = written.bytes;
            report.failed.extend(written.failed);
        }
        Err(error) => report.failed.push(("parts".to_owned(), format!("{error:#}"))),
    }
    report
}

/// One bike, with a missing mesh folder and a panic of the converter turned into a reason.
fn import_guarded(
    bike: &Found,
    car: &str,
    (input, shop): (&Input<'_>, Option<&Shop<'_>>),
    out: &Path,
) -> std::result::Result<(), String> {
    if !bike.dir.join("glb").is_dir() {
        return Err("the exporter's glb folder is missing (the meshes were cleaned away)".to_owned());
    }
    let request = Request {
        bike: &bike.dir,
        mod_dir: out,
        car,
        name: None,
        manufacturer: None,
        options: Options { max_texture: input.max_texture, ..Options::default() },
        physics: input.physics,
        tyre_tables: input.tyre_tables,
        sound_dir: input.sound,
        sound_required: false,
        write_mod_toml: false,
        shop,
    };
    let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| import_bike(&request)));
    match outcome {
        Ok(Ok(_)) => Ok(()),
        Ok(Err(error)) => Err(format!("{error:#}")),
        Err(_) => Err("the converter stopped on this bike".to_owned()),
    }
}

/// Threads for a run: half the cores, one to four.
pub fn default_threads() -> usize {
    std::thread::available_parallelism().map_or(1, |n| (n.get() / 2).clamp(1, 4))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-batch-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn bike(source: &Path, folder: &str, parts: &str, glb: bool) {
        std::fs::create_dir_all(source.join(folder)).unwrap();
        std::fs::write(source.join(folder).join("parts.json"), parts).unwrap();
        if glb {
            std::fs::create_dir_all(source.join(folder).join("glb")).unwrap();
        }
    }

    #[test]
    fn folders_with_a_part_table_are_found_in_vehicle_order_and_a_bad_table_is_listed() {
        let dir = temp("discover");
        bike(&dir, "b", r#"{"vehicle": "Yamaha_YZF-R6-2019"}"#, true);
        bike(&dir, "a", r#"{"vehicle": "Aprilia_RS-250-2001"}"#, true);
        bike(&dir, "bad", "not json", false);
        bike(&dir, "novehicle", "{}", false);
        std::fs::create_dir_all(dir.join("physdump")).unwrap();
        let (found, unreadable) = discover(&dir).unwrap();
        assert_eq!(
            found.iter().map(|f| f.vehicle.as_str()).collect::<Vec<_>>(),
            ["Aprilia_RS-250-2001", "Yamaha_YZF-R6-2019"]
        );
        assert_eq!(unreadable.iter().map(|(folder, _)| folder.as_str()).collect::<Vec<_>>(), ["bad", "novehicle"]);
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn every_bike_is_reported_with_its_reason() {
        let dir = temp("run");
        let out = temp("run-out");
        bike(&dir, "cleaned", r#"{"vehicle": "KTM_450-SMR-2022"}"#, false);
        bike(&dir, "broken", r#"{"vehicle": "Honda_CBR600RR-2018"}"#, true);
        bike(&dir, "worse", "x", false);
        let cancel = AtomicBool::new(false);
        let input = Input {
            source: &dir,
            physics: &dir,
            tyre_tables: None,
            sound: None,
            max_texture: 256,
            bike_parts: None,
            threads: 2,
            cancel: &cancel,
        };
        let report = run(&input, &out).unwrap();
        assert!(report.imported.is_empty());
        let skipped: Vec<&str> = report.skipped.iter().map(|(vehicle, _)| vehicle.as_str()).collect();
        assert_eq!(skipped, ["Honda_CBR600RR-2018", "KTM_450-SMR-2022", "worse"]);
        let reason = |vehicle: &str| report.skipped.iter().find(|(v, _)| v == vehicle).unwrap().1.clone();
        assert!(reason("KTM_450-SMR-2022").contains("glb folder is missing"));
        assert!(!reason("Honda_CBR600RR-2018").is_empty());
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&out).unwrap();
    }

    #[test]
    fn a_stopped_run_imports_no_further_bike_and_says_so() {
        let dir = temp("stopped");
        let out = temp("stopped-out");
        bike(&dir, "a", r#"{"vehicle": "Aprilia_RS-250-2001"}"#, true);
        let cancel = AtomicBool::new(true);
        let input = Input {
            source: &dir,
            physics: &dir,
            tyre_tables: None,
            sound: None,
            max_texture: 256,
            bike_parts: None,
            threads: 1,
            cancel: &cancel,
        };
        assert!(run(&input, &out).is_err());
        std::fs::remove_dir_all(&dir).unwrap();
        std::fs::remove_dir_all(&out).unwrap();
    }
}
