//! Ordered fixed-step traffic snapshots own all source lifetimes.
use super::{TrafficFrame, TrafficSound, emitter::Emitter, selection::Variants};
use crate::audio::{Audio, AudioView, trace::Record};
use blackbox_mixmap::MixMap;
use glam::Vec3;
use nfsmw_data::sound::{Stitch, whoosh_stitches};
use std::{collections::BTreeMap, sync::Arc};

/// Seconds after one horn before another may sound, for the autonomous and the requested horns alike.
const HORN_COOLDOWN: f32 = 3.0;

/// Whether a horn requested at `now` (seconds) may sound, given the time of the last one that did.
fn horn_allowed(now: f32, last_horn: f32) -> bool {
    now > last_horn + HORN_COOLDOWN
}

#[derive(Default)]
pub(in crate::audio) struct TrafficAudio {
    map: Option<Arc<MixMap>>,
    failed: bool,
    stitches: Vec<Stitch>,
    emitters: BTreeMap<u32, Emitter>,
    variants: Variants,
    random: u32,
    previous_camera: Option<Vec3>,
    pending: Vec<TrafficSound>,
    time: f32,
    last_horn: f32,
}

impl TrafficAudio {
    #[cfg(test)]
    pub fn emitter_count(&self) -> usize {
        self.emitters.len()
    }
    fn load(&mut self, audio: &mut Audio) -> bool {
        if self.failed {
            return false;
        }
        if self.map.is_some() {
            return true;
        }
        audio.preload_traffic_banks();
        let result = (|| {
            let map = crate::audio::mixer::mix_map(&audio.dir).map_err(|e| format!("{e:#}"))?;
            let bytes = nfsmw_data::shared::ingame_b(&audio.dir).map_err(|e| e.to_string())?;
            self.stitches = whoosh_stitches(&bytes);
            self.map = Some(map);
            Ok::<_, String>(())
        })();
        if let Err(error) = result {
            self.failed = true;
            log::warn!("authored traffic unavailable: {error}");
            return false;
        }
        true
    }

    pub fn update(&mut self, audio: &mut Audio, sounds: Vec<TrafficSound>, view: AudioView) {
        if sounds.is_empty() || !self.load(audio) {
            return;
        }
        let mut delay = 0.0;
        for sound in sounds {
            let TrafficSound::Frame(frame) = sound else {
                self.pending.push(sound);
                continue;
            };
            let dt = frame.dt;
            self.step(audio, frame, view, delay);
            if dt.is_finite() {
                delay += dt.max(0.0);
            }
        }
    }

    fn step(&mut self, audio: &mut Audio, frame: TrafficFrame, mut view: AudioView, delay: f32) {
        if !frame.dt.is_finite()
            || frame.dt <= 0.0
            || !frame.listener_position.is_finite()
            || !frame.listener_velocity.is_finite()
            || !frame.listener_forward.is_finite()
        {
            self.pending.clear();
            self.emitters.clear();
            return;
        }
        let offset = view.camera_position - view.car_position;
        self.time += frame.dt;
        view.car_position = frame.listener_position;
        view.car_forward = frame.listener_forward;
        view.camera_position = view.car_position + offset;
        let camera_velocity = self
            .previous_camera
            .replace(view.camera_position)
            .map_or(frame.listener_velocity, |p| (view.camera_position - p) / frame.dt);
        self.emitters.retain(|id, _| {
            frame
                .emitters
                .iter()
                .any(|e| e.id == *id && e.position.is_finite() && e.velocity.is_finite() && e.forward.is_finite())
        });
        let Some(map) = self.map.clone() else { return };
        for state in frame.emitters {
            if !state.position.is_finite() || !state.velocity.is_finite() || !state.forward.is_finite() {
                continue;
            }
            let emitter = self.emitters.entry(state.id).or_insert_with(|| Emitter::new(audio, map.clone(), state));
            if emitter.state.profile != state.profile {
                *emitter = Emitter::new(audio, map.clone(), state);
            }
            emitter.state = state;
        }
        for event in self.pending.drain(..) {
            match event {
                TrafficSound::Horn { emitter, duration } => {
                    // The horns share one clock with the autonomous ones: a horn within the cooldown is not played.
                    if !horn_allowed(self.time, self.last_horn) {
                        continue;
                    }
                    let Some(emitter) = self.emitters.get_mut(&emitter) else { continue };
                    self.last_horn = self.time;
                    audio.trace.record(Record {
                        event: "traffic_horn",
                        instance: u64::from(emitter.state.id),
                        logical_ms: audio.trace.now_ms(),
                        detail: "authored ENV_STATIC common horn",
                        ..Default::default()
                    });
                    emitter.horn(audio, duration);
                }
                TrafficSound::NearMiss { emitter, relative_speed } => {
                    let Some(emitter) = self.emitters.get_mut(&emitter) else { continue };
                    let Some(kind) = emitter.state.profile.whoosh else { continue };
                    let Some(id) = self.variants.select(kind, relative_speed) else { continue };
                    audio.trace.record(Record {
                        event: "traffic_whoosh",
                        instance: u64::from(emitter.state.id),
                        logical_ms: audio.trace.now_ms(),
                        params: &[id as i32, (relative_speed * 100.0) as i32],
                        detail: "authored family1 stitch",
                        ..Default::default()
                    });
                    if let Some(stitch) = self.stitches.get(id) {
                        emitter.whoosh(audio, stitch, &mut self.random);
                    }
                }
                TrafficSound::Frame(_) => {}
            }
        }
        for emitter in self.emitters.values_mut() {
            emitter.attempt_horn(
                audio,
                self.time,
                view,
                frame.listener_velocity,
                &mut self.last_horn,
                &mut self.random,
            );
            emitter.update(audio, frame.dt, delay, self.time, view, [frame.listener_velocity, camera_velocity]);
        }
    }
}

#[cfg(test)]
mod horn_cooldown_tests {
    use super::horn_allowed;

    #[test]
    fn a_horn_waits_three_seconds_after_the_last_one() {
        assert!(horn_allowed(10.0, 0.0), "the last horn is long past");
        assert!(!horn_allowed(2.9, 0.0), "inside the cooldown");
        assert!(horn_allowed(3.1, 0.0), "past the cooldown");
        assert!(!horn_allowed(12.0, 10.0), "a horn just before was the shared one");
    }
}
