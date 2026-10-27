//! The sound of the car being driven: the scene reports its telemetry each frame, the engine mixer
//! (`blackbox-carsound`) turns it into the Ginsu frequency and loop volumes the engine voice plays, and the
//! effects mixer into the shift, turbo, tire, road and collision sounds.
//!
//! A car a mod adds has no record in the sound database: its sound set is the class-wide defaults and the shared
//! banks, its engine is its own loops (or the built-in engine), and its shifts, turbo, backfires and pops are its own
//! recordings or silence (`own_engine`, `own_sfx`). It has no Ginsu voice and no sample layer.

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
mod engine_start;
pub(super) mod impacts;
mod nitrous_fx;
mod own_engine;
mod own_sfx;
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
    /// The effects of the nitrous variant the player chose, if it is a mod's: its sounds are played by `nitrous_fx`.
    pub nitrous_fx: Option<std::sync::Arc<blackbox_modding::content::NitrousProfile>>,
    /// The engine switch of a mod (`bb.engine`): whether the engine is silent, and the start sound to play now.
    pub engine: crate::engine_switch::EngineAudio,
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
    /// The Ginsu engine voice; `None` for a car a mod added, whose engine is `own_engine`.
    handle: Option<EngineHandle>,
    /// The engine of a car a mod added: its own loops, or the built-in engine (`None` for a car of the game).
    own_engine: Option<own_engine::OwnEngine>,
    /// A mod car's own shifts, turbo, backfires and pops (`None` for a car of the game or one with none).
    own_sfx: Option<own_sfx::OwnSfx>,
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
    /// The sounds of a mod's nitrous variant (`nitrous_fx.rs`).
    nitrous: nitrous_fx::NitrousVoice,
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
    nitrous_fx: Option<&'a std::sync::Arc<blackbox_modding::content::NitrousProfile>>,
    /// A mod has stopped the engine: it is silent.
    engine_stopped: bool,
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
            && let Ok(mut selected) = self.sound_set(&state.car, state.upgrades)
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
        if let Some(start) = state.engine.start.as_ref() {
            engine_start::play(self, start);
        }
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
                    nitrous_fx: state.nitrous_fx.as_ref(),
                    engine_stopped: state.engine.stopped,
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
                    nitrous_fx: state.nitrous_fx.as_ref(),
                    engine_stopped: state.engine.stopped,
                },
            );
        }
        *slot = Some(car);
    }

    /// The pops of the car's sputter since the last call (0 without a car, a sample layer or sputters). A car a mod added
    /// has no sample layer: its backfire and pops (`own_sfx`) are counted in its place.
    pub fn take_sputter_pops(&mut self) -> u32 {
        let Some(car) = self.car.as_mut() else { return 0 };
        let sputter = car.aems.as_mut().map_or(0, AemsLayer::take_pops);
        let own = car.own_sfx.as_mut().map_or(0, own_sfx::OwnSfx::take_bangs);
        sputter + own
    }

    /// The sound set of the car, its Ginsu voice (a car of the game only) and the lowest frequency of its loops.
    fn sound_and_voice(
        &mut self,
        name: &str,
        upgrades: SoundUpgrades,
    ) -> Result<(CarSound, Option<super::EngineVoice>, f32), String> {
        if nfsmw_data::car::mods::is_mod_car(name) {
            let sound = self.sound_set(name, upgrades)?;
            self.preload_sound_set(&sound);
            return Ok((sound, None, 0.0));
        }
        let loaded = self.load_car_engine_upgraded(name, upgrades)?;
        let lowest = loaded.accel_min_frequency();
        Ok((loaded.sound, Some(loaded.voice), lowest))
    }

    fn start_car(&mut self, name: &str, upgrades: SoundUpgrades) -> Result<CarAudio, String> {
        let (sound, voice, accel_min_frequency) = self.sound_and_voice(name, upgrades)?;
        let mut tuning = tuning(&sound, accel_min_frequency);
        if let Some(turbo) = tuning.turbo.as_mut() {
            for (id, seconds) in turbo.blowoff_seconds.iter_mut().enumerate() {
                let Some(target) = super::refs::resolve(&sound, SoundRef::TurboBlowoff(id as u8)) else {
                    continue;
                };
                match self.bank_sound(&target.bank, target.index) {
                    Ok(data) => *seconds = data.duration().as_secs_f32(),
                    Err(error) => log::warn!("turbo duration unavailable: {error}"),
                }
            }
        }
        let handle = match voice {
            Some(voice) => {
                let silent = EngineMix::shared(sound.engine.min_rpm, 1.0, 0.0, 0.0);
                log::info!("engine sound: {} ({})", sound.engine.name, sound.engine.accel_loop);
                Some(self.start_engine(super::EngineVoice { start: silent, ..voice })?)
            }
            None => None,
        };
        let dual = !sound.engine.decel_loop.is_empty();
        let mixer = CarMixer::load(&self.dir, dual)
            .map_err(|e| log::warn!("no mixer map, the effects play at their generated levels: {e}"))
            .ok();
        let (own_engine, own_sfx) = self.start_mod_sounds(name, handle.is_none())?;
        let aems = match handle {
            Some(_) => AemsLayer::load(self, &sound).map_err(|e| log::warn!("no sample layer for {name}: {e}")).ok(),
            None => None,
        };
        Ok(CarAudio {
            levels: Levels::unmixed(),
            sample_available: aems.as_ref().is_some_and(AemsLayer::engine_available),
            aems,
            mixer,
            master_volume: sound.engine.master_volume,
            car: name.to_owned(),
            upgrades,
            engine: EngineMixer::new(&tuning),
            effects: EffectsMixer::new(&tuning),
            sound,
            handle,
            own_engine,
            own_sfx,
            redlining: false,
            loops: HashMap::new(),
            commands: Vec::new(),
            contacts: contacts::ContactPlayback::default(),
            impacts: impacts::ImpactPlayback::default(),
            ambient: ambient::AmbientPlayback::default(),
            tyres: tyres::TyrePlayback::default(),
            transmission: transmission::TransmissionPlayback::default(),
            power: power_effects::PowerPlayback::default(),
            nitrous: nitrous_fx::NitrousVoice::default(),
            event_delay: 0.0,
        })
    }

    /// The engine and the effects of a car a mod added (`modded`); nothing for a car of the game.
    fn start_mod_sounds(
        &mut self,
        name: &str,
        modded: bool,
    ) -> Result<(Option<own_engine::OwnEngine>, Option<own_sfx::OwnSfx>), String> {
        if !modded {
            return Ok((None, None));
        }
        let engine = self.start_mod_engine(name)?;
        // A car with no recordings still decides its lift-offs, so that they light its tail pipes.
        let silent = nfsmw_data::car::mods::CarEffects::default();
        let effects = nfsmw_data::car::mods::car_effects(name).unwrap_or(&silent);
        let sfx = self.start_own_sfx(name, effects);
        Ok((Some(engine), Some(sfx)))
    }
}

#[cfg(test)]
mod measure;
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
