//! The invalid-message rule of protocol.md section 8.2: more than 16 invalid or unknown messages inside 10 s ends the
//! connection.

use std::collections::VecDeque;

/// Invalid or unknown messages a connection may send inside [`INVALID_WINDOW_MS`].
pub const INVALID_LIMIT: usize = 16;
/// The window of the rule, in milliseconds.
pub const INVALID_WINDOW_MS: u64 = 10_000;

/// Counts a connection's invalid and unknown messages over a sliding window. Holds at most `INVALID_LIMIT + 1` times.
#[derive(Debug, Clone, Default)]
pub struct InvalidMessages {
    times: VecDeque<u64>,
}

impl InvalidMessages {
    /// No messages counted.
    pub fn new() -> Self {
        Self::default()
    }

    /// Counts one invalid or unknown message at `now_ms`. Returns `true` when the connection must close: this is the
    /// 17th inside the window.
    pub fn record(&mut self, now_ms: u64) -> bool {
        while self.times.front().is_some_and(|first| now_ms.saturating_sub(*first) >= INVALID_WINDOW_MS) {
            self.times.pop_front();
        }
        self.times.push_back(now_ms);
        if self.times.len() > INVALID_LIMIT + 1 {
            self.times.pop_front();
        }
        self.times.len() > INVALID_LIMIT
    }

    /// Messages counted inside the window ending at the last call.
    pub fn count(&self) -> usize {
        self.times.len()
    }
}
