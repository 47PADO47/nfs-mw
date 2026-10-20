//! The sirens of the police cars. Up to four voices attach to cop cars within range of the camera; each follows its car's
//! siren state with the original's lag (`blackbox_pursuit::siren::Voice`) and plays the `SIREN` module of
//! `IG_GLOBAL/SIREN_MB.abk`, placed by the cop-car state of the dynamic mixer and played in the SFX group, so the
//! master, SFX and pause settings apply. Spec: `docs/specs/cop-siren-audio.md`.
mod mixer;
pub(in crate::audio) mod params;
#[cfg(test)]
mod tests;

use blackbox_pursuit::siren::{State, Voice};
use glam::Vec3;

use self::params::SLOTS;
use super::Audio;
use super::AudioView;
use super::Group;
use super::aems::{DelayedSampler, EffectModule};

const BANK: &str = "IG_GLOBAL/SIREN_MB.abk";
const CLASS: &str = "SIREN";

/// Where each slot's siren is and how it moves, when its voice is on.
type Placed = [Option<(Vec3, Vec3)>; SLOTS];
/// The type word each slot's voice plays, when it is on.
type Kinds = [Option<i32>; SLOTS];

/// One police car as the siren sound sees it, in render space.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SirenSource {
    /// A number that stays with the car while it exists.
    pub id: u32,
    /// What the car's siren state machine says.
    pub state: State,
    pub position: Vec3,
    pub velocity: Vec3,
}

/// Every police car for one frame. The scene feeds it while it has any.
#[derive(Debug, Clone, PartialEq)]
pub struct SirenFrame {
    /// Seconds since the last frame.
    pub dt: f32,
    pub sources: Vec<SirenSource>,
}

impl SirenFrame {
    fn is_usable(&self) -> bool {
        self.dt.is_finite() && self.dt > 0.0 && self.dt < 1.0
    }
}

/// One attached voice.
struct Attached {
    id: u32,
    voice: Voice,
    module: Option<EffectModule>,
}

/// The sirens' state across frames. Taken out of [`Audio`] while it runs, so the modules can use the audio.
#[derive(Default)]
pub(in crate::audio) struct Slot {
    /// Set once a file failed to load: the sirens stay silent for the session, and the failure was logged once.
    failed: bool,
    mixer: Option<mixer::SirenMixer>,
    voices: [Option<Attached>; SLOTS],
    clock: f64,
    random: u32,
    /// Where the car and the camera were last frame, for their velocities (the Doppler inputs).
    previous: Option<(Vec3, Vec3)>,
    /// A voice was on last frame.
    sounding: bool,
}

impl Audio {
    /// Plays the sirens of `frame`, heard from `view`. `None` stops them all.
    pub fn update_sirens(&mut self, frame: Option<SirenFrame>, mut view: AudioView) {
        view.speedbreaker_mix = self.speedbreaker.snapshot();
        let mut sirens = std::mem::take(&mut self.sirens);
        sirens.update(self, frame, view);
        self.sirens = sirens;
    }

    /// Stops every siren, as when the game is paused or the car is gone.
    pub fn stop_sirens(&mut self) {
        self.sirens.clear();
    }

    /// The cars that hold a siren voice, with the type word each plays (`None` while its voice is switched off).
    #[cfg(test)]
    pub fn siren_status(&self) -> Vec<(u32, Option<i32>)> {
        self.sirens.status()
    }
}

impl Slot {
    fn clear(&mut self) {
        self.voices = Default::default();
        self.previous = None;
    }

    #[cfg(test)]
    fn status(&self) -> Vec<(u32, Option<i32>)> {
        self.voices.iter().flatten().map(|v| (v.id, v.module.as_ref().map(|_| v.voice.held().word().max(0)))).collect()
    }

    fn next_random(&mut self) -> f32 {
        // xorshift32: the lag and the start delay of the voices only need to differ from car to car.
        let mut x = if self.random == 0 { 0x9E37_79B9 } else { self.random };
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.random = x;
        (x >> 8) as f32 / (1u32 << 24) as f32
    }

    fn update(&mut self, audio: &mut Audio, frame: Option<SirenFrame>, view: AudioView) {
        let Some(frame) = frame.filter(SirenFrame::is_usable) else {
            self.clear();
            return;
        };
        if self.failed || !self.load(audio) {
            return;
        }
        self.clock += f64::from(frame.dt);
        let velocities = match self.previous.replace((view.car_position, view.camera_position)) {
            Some((car, camera)) => [(view.car_position - car) / frame.dt, (view.camera_position - camera) / frame.dt],
            None => [Vec3::ZERO; 2],
        };
        self.attach(&frame, view);
        let (placed, kinds) = self.follow(&frame);
        // Nothing sounds and nothing did last frame: the map has nothing to run.
        let any = placed.iter().any(Option::is_some);
        if !any && !self.sounding {
            return;
        }
        self.sounding = any;
        let Some(mixer) = self.mixer.as_mut() else { return };
        for (index, siren) in placed.iter().enumerate() {
            mixer.place(index, *siren, view, velocities);
        }
        mixer.process(frame.dt, view);
        for index in 0..SLOTS {
            let (Some(kind), Some((position, _))) = (kinds[index], placed[index]) else { continue };
            let Some(attached) = self.voices[index].as_mut() else { continue };
            if attached.module.is_none() {
                match EffectModule::load(audio, BANK, CLASS, self.random ^ index as u32) {
                    Ok(module) => attached.module = Some(module),
                    Err(error) => {
                        self.failed = true;
                        log::warn!("the police sirens are unavailable: {error}");
                        return;
                    }
                }
            }
            let Some(module) = attached.module.as_mut() else { continue };
            let levels = mixer.levels(index);
            let metres = view.distances(position)[1] as f32 / 100.0;
            let words = params::class_words(levels.volume, levels.pitch, levels.azimuth, metres, kind);
            if !module.update(&mut DelayedSampler(audio, Group::Sfx, 0.0), frame.dt, &words) {
                attached.module = None;
            }
        }
    }

    /// Gives the voice slots to the cars in range and takes them from the cars out of range or gone.
    fn attach(&mut self, frame: &SirenFrame, view: AudioView) {
        let mut ids = self.voices.each_ref().map(|v| v.as_ref().map(|v| v.id));
        let cars: Vec<(u32, f32)> =
            frame.sources.iter().map(|s| (s.id, s.position.distance(view.camera_position))).collect();
        let changes = params::assign(&mut ids, &cars);
        for index in changes.detached {
            self.voices[index] = None;
        }
        for index in changes.attached {
            let random = self.next_random();
            let Some(id) = ids[index] else { continue };
            self.voices[index] = Some(Attached { id, voice: Voice::new(self.clock, random), module: None });
        }
    }

    /// Runs every attached voice against its car's state. Returns where the voices that are on are, and the type word
    /// each plays. A voice that switches off loses its sound.
    fn follow(&mut self, frame: &SirenFrame) -> (Placed, Kinds) {
        let mut placed = [None; SLOTS];
        let mut kinds = [None; SLOTS];
        for index in 0..SLOTS {
            let random = self.next_random();
            let Some(attached) = self.voices[index].as_mut() else { continue };
            let Some(source) = frame.sources.iter().find(|s| s.id == attached.id) else { continue };
            let Some(kind) = attached.voice.update(self.clock, frame.dt, source.state, random) else {
                attached.module = None;
                continue;
            };
            placed[index] = Some((source.position, source.velocity));
            kinds[index] = Some(kind);
        }
        (placed, kinds)
    }

    /// Loads the map. Returns `false` (and remembers the failure) when it is missing.
    fn load(&mut self, audio: &mut Audio) -> bool {
        if self.mixer.is_some() {
            return true;
        }
        match crate::audio::mixer::mix_map(&audio.dir) {
            Ok(map) => {
                self.mixer = Some(mixer::SirenMixer::new(map));
                true
            }
            Err(error) => {
                self.failed = true;
                log::warn!("the police sirens are unavailable: {error:#}");
                false
            }
        }
    }
}
