//! The sound of the car being driven: the scene reports its telemetry each frame, the engine mixer
//! (`blackbox-carsound`) turns it into the Ginsu frequency and loop volumes the engine voice plays, and the
//! effects mixer into the shift, turbo, tire, road and collision sounds.

use std::collections::HashMap;
use std::time::Duration;

use blackbox_carsound::{CarInput, EffectsMixer, EngineMixer, LoopId, ScrapeKind, SoundCommand, SoundRef};
use kira::Tween;
use kira::sound::static_sound::StaticSoundHandle;
use nfsmw_data::sound::{CarSound, EventKind};

use super::LoopMix;
use super::aems::{AemsLayer, Feed, KiraSampler};
use super::mixer::{CarMixer, Frame, Levels, scale_command};
use super::tuning::tuning;
use super::{Audio, EngineHandle, EngineMix};

/// A collision the physics reported since the last frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarEvent {
    pub kind: EventKind,
    /// The `simsurface` hash of the surface involved.
    pub surface: u32,
    /// 0 to 1.
    pub magnitude: f32,
    /// The hit is on the car's front or back, not its side.
    pub front: bool,
    /// A light prop that falls over (a cone, a sign) rather than a wall.
    pub smackable: bool,
}

/// What the scene tells the sound each frame about the car it drives.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSoundState {
    /// The car type (`BMWM3GTR`), which picks its sound set.
    pub car: String,
    pub input: CarInput,
    pub events: Vec<CarEvent>,
    /// What the car is rubbing along and how hard (0 to 1), if anything.
    pub scrape: Option<(ScrapeKind, f32)>,
    /// The `simsurface` hash under the car, for the sound of a landing.
    pub surface: u32,
}

/// The mixers, the voices and the sound set of one car.
pub(super) struct CarAudio {
    pub car: String,
    pub sound: CarSound,
    engine: EngineMixer,
    pub effects: EffectsMixer,
    /// The dynamic mixer; `None` when the map could not be read.
    mixer: Option<CarMixer>,
    /// What the mixer said in the last frame.
    pub levels: Levels,
    /// The sample layer (engine samples and sputters); `None` when the banks could not be run.
    aems: Option<AemsLayer>,
    /// `engineaudio.Master_Vol`.
    master_volume: u32,
    handle: EngineHandle,
    /// The engine was at the limiter in the last frame (for the log).
    redlining: bool,
    /// The loops that are playing, with the sound each plays.
    pub loops: HashMap<LoopId, (SoundRef, StaticSoundHandle)>,
    commands: Vec<SoundCommand>,
}

impl Drop for CarAudio {
    /// A kira handle that is dropped leaves its sound playing, so the loops (skids, road, wind, nitrous) are
    /// stopped here: otherwise they play on after the car is changed or gone.
    fn drop(&mut self) {
        let fade = Tween { duration: Duration::from_millis(120), ..Tween::default() };
        for (_, (_, mut handle)) in self.loops.drain() {
            handle.stop(fade);
        }
    }
}

impl Audio {
    /// Follow the scene's car: start its sounds when it changes, feed the mixers, silence them when there is no car.
    pub fn drive_car(&mut self, state: Option<&CarSoundState>, dt: f32) {
        let Some(state) = state else {
            self.car = None;
            self.failed = None;
            return;
        };
        if self.car.as_ref().is_none_or(|c| c.car != state.car) {
            self.car = None;
            if self.failed.as_deref() == Some(&state.car) || !self.available() {
                return;
            }
            match self.start_car(&state.car) {
                Ok(car) => self.car = Some(car),
                Err(e) => {
                    log::warn!("no engine sound for {}: {e}", state.car);
                    self.failed = Some(state.car.clone());
                    return;
                }
            }
        }
        let Some(mut car) = self.car.take() else { return };
        // The map's pitch for the engine (one frame old) is the dynamic mixer's pitch multiplier.
        let input = CarInput { pitch_multiplier: car.levels.engine_pitch, ..state.input };
        let out = car.engine.update(dt, &input);
        let gain = car.levels.engine_volume;
        if out.redlining != car.redlining {
            car.redlining = out.redlining;
            log::debug!(
                "{}: the limiter {}: engine loops {:.2}, sample layer {:.2}, gain {gain:.2}",
                car.car,
                if out.redlining { "starts" } else { "ends" },
                out.accel_volume,
                out.redline_sample_volume
            );
        }
        car.handle.set(EngineMix {
            accel: LoopMix {
                frequency: out.accel_loop.frequency,
                volume: out.accel_volume * gain,
                pitch: out.accel_loop.playback_rate,
            },
            decel: LoopMix {
                frequency: out.decel_loop.frequency,
                volume: out.decel_volume * gain,
                pitch: out.decel_loop.playback_rate,
            },
        });
        car.effects.scrape(state.scrape);
        let mut commands = std::mem::take(&mut car.commands);
        commands.clear();
        let landing = car.effects.update(dt, &input, &out, &mut commands);
        if let Some(mixer) = car.mixer.as_mut() {
            let sparks = car.aems.as_ref().is_some_and(AemsLayer::sparks);
            let frame = Frame {
                input: &input,
                engine: &out,
                effects: car.effects.signals(),
                master_volume: car.master_volume,
                sparks,
            };
            car.levels = mixer.update(dt, &frame);
        }
        if let Some(layer) = car.aems.as_mut() {
            layer.update(&mut KiraSampler(self), dt, &Feed { engine: &out, levels: &car.levels });
        }
        for event in &state.events {
            self.play_impact(&mut car, event);
        }
        if let Some(landing) = landing {
            let event = CarEvent {
                kind: EventKind::BottomOut,
                surface: state.surface,
                magnitude: landing.magnitude,
                front: false,
                smackable: false,
            };
            self.play_impact(&mut car, &event);
        }
        let levels = car.levels;
        for command in commands.drain(..) {
            self.apply(&mut car, scale_command(&levels, command));
        }
        car.commands = commands;
        self.car = Some(car);
    }

    fn start_car(&mut self, name: &str) -> Result<CarAudio, String> {
        let loaded = self.load_car_engine(name)?;
        let tuning = tuning(&loaded.sound, loaded.accel_min_frequency());
        let silent = EngineMix::shared(loaded.sound.engine.min_rpm, 1.0, 0.0, 0.0);
        let handle = self.start_engine(super::EngineVoice { start: silent, ..loaded.voice })?;
        log::info!("engine sound: {} ({})", loaded.sound.engine.name, loaded.sound.engine.accel_loop);
        let dual = !loaded.sound.engine.decel_loop.is_empty();
        let mixer = CarMixer::load(&self.dir, dual)
            .map_err(|e| log::warn!("no mixer map, the effects play at their generated levels: {e}"))
            .ok();
        let aems = AemsLayer::load(self, &loaded.sound).map_err(|e| log::warn!("no sample layer for {name}: {e}")).ok();
        Ok(CarAudio {
            levels: Levels::unmixed(),
            aems,
            mixer,
            master_volume: loaded.sound.engine.master_volume,
            car: name.to_owned(),
            engine: EngineMixer::new(&tuning),
            effects: EffectsMixer::new(&tuning),
            sound: loaded.sound,
            handle,
            redlining: false,
            loops: HashMap::new(),
            commands: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests;
