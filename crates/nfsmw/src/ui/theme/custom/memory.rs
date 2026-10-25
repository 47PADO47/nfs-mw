//! What the contextual layouts remember from frame to frame: the mode clock, the discovery chips, the condition chip's
//! last warning, and the moments a value of the race or the chase changed (a place, a heat level, a countdown step, a
//! banner line), for one-shot pulses and fades.
//!
//! The components are rebuilt every frame (`hud_component`), so the memory lives in a cell the feed cell carries
//! ([`LayoutMemory`]). It advances once per frame: the first component to read it on a new frame clock moves it on, the
//! others read the same [`Frame`]. Everything here is plain data with the clock passed in, so it is tested without
//! rendering.

use std::sync::{Arc, Mutex, PoisonError};

use super::condition::{ChipState, ConditionMemory};
use super::feed::CustomFeed;
use super::free_roam::discovery::{Chip, Discovery};
use super::mode::{HudMode, ModeClock, resolve};
use super::tokens::Motion;
use crate::hud::{Count, HudState, NavMarker, PursuitPhase};

/// A value and when it last changed, with the value it changed from.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Mark<T> {
    pub value: T,
    /// Elapsed seconds when `value` came.
    pub since: f32,
    /// The value before it; `None` when it is the first one seen.
    pub previous: Option<T>,
}

/// The mark after a frame whose value is `value` at `now`. No value forgets the mark, so a value that comes back is a
/// first one again.
pub fn mark<T: PartialEq + Copy>(old: Option<Mark<T>>, value: Option<T>, now: f32) -> Option<Mark<T>> {
    let value = value?;
    let Some(old) = old else { return Some(Mark { value, since: now, previous: None }) };
    if old.value == value {
        return Some(old);
    }
    Some(Mark { value, since: now, previous: Some(old.value) })
}

/// The moments the race and the chase changed.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Marks {
    /// The player's place in the race.
    pub place: Option<Mark<u32>>,
    /// The chase's heat level, while a chase runs.
    pub heat: Option<Mark<u32>>,
    /// The countdown step.
    pub count: Option<Mark<Count>>,
    /// The serial of the race's banner line.
    pub message: Option<Mark<u32>>,
}

impl Marks {
    fn update(&mut self, hud: &HudState, now: f32) {
        self.place = mark(self.place, hud.race.map(|race| race.place), now);
        self.heat = mark(self.heat, heat_level(hud), now);
        self.count = mark(self.count, hud.race.and_then(|race| race.count), now);
        self.message = mark(self.message, hud.message.as_ref().map(|message| message.serial), now);
    }
}

/// The chase's heat level, the game's whole multiplier (1 and up), while a chase runs.
pub fn heat_level(hud: &HudState) -> Option<u32> {
    let running = matches!(hud.pursuit_phase, PursuitPhase::Active | PursuitPhase::Search | PursuitPhase::Cooldown);
    if !running || hud.vehicle_heat < 1.0 || !hud.vehicle_heat.is_finite() {
        return None;
    }
    Some(hud.vehicle_heat.floor() as u32)
}

/// What the layouts read on one frame.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Frame {
    /// The frame clock, elapsed seconds.
    pub now: f32,
    pub clock: ModeClock,
    /// The discovery chips up, staying or leaving.
    pub chips: Vec<Chip>,
    pub condition: ChipState,
    pub marks: Marks,
}

impl Frame {
    pub fn mode(&self) -> HudMode {
        self.clock.mode()
    }

    /// The opacity of an element that shows in the modes `shows` accepts, crossfading over `motion.mode_change`.
    pub fn opacity(&self, shows: impl Fn(HudMode) -> bool, motion: &Motion) -> f32 {
        self.clock.opacity(shows, self.now, motion.mode_change)
    }

    /// Seconds since `since`, never negative.
    pub fn age(&self, since: f32) -> f32 {
        (self.now - since).max(0.0)
    }
}

/// The memory itself.
#[derive(Debug, Default)]
pub struct Memory {
    /// The frame clock it last advanced on.
    stamp: Option<f32>,
    clock: ModeClock,
    discovery: Discovery,
    condition: ConditionMemory,
    chip: ChipState,
    marks: Marks,
}

impl Memory {
    /// Moves on to the frame `feed`, once per frame clock.
    pub fn advance(&mut self, feed: &CustomFeed) {
        let now = feed.elapsed_s;
        if self.stamp == Some(now) {
            return;
        }
        self.stamp = Some(now);
        let hud = &*feed.hud;
        let mode = resolve(hud);
        // Back in free roam nothing of a race or a chase is kept: a restarted race starts its strip afresh.
        if self.clock.update(mode, now) && mode == HudMode::FreeRoam {
            self.marks = Marks::default();
        }
        self.marks.update(hud, now);
        let markers: &[NavMarker] = match mode {
            HudMode::FreeRoam => hud.markers.as_slice(),
            _ => &[],
        };
        let reached = hud.context.entry.is_some();
        let fade_out = Motion::for_settings(&feed.settings).fade_out;
        self.discovery.update(markers, hud.context.player, reached, now, fade_out);
        self.chip = self.condition.update(hud.condition.as_ref(), now);
    }

    /// What the layouts read on the frame it last advanced to, `now`.
    pub fn frame(&self, now: f32) -> Frame {
        Frame {
            now,
            clock: self.clock,
            chips: self.discovery.chips().to_vec(),
            condition: self.chip,
            marks: self.marks,
        }
    }
}

/// The shared memory cell. Clones share one memory.
#[derive(Clone, Debug, Default)]
pub struct LayoutMemory {
    memory: Arc<Mutex<Memory>>,
}

impl LayoutMemory {
    /// The layouts' frame for `feed`, advancing the memory if this is a new frame.
    pub fn frame(&self, feed: &CustomFeed) -> Frame {
        let mut memory = self.memory.lock().unwrap_or_else(PoisonError::into_inner);
        memory.advance(feed);
        memory.frame(feed.elapsed_s)
    }
}

#[cfg(test)]
#[path = "memory_tests.rs"]
mod tests;
