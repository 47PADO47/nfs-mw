//! Collision sounds: which level and sample an impact plays, and the scrape loop. Spec §8.

use super::{LoopId, SoundCommand, SoundRef};
use crate::math::q15_gain;
use crate::tuning::CollisionTuning;

/// A scrape fades out over this long after it ends.
const SCRAPE_FADE: f32 = 0.25;
const AUDIBLE: f32 = 0.01;
/// Impacts with `WALL` and `SMOKABLE` together at or below this intensity (of 127) are dropped.
const LIGHT_INTENSITY: u32 = 9;

/// What a scrape rubs against; the game picks the loop.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScrapeKind {
    Ground,
    Wall,
    Car,
}

/// A hit the physics reported.
#[derive(Debug, Clone, Copy)]
pub struct ImpactRequest<'a> {
    /// 0 to 1.
    pub magnitude: f32,
    /// The collection's `DESCRIPTION` has both `WALL` and `SMOKABLE`.
    pub light_wall: bool,
    pub tuning: &'a CollisionTuning,
}

/// The sample to play for an impact.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImpactPlay {
    /// Which of the collection's `STITCH_LEVEL_n` lists.
    pub level: usize,
    /// Position in that list.
    pub index: usize,
    /// Linear volume (the collection's `Volumes.Vol<n>`).
    pub volume: f32,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct CollisionFx {
    counter: u32,
    scrape_kind: Option<ScrapeKind>,
    scrape: f32,
    target: f32,
    playing: bool,
    dt: f32,
}

impl CollisionFx {
    pub fn impact(&mut self, request: &ImpactRequest<'_>) -> Option<ImpactPlay> {
        let magnitude = request.magnitude.clamp(0.0, 1.0);
        if !request.magnitude.is_finite() {
            return None;
        }
        let intensity = (magnitude * 127.0) as u32;
        if request.light_wall && intensity <= LIGHT_INTENSITY {
            return None;
        }
        let lengths = &request.tuning.level_lengths;
        let used = lengths.iter().filter(|&&n| n > 0).count();
        if used == 0 {
            return None;
        }
        let level = ((magnitude * (used - 1) as f32 + 0.5) as usize).min(used - 1);
        let index = (self.counter % lengths[level].max(1)) as usize;
        self.counter = self.counter.wrapping_add(1);
        Some(ImpactPlay { level, index, volume: q15_gain(request.tuning.volumes[level] as f32) })
    }

    pub fn set_scrape(&mut self, scrape: Option<(ScrapeKind, f32)>) {
        match scrape {
            Some((kind, magnitude)) => {
                self.scrape_kind = Some(kind);
                self.target = magnitude.clamp(0.0, 1.0);
            }
            None => self.target = 0.0,
        }
    }

    /// Frame time for the fade.
    pub fn tick(&mut self, dt: f32) {
        self.dt = dt.max(0.0);
        let rise = self.target - self.scrape;
        self.scrape = match rise > 0.0 {
            true => self.target,
            false => (self.scrape - self.dt / SCRAPE_FADE).max(self.target),
        };
    }

    pub fn emit_scrape(&mut self, out: &mut Vec<SoundCommand>) {
        let Some(kind) = self.scrape_kind else { return };
        if self.scrape < AUDIBLE {
            if std::mem::take(&mut self.playing) {
                out.push(SoundCommand::StopLoop(LoopId::Scrape));
            }
            return;
        }
        self.playing = true;
        out.push(SoundCommand::SetLoop {
            id: LoopId::Scrape,
            sound: SoundRef::Scrape(kind),
            volume: self.scrape,
            pitch: 0.8 + 0.4 * self.scrape,
        });
    }
}
