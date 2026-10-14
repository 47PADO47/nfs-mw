//! One collision instance of the installed map, with authored event classification.
use crate::audio::AudioView;
use blackbox_carsound::{CarInput, EngineOutput};
use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};
use glam::Vec3;
use nfsmw_data::sound::ImpactSound;
use std::sync::Arc;

const STATE: u8 = 7;
const OBJECT: ObjectRef = ObjectRef { state: STATE, instance: 0, object: 0 };

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CollisionLevels {
    pub volume: f32,
    pub pitch: f32,
    pub azimuth: u16,
    pub wet: f32,
}

pub struct CollisionMixer {
    mixer: Mixer,
    first: bool,
    impact_point: Option<Vec3>,
}

pub struct CollisionFrame<'a> {
    pub sound: &'a ImpactSound,
    pub magnitude: f32,
    pub point: Vec3,
    pub view: &'a AudioView,
    pub input: &'a CarInput,
    pub scrape: bool,
}

pub fn slots(sound: &ImpactSound) -> [usize; 4] {
    if sound.has("SMOKABLE") && sound.has("CAR") && sound.has("EVENT") {
        return [10, 2, 17, 13];
    }
    if sound.has("SMOKABLE") {
        return [11, 2, 17, 14];
    }
    if !sound.has("CAR") {
        return [7, 0, 16, 13];
    }
    if sound.has("WALL") {
        return [if sound.has("FRONT") { 7 } else { 8 }, 0, 16, 13];
    }
    if sound.has("BOTTOMOUT") {
        return [6, 0, 16, 13];
    }
    if sound.has("ROLLOVER") {
        return [5, 0, 16, 13];
    }
    if sound.has("SIDE") {
        return [4, 0, 16, 13];
    }
    if sound.has("FRONT") {
        return [3, 0, 16, 13];
    }
    [7, 0, 16, 13]
}

impl CollisionMixer {
    pub fn new(map: Arc<MixMap>) -> Self {
        let mut counts = [0; 13];
        counts[0] = 1;
        counts[2] = 1;
        counts[7] = 1;
        let mut mixer = Mixer::new(map, &counts);
        mixer.attach(OBJECT, true);
        Self { mixer, first: true, impact_point: None }
    }

    pub fn update(&mut self, dt: f32, frame: CollisionFrame<'_>) -> CollisionLevels {
        let CollisionFrame { sound, magnitude, point, view, input, scrape } = frame;
        self.mixer.set_camera(view.camera_state);
        let frame = super::Frame {
            input,
            engine: &EngineOutput::default(),
            effects: Default::default(),
            master_volume: 32767,
            sparks: false,
        };
        super::feed::publish(&mut self.mixer, &frame, view);
        for i in 0..3 {
            self.mixer.set_input(InputKey::object(STATE, 0, 0, i), 0);
        }
        if self.first && !scrape {
            let trigger = if sound.has("TWO_CAR") {
                Some(0)
            } else if sound.has("SMOKABLE") {
                Some(2)
            } else if sound.has("CAR") {
                Some(1)
            } else {
                None
            };
            if let Some(trigger) = trigger {
                self.mixer.set_input(InputKey::object(STATE, 0, 0, trigger), 32767);
            }
            self.mixer.set_input(InputKey::object(STATE, 0, 0, 4), (magnitude.clamp(0.0, 1.0) * 127.0) as i32 * 255);
        }
        self.first = false;
        let impact = *self.impact_point.get_or_insert(point);
        for (controller, position) in [(0, view.distances(impact)), (1, view.distances(point))] {
            for (i, value) in position.into_iter().enumerate() {
                self.mixer.set_input(InputKey::controller(STATE, 0, controller, i as u8), value);
            }
            self.mixer.set_input(InputKey::controller(STATE, 0, controller, 15), 1);
        }
        self.mixer.process(dt);
        let [volume, azimuth, pitch, wet] = if scrape { [12, 1, 18, 15] } else { slots(sound) };
        CollisionLevels {
            volume: self.mixer.volume(OBJECT, volume).unwrap_or(0.0),
            pitch: self.mixer.pitch(OBJECT, pitch).unwrap_or(1.0),
            azimuth: self.mixer.azimuth(OBJECT, azimuth).unwrap_or(0),
            wet: self.mixer.volume(OBJECT, wet).unwrap_or(0.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sound(flags: &[&str]) -> ImpactSound {
        ImpactSound { description: flags.iter().map(|s| (*s).into()).collect(), ..Default::default() }
    }
    #[test]
    fn collision_classes_follow_original_precedence_and_require_all_event_bits() {
        for (flags, expected) in [
            (vec!["SMOKABLE", "CAR", "EVENT", "WALL", "FRONT"], [10, 2, 17, 13]),
            (vec!["SMOKABLE", "CAR"], [11, 2, 17, 14]),
            (vec!["CAR", "WALL", "FRONT"], [7, 0, 16, 13]),
            (vec!["CAR", "WALL", "SIDE"], [8, 0, 16, 13]),
            (vec!["CAR", "BOTTOMOUT", "SIDE"], [6, 0, 16, 13]),
            (vec!["CAR", "ROLLOVER", "FRONT"], [5, 0, 16, 13]),
            (vec!["CAR", "SIDE", "FRONT"], [4, 0, 16, 13]),
            (vec!["CAR", "FRONT"], [3, 0, 16, 13]),
        ] {
            assert_eq!(slots(&sound(&flags)), expected);
        }
    }
}
