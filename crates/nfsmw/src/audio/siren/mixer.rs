//! The cop-car state of the dynamic mixer, one instance per siren voice. Spec: `docs/specs/cop-siren-audio.md` section 5.
use std::sync::Arc;

use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};
use glam::Vec3;

use super::params::SLOTS;
use crate::audio::traffic::mixer::Motion;
use crate::audio::{AudioView, speedbreaker};

/// The cop car's state of the map (state 4 of the thirteen, `docs/formats/mixmap.md`). [decomp]
const STATE: u8 = 4;
/// The siren is object 17 of a cop car (`eCARSFX_SIREN`). [decomp, verified: the map's masters of state 4]
const OBJECT: u8 = 17;
/// The object input that gates the siren's volume: 0x7FFF while the voice is on. [decomp, verified: the map's control 0
/// of state 4 is scaled by it]
const TRIGGER_INDEX: u8 = 5;
/// The cop car's position controller, whose block feeds the 3D controls of the state. [verified: the spatial ids of
/// state 4 carry controller 7]
const POSITION_CONTROLLER: u8 = 7;
/// Output slots of the siren's masters: azimuth 0, volume 1, pitch 3. [decomp, verified in the map]
const AZIMUTH_SLOT: usize = 0;
const VOLUME_SLOT: usize = 1;
const PITCH_SLOT: usize = 3;

/// The mixer's answer for one siren.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct Levels {
    pub volume: i32,
    pub azimuth: u16,
    /// Q12.
    pub pitch: i32,
}

pub(super) struct SirenMixer {
    mixer: Mixer,
    motion: [Motion; SLOTS],
}

fn object(slot: usize) -> ObjectRef {
    ObjectRef { state: STATE, instance: slot as u8, object: OBJECT }
}

impl SirenMixer {
    pub fn new(map: Arc<MixMap>) -> Self {
        let mut counts = [0; 13];
        counts[0] = 1;
        counts[usize::from(STATE)] = SLOTS as u8;
        let mixer = Mixer::new(map, &counts);
        Self { mixer, motion: Default::default() }
    }

    /// Tells the mixer where a siren is, or that the slot has none. `velocities` are the car's and the camera's.
    pub fn place(&mut self, slot: usize, siren: Option<(Vec3, Vec3)>, view: AudioView, velocities: [Vec3; 2]) {
        self.mixer.attach(object(slot), siren.is_some());
        let trigger = InputKey::object(STATE, slot as u8, OBJECT, TRIGGER_INDEX);
        let Some((position, velocity)) = siren else {
            self.mixer.set_input(trigger, 0);
            self.motion[slot] = Motion::default();
            return;
        };
        self.mixer.set_input(trigger, 0x7FFF);
        let block = self.motion[slot].inputs_at(position, velocity, view, velocities);
        for (index, value) in block.into_iter().enumerate() {
            self.mixer.set_input(InputKey::controller(STATE, slot as u8, POSITION_CONTROLLER, index as u8), value);
        }
    }

    /// Runs the map for `dt` seconds with the listener's view.
    pub fn process(&mut self, dt: f32, view: AudioView) {
        self.mixer.set_camera(view.camera_state);
        speedbreaker::publish(&mut self.mixer, &view);
        self.mixer.process(dt);
    }

    pub fn levels(&self, slot: usize) -> Levels {
        let object = object(slot);
        Levels {
            volume: (self.mixer.volume(object, VOLUME_SLOT).unwrap_or(0.0) * 32767.0) as i32,
            azimuth: self.mixer.azimuth(object, AZIMUTH_SLOT).unwrap_or(0),
            pitch: (self.mixer.pitch(object, PITCH_SLOT).unwrap_or(1.0) * 4096.0) as i32,
        }
    }
}
