//! Authored collision-piece playback, shared by body impacts and tyre landings.
use super::CarEvent;
use crate::audio::{
    Audio, AudioView, Group,
    aems::{DelayedSampler, EffectModule},
    mixer::{
        CarMixer,
        collision::{CollisionFrame, CollisionMixer},
    },
};
use blackbox_carsound::CarInput;
use nfsmw_data::sound::{ImpactSound, Stitch};

const BANK: &str = crate::audio::refs::COLLISION_BANK;

struct Piece {
    module: EffectModule,
    params: [i32; 10],
    volume: i32,
    priority: u8,
}

struct Impact {
    pieces: Vec<Piece>,
    source: Source,
}

enum Source {
    Collision {
        mixer: Option<Box<CollisionMixer>>,
        sound: Box<ImpactSound>,
        event: CarEvent,
        volume: i32,
        initial_azimuth: Option<u16>,
    },
    Landing {
        volume: i32,
    },
}

#[derive(Default)]
pub(in crate::audio) struct ImpactPlayback {
    active: Vec<Impact>,
    random: u32,
    landing_counter: u32,
}

impl ImpactPlayback {
    fn random_below(&mut self, range: u32) -> i32 {
        if range == 0 {
            return 0;
        }
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random % range) as i32
    }

    fn start(&mut self, audio: &mut Audio, stitch: &Stitch, source: Source) {
        let mut pieces = Vec::new();
        for piece in stitch.pieces.iter().rev().filter(|p| p.family == 0) {
            let volume =
                ((piece.volume * 32767.0).round() as i32 - self.random_below(u32::from(piece.random_volume))).max(0);
            let pitch = (i32::from(piece.pitch) - self.random_below(i32::from(piece.random_pitch).unsigned_abs()))
                .clamp(0, 8192);
            let params = [
                0,
                i32::from(piece.sample),
                0,
                pitch,
                i32::from(piece.azimuth),
                i32::from(piece.offset),
                32767,
                0,
                25000,
                0,
            ];
            match EffectModule::load(audio, BANK, "AEMS_StichCollision", self.random) {
                Ok(module) => pieces.push(Piece { module, params, volume, priority: piece.priority }),
                Err(error) => {
                    log::warn!("collision stitch unavailable: {error}");
                    return;
                }
            }
        }
        if pieces.is_empty() {
            return;
        }
        self.active.push(Impact { pieces, source });
        while self.active.iter().map(|i| i.pieces.len()).sum::<usize>() > 25 {
            let lowest = self
                .active
                .iter()
                .enumerate()
                .flat_map(|(i, impact)| impact.pieces.iter().enumerate().map(move |(p, piece)| (piece.priority, i, p)))
                .min();
            let Some((_, group, piece)) = lowest else { break };
            self.active[group].pieces.remove(piece);
        }
        self.active.retain(|i| !i.pieces.is_empty());
    }

    pub fn collision(
        &mut self,
        audio: &mut Audio,
        stitch: &Stitch,
        sound: &ImpactSound,
        event: CarEvent,
        mixer: Option<&CarMixer>,
        volume: i32,
    ) {
        self.collision_with_mixer(audio, stitch, sound, event, mixer.map(CarMixer::collision), volume);
    }

    pub(in crate::audio) fn collision_with_mixer(
        &mut self,
        audio: &mut Audio,
        stitch: &Stitch,
        sound: &ImpactSound,
        event: CarEvent,
        mixer: Option<CollisionMixer>,
        volume: i32,
    ) {
        self.start(
            audio,
            stitch,
            Source::Collision {
                mixer: mixer.map(Box::new),
                sound: Box::new(sound.clone()),
                event,
                volume,
                initial_azimuth: None,
            },
        );
    }

    pub fn landing(&mut self, audio: &mut Audio, stitch: &Stitch, volume: i32) {
        if self.active.iter().filter(|i| matches!(i.source, Source::Landing { .. })).count() >= 3 {
            return;
        }
        self.start(audio, stitch, Source::Landing { volume });
    }

    pub fn landing_index(&mut self, magnitude: f32, hard: bool) -> Option<usize> {
        if self.active.iter().filter(|i| matches!(i.source, Source::Landing { .. })).count() >= 3 {
            return None;
        }
        let intensity = (magnitude.clamp(0.0, 1.0) * 127.0) as usize;
        let base = match hard {
            true => 93,
            false => 81 + (intensity >> 5).min(3) * 4,
        };
        let index = base + (self.landing_counter % 4) as usize;
        self.landing_counter = self.landing_counter.wrapping_add(1);
        Some(index)
    }

    pub fn update(
        &mut self,
        audio: &mut Audio,
        dt: f32,
        delay: f32,
        input: &CarInput,
        view: &AudioView,
        landing_level: (f32, u16),
    ) {
        self.active.retain_mut(|impact| {
            let (volume, azimuth, wet) = match &mut impact.source {
                Source::Collision { mixer, sound, event, volume, initial_azimuth } => {
                    let levels = mixer.as_mut().map(|m| {
                        m.update(
                            dt,
                            CollisionFrame {
                                sound,
                                magnitude: event.magnitude,
                                point: event.position,
                                view,
                                input,
                                scrape: false,
                            },
                        )
                    });
                    let map = (levels.map_or(1.0, |l| l.volume) * 32767.0).round() as i32;
                    let azimuth = *initial_azimuth.get_or_insert_with(|| levels.map_or(0, |l| l.azimuth));
                    ((map * *volume) >> 15, azimuth, levels.map_or(0.0, |l| l.wet))
                }
                Source::Landing { volume } => {
                    (((landing_level.0 * 32767.0).round() as i32 * *volume) >> 15, landing_level.1, 0.0)
                }
            };
            impact.pieces.retain_mut(|piece| {
                piece.params[2] = (piece.volume * volume) >> 15;
                let local = piece.params[4] as u16;
                let mut params = piece.params;
                params[4] = i32::from(local.wrapping_add(azimuth));
                params[7] = (wet * 32767.0) as i32;
                piece.module.update(&mut DelayedSampler(audio, Group::Sfx, delay), dt, &params)
                    && !piece.module.finished()
            });
            !impact.pieces.is_empty()
        });
    }

    #[cfg(test)]
    pub fn active_count(&self) -> usize {
        self.active.len()
    }
}

#[cfg(test)]
mod tests;
