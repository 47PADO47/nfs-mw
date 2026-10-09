//! What has been said: how often and when each event was heard, and the last few events in order.

use std::collections::{HashMap, VecDeque};

/// How many of the latest events are remembered in order (the original keeps ten).
const RECENT: usize = 10;

#[derive(Debug, Clone, Copy)]
struct Said {
    count: u32,
    last: f32,
}

#[derive(Debug, Default)]
pub struct History {
    said: HashMap<u32, Said>,
    /// Latest first.
    recent: VecDeque<u32>,
}

impl History {
    /// Record that `event` was heard at time `now`.
    pub fn touch(&mut self, event: u32, now: f32) {
        let said = self.said.entry(event).or_insert(Said { count: 0, last: now });
        said.count += 1;
        said.last = now;
        self.recent.push_front(event);
        self.recent.truncate(RECENT);
    }

    /// How often `event` was heard.
    pub fn count(&self, event: u32) -> u32 {
        self.said.get(&event).map_or(0, |s| s.count)
    }

    /// When `event` was last heard.
    pub fn last_time(&self, event: u32) -> Option<f32> {
        self.said.get(&event).map(|s| s.last)
    }

    /// The event heard most recently.
    pub fn last_event(&self) -> Option<u32> {
        self.recent.front().copied()
    }

    /// Forget everything (a new pursuit starts with a clean slate).
    pub fn clear(&mut self) {
        self.said.clear();
        self.recent.clear();
    }
}
