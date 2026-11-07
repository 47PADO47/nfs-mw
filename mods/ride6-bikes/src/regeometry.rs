//! The car files of bikes that are already imported, built again from the exporter's output (`ride6-bike-import geometry`,
//! `docs/modding/ride6-bikes-batch.md`).
//!
//! The meshes, the textures and the liveries of the importer change more often than its physics, and a whole import also writes
//! the manifest again, which the sound and camera tables have been added to since. This writes `GEOMETRY.BIN`, `TEXTURES.BIN`
//! and `BOUNDS.BIN` of each bike and the livery choices of its manifest, and touches nothing else of the mod.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use anyhow::{Context, Result};

use crate::bake::Options;
use crate::batch::{self, Found, STACK_BYTES, Skipped};
use crate::build::build;
use crate::manifest;
use crate::naming;
use crate::summary::Summary;

/// What to build again and from where.
pub struct Run<'a> {
    /// The exporter's output: one folder per bike.
    pub source: &'a Path,
    /// The mod folder the bikes were imported into.
    pub mod_dir: &'a Path,
    pub max_texture: u32,
    /// Bikes built at the same time (each holds its meshes in memory).
    pub threads: usize,
    /// Only the bikes whose vehicle id contains this text.
    pub only: Option<&'a str>,
}

/// Build the bikes of `run.source` again. Returns how many were written and the bikes that were not, with the reason.
pub fn run(run: &Run<'_>) -> Result<(usize, Vec<Skipped>)> {
    let (found, mut failed) = batch::discover(run.source)?;
    let vehicles: Vec<String> = found.iter().map(|f| f.vehicle.clone()).collect();
    let ids = naming::assign(&vehicles);
    let work: Vec<(&Found, &String)> =
        found.iter().zip(&ids).filter(|(bike, _)| run.only.is_none_or(|only| bike.vehicle.contains(only))).collect();
    let (written, skipped) = (AtomicUsize::new(0), Mutex::new(Vec::new()));
    let next = AtomicUsize::new(0);
    thread::scope(|scope| -> Result<()> {
        for _ in 0..run.threads.max(1) {
            let worker = thread::Builder::new().stack_size(STACK_BYTES).spawn_scoped(scope, || {
                loop {
                    let at = next.fetch_add(1, Ordering::Relaxed);
                    let Some((bike, car)) = work.get(at) else { return };
                    match one(bike, car, run) {
                        Ok(()) => {
                            written.fetch_add(1, Ordering::Relaxed);
                        }
                        Err(reason) => {
                            let mut list = skipped.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                            list.push((bike.vehicle.clone(), reason));
                        }
                    }
                }
            });
            worker.context("starting a thread")?;
        }
        Ok(())
    })?;
    failed.extend(skipped.into_inner().unwrap_or_else(std::sync::PoisonError::into_inner));
    failed.sort();
    Ok((written.into_inner(), failed))
}

/// One bike, with a panic of the converter turned into a reason.
fn one(bike: &Found, car: &str, run: &Run<'_>) -> std::result::Result<(), String> {
    let manifest = run.mod_dir.join("cars").join(format!("{}.toml", car.to_ascii_lowercase()));
    if !manifest.is_file() {
        return Err(format!("{} does not exist: the bike was not imported under the id {car}", manifest.display()));
    }
    let options = Options { max_texture: run.max_texture, ..Options::default() };
    match catch_unwind(AssertUnwindSafe(|| write(bike, car, run.mod_dir, options))) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(error)) => Err(format!("{error:#}")),
        Err(_) => Err("the converter stopped on this bike".to_owned()),
    }
}

fn write(bike: &Found, car: &str, mod_dir: &Path, options: Options) -> Result<()> {
    let built = build(&bike.dir, car, options)?;
    let car_dir = mod_dir.join("files").join("CARS").join(car.to_ascii_uppercase());
    std::fs::create_dir_all(&car_dir).with_context(|| format!("creating {}", car_dir.display()))?;
    for (file, bytes) in
        [("GEOMETRY.BIN", &built.geometry), ("TEXTURES.BIN", &built.textures), ("BOUNDS.BIN", &built.bounds)]
    {
        std::fs::write(car_dir.join(file), bytes).with_context(|| format!("writing {file}"))?;
    }
    Summary::of(&bike.vehicle, &built).write(&bike.dir)?;
    manifest::apply_liveries(mod_dir, car, &built.liveries)?;
    manifest::apply_bodies(mod_dir, car, &built.extra_bodies)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bike_without_a_manifest_is_reported_not_built() {
        let dir = std::env::temp_dir().join(format!("ride6-bikes-regeometry-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("source").join("a")).unwrap();
        std::fs::write(dir.join("source").join("a").join("parts.json"), r#"{"vehicle": "Aprilia_RS-250-2001"}"#)
            .unwrap();
        let mod_dir = dir.join("mod");
        std::fs::create_dir_all(&mod_dir).unwrap();
        let source = dir.join("source");
        let (written, failed) =
            run(&Run { source: &source, mod_dir: &mod_dir, max_texture: 64, threads: 1, only: None }).unwrap();
        assert_eq!(written, 0);
        assert_eq!(failed.len(), 1);
        assert!(failed[0].1.contains("does not exist"), "{failed:?}");
        let (none, _) =
            run(&Run { source: &source, mod_dir: &mod_dir, max_texture: 64, threads: 1, only: Some("KTM") }).unwrap();
        assert_eq!(none, 0);
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
