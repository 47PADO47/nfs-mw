//! A converted car's recordings: its engine layers and its other sounds, from the samples of its 2012 bundle.

use anyhow::Result;
use chameleon_bundle::Bundle;

use crate::config::Config;
use crate::effects::{self, EffectFiles};
use crate::emit::EngineFiles;
use crate::engine;
use crate::sample;
use crate::source::{CarFile, Source};

/// What a car's bundle gives. A part that is `None` is one the car does not have: the game plays its built-in engine in
/// place of missing engine layers, and a missing effect is silent.
#[derive(Default)]
pub struct CarSounds {
    pub engine: Option<EngineFiles>,
    pub effects: Option<EffectFiles>,
}

/// The recordings of the car in `file` (named `id` in the log), when the settings ask for them. `rpm` is the engine's
/// idle and redline, which the engine layers are given their speeds from; without it the car gets no engine layers.
/// A car whose bundle cannot be read still converts, without recordings.
pub fn of(source: &Source, file: &CarFile, id: &str, config: &Config, rpm: Option<(f32, f32)>) -> CarSounds {
    if !config.own_engine_sound {
        return CarSounds::default();
    }
    read(source, file, rpm).unwrap_or_else(|error| {
        log::warn!("{id}: no recordings of its own: {error:#}");
        CarSounds::default()
    })
}

fn read(source: &Source, file: &CarFile, rpm: Option<(f32, f32)>) -> Result<CarSounds> {
    let bytes = source.bundle_bytes(file)?;
    let bundle = Bundle::parse(&bytes)?;
    let samples = sample::read_all(&bundle);
    let graphs = chameleon_sound_graph::SoundGraphs::read(&bundle);
    let mut found = effects::of(&bundle, &samples, source.shared_samples());
    found.files.no_clunk = effects::has_no_gear_change(&graphs);
    found.files.ignition = crate::ignition::wav(&bundle, source.shared_samples());
    let engine = rpm.filter(|&(idle, redline)| idle > 0.0 && redline > idle).and_then(|(idle, redline)| {
        let mut layers = crate::graph_voices::build(&bundle, &graphs, &samples);
        if layers.is_empty() {
            layers = engine::layers(engine::loops(&samples, &found.not_engine), idle, redline);
        }
        (!layers.is_empty()).then_some(EngineFiles { idle_rpm: idle, redline_rpm: redline, layers })
    });
    Ok(CarSounds { engine, effects: (!found.files.is_empty()).then_some(found.files) })
}
