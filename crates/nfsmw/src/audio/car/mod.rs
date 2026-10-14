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
use super::mixer::{CarMixer, Frame, Levels, scale_command};
use super::tuning::tuning;
use super::{Audio, EngineHandle, EngineMix};

pub(super) mod ambient;
mod telemetry;
pub use ambient::{RoadContext, RoadTerrain};
pub use telemetry::CarAudioTelemetry;
mod contact;
mod contacts;
mod impacts;
pub(super) mod transmission;
pub(super) mod tyres;
pub use contact::{CarContact, ContactSource};

/// Every physics state since the previous audio update, in simulation order.
#[derive(Debug, Clone, PartialEq)]
pub struct CarSoundTick {
    pub telemetry: CarAudioTelemetry,
    pub input: CarInput,
    pub contacts: Vec<CarContact>,
    pub surface: u32,
}

/// A collision the physics reported since the last frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CarEvent {
    pub sound_profile: u32,
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
        let Some(state) = state else {
            self.car = None;
            self.failed = None;
            return;
        };
        let selectors_changed = self.car.as_ref().is_some_and(|c| c.car == state.car && c.upgrades != state.upgrades);
        if selectors_changed
            && let Ok(database) = self.database()
            && let Ok(mut selected) = nfsmw_data::sound::car_sound(&database, &state.car, state.upgrades)
            && let Some(car) = self.car.as_mut()
        {
            selected.engine_level = car.sound.engine_level;
            if car.sound == selected {
                car.upgrades = state.upgrades;
            }
        }
        if self.car.as_ref().is_none_or(|c| c.car != state.car || c.upgrades != state.upgrades) {
            self.car = None;
            if self.failed.as_ref().is_some_and(|(car, upgrades)| car == &state.car && *upgrades == state.upgrades)
                || !self.available()
            {
                return;
            }
            match self.start_car(&state.car, state.upgrades) {
                Ok(car) => {
                    self.failed = None;
                    self.car = Some(car);
                }
                Err(e) => {
                    log::warn!("no engine sound for {}: {e}", state.car);
                    self.failed = Some((state.car.clone(), state.upgrades));
                    return;
                }
            }
        }
        let Some(mut car) = self.car.take() else { return };
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
            self.car = Some(car);
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
                    sim_dt: blackbox_vehicle::FIXED_STEP,
                    delay: i as f32 * delta,
                },
            );
        }
        self.car = Some(car);
    }

    fn advance_car(&mut self, car: &mut CarAudio, frame: &DriveFrame<'_>) {
        car.event_delay = frame.delay;
        let sample_available = car.aems.as_ref().is_some_and(AemsLayer::engine_available);
        if car.sample_available && !sample_available {
            log::warn!("{}: engine sample layer failed; Ginsu remains audible at the limiter", car.car);
        }
        car.sample_available = sample_available;
        car.engine.set_redline_sample_available(sample_available);
        // Keep the recording-range ratio separate from the dynamic mixer's pitch.
        let input = CarInput { pitch_multiplier: 1.0, ..*frame.input };
        let out = car.engine.update(frame.dt, &input);
        let mut commands = std::mem::take(&mut car.commands);
        commands.clear();
        let landing = car.effects.update(frame.dt, &input, &out, &mut commands);
        if let Some(mixer) = car.mixer.as_mut() {
            mixer.set_view(*frame.view);
            mixer.set_landing(landing);
            let sparks = car.aems.as_ref().is_some_and(AemsLayer::sparks);
            car.levels = mixer.update(
                frame.dt,
                &Frame {
                    input: &input,
                    engine: &out,
                    effects: car.effects.signals(),
                    master_volume: car.master_volume,
                    sparks,
                },
            );
        }
        let levels = car.levels;
        car.handle.set_tick(
            EngineMix {
                accel: LoopMix {
                    frequency: out.accel_loop.frequency,
                    volume: out.accel_volume * levels.engine_volume,
                    pitch: out.accel_loop.playback_rate * levels.engine_pitch,
                },
                decel: LoopMix {
                    frequency: out.decel_loop.frequency,
                    volume: out.decel_volume * levels.engine_volume,
                    pitch: out.decel_loop.playback_rate * levels.engine_pitch,
                },
                low_pass_hz: out.low_pass_hz.min(levels.engine_low_pass_hz),
                azimuth: levels.engine_azimuth as u16,
                dry: (levels.environment.ginsu_dry.clamp(0, 32767) >> 8) as f32 / 127.0,
            },
            out.tick_revision,
        );
        if out.redlining != car.redlining {
            car.redlining = out.redlining;
            log::debug!("{}: engine limiter {}", car.car, if out.redlining { "starts" } else { "ends" });
        }
        if let Some(layer) = car.aems.as_mut() {
            let below_min = if out.accel_loop.frequency > 0.0 {
                (out.ginsu_frequency / out.accel_loop.frequency).min(1.0)
            } else {
                1.0
            };
            layer.set_synth_frequency(car.handle.current_frequency() * below_min);
            layer.update(&mut KiraSampler(self), frame.dt, &Feed { engine: &out, levels: &levels });
        }
        let hits = car.contacts.observe(self, &car.sound, car.mixer.as_ref(), frame.contacts, frame.sim_dt);
        for event in hits.iter().chain(frame.events) {
            self.play_impact(car, event, &input, frame.view);
        }
        if let Some(landing) = landing {
            self.play_landing(car, landing);
        }
        let landing_level = car.mixer.as_ref().map_or((1.0, 0), CarMixer::landing_level);
        car.impacts.update(self, frame.dt, frame.delay, &input, frame.view, landing_level);
        car.contacts.update(self, frame.dt, &input, frame.view);
        let tyre_frame = tyres::TyreFrame {
            understeer: frame.telemetry.understeer,
            oversteer: frame.telemetry.oversteer,
            side_surface: [[0, 3], [1, 2]]
                .map(|side| side.into_iter().map(|i| i32::from(input.wheels[i].skid_surface)).max().unwrap_or(0)),
            skid_type: 0,
        };
        if car.tyres.update(self, &car.sound, &input, tyre_frame, car.mixer.as_ref().map(CarMixer::tyres), frame.dt) {
            for side in 0..2 {
                self.apply(car, SoundCommand::StopLoop(LoopId::Skid(side)));
            }
            tyres::TyrePlayback::consume(&mut commands);
        }
        car.transmission.update(
            self,
            &car.sound,
            transmission::TransmissionFrame {
                forward_speed: frame.telemetry.forward_speed,
                shifting: out.shift_state != blackbox_carsound::ShiftState::None,
            },
            car.mixer.as_ref().map(CarMixer::transmission),
            frame.dt,
        );
        car.ambient.update_with_context(
            self,
            &input,
            &mut commands,
            car.mixer.as_ref().map_or_else(Default::default, CarMixer::ambient),
            frame.dt,
            frame.telemetry.road.as_ref(),
        );
        for id in [LoopId::Wind, LoopId::Road(0), LoopId::Road(1)] {
            if car.ambient.owns_loop(id) {
                self.apply(car, SoundCommand::StopLoop(id));
            }
        }
        for command in commands.drain(..) {
            self.apply(car, scale_command(&levels, command));
        }
        car.commands = commands;
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
            event_delay: 0.0,
        })
    }
}

#[cfg(test)]
mod regression_tests;
#[cfg(test)]
mod tests;
