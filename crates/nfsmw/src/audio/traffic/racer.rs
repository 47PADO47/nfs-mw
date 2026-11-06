//! Race opponents' engine voices: the AI-racer sound state (main map state 3), at most four connected at once.
//! Spec: `authored-traffic-audio.md` "Actor roles".
use super::{AudioView, RacerEmitter, mixer::Motion};
use crate::audio::{
    Audio, Group,
    aems::{DelayedSampler, EffectModule, engine_bank_path},
    trace::Record,
};
use blackbox_mixmap::{InputKey, MixMap, Mixer, ObjectRef};
use glam::Vec3;
use std::{collections::BTreeMap, sync::Arc};

/// The AI-racer state of the map and its engine object, physics controller and car-position controller.
const STATE: u8 = 3;
const ENGINE_OBJECT: u8 = 0;
const PHYSICS: u8 = 0;
const ENGINE: u8 = 4;
const CAR_POSITION: u8 = 7;
/// At most this many opponents are heard (the AI-racer engine slots).
pub(super) const MAX_VOICES: usize = 4;
/// An opponent connects within this camera distance, metres.
pub(super) const CONNECT_DISTANCE: f32 = 135.0;
/// The engine source sits this far ahead of the car's centre (chase views), metres.
const SOURCE_AHEAD: f32 = 0.2;
/// The engine controller's upgrade input, as the player's car publishes it.
const ENGINE_UPGRADE_LEVEL: i32 = 21_844;
/// `CAR` class words the AI-racer engine sets.
const CAR_WORDS: usize = 26;
const WORD_CLASS: usize = 0;
const WORD_RPM: usize = 1;
const WORD_TORQUE: usize = 3;
const WORD_VOL_ENG: usize = 4;
const WORD_VOL_EXH: usize = 5;
const WORD_SPU_OR_EE: usize = 12;
const WORD_AZIMUTH: usize = 15;
const WORD_PITCH_OFFSET: usize = 16;
const WORD_LOW_PASS: usize = 21;

/// What one voice needs of the car's sound set.
pub(super) struct EngineData {
    pub car_id: u32,
    pub min_rpm: f32,
    pub max_rpm: f32,
    pub master_volume: u32,
}

/// What the AI-racer state's engine object reads out of the map (volume 0..1, pitch multiplier, azimuth), and the
/// engine controller's camera-distance low-pass cutoff in hertz.
#[derive(Debug, Clone, Copy)]
pub(super) struct MixOut {
    pub volume: f32,
    pub pitch: f32,
    pub azimuth: i32,
    pub low_pass: i32,
}

/// The step a frame advances: listener velocities (car, camera), seconds, and the output delay of the step.
#[derive(Debug, Clone, Copy)]
pub(super) struct Step {
    pub velocities: [Vec3; 2],
    pub dt: f32,
    pub delay: f32,
}

struct Voice {
    car: String,
    mixer: Mixer,
    motion: Motion,
    module: Option<EffectModule>,
    data: EngineData,
    pitch_offset: i32,
    torque: f32,
}

/// The `CAR` words of one tick: engine RPM on the sound scale (1000..10000) mapped onto the set's RPM range,
/// torque in percent times 10.24, attenuation from the mixer volume, the voice's pitch offset scaled by the mixer
/// pitch, and the mixer azimuth.
pub(super) fn car_words(
    data: &EngineData,
    rpm_pct: f32,
    torque: f32,
    mix: MixOut,
    pitch_offset: i32,
) -> [i32; CAR_WORDS] {
    let engine_rpm = rpm_pct.clamp(0.0, 1.0) * 9000.0 + 1000.0;
    let scaled = (engine_rpm - 1000.0) * (data.max_rpm - data.min_rpm) / 9000.0 + data.min_rpm;
    let level = ((mix.volume.clamp(0.0, 1.0) * 32767.0) as i32 * 0x7FFF) >> 15;
    let mut words = [0; CAR_WORDS];
    words[WORD_CLASS] = data.car_id as i32;
    words[WORD_RPM] = scaled.max(0.0) as i32;
    words[WORD_TORQUE] = (torque * 10.24) as i32;
    words[WORD_VOL_ENG] = level - 0x7FFF;
    words[WORD_VOL_EXH] = level - 0x7FFF;
    words[WORD_SPU_OR_EE] = 1;
    words[WORD_AZIMUTH] = mix.azimuth;
    words[WORD_LOW_PASS] = mix.low_pass.clamp(0, 25000);
    words[WORD_PITCH_OFFSET] = ((pitch_offset + 16000) as f32 * mix.pitch - 16000.0) as i32;
    words
}

/// The opponents to hear: the nearest `MAX_VOICES` within `CONNECT_DISTANCE` of the camera, nearest first.
pub(super) fn connected(racers: &[RacerEmitter], camera: Vec3) -> Vec<usize> {
    let mut near: Vec<(usize, f32)> = racers
        .iter()
        .enumerate()
        .filter(|(_, r)| r.position.is_finite() && r.velocity.is_finite() && r.forward.is_finite())
        .map(|(i, r)| (i, r.position.distance(camera)))
        .filter(|&(_, d)| d < CONNECT_DISTANCE)
        .collect();
    near.sort_by(|a, b| a.1.total_cmp(&b.1));
    near.into_iter().take(MAX_VOICES).map(|(i, _)| i).collect()
}

#[derive(Default)]
pub(super) struct RacerVoices {
    voices: BTreeMap<u32, Voice>,
    /// The next voice's pitch offset seed: each new voice steps it by 500, wrapping at 3000.
    pitch_seed: i32,
}

impl RacerVoices {
    #[cfg(test)]
    pub fn count(&self) -> usize {
        self.voices.len()
    }

    pub fn update(
        &mut self,
        audio: &mut Audio,
        map: &Arc<MixMap>,
        racers: &[RacerEmitter],
        view: AudioView,
        step: Step,
    ) {
        let heard = connected(racers, view.camera_position);
        self.voices.retain(|id, voice| heard.iter().any(|&i| racers[i].id == *id && racers[i].car == voice.car));
        for &index in &heard {
            let racer = &racers[index];
            if !self.voices.contains_key(&racer.id) {
                let Some(voice) = self.connect(audio, map, racer) else { continue };
                self.voices.insert(racer.id, voice);
            }
            let Some(voice) = self.voices.get_mut(&racer.id) else { continue };
            voice.update(audio, racer, view, step);
        }
    }

    fn connect(&mut self, audio: &mut Audio, map: &Arc<MixMap>, racer: &RacerEmitter) -> Option<Voice> {
        let sound = match audio.sound_set(&racer.car, Default::default()) {
            Ok(sound) => sound,
            Err(error) => {
                log::warn!("no racer engine sound for {}: {error}", racer.car);
                return None;
            }
        };
        let engine = &sound.engine;
        let module = match EffectModule::load(audio, &engine_bank_path(&engine.bank_main), "CAR", racer.id) {
            Ok(module) => Some(module),
            Err(error) => {
                log::warn!("racer engine module unavailable for {}: {error}", racer.car);
                None
            }
        };
        self.pitch_seed = (self.pitch_seed + 500) % 3000;
        let mut counts = [0; 13];
        counts[0] = 1;
        counts[2] = 1;
        counts[usize::from(STATE)] = 1;
        let mut mixer = Mixer::new(map.clone(), &counts);
        mixer.attach(engine_object(), true);
        audio.trace.record(Record {
            event: "racer_engine_connect",
            instance: u64::from(racer.id),
            class: "CAR",
            logical_ms: audio.trace.now_ms(),
            success: Some(module.is_some()),
            detail: &racer.car,
            ..Default::default()
        });
        Some(Voice {
            car: racer.car.clone(),
            mixer,
            motion: Motion::default(),
            module,
            data: EngineData {
                car_id: engine.car_id,
                min_rpm: engine.min_rpm,
                max_rpm: engine.max_rpm,
                master_volume: engine.master_volume,
            },
            pitch_offset: self.pitch_seed - 1500,
            torque: 0.0,
        })
    }
}

fn engine_object() -> ObjectRef {
    ObjectRef { state: STATE, instance: 0, object: ENGINE_OBJECT }
}

fn controller(controller: u8, index: u8) -> InputKey {
    InputKey::controller(STATE, 0, controller, index)
}

impl Voice {
    fn update(&mut self, audio: &mut Audio, racer: &RacerEmitter, view: AudioView, step: Step) {
        let Step { velocities, dt, delay } = step;
        let m = &mut self.mixer;
        m.set_camera(view.camera_state);
        crate::audio::speedbreaker::publish(m, &view);
        let source = racer.position + racer.forward.normalize_or_zero() * SOURCE_AHEAD;
        for (index, value) in self.motion.inputs_at(source, racer.velocity, view, velocities).into_iter().enumerate() {
            m.set_input(controller(CAR_POSITION, index as u8), value);
        }
        let own_mph = racer.velocity.length() * 2.23699;
        let listener_mph = velocities[0].length() * 2.23699;
        let rpm = racer.rpm_pct.clamp(0.0, 1.0) * 9000.0 + 1000.0;
        for (index, scale) in [1092.2334, 546.1167, 327.67, 234.05].into_iter().enumerate() {
            m.set_input(controller(PHYSICS, index as u8), ((own_mph * scale) as i32).clamp(0, 32767));
            m.set_input(InputKey::controller(2, 0, 0, index as u8), ((listener_mph * scale) as i32).clamp(0, 32767));
        }
        m.set_input(controller(PHYSICS, 4), (((10_000.0 - rpm) * 3.640_777_8) as i32).clamp(0, 32767));
        m.set_input(controller(PHYSICS, 5), i32::from(racer.wheels_on_ground.min(4)) * 32767 / 4);
        m.set_input(controller(PHYSICS, 6), view.view_value);
        m.set_input(controller(PHYSICS, 10), if racer.throttle > 0.3 { 32767 } else { 0 });
        m.set_input(controller(ENGINE, 0), ENGINE_UPGRADE_LEVEL);
        m.set_input(controller(ENGINE, 2), self.data.master_volume.min(32767) as i32);
        m.process(dt);
        let object = engine_object();
        let mix = MixOut {
            volume: m.volume(object, 1).unwrap_or(0.0),
            pitch: m.pitch(object, 4).unwrap_or(1.0),
            azimuth: i32::from(m.azimuth(object, 0).unwrap_or(0)),
            low_pass: crate::audio::environment::distance_filter(
                (source.distance(view.camera_position) * 100.0) as i32,
            ),
        };
        // `PhysicsTRQ` follows the throttle (percent) by at most 100 per tick.
        self.torque = racer.throttle.clamp(0.0, 1.0) * 100.0;
        let words = car_words(&self.data, racer.rpm_pct, self.torque, mix, self.pitch_offset);
        if audio.trace.enabled() {
            audio.trace.record(Record {
                event: "racer_engine_tick",
                instance: u64::from(racer.id),
                class: "CAR",
                logical_ms: audio.trace.now_ms(),
                params: &[
                    words[WORD_RPM],
                    words[WORD_TORQUE],
                    words[WORD_VOL_ENG],
                    words[WORD_PITCH_OFFSET],
                    mix.azimuth,
                    (racer.position.distance(view.camera_position) * 100.0) as i32,
                ],
                detail: "rpm, torque, attenuation, pitch offset, azimuth, camera cm",
                ..Default::default()
            });
        }
        let Some(module) = &mut self.module else { return };
        if !module.update(&mut DelayedSampler(audio, Group::Engine, delay), dt, &words) {
            self.module = None;
        }
    }
}
