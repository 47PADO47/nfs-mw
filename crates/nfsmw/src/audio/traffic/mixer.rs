//! Installed traffic mixer objects and moving source controller inputs.
use super::{AudioView, TrafficEmitter};
use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};
use glam::Vec3;
use std::sync::Arc;

fn object(id: u8) -> ObjectRef {
    ObjectRef { state: 5, instance: 0, object: id }
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Levels {
    pub azimuth: [i32; 3],
    pub engine: i32,
    pub horn: i32,
    pub whoosh: i32,
    pub pitch: [i32; 3],
}

#[derive(Default)]
pub(super) struct Motion {
    previous_distances: Option<[f32; 2]>,
    previous_speeds: [f32; 2],
}

impl Motion {
    pub fn inputs(&mut self, emitter: &TrafficEmitter, view: AudioView, velocities: [Vec3; 2]) -> [i32; 16] {
        let mut inputs = [0; 16];
        inputs[..4].copy_from_slice(&view.distances(emitter.position));
        let distances = [view.car_position, view.camera_position].map(|p| p.distance(emitter.position));
        let speeds: [f32; 2] = std::array::from_fn(|i| {
            let speed = (emitter.velocity - velocities[i]).length();
            if self.previous_distances.is_some_and(|previous| previous[i] > distances[i]) {
                return -speed;
            }
            speed
        });
        inputs[15] = 1;
        for i in 0..2 {
            inputs[13 + i] = (speeds[i] * 100.0) as i32;
            if self.previous_speeds[i] * speeds[i] < 0.0 {
                inputs[15] |= (1u32 << (31 - i)) as i32;
            }
        }
        self.previous_distances = Some(distances);
        self.previous_speeds = speeds;
        inputs
    }
}

pub(super) struct TrafficMixer {
    mixer: Mixer,
    motion: Motion,
}

impl TrafficMixer {
    pub fn new(map: Arc<MixMap>) -> Self {
        let mut counts = [0; 13];
        counts[0] = 1;
        counts[2] = 1;
        counts[5] = 1;
        let mut mixer = Mixer::new(map, &counts);
        for id in [0, 1, 2] {
            mixer.attach(object(id), true);
        }
        Self { mixer, motion: Motion::default() }
    }

    pub fn update(
        &mut self,
        dt: f32,
        emitter: &TrafficEmitter,
        view: AudioView,
        velocities: [Vec3; 2],
        horn: bool,
        whoosh: bool,
    ) -> Levels {
        self.mixer.set_camera(view.camera_state);
        self.mixer.override_event(0, 0, 2, view.speedbreaker_pitch);
        for index in [0, 1, 2, 3] {
            self.mixer.set_input(InputKey::controller(0, 0, 0, index), 0);
        }
        self.mixer.set_input(InputKey::object(5, 0, 2, 2), if horn { 32767 } else { 0 });
        self.mixer.set_input(InputKey::object(5, 0, 1, 3), if whoosh { 32767 } else { 0 });
        for (index, value) in self.motion.inputs(emitter, view, velocities).into_iter().enumerate() {
            self.mixer.set_input(InputKey::controller(5, 0, 0, index as u8), value);
        }
        let mph = velocities[0].length() * 2.23699;
        for (index, scale) in [1092.2334, 546.1167, 327.67, 234.05].into_iter().enumerate() {
            self.mixer.set_input(InputKey::controller(2, 0, 0, index as u8), ((mph * scale) as i32).clamp(0, 32767));
        }
        self.mixer.process(dt);
        let gain = |id, slot| (self.mixer.volume(object(id), slot).unwrap_or(0.0) * 32767.0) as i32;
        Levels {
            azimuth: [0, 1, 2].map(|id| i32::from(self.mixer.azimuth(object(id), 0).unwrap_or(0))),
            engine: gain(0, 1),
            horn: gain(2, 2),
            whoosh: gain(1, 3),
            pitch: [0, 1, 2].map(|id| (self.mixer.pitch(object(id), 4).unwrap_or(1.0) * 4096.0) as i32),
        }
    }
}
