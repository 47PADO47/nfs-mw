//! The sound of the car being driven: the scene reports its telemetry each frame, the engine mixer
//! (`blackbox-carsound`) turns it into the Ginsu frequency and loop volumes the engine voice plays, and the
//! effects mixer into the shift, turbo, tire, road and collision sounds.

use std::collections::HashMap;
use std::time::Duration;

use blackbox_carsound::{CarInput, EffectsMixer, EngineMixer, LoopId, ScrapeKind, SoundCommand, SoundRef};
use kira::Tween;
use kira::sound::static_sound::StaticSoundHandle;
use nfsmw_data::sound::{CarSound, EventKind, SoundUpgrades};

use super::LoopMix;
use super::aems::{AemsLayer, Feed, KiraSampler};
use super::mixer::{CarMixer, Frame, Levels};
use super::tuning::tuning;
use super::{Audio, EngineHandle, EngineMix};

pub(super) mod ambient;
pub(super) mod power_effects;
pub(super) mod routing;
mod telemetry;
mod update;
mod visual_rpm;
pub use ambient::{RoadContext, RoadTerrain};
pub use telemetry::CarAudioTelemetry;
mod contact;
mod contacts;
pub(super) mod impacts;
pub(super) mod transmission;
pub(super) mod tyres;
pub use contact::{CarContact, ContactSource, ContactVehicle};

/// Every physics state since the previous audio update, in simulation order.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSoundTick {
    /// Simulation duration of this completed physics update, independent of the audio/render interval.
    pub sim_dt: f32,
    pub telemetry: CarAudioTelemetry,
    pub input: CarInput,
    pub contacts: Vec<CarContact>,
    pub surface: u32,
}

/// A collision the physics reported since the last frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarEvent {
    pub sound_profile: u32,
    pub vehicle: Option<contact::ContactVehicle>,
    pub audio_zone: Option<nfsmw_data::sound::CollisionZone>,
    pub kind: EventKind,
    /// The `simsurface` hash of the surface involved.
    pub surface: u32,
    /// 0 to 1.
    pub magnitude: f32,
    /// The hit is on the car's front or back, not its side.
    pub front: bool,
    /// A light prop that falls over (a cone, a sign) rather than a wall.
    pub smackable: bool,
    pub position: glam::Vec3,
}

/// What the scene tells the sound each frame about the car it drives.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSoundState {
    pub telemetry: CarAudioTelemetry,
    /// The car type (`BMWM3GTR`), which picks its sound set.
    pub car: String,
    /// The performance levels that select engine, shifting and induction sound sets.
    pub upgrades: SoundUpgrades,
    pub input: CarInput,
    pub events: Vec<CarEvent>,
    /// What the car is rubbing along and how hard (0 to 1), if anything.
    pub scrape: Option<(ScrapeKind, f32)>,
    /// The `simsurface` hash under the car, for the sound of a landing.
    pub surface: u32,
    pub ticks: Vec<CarSoundTick>,
    pub view: super::AudioView,
}

/// The mixers, the voices and the sound set of one car.
pub(super) struct CarAudio {
    pub car: String,
    pub upgrades: SoundUpgrades,
    pub sound: CarSound,
    engine: EngineMixer,
    pub effects: EffectsMixer,
    /// The dynamic mixer; `None` when the map could not be read.
    pub(super) mixer: Option<CarMixer>,
    /// What the mixer said in the last frame.
    pub levels: Levels,
    /// The sample layer (engine samples and sputters); `None` when the banks could not be run.
    aems: Option<AemsLayer>,
    sample_available: bool,
    /// `engineaudio.Master_Vol`.
    master_volume: u32,
    handle: EngineHandle,
    /// The engine was at the limiter in the last frame (for the log).
    redlining: bool,
    /// The loops that are playing, with the sound each plays.
    pub loops: HashMap<LoopId, (SoundRef, StaticSoundHandle)>,
    commands: Vec<SoundCommand>,
    contacts: contacts::ContactPlayback,
    pub(super) impacts: impacts::ImpactPlayback,
    ambient: ambient::AmbientPlayback,
    tyres: tyres::TyrePlayback,
    transmission: transmission::TransmissionPlayback,
    power: power_effects::PowerPlayback,
    pub(super) event_delay: f32,
}

struct DriveFrame<'a> {
    telemetry: &'a CarAudioTelemetry,
    input: &'a CarInput,
    contacts: &'a [CarContact],
    events: &'a [CarEvent],
    view: &'a super::AudioView,
    dt: f32,
    sim_dt: f32,
    delay: f32,
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
        if let Some(state) = state {
            self.speedbreaker.view = state.view;
            self.preload_car_sounds(&state.car, state.upgrades);
        }
        if state.is_some() {
            self.prepare_breaker_moments();
            self.prepare_explosion();
            self.prepare_thunder();
        }
        self.update_speedbreaker(state.is_some_and(|state| state.view.speedbreaker), dt);
        self.update_thunder(state.map(|state| state.view), dt);
        let mut car = self.car.take();
        let mut failed = self.failed.take();
        self.follow_car(&mut car, &mut failed, state, dt);
        self.car = car;
        self.failed = failed;
    }

    /// Player 2's engine in a co-op session: the same car path as `drive_car`, a second voice of its own. Without a
    /// guest's car (`None`) the voice stops and nothing is created.
    pub fn drive_second_car(&mut self, state: Option<&CarSoundState>, dt: f32) {
        let mut car = self.second_car.take();
        let mut failed = self.second_failed.take();
        self.follow_car(&mut car, &mut failed, state, dt);
        self.second_car = car;
        self.second_failed = failed;
    }

    /// Plays one engine voice (`slot`) of the car `state` describes: starts its sounds when the car changes, feeds the
    /// mixers, and silences the voice when there is no car. `failed` keeps the selector whose sound did not load.
    fn follow_car(
        &mut self,
        slot: &mut Option<CarAudio>,
        failed: &mut Option<(String, SoundUpgrades)>,
        state: Option<&CarSoundState>,
        dt: f32,
    ) {
        let Some(state) = state else {
            *slot = None;
            *failed = None;
            return;
        };
        let selectors_changed = slot.as_ref().is_some_and(|c| c.car == state.car && c.upgrades != state.upgrades);
        if selectors_changed
            && let Ok(database) = self.database()
            && let Ok(mut selected) = nfsmw_data::sound::car_sound(&database, &state.car, state.upgrades)
            && let Some(car) = slot.as_mut()
        {
            selected.engine_level = car.sound.engine_level;
            if car.sound == selected {
                car.upgrades = state.upgrades;
            }
        }
        if slot.as_ref().is_none_or(|c| c.car != state.car || c.upgrades != state.upgrades) {
            *slot = None;
            if failed.as_ref().is_some_and(|(car, upgrades)| car == &state.car && *upgrades == state.upgrades)
                || !self.available()
            {
                return;
            }
            match self.start_car(&state.car, state.upgrades) {
                Ok(car) => {
                    *failed = None;
                    *slot = Some(car);
                }
                Err(e) => {
                    log::warn!("no engine sound for {}: {e}", state.car);
                    *failed = Some((state.car.clone(), state.upgrades));
                    return;
                }
            }
        }
        let Some(mut car) = slot.take() else { return };
        let dt = if dt.is_finite() { dt.max(0.0) } else { 0.0 };
        if state.ticks.is_empty() {
            self.advance_car(
                &mut car,
                &DriveFrame {
                    input: &state.input,
                    telemetry: &state.telemetry,
                    contacts: &[],
                    events: &state.events,
                    view: &state.view,
                    dt,
                    sim_dt: 0.0,
                    delay: 0.0,
                },
            );
            *slot = Some(car);
            return;
        }
        let delta = dt / state.ticks.len() as f32;
        for (i, tick) in state.ticks.iter().enumerate() {
            self.advance_car(
                &mut car,
                &DriveFrame {
                    input: &tick.input,
                    telemetry: &tick.telemetry,
                    contacts: &tick.contacts,
                    events: if i == 0 { &state.events } else { &[] },
                    view: &state.view,
                    dt: delta,
                    sim_dt: tick.sim_dt,
                    delay: i as f32 * delta,
                },
            );
        }
        *slot = Some(car);
    }

    /// The pops of the car's sputter since the last call (0 without a car, a sample layer or sputters).
    pub fn take_sputter_pops(&mut self) -> u32 {
        self.car.as_mut().and_then(|car| car.aems.as_mut()).map_or(0, AemsLayer::take_pops)
    }

    fn start_car(&mut self, name: &str, upgrades: SoundUpgrades) -> Result<CarAudio, String> {
        let loaded = self.load_car_engine_upgraded(name, upgrades)?;
        let mut tuning = tuning(&loaded.sound, loaded.accel_min_frequency());
        if let Some(turbo) = tuning.turbo.as_mut() {
            for (id, seconds) in turbo.blowoff_seconds.iter_mut().enumerate() {
                let Some(target) = super::refs::resolve(&loaded.sound, SoundRef::TurboBlowoff(id as u8)) else {
                    continue;
                };
                match self.bank_sound(&target.bank, target.index) {
                    Ok(data) => *seconds = data.duration().as_secs_f32(),
                    Err(error) => log::warn!("turbo duration unavailable: {error}"),
                }
            }
        }
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
            sample_available: aems.as_ref().is_some_and(AemsLayer::engine_available),
            aems,
            mixer,
            master_volume: loaded.sound.engine.master_volume,
            car: name.to_owned(),
            upgrades,
            engine: EngineMixer::new(&tuning),
            effects: EffectsMixer::new(&tuning),
            sound: loaded.sound,
            handle,
            redlining: false,
            loops: HashMap::new(),
            commands: Vec::new(),
            contacts: contacts::ContactPlayback::default(),
            impacts: impacts::ImpactPlayback::default(),
            ambient: ambient::AmbientPlayback::default(),
            tyres: tyres::TyrePlayback::default(),
            transmission: transmission::TransmissionPlayback::default(),
            power: power_effects::PowerPlayback::default(),
            event_delay: 0.0,
        })
    }
}

#[cfg(test)]
mod regression_tests;
#[cfg(test)]
mod routing_tests;
#[cfg(test)]
mod second_voice_tests;
#[cfg(test)]
mod speedbreaker_engine_tests;
#[cfg(test)]
mod tests;
