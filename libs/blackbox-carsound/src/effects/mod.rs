//! The sound effects around the engine: gear shifts, reverse whine, brake mash, turbo, nitrous, tire skids,
//! road and wind noise, landings and collisions. Spec: `docs/specs/engine-sound-effects.md`.
//!
//! [`EffectsMixer::update`] takes the same telemetry as the engine mix plus the engine mix's own output, and
//! returns [`SoundCommand`]s: plain values that name a sound ([`SoundRef`]), its volume (linear, 0 to 1) and
//! pitch (a playback ratio, 1 = unchanged). The game resolves a [`SoundRef`] to a bank sound and plays it.
//! The controllers run on the engine mix's fixed tick.

mod collision;
mod landing;
mod nitrous;
mod road;
mod shift_fx;
mod skid;
mod turbo;

pub use collision::{ImpactPlay, ImpactRequest, ScrapeKind};
pub use landing::Landing;

use self::collision::CollisionFx;
use self::landing::LandingFx;
use self::nitrous::NitrousFx;
use self::road::{RoadFx, WindFx};
use self::shift_fx::ShiftFx;
use self::skid::SkidFx;
use self::turbo::TurboFx;
use crate::engine::{EngineOutput, Ticker};
use crate::input::CarInput;
use crate::tuning::CarSoundTuning;

/// Which sound, in the vocabulary of the original's effect systems. Bank sounds are the game's business.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoundRef {
    /// The shift clunk (`FX_SHIFTING_01` sample 0 up, 1 down).
    GearClunk {
        up: bool,
    },
    /// The brake mash (`FX_SHIFTING_01` sample 2).
    BrakeMash,
    /// A shift sweetener (`CAR_SWTN` sample 0 or 1).
    Sweetener(u8),
    ReverseWhine,
    /// The turbo or supercharger whine loop (`FX_TURBO_01` id 0).
    TurboSpool,
    /// A blow-off (`FX_TURBO_01` id 1, 2 or 3 as `0`, `1`, `2`).
    TurboBlowoff(u8),
    Nitrous,
    Purge,
    /// A tire loop: the surface's `Aud_Skid_Type` and whether it is the sideways (squeal) or the forward (burnout) one.
    Skid {
        surface: u8,
        sideways: bool,
    },
    /// A road noise loop: the surface's `Aud_Roadnoise_LOOP` (1 and up).
    RoadNoise(u8),
    Wind,
    Scrape(ScrapeKind),
}

/// A voice the game keeps between updates, so `SetLoop` changes it and `StopLoop` ends it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LoopId {
    Reverse,
    Turbo,
    Nitrous,
    /// 0 front axle, 1 rear axle.
    Skid(u8),
    /// 0 left, 1 right.
    Road(u8),
    Wind,
    Scrape,
}

/// What the game should do with its voices.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SoundCommand {
    /// Fire and forget.
    Play {
        sound: SoundRef,
        volume: f32,
        pitch: f32,
    },
    /// Start the loop `id` with `sound`, or change the one playing (a different `sound` replaces it).
    SetLoop {
        id: LoopId,
        sound: SoundRef,
        volume: f32,
        pitch: f32,
    },
    StopLoop(LoopId),
}

/// The effects controllers of one car.
#[derive(Debug, Clone)]
pub struct EffectsMixer {
    ticker: Ticker,
    shift: ShiftFx,
    turbo: Option<TurboFx>,
    nitrous: NitrousFx,
    skids: SkidFx,
    road: RoadFx,
    wind: WindFx,
    landing: LandingFx,
    collision: CollisionFx,
    reverse: bool,
}

impl EffectsMixer {
    pub fn new(tuning: &CarSoundTuning) -> Self {
        Self {
            ticker: Ticker::default(),
            shift: ShiftFx::new(tuning),
            turbo: tuning.turbo.map(|t| TurboFx::new(t, tuning.seed)),
            nitrous: NitrousFx::new(tuning.nitrous),
            skids: SkidFx::new(tuning.skid),
            road: RoadFx::default(),
            wind: WindFx::default(),
            landing: LandingFx::default(),
            collision: CollisionFx::default(),
            reverse: false,
        }
    }

    /// Advances by `dt` seconds. `engine` is the engine mix's latest output (the same call's). Commands are
    /// appended to `out`; a landing the wheels made is returned for the game to play as a collision.
    pub fn update(
        &mut self,
        dt: f32,
        input: &CarInput,
        engine: &EngineOutput,
        out: &mut Vec<SoundCommand>,
    ) -> Option<Landing> {
        let input = &input.sanitized();
        let ticks = self.ticker.advance(dt);
        let mut landing = None;
        self.shift.events(engine, out);
        for _ in 0..ticks {
            self.shift.tick(input, out);
            if let Some(turbo) = self.turbo.as_mut() {
                turbo.tick(engine, out);
            }
            self.nitrous.tick(input, out);
            self.skids.tick(input);
            self.road.tick(input);
            self.landing.tick(input, &mut landing);
        }
        self.collision.tick(dt);
        self.reverse_whine(input, engine, out);
        if let Some(turbo) = self.turbo.as_mut() {
            turbo.emit(out);
        }
        self.skids.emit(out);
        self.road.emit(input, out);
        self.wind.emit(input, out);
        self.collision.emit_scrape(out);
        landing
    }

    fn reverse_whine(&mut self, input: &CarInput, engine: &EngineOutput, out: &mut Vec<SoundCommand>) {
        let reversing = input.gear == crate::input::GEAR_REVERSE;
        if !reversing {
            if std::mem::take(&mut self.reverse) {
                out.push(SoundCommand::StopLoop(LoopId::Reverse));
            }
            return;
        }
        self.reverse = true;
        let rpm = ((engine.eng_rpm - 1000.0) / 9000.0).clamp(0.0, 1.0);
        out.push(SoundCommand::SetLoop {
            id: LoopId::Reverse,
            sound: SoundRef::ReverseWhine,
            volume: rpm,
            pitch: 0.8 + 0.4 * rpm,
        });
    }

    /// A hit: the level and sample to play, if any (spec §8). Call once per physics event.
    pub fn impact(&mut self, request: &ImpactRequest<'_>) -> Option<ImpactPlay> {
        self.collision.impact(request)
    }

    /// The scrape going on this frame, if any (`magnitude` 0 to 1); call every frame, `None` when there is none.
    pub fn scrape(&mut self, scrape: Option<(ScrapeKind, f32)>) {
        self.collision.set_scrape(scrape);
    }
}

#[cfg(test)]
mod tests;
