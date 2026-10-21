//! Sequenced scenery one-shots. Spec: `scenery-effect-audio.md`.
use super::{Audio, AudioView, CarEvent, car::impacts::ImpactPlayback, mixer::collision::CollisionMixer};
use blackbox_carsound::CarInput;
use glam::Vec3;
use nfsmw_data::sound::{CollisionZone, EffectSound, EventKind};
use std::collections::HashMap;

#[derive(Clone, Copy, Debug)]
pub struct WorldEffectAudio {
    pub effect: u32,
    pub zone: CollisionZone,
    pub position: Vec3,
    pub magnitude: f32,
}

#[derive(Default)]
pub(super) struct Slot {
    definitions: HashMap<(u32, CollisionZone), Option<EffectSound>>,
    playback: ImpactPlayback,
}

impl Slot {
    fn start(&mut self, audio: &mut Audio, request: WorldEffectAudio, view: AudioView) {
        if !request.position.is_finite() {
            return;
        }
        let key = (request.effect, request.zone);
        if let std::collections::hash_map::Entry::Vacant(entry) = self.definitions.entry(key) {
            let definition = audio.database().ok().and_then(|db| EffectSound::read(&db, key.0, key.1));
            entry.insert(definition);
        }
        let Some(definition) = &self.definitions[&key] else { return };
        let Some(magnitude) = definition.magnitude(request.magnitude, view.camera_position.distance(request.position))
        else {
            return;
        };
        let sound = &definition.sound;
        let count = sound.levels.iter().take_while(|level| !level.is_empty()).count();
        if count == 0 {
            return;
        }
        let level = ((magnitude * (count - 1) as f32 + 0.5) as usize).min(count - 1);
        audio.impact_counter = audio.impact_counter.wrapping_add(1);
        let index = audio.impact_counter as usize % sound.levels[level].len();
        let stitches = audio.stitches();
        let Some(stitch) = stitches.get(sound.levels[level][index] as usize) else { return };
        let mixer = super::mixer::mix_map(&audio.dir).ok().map(CollisionMixer::new);
        let event = CarEvent {
            sound_profile: 0,
            vehicle: None,
            audio_zone: Some(request.zone),
            kind: EventKind::Detached,
            surface: 0,
            magnitude,
            front: false,
            smackable: sound.has("SMOKABLE"),
            position: request.position,
        };
        let volume = ((sound.volumes[level] * 32768.0).round() as i32).clamp(0, 32767);
        self.playback.collision_with_mixer(audio, stitch, sound, event, mixer, volume);
    }
}

impl Audio {
    pub(crate) fn update_world_effect_audio(&mut self, requests: Vec<WorldEffectAudio>, view: AudioView, dt: f32) {
        let mut slot = std::mem::take(&mut self.world_effects);
        for request in requests {
            slot.start(self, request, view);
        }
        slot.playback.update(self, dt, 0.0, &CarInput::default(), &view, (1.0, 0));
        self.world_effects = slot;
    }

    pub(super) fn stop_world_effect_audio(&mut self) {
        self.world_effects.playback = ImpactPlayback::default();
    }
}

#[cfg(test)]
mod tests;
