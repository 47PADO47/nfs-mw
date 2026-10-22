//! The server's counters, its periodic status line and its totals at shutdown.

use std::time::Duration;

use blackbox_net_core::endpoint::EndpointStats;

/// Everything the server counts. Cumulative from the start unless a field says otherwise.
#[derive(Debug, Clone, Default)]
pub struct ServerStats {
    pub ticks: u64,
    /// Time spent in [`super::Server::tick_once`].
    pub tick_time: Duration,
    /// The longest single tick.
    pub tick_time_max: Duration,
    /// Ticks the timer skipped because the loop fell behind.
    pub skipped_ticks: u64,
    pub connections: u64,
    pub joins: u64,
    pub rejected: u64,
    pub kicks: u64,
    pub timeouts: u64,
    pub resumes: u64,
    pub messages_in: u64,
    pub messages_out: u64,
    /// Reliable messages dropped over a client's message rate.
    pub messages_dropped: u64,
    /// Messages and frames that did not decode.
    pub invalid_messages: u64,
    pub strikes: u64,
    pub inputs_in: u64,
    pub pongs: u64,
    pub snapshots_sent: u64,
    pub snapshot_bytes: u64,
    pub snapshots_delivered: u64,
    pub snapshots_lost: u64,
    /// Snapshots that could not be written (a client resynced).
    pub snapshot_errors: u64,
    /// The net thread's last counters.
    pub endpoint: Option<EndpointStats>,
}

impl ServerStats {
    /// Counts one tick that took `took`.
    pub(crate) fn on_tick(&mut self, took: Duration) {
        self.ticks += 1;
        self.tick_time += took;
        self.tick_time_max = self.tick_time_max.max(took);
    }

    /// The mean tick time in microseconds.
    pub fn mean_tick_us(&self) -> u64 {
        let micros = self.tick_time.as_micros() / u128::from(self.ticks.max(1));
        u64::try_from(micros).unwrap_or(u64::MAX)
    }

    /// The one-line totals logged at shutdown.
    pub fn summary(&self) -> String {
        format!(
            "{} ticks (mean {} us, max {} us, {} skipped), {} connections, {} joins, {} refused, {} kicks, {} timeouts, \
             {} resumes, {} snapshots ({} bytes, {} delivered, {} lost), {} invalid messages, {} strikes",
            self.ticks,
            self.mean_tick_us(),
            self.tick_time_max.as_micros(),
            self.skipped_ticks,
            self.connections,
            self.joins,
            self.rejected,
            self.kicks,
            self.timeouts,
            self.resumes,
            self.snapshots_sent,
            self.snapshot_bytes,
            self.snapshots_delivered,
            self.snapshots_lost,
            self.invalid_messages,
            self.strikes,
        )
    }
}
