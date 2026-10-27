//! The sounds all 2012 cars share, converted once for the whole mod (docs/formats/chameleon-shared-sounds.md).
//!
//! The 2012 game keeps the recordings that every car uses in two shared bundles: `SOUND/METALCAR.BNDL` (the body hits)
//! and `VEHICLES/WAVES.BNDL` (the exhaust pops the cars' graphs name), and names them in sound graphs. This module
//! finds the ones a graph names for a purpose, by the name of the graph:
//!
//! | The graph | Recordings | Becomes |
//! |---|---|---|
//! | `Player Vehicle Collision Small`, `Medium`, `Big` | the first selector of each graph | `impact_small`, `impact_medium`, `impact_big` |
//! | `Player Vehicle Scraping` (`HAWAII/SURFACESOUNDS.BNDL`) | the three METALCAR samples it imports | `scrape` |
//! | a car's graph that reads `GetPopProbability` or `PopGain` | the WAVES samples it imports | `pops` |
//!
//! They are written once under `files/SOUND/MODSHARED/` and each car's manifest names them in `[car_sound.shared]`, so no
//! car holds a copy. What the graphs do not name stays out: the car then plays what the game plays for a car without it.

mod read;
mod write;

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use anyhow::{Context, Result};
use chameleon_bundle::Bundle;

use crate::config::Config;
use crate::emit::ModFolder;
use crate::graphs::Graphs;
use crate::sample;
use crate::source::Source;
use write::{Shot, lone_shot, shots};

const METALCAR: &str = "SOUND/METALCAR.BNDL";
const SURFACE_SOUNDS: &str = "HAWAII/SURFACESOUNDS.BNDL";
/// The names of the graphs, and the manifest key each one's recordings go to.
const IMPACTS: [(&str, &str); 3] = [
    ("impact_small", "Player Vehicle Collision Small"),
    ("impact_medium", "Player Vehicle Collision Medium"),
    ("impact_big", "Player Vehicle Collision Big"),
];
const SCRAPE: &str = "Player Vehicle Scraping";
/// The live values a car's pop graphs read.
const POP_VALUES: [&str; 2] = ["GetPopProbability", "PopGain"];
/// A pop longer than this is not a bang.
const MAX_POP_SECONDS: f32 = 0.5;

/// The shared recordings of the install, as the mod writes them.
#[derive(Default)]
pub struct SharedSet {
    impact: [Vec<Shot>; 3],
    scrape: Vec<Shot>,
    /// The shared sample bundle, where the pops are.
    waves: Option<Vec<u8>>,
    /// The pops the cars asked for, by resource id (`None` for one that does not decode or is no bang).
    pops: Mutex<BTreeMap<u64, Option<Shot>>>,
}

impl SharedSet {
    /// A set with nothing in it: the cars get no shared recordings.
    pub fn none() -> Self {
        Self::default()
    }

    /// Read the shared recordings of the install, when the settings ask for recordings. A bundle that is missing or does
    /// not read leaves its sounds out, and the log says so.
    pub fn open(source: &Source, config: &Config) -> Self {
        if !config.own_engine_sound {
            return Self::none();
        }
        let metalcar = source.install_file(METALCAR);
        let Some(metal) = metalcar.as_deref().and_then(|bytes| Bundle::parse(bytes).ok()) else {
            log::warn!("the 2012 install has no readable {METALCAR}: the cars get no shared body hits");
            return Self { waves: source.shared_samples().map(<[u8]>::to_vec), ..Self::none() };
        };
        let mut impact: [Vec<Shot>; 3] = Default::default();
        for (slot, (_, label)) in impact.iter_mut().zip(IMPACTS) {
            let ids = read::first_selector_samples(&metal, label);
            *slot = shots("IMPACT", &read::decode(&metal, &ids));
        }
        let surfaces = source.install_file(SURFACE_SOUNDS);
        let scrape = surfaces
            .as_deref()
            .and_then(|bytes| Bundle::parse(bytes).ok())
            .map(|bundle| shots("SCRAPE", &read::decode(&metal, &read::samples_imported(&bundle, SCRAPE, &metal))));
        Self {
            impact,
            scrape: scrape.unwrap_or_default(),
            waves: source.shared_samples().map(<[u8]>::to_vec),
            ..Self::none()
        }
    }

    /// The `[car_sound.shared]` table of the car whose sound graphs are `graphs`: the shared body hits and scrape, and the
    /// pops its graphs name. Empty when there is nothing to name.
    pub fn table_for(&self, graphs: &Graphs) -> String {
        let paths = |shots: &[Shot]| shots.iter().map(|shot| shot.path.clone()).collect::<Vec<String>>();
        let mut lines = String::new();
        for ((key, _), shots) in IMPACTS.iter().zip(&self.impact) {
            lines.push_str(&write::list_line(key, &paths(shots)));
        }
        lines.push_str(&write::list_line("scrape", &paths(&self.scrape)));
        lines.push_str(&write::list_line("pops", &self.pops_for(graphs)));
        if lines.is_empty() {
            return lines;
        }
        format!("\n[car_sound.shared]\n{lines}")
    }

    /// The paths of the pops the car's graphs import from the shared bundle, in id order. Each is decoded once.
    fn pops_for(&self, graphs: &Graphs) -> Vec<String> {
        let Some(bundle) = self.waves.as_deref().and_then(|bytes| Bundle::parse(bytes).ok()) else { return Vec::new() };
        let ids: BTreeSet<u64> = POP_VALUES.iter().flat_map(|value| graphs.shared_imports_reading(value)).collect();
        let mut cache = self.pops.lock().unwrap_or_else(PoisonError::into_inner);
        let found = ids.into_iter().filter_map(|id| cache.entry(id).or_insert_with(|| pop(&bundle, id)).clone());
        found.map(|shot| shot.path).take(write::MAX_FILES).collect()
    }

    /// Write the recordings under `files/SOUND/MODSHARED/` of the mod folder `root`: the body hits and scrape, and every
    /// pop a car asked for. Call it after the cars.
    pub fn write(&self, root: &Path) -> Result<()> {
        let pops = self.pops.lock().unwrap_or_else(PoisonError::into_inner);
        let all = self.impact.iter().flatten().chain(&self.scrape).chain(pops.values().flatten());
        for shot in all {
            let path = root.join("files").join(&shot.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).with_context(|| format!("creating {}", parent.display()))?;
            }
            std::fs::write(&path, &shot.wav).with_context(|| format!("writing {}", path.display()))?;
        }
        Ok(())
    }

    /// Name the shared recordings in the manifest of the car `id`, whose bundle is `bytes`. A car that has nothing to name
    /// is left as it is.
    pub fn add_to_car(&self, folder: &ModFolder, id: &str, bytes: &[u8]) -> Result<()> {
        let bundle = Bundle::parse(bytes)?;
        let table = self.table_for(&Graphs::new(&bundle));
        if table.is_empty() {
            return Ok(());
        }
        folder.append_to_manifest(id, &table)
    }
}

/// The pop `id` of the shared bundle, when it is a short one-shot.
fn pop(bundle: &Bundle, id: u64) -> Option<Shot> {
    let sample = sample::read(bundle, bundle.find(id)?)?;
    if sample.looped || sample.seconds() > MAX_POP_SECONDS {
        return None;
    }
    Some(lone_shot("POP", id, &sample))
}

#[cfg(test)]
mod real_tests;
