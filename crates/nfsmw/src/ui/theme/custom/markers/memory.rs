//! What the world markers remember between frames: the steadied label positions per marker, the gate the player was
//! driving to (so a checkpoint pass can be confirmed where it happened), the pass's tick, and the wrong-way recovery.
//!
//! The component is rebuilt every frame, so the memory lives in a cell next to the feed ([`MarkerMemoryCell`], held by
//! `CustomFeedCell`), and every clone shares it. Time is the feed's frame clock: the memory counts the time since the
//! frame it last laid out, so a second layout in the same frame does not step the smoothing twice.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use glam::Vec2;

use crate::hud::{HudEvent, HudState};

use super::project::follow;

/// After a wrong way, the next gate draws as a recovery arrow for this long, seconds. **[ours]**
pub const RECOVERY_SECS: f32 = 5.0;

/// The tick that confirms a checkpoint pass: where the passed gate stood, and when.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tick {
    pub at: [f32; 3],
    pub started: f32,
}

/// The markers' memory.
#[derive(Debug, Default)]
pub struct MarkerMemory {
    /// The steadied label position per marker id, canvas units.
    labels: HashMap<u64, Vec2>,
    /// The frame clock of the last layout.
    clock: Option<f32>,
    /// The HUD events frame already taken.
    events_frame: Option<u64>,
    /// The next gate of the previous frame: the gate a checkpoint event says was just passed.
    gate: Option<[f32; 3]>,
    tick: Option<Tick>,
    wrong_way_at: Option<f32>,
}

impl MarkerMemory {
    /// Takes the frame at `now`: its events once per events frame (a checkpoint starts the tick at the gate the player
    /// was driving to, a wrong way starts the recovery), then the gate it drives to now. Returns the time since the last
    /// layout, seconds (zero for the first one, or a second in the same frame).
    pub fn observe(&mut self, hud: &HudState, now: f32) -> f32 {
        let dt = self.clock.map_or(0.0, |before| (now - before).max(0.0));
        self.clock = Some(now);
        if self.events_frame != Some(hud.events_frame) {
            self.events_frame = Some(hud.events_frame);
            for event in &hud.events {
                self.take(*event, now);
            }
        }
        self.gate = hud.race_info.as_ref().and_then(|race| race.next_gate);
        if hud.race_info.is_none() {
            self.wrong_way_at = None;
        }
        dt
    }

    fn take(&mut self, event: HudEvent, now: f32) {
        match event {
            HudEvent::Checkpoint { .. } => self.tick = self.gate.map(|at| Tick { at, started: now }),
            HudEvent::WrongWay => self.wrong_way_at = Some(now),
            _ => {}
        }
    }

    /// Whether the next gate is still in its recovery time after a wrong way.
    pub fn recovering(&self, now: f32) -> bool {
        self.wrong_way_at.is_some_and(|at| (0.0..RECOVERY_SECS).contains(&(now - at)))
    }

    /// The checkpoint tick and how far through `duration` it is at `now` (0 to 1); `None` once it is over.
    pub fn tick(&self, now: f32, duration: f32) -> Option<(Tick, f32)> {
        let tick = self.tick?;
        let age = now - tick.started;
        if age < 0.0 || age >= duration || duration <= 0.0 {
            return None;
        }
        Some((tick, age / duration))
    }

    /// The label position of marker `id` this frame: its last position followed toward `target` over `dt` seconds, or
    /// `target` for a marker new this frame.
    pub fn label(&mut self, id: u64, target: Vec2, dt: f32) -> Vec2 {
        let at = self.labels.get(&id).map_or(target, |&previous| follow(previous, target, dt));
        self.labels.insert(id, at);
        at
    }

    /// Forgets the labels of the markers not drawn this frame, so one coming back starts where it is.
    pub fn keep_labels(&mut self, ids: &[u64]) {
        self.labels.retain(|id, _| ids.contains(id));
    }

    /// Forgets everything: the markers are off or hidden.
    pub fn clear(&mut self) {
        *self = Self::default();
    }
}

/// The shared cell holding the markers' memory. Clones share one memory.
#[derive(Clone, Debug, Default)]
pub struct MarkerMemoryCell {
    memory: Arc<Mutex<MarkerMemory>>,
}

impl MarkerMemoryCell {
    /// The memory. A poisoned lock still holds a whole memory (a frame at worst half updated), so the poison is ignored.
    pub fn lock(&self) -> MutexGuard<'_, MarkerMemory> {
        self.memory.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
