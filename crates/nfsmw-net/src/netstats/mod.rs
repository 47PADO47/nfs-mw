//! The data of the network overlay line (features.md MP-82): round trip, jitter, loss, snapshot age, corrections and
//! snaps per minute, bytes in and out. Plain counters over rolling windows, fed by the caller with its own times; the
//! game prints [`NetLine`] in the advanced level of `--show-metrics`.

mod line;
mod rolling;

#[cfg(test)]
mod tests;

pub use line::NetLine;
pub use rolling::Rolling;

use nfsmw_sim::{TICK_HZ, Tick};

/// Bandwidth is averaged over 2 s.
const BYTES_BUCKET_MS: u64 = 250;
const BYTES_BUCKETS: usize = 8;
/// Loss is counted over 10 s.
const LOSS_BUCKET_MS: u64 = 1000;
const LOSS_BUCKETS: usize = 10;
/// Corrections and snaps are counted over a minute.
const MINUTE_BUCKET_MS: u64 = 1000;
const MINUTE_BUCKETS: usize = 60;
/// The jitter filter's gain, as RFC 3550 section 6.4.1: 1/16.
const JITTER_GAIN: f64 = 1.0 / 16.0;

/// The client's network counters.
#[derive(Debug, Clone)]
pub struct NetStats {
    /// Ticks between two snapshots, for counting lost ones.
    interval_ticks: u32,
    bytes_in: Rolling,
    bytes_out: Rolling,
    delivered: Rolling,
    lost: Rolling,
    corrections: Rolling,
    snaps: Rolling,
    rtt_ms: Option<f64>,
    jitter_ms: f64,
    /// The newest snapshot's tick and arrival time.
    last_snapshot: Option<(Tick, u64)>,
}

impl NetStats {
    /// Counters for a session sending a snapshot every `interval_ticks` ticks (3 at 20 Hz).
    pub fn new(interval_ticks: u32) -> Self {
        Self {
            interval_ticks: interval_ticks.max(1),
            bytes_in: Rolling::new(BYTES_BUCKET_MS, BYTES_BUCKETS),
            bytes_out: Rolling::new(BYTES_BUCKET_MS, BYTES_BUCKETS),
            delivered: Rolling::new(LOSS_BUCKET_MS, LOSS_BUCKETS),
            lost: Rolling::new(LOSS_BUCKET_MS, LOSS_BUCKETS),
            corrections: Rolling::new(MINUTE_BUCKET_MS, MINUTE_BUCKETS),
            snaps: Rolling::new(MINUTE_BUCKET_MS, MINUTE_BUCKETS),
            rtt_ms: None,
            jitter_ms: 0.0,
            last_snapshot: None,
        }
    }

    /// A datagram of `bytes` arrived at `now_ms`.
    pub fn on_bytes_in(&mut self, bytes: usize, now_ms: u64) {
        self.bytes_in.add(now_ms, bytes as u64);
    }

    /// A datagram of `bytes` left at `now_ms`.
    pub fn on_bytes_out(&mut self, bytes: usize, now_ms: u64) {
        self.bytes_out.add(now_ms, bytes as u64);
    }

    /// A round-trip measurement, milliseconds (from [`crate::clock::NetClock::on_pong`]). Non-finite values are ignored.
    pub fn on_rtt(&mut self, rtt_ms: f64) {
        if rtt_ms.is_finite() {
            self.rtt_ms = Some(rtt_ms.max(0.0));
        }
    }

    /// The snapshot of `tick` arrived at `now_ms`. Snapshots skipped since the newest one count as lost; the change in
    /// spacing between arrival and tick feeds the jitter filter. A snapshot older than the newest (reordered) counts as
    /// delivered only.
    pub fn on_snapshot(&mut self, tick: Tick, now_ms: u64) {
        self.delivered.add(now_ms, 1);
        let Some((last_tick, last_ms)) = self.last_snapshot else {
            self.last_snapshot = Some((tick, now_ms));
            return;
        };
        if tick <= last_tick {
            return;
        }
        let gap = tick - last_tick;
        let missed = (gap / self.interval_ticks).saturating_sub(1);
        if missed > 0 {
            self.lost.add(now_ms, u64::from(missed));
        }
        let spacing = now_ms as f64 - last_ms as f64 - f64::from(gap) * 1000.0 / f64::from(TICK_HZ);
        self.jitter_ms += (spacing.abs() - self.jitter_ms) * JITTER_GAIN;
        self.last_snapshot = Some((tick, now_ms));
    }

    /// The reconciler corrected the own car at `now_ms`.
    pub fn on_correction(&mut self, now_ms: u64) {
        self.corrections.add(now_ms, 1);
    }

    /// The own car snapped at `now_ms`.
    pub fn on_snap(&mut self, now_ms: u64) {
        self.snaps.add(now_ms, 1);
    }

    /// The last round trip, milliseconds.
    pub fn rtt_ms(&self) -> Option<f64> {
        self.rtt_ms
    }

    /// The smoothed snapshot arrival jitter, milliseconds.
    pub fn jitter_ms(&self) -> f64 {
        self.jitter_ms
    }

    /// Milliseconds since the newest snapshot arrived.
    pub fn snapshot_age_ms(&self, now_ms: u64) -> Option<u64> {
        self.last_snapshot.map(|(_, at)| now_ms.saturating_sub(at))
    }

    /// The share of snapshots lost over the last 10 s, 0..1.
    pub fn loss(&mut self, now_ms: u64) -> f64 {
        let lost = self.lost.sum(now_ms);
        let total = lost + self.delivered.sum(now_ms);
        match total {
            0 => 0.0,
            total => lost as f64 / total as f64,
        }
    }

    /// Everything the overlay line shows, at `now_ms`.
    pub fn line(&mut self, now_ms: u64) -> NetLine {
        NetLine {
            rtt_ms: self.rtt_ms,
            jitter_ms: self.jitter_ms,
            loss: self.loss(now_ms),
            snapshot_age_ms: self.snapshot_age_ms(now_ms),
            corrections_per_min: self.corrections.sum(now_ms),
            snaps_per_min: self.snaps.sum(now_ms),
            bytes_in_per_s: self.bytes_in.per_second(now_ms),
            bytes_out_per_s: self.bytes_out.per_second(now_ms),
        }
    }
}
