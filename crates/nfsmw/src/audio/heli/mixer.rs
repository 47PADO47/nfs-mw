//! The helicopter's mixer object: the rotor's state of the installed map, fed the listener's view of the helicopter.
use std::sync::Arc;

use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};
use glam::Vec3;

use crate::audio::{AudioView, speedbreaker};

/// The helicopter state of the map (state 11 of the thirteen, `docs/formats/mixmap.md`). [decomp]
const STATE: u8 = 11;
const OBJECT: ObjectRef = ObjectRef { state: STATE, instance: 0, object: 0 };
/// The controller that feeds the state's 3D control: its object number in the map's spatial record. [verified: the
/// installed `MAPOUTPUT.mxb`, state 11 spatial ids carry object 1]
const SPATIAL_CONTROLLER: u8 = 1;
/// The controller's inputs of a 3D control, as the mixer reads them: distance to the car and to the camera (cm),
/// azimuth to the car and to the camera, and the has-position flag (bit 0) at index 15. [decomp: the mixer's
/// `spatial_node`]
const FLAGS_INDEX: u8 = 15;
/// The output slots the state's master channels write: azimuth 0, volume 1, pitch 2. [verified: the master channels of
/// state 11, object 0, in the installed map]
const AZIMUTH_SLOT: usize = 0;
const VOLUME_SLOT: usize = 1;
const PITCH_SLOT: usize = 2;

/// The mixer's answer for the rotor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Levels {
    pub volume: i32,
    pub azimuth: u16,
    pub pitch: i32,
}

pub(super) struct RotorMixer {
    mixer: Mixer,
}

impl RotorMixer {
    pub fn new(map: Arc<MixMap>) -> Self {
        let mut counts = [0; 13];
        counts[0] = 1;
        counts[usize::from(STATE)] = 1;
        let mut mixer = Mixer::new(map, &counts);
        mixer.attach(OBJECT, true);
        Self { mixer }
    }

    /// Feeds the view's distances and azimuths to the rotor's 3D control, then reads the rotor's levels. The map does
    /// the distance rolloff; this only publishes where the rotor is.
    pub fn update(&mut self, dt: f32, position: Vec3, view: AudioView) -> Levels {
        self.mixer.set_camera(view.camera_state);
        speedbreaker::publish(&mut self.mixer, &view);
        for (index, value) in view.distances(position).into_iter().enumerate() {
            self.mixer.set_input(InputKey::controller(STATE, 0, SPATIAL_CONTROLLER, index as u8), value);
        }
        self.mixer.set_input(InputKey::controller(STATE, 0, SPATIAL_CONTROLLER, FLAGS_INDEX), 1);
        self.mixer.process(dt);
        Levels {
            volume: (self.mixer.volume(OBJECT, VOLUME_SLOT).unwrap_or(0.0) * 32767.0) as i32,
            azimuth: self.mixer.azimuth(OBJECT, AZIMUTH_SLOT).unwrap_or(0),
            pitch: (self.mixer.pitch(OBJECT, PITCH_SLOT).unwrap_or(1.0) * 4096.0) as i32,
        }
    }
}
