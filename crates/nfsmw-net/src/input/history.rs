//! The client's record of the inputs it sent, and the redundant frames built from it.

use nfsmw_sim::Tick;

use super::code::InputCode;
use super::frame::{InputFrame, MAX_FRAME_TICKS};

/// Ticks of input the history keeps: about a second at 60 Hz, more than any prediction window.
pub const HISTORY_TICKS: usize = 64;

/// The last [`HISTORY_TICKS`] ticks of a player's input, by tick.
#[derive(Debug, Clone)]
pub struct InputHistory {
    slots: [Option<(Tick, InputCode)>; HISTORY_TICKS],
    newest: Option<Tick>,
    uses_clutch: bool,
}

impl InputHistory {
    /// An empty history. `uses_clutch` is the session's setting, copied into every frame.
    pub fn new(uses_clutch: bool) -> Self {
        Self { slots: [None; HISTORY_TICKS], newest: None, uses_clutch }
    }

    /// Records the input of `tick`, replacing an older tick in the same slot.
    pub fn record(&mut self, tick: Tick, code: InputCode) {
        let index = tick as usize % HISTORY_TICKS;
        if let Some(slot) = self.slots.get_mut(index) {
            *slot = Some((tick, code));
        }
        if self.newest.is_none_or(|newest| tick > newest) {
            self.newest = Some(tick);
        }
    }

    /// The input recorded for `tick`, if it is still held.
    pub fn get(&self, tick: Tick) -> Option<InputCode> {
        let (held, code) = (*self.slots.get(tick as usize % HISTORY_TICKS)?)?;
        match held == tick {
            true => Some(code),
            false => None,
        }
    }

    /// The newest recorded tick.
    pub fn newest(&self) -> Option<Tick> {
        self.newest
    }

    /// The frame to send now: the newest inputs up to `max_ticks` (at most 8) after `acked`, the last tick the server
    /// confirmed applying (`ack_input` of a snapshot), oldest first. Every input is thereby repeated in each frame
    /// until the server acknowledges it. `None` when there is nothing to send.
    pub fn frame(&self, acked: Option<Tick>, max_ticks: usize) -> Option<InputFrame> {
        let newest = self.newest?;
        let span = max_ticks.clamp(1, MAX_FRAME_TICKS) as u32;
        let floor = acked.map_or(0, |acked| acked.saturating_add(1));
        if newest < floor {
            return None;
        }
        self.get(newest)?;
        // Walk back over the run of recorded ticks that ends at the newest.
        let mut first = newest;
        while newest - first + 1 < span && first > floor && self.get(first - 1).is_some() {
            first -= 1;
        }
        let mut frame = InputFrame::new(first as u16, self.uses_clutch);
        for tick in first..=newest {
            let code = self.get(tick)?;
            frame.push(code).ok()?;
        }
        Some(frame)
    }
}
