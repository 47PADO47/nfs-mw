//! A mod car's body hits and scrapes from the recordings its mod shares between its cars (`[car_sound.shared]`,
//! docs/modding/car-sound.md, "The shared sounds"), played in place of the game's class-wide collision banks.
//!
//! Which hits are loud enough to be heard and how strong they are (0 to 1) is decided before this: the collision rules of
//! the physics contacts give each [`CarEvent`] its magnitude. What plays for it is chosen here by the game's own simple
//! rules: the class from the strength (light, medium, heavy), a variant at random, the volume from the strength. A hit the
//! car has no recording for (the ground, a part coming off, a prop that falls over) is left to the game's own sounds.

use std::time::Duration;

use kira::StartTime;
use kira::Tween;
use kira::sound::static_sound::{StaticSoundData, StaticSoundHandle};
use nfsmw_data::car::mods::SharedSound;
use nfsmw_data::sound::EventKind;

use super::logic::Rng;
use crate::audio::volume::decibels;
use crate::audio::{Audio, CarEvent, Group};

/// Two body hits are never closer than this: a blow that the physics reports on two ticks is one sound.
const IMPACT_GAP: f32 = 0.1;
/// A scrape one-shot is followed by the next when this share of it has played.
const SCRAPE_OVERLAP: f32 = 0.85;
/// A scrape that has stopped for this long is faded out.
const SCRAPE_RELEASE_AFTER: f32 = 0.15;
/// The fade of a scrape that ends.
const SCRAPE_FADE: Duration = Duration::from_millis(120);

/// The class of a blow of strength `magnitude` (0 to 1): 0 light, 1 medium, 2 heavy.
pub fn impact_class(magnitude: f32) -> usize {
    ((magnitude.clamp(0.0, 1.0) * 2.0 + 0.5) as usize).min(2)
}

/// The volume of a body hit of strength `magnitude`: a light blow is not silent.
pub fn impact_volume(magnitude: f32) -> f32 {
    0.55 + 0.45 * magnitude.clamp(0.0, 1.0)
}

/// The volume of a scrape one-shot at scrape strength `level` (0 to 1).
pub fn scrape_volume(level: f32) -> f32 {
    0.35 + 0.65 * level.clamp(0.0, 1.0)
}

/// What the scrape clock asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ScrapeStep {
    Idle,
    /// Start the next one-shot at this volume.
    Trigger {
        volume: f32,
    },
    /// The scrape is over: fade out what plays.
    Release,
}

/// When the next scrape one-shot starts: one after another while the body scrapes, none once it has stopped.
#[derive(Debug, Default)]
pub struct ScrapeClock {
    wait: f32,
    quiet: f32,
    playing: bool,
}

impl ScrapeClock {
    /// One step of `dt` seconds at scrape strength `level` (0 for none).
    pub fn step(&mut self, level: f32, dt: f32) -> ScrapeStep {
        self.wait = (self.wait - dt).max(0.0);
        if level.is_nan() || level <= 0.0 {
            self.quiet += dt;
            if self.playing && self.quiet >= SCRAPE_RELEASE_AFTER {
                self.playing = false;
                return ScrapeStep::Release;
            }
            return ScrapeStep::Idle;
        }
        self.quiet = 0.0;
        if self.wait > 0.0 {
            return ScrapeStep::Idle;
        }
        self.playing = true;
        ScrapeStep::Trigger { volume: scrape_volume(level) }
    }

    /// A one-shot of `seconds` has started: the next follows when most of it has played.
    pub fn started(&mut self, seconds: f32) {
        self.wait = seconds.max(0.0) * SCRAPE_OVERLAP;
    }
}

/// The shared recordings of a car that is playing.
pub struct SharedSfx {
    /// Body hits by class (light, medium, heavy).
    impacts: [Vec<StaticSoundData>; 3],
    scrapes: Vec<StaticSoundData>,
    rng: Rng,
    gap: f32,
    clock: ScrapeClock,
    scrape_voice: Option<StaticSoundHandle>,
    last_scrape: Option<usize>,
}

impl SharedSfx {
    pub fn new(impacts: [Vec<StaticSoundData>; 3], scrapes: Vec<StaticSoundData>, seed: u32) -> Self {
        Self {
            impacts,
            scrapes,
            rng: Rng::new(seed),
            gap: 0.0,
            clock: ScrapeClock::default(),
            scrape_voice: None,
            last_scrape: None,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.impacts.iter().all(Vec::is_empty) && self.scrapes.is_empty()
    }

    /// Whether the car scrapes with its own recordings (the contacts then start no scrape of the game's).
    pub fn has_scrapes(&self) -> bool {
        !self.scrapes.is_empty()
    }

    /// Advance the clock of the hits by `dt` seconds.
    pub fn tick(&mut self, dt: f32) {
        self.gap = (self.gap - dt).max(0.0);
    }

    /// The class to play for class `class`: itself, or the nearest class that has recordings (heavier first).
    fn class_with_files(&self, class: usize) -> Option<usize> {
        let order = [class, class + 1, class.wrapping_sub(1)];
        order.into_iter().find(|&at| self.impacts.get(at).is_some_and(|list| !list.is_empty()))
    }

    /// Whether this hit is the kind the recordings are for: the car's body against the world, another car or a prop that
    /// stays up. True also for a hit that is dropped because the last one was a moment ago, so the game's sound does not
    /// play in its place.
    pub fn takes(&self, event: &CarEvent) -> bool {
        let body = matches!(event.kind, EventKind::HitWorld | EventKind::HitObject);
        body && !event.smackable && self.impacts.iter().any(|list| !list.is_empty())
    }

    /// Play the hit `event`, `delay` seconds from now. False (nothing played, nothing taken) for a hit these recordings
    /// are not for.
    pub fn impact(&mut self, audio: &mut Audio, event: &CarEvent, delay: f32) -> bool {
        if !self.takes(event) {
            return false;
        }
        if self.gap > 0.0 {
            return true;
        }
        let Some(class) = self.class_with_files(impact_class(event.magnitude)) else { return false };
        let variant = self.rng.below(self.impacts[class].len());
        let data = self.impacts[class][variant]
            .clone()
            .volume(decibels(impact_volume(event.magnitude)))
            .start_time(delayed(delay));
        self.gap = IMPACT_GAP;
        if let Err(e) = audio.play(Group::Sfx, data) {
            log::debug!("a body hit: {e}");
        }
        true
    }

    /// One step of the scrape: `level` is the strongest scrape the contacts report (0 for none), over `dt` seconds.
    pub fn scrape(&mut self, audio: &mut Audio, level: f32, dt: f32, delay: f32) {
        if self.scrapes.is_empty() {
            return;
        }
        match self.clock.step(level, dt) {
            ScrapeStep::Idle => {}
            ScrapeStep::Release => {
                if let Some(mut voice) = self.scrape_voice.take() {
                    voice.stop(Tween { duration: SCRAPE_FADE, ..Tween::default() });
                }
            }
            ScrapeStep::Trigger { volume } => self.start_scrape(audio, volume, delay),
        }
    }

    fn start_scrape(&mut self, audio: &mut Audio, volume: f32, delay: f32) {
        let count = self.scrapes.len();
        let mut variant = self.rng.below(count);
        if count > 1 && self.last_scrape == Some(variant) {
            variant = (variant + 1 + self.rng.below(count - 1)) % count;
        }
        self.last_scrape = Some(variant);
        let data = self.scrapes[variant].clone();
        self.clock.started(data.duration().as_secs_f32());
        let data = data.volume(decibels(volume)).start_time(delayed(delay));
        match audio.play(Group::Sfx, data) {
            Ok(handle) => self.scrape_voice = Some(handle),
            Err(e) => log::debug!("a scrape: {e}"),
        }
    }
}

impl Drop for SharedSfx {
    /// A kira handle that is dropped leaves its sound playing, so the scrape in progress is faded out here.
    fn drop(&mut self) {
        if let Some(mut voice) = self.scrape_voice.take() {
            voice.stop(Tween { duration: SCRAPE_FADE, ..Tween::default() });
        }
    }
}

fn delayed(delay: f32) -> StartTime {
    let delay = if delay.is_finite() { delay.max(0.0) } else { 0.0 };
    StartTime::Delayed(Duration::from_secs_f32(delay))
}

impl Audio {
    /// Read the shared recordings of mod car `name`. A file that cannot be read is left out, and the log says why.
    pub(super) fn read_shared(&mut self, name: &str, shared: &SharedSound) -> SharedSfx {
        let impacts = [
            self.read_shots(name, "impact_small", &shared.impact_small),
            self.read_shots(name, "impact_medium", &shared.impact_medium),
            self.read_shots(name, "impact_big", &shared.impact_big),
        ];
        let scrapes = self.read_shots(name, "scrape", &shared.scrape);
        let seed = name.bytes().fold(0x85eb_ca6bu32, |seed, byte| seed.rotate_left(7) ^ u32::from(byte));
        SharedSfx::new(impacts, scrapes, seed)
    }
}

#[cfg(test)]
mod tests;
