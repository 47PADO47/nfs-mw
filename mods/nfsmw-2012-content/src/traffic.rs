//! The 2012 traffic cars as traffic cars of the 2005 game.
//!
//! The install has eleven traffic bundles (`VEHICLES/VEH_<n>_TR.BNDL`): an Audi A3, a Cadillac CTS-V, a Chevrolet Express Van,
//! a Dodge Caliber, a Dodge Grand Caravan, a Dodge Ram 1500, a Chevrolet Cobalt, an Infiniti G35, a Nissan Frontier, a Nissan
//! Versa and a GMC Topkick. Each is a car bundle without stats, paints, skeleton or driver: its body at five levels of detail
//! (9,000 to 11,000 triangles down to 100), its wheels, its lamps and its collision soup
//! ([`docs/formats/chameleon-traffic.md`](../../../docs/formats/chameleon-traffic.md)). They are converted like the player
//! cars (the same body, wheel and collision conversion), with `role = "traffic"` and a `[traffic]` table
//! (`docs/modding/api-traffic.md`), so the game's traffic manager puts them on the roads beside its own cars. The setting
//! `traffic = true` (the default) of the mod's `config.toml` turns them on.
//!
//! What the bundles lack is made up by class (`class.rs`): the stats are the numbers of a road car of the kind, and the paint
//! is one colour per car. Nothing of either game's cars is copied: the stats are design values.

mod car;
mod class;
mod ids;
mod manifest;
mod stats;

use std::path::Path;

use anyhow::Result;
use rayon::prelude::*;

use crate::config::Config;
use crate::import::{ImportedCar, Report};
use crate::source::{CarFile, Source};

use car::{Done, Named};

/// The traffic bundles of the install, in file-number order: `VEHICLES/VEH_<n>_TR.BNDL`.
pub fn bundles(source: &Source) -> Vec<CarFile> {
    let dir = source.root().join("VEHICLES");
    let mut files: Vec<CarFile> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_ascii_uppercase();
            let number = name.strip_prefix("VEH_")?.strip_suffix("_TR.BNDL")?.parse().ok()?;
            Some(CarFile { number, path: entry.path() })
        })
        .collect();
    files.sort_by_key(|file| file.number);
    files
}

/// Convert the traffic cars into the mod folder `root` and add them to `report`: the ones that converted to its `traffic` list, the
/// others, with the reason, to `skipped`. Does nothing when the setting `traffic` is off or the install has no traffic bundle.
/// `cars` and `exclude` of the settings pick traffic cars like the others, by id (`AUDA3_TR`) or by a part of the name (every
/// traffic car's name has `traffic` in it).
pub fn import_all(source: &Source, config: &Config, root: &Path, report: &mut Report) {
    if !config.traffic {
        return;
    }
    let files = bundles(source);
    if files.is_empty() {
        log::info!("the 2012 install has no traffic bundle: no traffic car is added");
        return;
    }
    let named: Vec<Named> =
        car::name_all(files, source).into_iter().filter(|car| config.wants(&car.id, &car.display)).collect();
    let results: Vec<(String, Result<Done>)> = named
        .par_iter()
        .map(|car| (format!("{} {}", car.id, car.display), car::import(source, car, config, root)))
        .collect();
    for (label, result) in results {
        match result {
            Ok(done) => {
                log::info!(
                    "traffic car {} ({}): {} triangles, {} KB",
                    done.id,
                    done.class,
                    done.triangles,
                    done.bytes / 1024
                );
                report.traffic.push(ImportedCar {
                    id: done.id,
                    name: done.display,
                    stretch: 1.0,
                    triangles: done.triangles,
                    liveries: 0,
                });
            }
            Err(error) => report.skipped.push((label, format!("{error:#}"))),
        }
    }
}

#[cfg(test)]
mod tests;
