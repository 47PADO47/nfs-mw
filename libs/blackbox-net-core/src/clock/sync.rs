//! Ping samples and the server tick estimate.

use std::collections::VecDeque;

/// Samples kept for the estimate.
const KEPT: usize = 10;

/// One ping exchange: when it was sent, the server's tick when it answered, and when the answer arrived.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Sample {
    sent_ms: u64,
    server_tick: f64,
    received_ms: u64,
}

impl Sample {
    fn rtt_ms(&self) -> u64 {
        self.received_ms.saturating_sub(self.sent_ms)
    }

    /// The client time at which the server is assumed to have answered: the middle of the round trip.
    fn midpoint_ms(&self) -> f64 {
        (self.sent_ms as f64 + self.received_ms as f64) / 2.0
    }
}

/// The client's estimate of the server tick, from the last ten ping samples.
#[derive(Debug, Clone, PartialEq)]
pub struct ClockSync {
    tick_hz: u32,
    samples: VecDeque<Sample>,
}

impl ClockSync {
    /// An empty filter for a server running at `tick_hz` ticks per second.
    pub fn new(tick_hz: u32) -> Self {
        Self { tick_hz, samples: VecDeque::with_capacity(KEPT) }
    }

    /// Records one ping exchange. Samples that arrive before they were sent, or carry a non-finite tick, are ignored.
    /// When ten samples are kept, the oldest is dropped.
    pub fn add_sample(&mut self, sent_ms: u64, server_tick: f64, received_ms: u64) {
        if received_ms < sent_ms || !server_tick.is_finite() {
            return;
        }
        if self.samples.len() >= KEPT {
            self.samples.pop_front();
        }
        self.samples.push_back(Sample { sent_ms, server_tick, received_ms });
    }

    /// The round trip of the best kept sample, in milliseconds.
    pub fn rtt_ms(&self) -> Option<f64> {
        self.best().map(|s| s.rtt_ms() as f64)
    }

    /// The estimated server tick (fractional) at `now_ms`, from the kept sample with the lowest round trip. On a tie the
    /// newer sample wins.
    pub fn server_tick_at(&self, now_ms: u64) -> Option<f64> {
        let best = self.best()?;
        let elapsed_ms = now_ms as f64 - best.midpoint_ms();
        Some(best.server_tick + elapsed_ms * f64::from(self.tick_hz) / 1000.0)
    }

    fn best(&self) -> Option<&Sample> {
        self.samples.iter().rev().min_by_key(|s| s.rtt_ms())
    }
}
