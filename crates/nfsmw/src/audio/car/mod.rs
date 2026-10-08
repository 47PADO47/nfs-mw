//! The sound of the car being driven: the scene reports its telemetry each frame, the engine mixer
//! (`blackbox-carsound`) turns it into the Ginsu frequency and loop volumes the engine voice plays, and the
//! effects mixer into the shift, turbo, tire, road and collision sounds.

use std::collections::HashMap;

use blackbox_carsound::{CarInput, EffectsMixer, EngineMixer, LoopId, ScrapeKind, SoundCommand, SoundRef};
use kira::sound::static_sound::StaticSoundHandle;
use nfsmw_data::sound::{CarSound, EventKind};

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
    handle: EngineHandle,
    /// The loops that are playing, with the sound each plays.
    pub loops: HashMap<LoopId, (SoundRef, StaticSoundHandle)>,
    commands: Vec<SoundCommand>,
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
        let out = car.engine.update(dt, &state.input);
        car.handle.set(EngineMix {
            frequency: out.ginsu_frequency,
            accel_volume: out.accel_volume,
            decel_volume: out.decel_volume,
            pitch: state.input.pitch_multiplier,
        });
        car.effects.scrape(state.scrape);
        let mut commands = std::mem::take(&mut car.commands);
        commands.clear();
        let landing = car.effects.update(dt, &state.input, &out, &mut commands);
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
        for command in commands.drain(..) {
            self.apply(&mut car, command);
        }
        car.commands = commands;
        self.car = Some(car);
    }

    fn start_car(&mut self, name: &str) -> Result<CarAudio, String> {
        let loaded = self.load_car_engine(name)?;
        let tuning = tuning(&loaded.sound);
        let silent = EngineMix { frequency: loaded.sound.engine.min_rpm, ..EngineMix::default() };
        let handle = self.start_engine(super::EngineVoice { start: silent, ..loaded.voice })?;
        log::info!("engine sound: {} ({})", loaded.sound.engine.name, loaded.sound.engine.accel_loop);
        Ok(CarAudio {
            car: name.to_owned(),
            engine: EngineMixer::new(&tuning),
            effects: EffectsMixer::new(&tuning),
            sound: loaded.sound,
            handle,
            loops: HashMap::new(),
            commands: Vec::new(),
        })
    }
}

#[cfg(test)]
mod tests;
