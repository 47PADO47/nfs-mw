//! Authored power effects; spec: docs/specs/car-audio-power-effects.md.
mod params;
#[cfg(test)]
mod tests;
mod turbo;

use blackbox_carsound::{CarInput, SoundRef};
use nfsmw_data::sound::CarSound;

use crate::audio::aems::{EffectModule, EffectSampler};
use crate::audio::{Audio, Group};

/// Decoded/raw dynamic-mixer outputs, with no legacy makeup factor.
#[derive(Debug, Clone, Copy)]
pub(in crate::audio) struct PowerLevels {
    pub nitrous_volume: i32,
    pub purge_volume: i32,
    pub nitrous_azimuth: i32,
    pub nitrous_pitch: i32,
    pub nitrous_low_pass: i32,
    pub shift_azimuth: i32,
    /// Up, down, engage, disengage, accelerate, engine-off, brake mash.
    pub shift_volume: [i32; 7],
    pub turbo_azimuth: i32,
    /// Spool, short blowoff, long blowoff (raw slots1,2,3).
    pub turbo_volume: [i32; 3],
    pub whine_volume: i32,
}

#[derive(Debug, Clone, Copy, Default)]
pub(in crate::audio) struct PowerFrame {
    pub physics_rpm: f32,
    pub rpm_at_shift: f32,
    /// Original POV equals zero; the caller supplies the established mapping.
    pub camera_pov_zero: bool,
    pub engine_torque: f32,
    pub physics_torque: f32,
    pub shifting: bool,
    /// Physical vehicle maximum RPM; engineaudio.MaxRPM is not a substitute.
    pub max_physics_rpm: f32,
    pub exhaust_rotation: i32,
}

struct Running {
    module: EffectModule,
    words: Vec<i32>,
    group: Group,
}

impl Running {
    fn update(&mut self, audio: &mut Audio, dt: f32) -> bool {
        self.module.update(&mut EffectSampler(audio, self.group), dt, &self.words) && !self.module.finished()
    }
}

/// Separate ownership of nitrous/purge, clunk/brake, and four sweetener objects.
#[derive(Default)]
pub(in crate::audio) struct PowerPlayback {
    nitrous: Option<Running>,
    purge: Option<Running>,
    shots: [Option<Running>; 6],
    shot_healthy: [bool; 6],
    purge_healthy: bool,
    active: bool,
    nitrous_owned: bool,
    whine: Option<Running>,
    whine_owned: bool,
    whine_failed: bool,
    turbo: turbo::TurboPlayback,
    seed: u32,
    /// A mod's nitrous variant plays its own sounds, so the game's are not started (`nitrous_fx.rs`).
    silence_nitrous: bool,
}

impl PowerPlayback {
    /// Keep the game's own nitrous sound silent (a mod variant stands in for it), or let it play again. Off by default, so
    /// the game's nitrous is exactly as it was unless a variant asks.
    pub fn set_silence_nitrous(&mut self, silent: bool) {
        self.silence_nitrous = silent;
    }

    /// Whether an authored route remains healthy, including naturally completed one-shots.
    pub fn owns(&self, sound: SoundRef) -> bool {
        match sound {
            SoundRef::Nitrous => self.nitrous_owned,
            SoundRef::ReverseWhine => self.whine_owned,
            SoundRef::TurboSpool | SoundRef::TurboBlowoff(_) => self.turbo.owns(),
            SoundRef::Purge => self.purge_healthy,
            SoundRef::GearClunk { .. } => self.shot_healthy[0],
            SoundRef::BrakeMash => self.shot_healthy[1],
            SoundRef::Sweetener(kind) => {
                self.shot_healthy[match kind {
                    blackbox_carsound::SweetenerKind::Disengage => 2,
                    blackbox_carsound::SweetenerKind::Engage => 3,
                    blackbox_carsound::SweetenerKind::Accelerate => 4,
                    blackbox_carsound::SweetenerKind::EngineOff => 5,
                }]
            }
            _ => false,
        }
    }

    /// Route one semantic one-shot, returning false without consuming it if unavailable.
    /// Call before `update` for the same ordered simulation interval.
    pub fn play(
        &mut self,
        audio: &mut Audio,
        sound: &CarSound,
        frame: PowerFrame,
        levels: Option<PowerLevels>,
        event: SoundRef,
    ) -> bool {
        let Some(levels) = levels else { return false };
        if let SoundRef::TurboBlowoff(id) = event {
            return self.turbo.play(audio, sound, frame, levels, id);
        }
        if event == SoundRef::Purge {
            if self.purge.is_some() {
                return true;
            }
            let Some(bank) = sound.banks.nitrous_bank() else { return false };
            self.purge =
                self.start(audio, &format!("NOS/{bank}"), "FX_PURGE", params::purge(levels).to_vec(), Group::Sfx);
            self.purge_healthy = self.purge.is_some();
            return self.purge_healthy;
        }
        let Some((slot, class, words)) = params::shot(sound, frame, levels, event) else { return false };
        let (bank, group) = match class {
            "CAR_SWTN" => {
                let Some(bank) = sound.engine.sweet_banks.first().filter(|bank| !bank.is_empty()) else {
                    return false;
                };
                (format!("ENGINE/{bank}"), Group::Engine)
            }
            _ => (format!("SHIFTING/{}", sound.shift.bank), Group::Sfx),
        };
        self.shots[slot] = self.start(audio, &bank, class, words, group);
        self.shot_healthy[slot] = self.shots[slot].is_some();
        self.shot_healthy[slot]
    }

    pub fn update(
        &mut self,
        audio: &mut Audio,
        sound: &CarSound,
        input: &CarInput,
        frame: PowerFrame,
        levels: Option<PowerLevels>,
        dt: f32,
    ) {
        let Some(levels) = levels else {
            self.nitrous = None;
            self.purge = None;
            self.shots = Default::default();
            self.shot_healthy = [false; 6];
            self.purge_healthy = false;
            self.whine = None;
            self.whine_owned = false;
            self.whine_failed = false;
            self.turbo = Default::default();
            self.nitrous_owned = false;
            self.active = false;
            return;
        };
        self.turbo.update(audio, sound, frame, levels, dt);
        if input.gear != blackbox_carsound::GEAR_REVERSE {
            self.whine = None;
            self.whine_failed = false;
        }
        if input.gear == blackbox_carsound::GEAR_REVERSE {
            if self.whine.is_none() && !self.whine_failed {
                self.whine = sound.engine.sweet_banks.get(1).and_then(|bank| {
                    self.start(
                        audio,
                        &format!("ENGINE/{bank}"),
                        "CAR_WHINE",
                        params::whine(frame, levels).to_vec(),
                        Group::Engine,
                    )
                });
                self.whine_owned = self.whine.is_some();
                self.whine_failed = self.whine.is_none();
            }
            if let Some(running) = self.whine.as_mut() {
                running.words = params::whine(frame, levels).to_vec();
                if !running.update(audio, dt) {
                    self.whine = None;
                    self.whine_owned = false;
                    self.whine_failed = true;
                }
            }
        }
        let nos_active = input.nos_active && !self.silence_nitrous;
        if nos_active && !self.active {
            self.nitrous = sound.banks.nitrous_bank().and_then(|bank| {
                self.start(
                    audio,
                    &format!("NOS/{bank}"),
                    "FX_NITROUS",
                    params::nitrous(levels, false).to_vec(),
                    Group::Sfx,
                )
            });
            self.nitrous_owned = self.nitrous.is_some();
        }
        if !nos_active
            && self.active
            && let Some(running) = self.nitrous.as_mut()
        {
            running.words[3] = 1;
            running.module.release();
        }
        self.active = nos_active;
        if let Some(running) = self.nitrous.as_mut() {
            running.words = params::nitrous(levels, running.words[3] != 0).to_vec();
            if !running.update(audio, dt) {
                // A completed released graph is healthy; an unexpected active completion is unavailable.
                if self.active {
                    self.nitrous_owned = false;
                }
                self.nitrous = None;
            }
        }
        if let Some(running) = self.purge.as_mut() {
            running.words = params::purge(levels).to_vec();
            if !running.update(audio, dt) {
                if !running.module.finished() {
                    self.purge_healthy = false;
                }
                self.purge = None;
            }
        }
        if input.brake < 0.999 {
            self.shots[1] = None;
        }
        for (index, slot) in self.shots.iter_mut().enumerate() {
            if slot.as_mut().is_some_and(|running| {
                if running.update(audio, dt) {
                    return false;
                }
                if !running.module.finished() {
                    self.shot_healthy[index] = false;
                }
                true
            }) {
                *slot = None;
            }
        }
    }

    fn start(&mut self, audio: &mut Audio, bank: &str, class: &str, words: Vec<i32>, group: Group) -> Option<Running> {
        self.seed = self.seed.wrapping_add(1);
        match EffectModule::load(audio, bank, class, self.seed) {
            Ok(module) => Some(Running { module, words, group }),
            Err(error) => {
                log::warn!("authored {class} unavailable: {error}");
                None
            }
        }
    }
}
