//! What the cluster remembers between frames. The theme builds a new cluster component every frame, so the state that
//! animates over time lives here, in a cell the [`CustomFeedCell`](super::super::CustomFeedCell) carries and every clone
//! of it shares: the speed scale's spring, the nitrous charge's spring, the nitrous and shift-cue pulses, and how long
//! the speedbreaker has rested full.
//!
//! The memory is stepped once per frame: a second layout in the same frame (the same `elapsed_s`) reads it without
//! stepping again, so a frame laid out twice does not run the springs twice.

use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use super::super::feed::CustomFeed;
use super::super::tokens::{Motion, ramp};

/// The angular frequency times the 2% settling time of a critically damped spring: `(1 + x) e^-x = 0.02` at
/// `x = 5.834`. A spring that settles in `t` seconds has an angular frequency of `SETTLE_2PC / t`.
pub const SETTLE_2PC: f32 = 5.834;
/// How long the speedbreaker rests full in free roam before its bar hides, in seconds.
pub const SPEEDBREAKER_HIDE_S: f32 = 3.0;
/// The speedbreaker counts as full from this charge.
const SPEEDBREAKER_FULL: f32 = 0.999;

/// A critically damped spring following a target: it reaches it as fast as it can without passing it. Integrated with
/// the exact solution, so any frame time is stable and a step from rest never overshoots.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spring {
    /// Where the spring is; `None` before its first step, which starts it on the target.
    value: Option<f32>,
    velocity: f32,
}

impl Spring {
    /// Moves the spring `dt` seconds towards `target`, with a 2% settling time of `settle` seconds, and returns where
    /// it is. The first step starts it on the target; a spring with no settling time snaps to it.
    pub fn step(&mut self, target: f32, dt: f32, settle: f32) -> f32 {
        let Some(value) = self.value else { return self.snap(target) };
        if !target.is_finite() || !dt.is_finite() || dt <= 0.0 {
            return value;
        }
        if settle.is_nan() || settle <= 0.0 {
            return self.snap(target);
        }
        let omega = SETTLE_2PC / settle;
        let offset = value - target;
        let c = self.velocity + omega * offset;
        let decay = (-omega * dt).exp();
        let offset = (offset + c * dt) * decay;
        self.velocity = (self.velocity - omega * c * dt) * decay;
        self.value = Some(target + offset);
        target + offset
    }

    /// Where the spring is, or `None` before its first step.
    pub fn value(&self) -> Option<f32> {
        self.value
    }

    fn snap(&mut self, target: f32) -> f32 {
        self.value = Some(target);
        self.velocity = 0.0;
        target
    }
}

/// A one-shot pulse on the rising edge of a condition: it starts when the condition turns on and does not start again
/// until the condition has been off.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Pulse {
    on: bool,
    started: Option<f32>,
}

impl Pulse {
    /// Follows the condition `on` at `now` (seconds on the frame clock). Returns `true` when this step started a pulse.
    pub fn follow(&mut self, on: bool, now: f32) -> bool {
        let rising = on && !self.on;
        self.on = on;
        if rising {
            self.started = Some(now);
        }
        rising
    }

    /// How strong the pulse is at `now`, 0 to 1. With motion it falls from 1 to 0 over `motion.pulse`; under motion
    /// reduction it is a static highlight, 1 for that long. 0 once the pulse is over, or before any pulse.
    pub fn level(&self, now: f32, motion: &Motion) -> f32 {
        let Some(started) = self.started else { return 0.0 };
        let age = now - started;
        if age < 0.0 || age >= motion.pulse {
            return 0.0;
        }
        match motion.pulse_moves {
            true => 1.0 - ramp(age, motion.pulse),
            false => 1.0,
        }
    }
}

/// The cluster's state between frames.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Memory {
    /// The frame clock of the last step; `None` before the first.
    frame: Option<f32>,
    /// The speed the scale shows, in metres per second.
    pub speed: Spring,
    /// The nitrous charge the bar shows, 0 to 1.
    pub nos: Spring,
    /// The nitrous activation, on the rising edge of the burn.
    pub nos_pulse: Pulse,
    /// The start of a shift cue.
    pub cue_pulse: Pulse,
    /// Since when the speedbreaker has rested full in free roam; `None` while it is not.
    speedbreaker_full_since: Option<f32>,
}

/// What the memory shows this frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Presented {
    /// The smoothed speed, in metres per second.
    pub speed: f32,
    /// The smoothed nitrous charge, 0 to 1.
    pub nos: f32,
    /// The nitrous activation pulse, 0 to 1 ([`Pulse::level`]).
    pub nos_pulse: f32,
    /// The shift cue's start pulse, 0 to 1.
    pub cue_pulse: f32,
    /// The speedbreaker has rested full in free roam long enough to hide.
    pub speedbreaker_hidden: bool,
}

impl Memory {
    /// Steps the memory to `feed`'s frame, with the shift cue `cue` (worked out from the same frame), and returns what it
    /// shows. A second call for the same frame does not step again.
    pub fn step(&mut self, feed: &CustomFeed, cue: bool) -> Presented {
        let now = feed.elapsed_s;
        if self.frame != Some(now) {
            self.frame = Some(now);
            self.advance(feed, cue);
        }
        self.presented(feed)
    }

    fn advance(&mut self, feed: &CustomFeed, cue: bool) {
        let hud = &feed.hud;
        let now = feed.elapsed_s;
        let settle = Motion::for_settings(&feed.settings).speed_spring;
        self.speed.step(hud.speed.max(0.0), feed.delta_s, settle);
        self.nos.step(hud.nos.clamp(0.0, 1.0), feed.delta_s, settle);
        self.nos_pulse.follow(hud.has_nos && hud.telemetry.nos_burning, now);
        self.cue_pulse.follow(cue, now);
        let free_roam = hud.race.is_none() && hud.pursuit.is_none();
        let resting = free_roam && hud.speedbreaker >= SPEEDBREAKER_FULL && !hud.speedbreaker_active;
        self.speedbreaker_full_since = match resting {
            true => Some(self.speedbreaker_full_since.unwrap_or(now)),
            false => None,
        };
    }

    fn presented(&self, feed: &CustomFeed) -> Presented {
        let now = feed.elapsed_s;
        let motion = Motion::for_settings(&feed.settings);
        Presented {
            speed: self.speed.value().unwrap_or(feed.hud.speed.max(0.0)),
            nos: self.nos.value().unwrap_or(feed.hud.nos.clamp(0.0, 1.0)),
            nos_pulse: self.nos_pulse.level(now, &motion),
            cue_pulse: self.cue_pulse.level(now, &motion),
            speedbreaker_hidden: self.speedbreaker_full_since.is_some_and(|since| now - since >= SPEEDBREAKER_HIDE_S),
        }
    }
}

/// The shared memory cell. Clones share one memory, as clones of the feed cell share one feed.
#[derive(Clone, Debug, Default)]
pub struct ClusterMemory {
    memory: Arc<Mutex<Memory>>,
}

impl ClusterMemory {
    /// Steps the memory to `feed`'s frame (once per frame) and returns what it shows; see [`Memory::step`].
    pub fn step(&self, feed: &CustomFeed, cue: bool) -> Presented {
        self.lock().step(feed, cue)
    }

    /// The memory lock. A poisoned lock still holds a whole memory, so the poison is ignored.
    fn lock(&self) -> MutexGuard<'_, Memory> {
        self.memory.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
