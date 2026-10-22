//! Silence detection per slot (MP-50): any datagram from a peer counts as hearing from it.

use std::collections::BTreeMap;

/// Silence after which a peer is dropped (MP-50).
pub const TIMEOUT_MS: u64 = 10_000;
/// Silence after which a peer's car is parked and the peer is marked lagging (sync-model.md 2.5).
pub const LAG_MS: u64 = 2_000;

/// How recently a peer was heard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Liveness {
    Fresh,
    /// Silent for [`LAG_MS`] or more.
    Lagging,
    /// Silent for [`TIMEOUT_MS`] or more.
    TimedOut,
}

/// When each connected slot was last heard from.
#[derive(Debug, Clone, Default)]
pub struct Presence {
    last: BTreeMap<u8, u64>,
}

impl Presence {
    /// Nobody tracked.
    pub fn new() -> Self {
        Self::default()
    }

    /// `slot` sent something at `now_ms`. Starts tracking a new slot. Time never moves back for a slot.
    pub fn heard(&mut self, slot: u8, now_ms: u64) {
        let last = self.last.entry(slot).or_insert(now_ms);
        *last = (*last).max(now_ms);
    }

    /// Stops tracking `slot` (it left cleanly or was removed).
    pub fn forget(&mut self, slot: u8) {
        self.last.remove(&slot);
    }

    /// How `slot` stands at `now_ms`, or `None` when it is not tracked.
    pub fn liveness(&self, slot: u8, now_ms: u64) -> Option<Liveness> {
        let silent = now_ms.saturating_sub(*self.last.get(&slot)?);
        if silent >= TIMEOUT_MS {
            return Some(Liveness::TimedOut);
        }
        if silent >= LAG_MS {
            return Some(Liveness::Lagging);
        }
        Some(Liveness::Fresh)
    }

    /// The slots silent for [`TIMEOUT_MS`] at `now_ms`, in slot order. They are no longer tracked.
    pub fn take_timed_out(&mut self, now_ms: u64) -> Vec<u8> {
        let gone: Vec<u8> = self
            .last
            .iter()
            .filter(|(_, last)| now_ms.saturating_sub(**last) >= TIMEOUT_MS)
            .map(|(slot, _)| *slot)
            .collect();
        for slot in &gone {
            self.last.remove(slot);
        }
        gone
    }
}
