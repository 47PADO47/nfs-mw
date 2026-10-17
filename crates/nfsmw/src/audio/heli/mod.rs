//! The police helicopter's rotor loop. The helicopter's `FX_Helicopter` AEMS module (`IG_GLOBAL/FX_Helicopter_MB.abk`,
//! `AEMS_MiscBanks`) runs like the traffic engines: one module per helicopter, fed the twelve class words every tick,
//! placed by the map's 3D control from the listener's view and mixed in the SFX group, so the master, SFX and pause
//! settings apply. Spec: `docs/specs/ai-helicopter.md` section 7.
mod mixer;
mod params;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use blackbox_mixmap::MixMap;
use glam::Vec3;

use super::Audio;
use super::AudioView;
use super::Group;
use super::aems::{DelayedSampler, EffectModule};

const BANK: &str = "IG_GLOBAL/FX_Helicopter_MB.abk";
const CLASS: &str = "FX_Helicopter";
const MAP: &str = "SOUND/MIXMAPS/MAPOUTPUT.mxb";
/// The module's sample seed. One helicopter flies at a time, so the seed is fixed. [unconfirmed]
const SEED: u32 = 0;

/// One frame of the helicopter, in render space. The scene feeds it while a helicopter is in the chase.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HeliFrame {
    /// Seconds since the last frame.
    pub dt: f32,
    pub position: Vec3,
    /// The direction the helicopter moves in, flat (the scene has no heading accessor yet).
    pub forward: Vec3,
    pub velocity: Vec3,
}

impl HeliFrame {
    fn is_usable(&self) -> bool {
        self.dt.is_finite()
            && self.dt > 0.0
            && self.position.is_finite()
            && self.forward.is_finite()
            && self.velocity.is_finite()
    }
}

/// The rotor's state across frames. Taken out of [`Audio`] while it runs, so the module can use the audio.
#[derive(Default)]
pub(in crate::audio) struct Slot {
    map: Option<Arc<MixMap>>,
    /// Set once a file failed to load: the rotor stays silent for the session, and the failure was logged once.
    failed: bool,
    rotor: Option<Rotor>,
}

struct Rotor {
    module: EffectModule,
    mixer: mixer::RotorMixer,
}

impl Audio {
    /// Plays the helicopter's rotor at `frame`, heard from `view`. `None` stops it.
    pub fn update_heli(&mut self, frame: Option<HeliFrame>, view: AudioView) {
        let mut heli = std::mem::take(&mut self.heli);
        heli.update(self, frame, view);
        self.heli = heli;
    }

    /// Stops the rotor, as when the game is paused or the car is gone.
    pub fn stop_heli(&mut self) {
        self.heli.rotor = None;
    }
}

impl Slot {
    fn update(&mut self, audio: &mut Audio, frame: Option<HeliFrame>, view: AudioView) {
        let Some(frame) = frame.filter(HeliFrame::is_usable) else {
            self.rotor = None;
            return;
        };
        if self.failed {
            return;
        }
        if self.rotor.is_none() && !self.start(audio) {
            return;
        }
        let Some(rotor) = self.rotor.as_mut() else { return };
        let levels = rotor.mixer.update(frame.dt, frame.position, view);
        let offset = frame.position - view.car_position;
        let inputs = params::RotorInputs {
            volume: levels.volume,
            azimuth: levels.azimuth,
            pitch: levels.pitch,
            in_front: offset.dot(view.car_forward) >= 0.0,
            distance: offset.length(),
            forward_speed: frame.velocity.dot(frame.forward.normalize_or_zero()),
            bearing: frame.forward.normalize_or_zero().dot((-offset).normalize_or_zero()).clamp(-1.0, 1.0).acos(),
        };
        let words = params::class_words(inputs);
        if !rotor.module.update(&mut DelayedSampler(audio, Group::Sfx, 0.0), frame.dt, &words) {
            self.rotor = None;
        }
    }

    /// Loads the map and the module. Returns `false` (and remembers the failure) when either is missing.
    fn start(&mut self, audio: &mut Audio) -> bool {
        let map = match self.map.clone() {
            Some(map) => map,
            None => match load_map(audio) {
                Ok(map) => {
                    self.map = Some(map.clone());
                    map
                }
                Err(error) => return self.fail(&error),
            },
        };
        match EffectModule::load(audio, BANK, CLASS, SEED) {
            Ok(module) => {
                self.rotor = Some(Rotor { module, mixer: mixer::RotorMixer::new(map) });
                true
            }
            Err(error) => self.fail(&error),
        }
    }

    fn fail(&mut self, error: &str) -> bool {
        self.failed = true;
        log::warn!("the police helicopter's rotor is unavailable: {error}");
        false
    }
}

fn load_map(audio: &mut Audio) -> Result<Arc<MixMap>, String> {
    let bytes = nfsmw_data::shared::map_output(&audio.dir).map_err(|e| format!("{MAP}: {e}"))?;
    MixMap::parse(&bytes).map(Arc::new).map_err(|e| format!("{MAP}: {e}"))
}
