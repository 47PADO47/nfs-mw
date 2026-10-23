//! When to send a PING (sync-model.md section 5.1).

use super::Ping;

/// The fast phase after the first ping, milliseconds.
pub const FAST_PHASE_MS: u64 = 5000;
/// The ping interval in the fast phase: 4 per second.
pub const FAST_PING_MS: u64 = 250;
/// The ping interval after it: 1 per second.
pub const SLOW_PING_MS: u64 = 1000;

/// The ping timer: 4 pings per second for the first 5 s, then 1 per second.
#[derive(Debug, Clone, Default)]
pub struct PingSchedule {
    started_ms: Option<u64>,
    last_ms: Option<u64>,
    seq: u8,
}

impl PingSchedule {
    /// The ping to send at `now_ms`, or `None` when the next is not due. The first call always sends.
    pub fn due(&mut self, now_ms: u64) -> Option<Ping> {
        let started = *self.started_ms.get_or_insert(now_ms);
        if let Some(last) = self.last_ms {
            let interval = match now_ms.saturating_sub(started) < FAST_PHASE_MS {
                true => FAST_PING_MS,
                false => SLOW_PING_MS,
            };
            if now_ms.saturating_sub(last) < interval {
                return None;
            }
        }
        self.last_ms = Some(now_ms);
        let ping = Ping { seq: self.seq, client_time_ms: now_ms as u32 };
        self.seq = self.seq.wrapping_add(1);
        Some(ping)
    }

    /// Start the fast phase again (a reconnect, a clock step).
    pub fn restart(&mut self) {
        self.started_ms = None;
        self.last_ms = None;
    }
}
