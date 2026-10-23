//! The client's clocks (sync-model.md section 5, MP-80 and MP-81): the estimated server tick, the ping schedule and the
//! input lead.
//!
//! - [`NetClock`]: feeds PONG samples to [`blackbox_net_core::clock::ClockSync`] and runs a
//!   [`blackbox_net_core::clock::LocalClock`] toward its estimate, slewing within ±2 % and stepping past 6 ticks.
//!   [`NetClock::server_tick`] is `S_est`; [`NetClock::ping_due`] says when to send a PING (4 per second for the first
//!   5 s, then 1 per second).
//! - [`LeadControl`]: how far ahead of `S_est` the owner predicts (`C = S_est + lead`), kept so the server's input
//!   report shows inputs arriving 1 to 2 ticks early.
//!
//! Times are the caller's monotonic milliseconds; nothing here reads a clock.

mod lead;
mod ping;

#[cfg(test)]
mod tests;

pub use lead::LeadControl;
pub use ping::{FAST_PHASE_MS, FAST_PING_MS, PingSchedule, SLOW_PING_MS};

use blackbox_net_core::clock::{Adjust, ClockSync, LocalClock, adjust};
use nfsmw_sim::TICK_HZ;

/// A PING to send (protocol.md section 6.4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Ping {
    pub seq: u8,
    /// The low 32 bits of the client's monotonic milliseconds, echoed by the PONG.
    pub client_time_ms: u32,
}

/// A PONG received (protocol.md section 6.4).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Pong {
    pub seq: u8,
    /// The echo of [`Ping::client_time_ms`].
    pub client_time_ms: u32,
    /// The server tick, with its 1/256 fraction, when the PING arrived.
    pub server_recv_tick: f64,
    /// The server tick when the PONG left.
    pub server_send_tick: f64,
}

/// What [`NetClock::update`] did to the local tick clock.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ClockUpdate {
    /// No PONG yet: there is no server tick estimate.
    Unsynced,
    /// The rate factor was set; the tick runs on smoothly.
    Slewed,
    /// The tick jumped by this many ticks (the first estimate, or an error above 6 ticks). The prediction is void:
    /// clear it and snap ([`crate::reconcile::Reconciler::clock_step`]).
    Stepped(f64),
}

/// The client's estimate of the server tick.
#[derive(Debug, Clone)]
pub struct NetClock {
    sync: ClockSync,
    local: LocalClock,
    pings: PingSchedule,
    synced: bool,
    /// The network round trip of the last PONG, server time removed, milliseconds.
    last_rtt_ms: Option<f64>,
}

impl Default for NetClock {
    fn default() -> Self {
        Self::new()
    }
}

impl NetClock {
    pub fn new() -> Self {
        Self {
            sync: ClockSync::new(TICK_HZ),
            local: LocalClock::new(TICK_HZ),
            pings: PingSchedule::default(),
            synced: false,
            last_rtt_ms: None,
        }
    }

    /// The PING to send at `now_ms`, if one is due.
    pub fn ping_due(&mut self, now_ms: u64) -> Option<Ping> {
        self.pings.due(now_ms)
    }

    /// Take in a PONG that arrived at `now_ms`. Returns the network round trip in milliseconds (the server's time
    /// between receive and send removed), or `None` for a PONG that cannot be right (an echo from the future, a
    /// non-finite or backward server time).
    pub fn on_pong(&mut self, pong: &Pong, now_ms: u64) -> Option<f64> {
        let elapsed = (now_ms as u32).wrapping_sub(pong.client_time_ms);
        let sent_ms = now_ms.checked_sub(u64::from(elapsed))?;
        let (recv, send) = (pong.server_recv_tick, pong.server_send_tick);
        if !recv.is_finite() || !send.is_finite() || send < recv {
            return None;
        }
        let held_ms = (send - recv) * 1000.0 / f64::from(TICK_HZ);
        let round_trip = (now_ms - sent_ms) as f64;
        if held_ms > round_trip {
            return None;
        }
        // Take the server's holding time out of both ends, so the sample ranks by network round trip and its middle
        // is the middle of the server's handling.
        let half = (held_ms / 2.0).floor() as u64;
        self.sync.add_sample(sent_ms + half, (recv + send) / 2.0, now_ms - half);
        let rtt = round_trip - held_ms;
        self.last_rtt_ms = Some(rtt);
        Some(rtt)
    }

    /// Advance the local tick clock to `now_ms` and pull it toward the estimate. Call once a frame.
    pub fn update(&mut self, now_ms: u64) -> ClockUpdate {
        self.local.advance(now_ms);
        let Some(estimate) = self.sync.server_tick_at(now_ms) else {
            return ClockUpdate::Unsynced;
        };
        let error = estimate - self.local.tick();
        if !self.synced {
            self.synced = true;
            self.local.apply(Adjust::Step(error));
            return ClockUpdate::Stepped(error);
        }
        let decision = adjust(error);
        self.local.apply(decision);
        match decision {
            Adjust::Step(by) => ClockUpdate::Stepped(by),
            Adjust::Slew(_) => ClockUpdate::Slewed,
        }
    }

    /// `S_est`: the estimated server tick (fractional), or `None` before the first PONG.
    pub fn server_tick(&self) -> Option<f64> {
        self.synced.then(|| self.local.tick())
    }

    pub fn is_synced(&self) -> bool {
        self.synced
    }

    /// The round trip of the best kept sample, milliseconds: the one the estimate uses.
    pub fn best_rtt_ms(&self) -> Option<f64> {
        self.sync.rtt_ms()
    }

    /// The round trip of the last PONG, milliseconds.
    pub fn last_rtt_ms(&self) -> Option<f64> {
        self.last_rtt_ms
    }
}
