//! The authored two-slot stitch loop used by road ID 8.
use crate::audio::{
    Audio, Group,
    aems::{EffectModule, EffectSampler},
};
use nfsmw_data::sound::Stitch;

struct Piece {
    module: EffectModule,
    params: [i32; 10],
    last_params: [i32; 10],
    volume: i32,
}

struct Profile {
    stitch: Stitch,
    overlap: i16,
}

#[derive(Default)]
pub(super) struct StitchRoad {
    profile: Option<Profile>,
    slots: [Vec<Piece>; 2],
    retired: Vec<Piece>,
    timer: i16,
    random: u32,
    failed: bool,
    clock: RepeatClock,
}

impl StitchRoad {
    pub fn release(&mut self) {
        self.profile = None;
        for slot in &mut self.slots {
            for mut piece in slot.drain(..) {
                piece.module.release();
                self.retired.push(piece);
            }
        }
    }

    pub fn start(&mut self, audio: &mut Audio, seed: u32) {
        self.release();
        self.failed = false;
        self.random = seed;
        self.clock = RepeatClock::default();
        let profile = (|| {
            let db = audio.database().ok()?;
            let collection = db.collection_by_key(blackbox_attrib::vlt_hash("aud_stitch_loop"), 0x4b41dec8)?;
            let index = usize::try_from(collection.get_u32("stitch")?).ok()?;
            let overlap =
                collection.get_i32("overlap").or_else(|| collection.get_u32("overlap").map(|v| v as i32))? as i16;
            let stitch = audio.stitches().get(index)?.clone();
            Some(Profile { stitch, overlap })
        })();
        let Some(profile) = profile else {
            self.failed = true;
            log::warn!("road stitch profile unavailable");
            return;
        };
        self.timer = profile.overlap;
        self.profile = Some(profile);
        self.spawn(audio, 0);
    }

    fn random_below(&mut self, range: u32) -> i32 {
        if range == 0 {
            return 0;
        }
        self.random = self.random.wrapping_mul(1664525).wrapping_add(1013904223);
        (self.random % range) as i32
    }

    fn spawn(&mut self, audio: &mut Audio, slot: usize) {
        let Some(profile) = &self.profile else { return };
        let pieces = profile.stitch.pieces.clone();
        for piece in pieces.iter().rev().filter(|p| p.family == 0) {
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
            match EffectModule::load(audio, crate::audio::refs::COLLISION_BANK, "AEMS_StichCollision", self.random) {
                Ok(module) => self.slots[slot].push(Piece { module, params, last_params: params, volume }),
                Err(error) => {
                    self.failed = true;
                    log::warn!("road stitch piece unavailable: {error}");
                }
            }
        }
    }

    pub fn owns_loop(&self) -> bool {
        self.profile.is_some() && !self.failed
    }

    #[cfg(test)]
    pub fn has_profile(&self) -> bool {
        self.profile.is_some()
    }

    pub fn update(&mut self, audio: &mut Audio, dt: f32, volume: i32, azimuth: i32, wet: i32) {
        let mut failed = false;
        let mut update = |piece: &mut Piece, audio: &mut Audio| {
            let mut params = piece.params;
            params[2] = super::road::multiply(piece.volume, volume);
            params[4] = i32::from((params[4] as u16).wrapping_add(azimuth as u16));
            params[7] = wet;
            piece.last_params = params;
            let usable = piece.module.update(&mut EffectSampler(audio, Group::Sfx), dt, &params);
            failed |= !usable;
            usable && !piece.module.finished()
        };
        for slot in &mut self.slots {
            slot.retain_mut(|piece| update(piece, audio));
        }
        self.failed |= failed;
        self.retired.retain_mut(|piece| {
            piece.module.update(&mut EffectSampler(audio, Group::Sfx), dt, &piece.last_params)
                && !piece.module.finished()
        });
        let Some(profile) = &self.profile else { return };
        let overlap = profile.overlap;
        for _ in 0..self.clock.advance(dt) {
            let Some(slot) =
                repeat_slot(&mut self.timer, overlap, 1.0 / 60.0, self.slots.each_ref().map(Vec::is_empty))
            else {
                continue;
            };
            self.spawn(audio, slot);
        }
    }
}

#[derive(Default)]
pub(super) struct RepeatClock {
    debt: f64,
}

impl RepeatClock {
    pub fn advance(&mut self, dt: f32) -> usize {
        if !dt.is_finite() || dt < 0.0 {
            return 0;
        }
        self.debt += f64::from(dt) * 60.0;
        let ticks = (self.debt + 0.000001).floor().clamp(0.0, 64.0) as usize;
        self.debt -= ticks as f64;
        ticks
    }
}

pub(super) fn repeat_slot(timer: &mut i16, overlap: i16, dt: f32, free: [bool; 2]) -> Option<usize> {
    let elapsed = match dt {
        t if t.is_finite() && t >= 0.0 => (t * 1000.0) as i32,
        _ => 0,
    };
    *timer = timer.wrapping_sub(elapsed as i16);
    if *timer >= 0 {
        return None;
    }
    *timer = overlap;
    (0..2).rev().find(|&slot| free[slot])
}
