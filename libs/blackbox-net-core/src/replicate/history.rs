//! The history ring: the last few ticks of frames, so that deltas can be taken against an acknowledged baseline.

use super::frame::Frame;

/// One ring slot: a frame and whether it was ever begun.
#[derive(Debug, Clone, Default)]
struct Slot {
    frame: Frame,
    written: bool,
}

/// A ring of `capacity` frames. Tick `t` lives in slot `t % capacity`, so a frame is overwritten `capacity` ticks
/// later.
#[derive(Debug, Clone)]
pub struct History {
    slots: Vec<Slot>,
    latest: Option<u32>,
}

impl History {
    /// A history with `capacity` slots (at least one).
    pub fn new(capacity: usize) -> Self {
        let capacity = capacity.max(1);
        Self { slots: (0..capacity).map(|_| Slot::default()).collect(), latest: None }
    }

    /// The number of slots.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// Starts `tick`: reuses its slot (the one of the oldest frame), clears it to `tick` and returns it to be filled.
    ///
    /// Ticks should increase. A tick at or below the latest one still clears and returns its slot, which may hold a
    /// newer frame; `latest_tick` is then unchanged.
    pub fn begin(&mut self, tick: u32) -> &mut Frame {
        self.latest = Some(match self.latest {
            Some(latest) if latest >= tick => latest,
            _ => tick,
        });
        let index = self.index_of(tick);
        // `index` is a remainder of `len`, and `new` keeps at least one slot, so it is in bounds.
        let slot = &mut self.slots[index];
        slot.frame.clear(tick);
        slot.written = true;
        &mut slot.frame
    }

    /// The frame of `tick`, if it was begun and its slot has not been reused since.
    pub fn frame(&self, tick: u32) -> Option<&Frame> {
        let slot = self.slots.get(self.index_of(tick))?;
        if !slot.written || slot.frame.tick() != tick {
            return None;
        }
        Some(&slot.frame)
    }

    /// The highest tick begun so far.
    pub fn latest_tick(&self) -> Option<u32> {
        self.latest
    }

    /// The slot index of `tick`.
    fn index_of(&self, tick: u32) -> usize {
        tick as usize % self.slots.len()
    }
}
