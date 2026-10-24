//! What the map keeps from one frame to the next. The theme resolves a fresh map component every frame, so this state
//! lives in a [`MapMemoryCell`] the feed cell holds (`feed.rs`), not in the component:
//!
//! - the local set of roads round the car ([`LocalSet`], `roads.rs`), rebuilt only past a move or zoom threshold;
//! - the zoom's spring ([`ZoomSpring`]): a zoom step glides on a critically damped spring that settles in about
//!   `Motion::map_zoom`; with motion reduction the zoom jumps and the overlay fades back in instead;
//! - the mode change in progress ([`ModeFade`]).

use std::sync::{Arc, Mutex, PoisonError};

use super::roads::LocalSet;
use crate::settings::hud::MapMode;
use crate::ui::theme::custom::tokens::ramp;

/// The map's state between frames.
#[derive(Debug, Default)]
pub struct MapMemory {
    pub local: LocalSet,
    pub zoom: ZoomSpring,
    pub mode: ModeFade,
}

/// The shared memory. Clones share one memory; it is `Send + Sync`, as the theme requires.
#[derive(Clone, Debug, Default)]
pub struct MapMemoryCell(Arc<Mutex<MapMemory>>);

impl MapMemoryCell {
    /// Runs `f` on the memory. A poisoned lock still holds a whole memory, so the poison is ignored.
    pub fn with<R>(&self, f: impl FnOnce(&mut MapMemory) -> R) -> R {
        let mut memory = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        f(&mut memory)
    }
}

/// The longest frame the spring integrates at once, seconds: a hitch does not throw the zoom.
const MAX_STEP: f32 = 0.1;

/// The zoom the map draws at, following `map_zoom`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ZoomSpring {
    /// The zoom drawn; `None` before the first frame, which takes the target as it is.
    value: Option<f32>,
    velocity: f32,
    /// The target of the last frame, and when it last changed (frame clock seconds).
    target: f32,
    changed_at: f32,
}

impl ZoomSpring {
    /// The zoom to draw this frame for `target`, `dt` seconds after the last at `elapsed` on the frame clock. With
    /// `reduced` motion it is the target at once; otherwise a spring that settles in about `settle` seconds.
    pub fn step(&mut self, target: f32, dt: f32, elapsed: f32, settle: f32, reduced: bool) -> f32 {
        let Some(value) = self.value else {
            *self = Self { value: Some(target), velocity: 0.0, target, changed_at: f32::NEG_INFINITY };
            return target;
        };
        if target != self.target {
            self.target = target;
            self.changed_at = elapsed;
        }
        if reduced || settle <= 0.0 || !dt.is_finite() {
            self.value = Some(target);
            self.velocity = 0.0;
            return target;
        }
        // The closed form of a critically damped spring over one step: x(t) = (x0 + (v0 + w x0) t) e^(-w t).
        let omega = 4.0 / settle;
        let dt = dt.clamp(0.0, MAX_STEP);
        let offset = value - target;
        let pull = (self.velocity + omega * offset) * dt;
        let decay = (-omega * dt).exp();
        self.velocity = (self.velocity - omega * pull) * decay;
        let next = target + (offset + pull) * decay;
        self.value = Some(next);
        next
    }

    /// How far back in the overlay is after a zoom change with motion reduced: 0 just after the change, 1 once
    /// `duration` has passed (and always with full motion, where the zoom glides instead).
    pub fn crossfade(&self, elapsed: f32, duration: f32, reduced: bool) -> f32 {
        if !reduced {
            return 1.0;
        }
        ramp(elapsed - self.changed_at, duration)
    }
}

/// The map's mode and the change into it: the mode before, and when the change started.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ModeFade {
    current: Option<MapMode>,
    previous: MapMode,
    since: f32,
}

impl ModeFade {
    /// Takes this frame's mode at `elapsed` on the frame clock. The first frame starts settled; a different mode starts
    /// a change from the one before.
    pub fn update(&mut self, mode: MapMode, elapsed: f32) {
        let Some(current) = self.current else {
            *self = Self { current: Some(mode), previous: mode, since: f32::NEG_INFINITY };
            return;
        };
        if current == mode {
            return;
        }
        *self = Self { current: Some(mode), previous: current, since: elapsed };
    }

    /// The mode the change started from (the mode itself once settled).
    pub fn previous(&self) -> MapMode {
        self.previous
    }

    /// The mode now.
    pub fn current(&self) -> MapMode {
        self.current.unwrap_or_default()
    }

    /// How far through the change the map is at `elapsed`, 0 to 1, for a change of `duration` seconds.
    pub fn progress(&self, elapsed: f32, duration: f32) -> f32 {
        ramp(elapsed - self.since, duration)
    }
}
