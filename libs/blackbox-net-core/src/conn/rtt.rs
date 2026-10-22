//! Round-trip time and loss estimates: smoothed RTT and variance with the RFC 6298 gains, and loss as an exponential
//! average of packet outcomes.

/// Smoothed RTT before the first sample.
const INITIAL_SRTT_MS: f64 = 100.0;
/// Variance before the first sample.
const INITIAL_RTTVAR_MS: f64 = 50.0;
/// Gain of the loss average: each outcome moves it by 1/16 of the difference.
const LOSS_GAIN: f64 = 1.0 / 16.0;
/// Shortest interval between two sends of the same reliable fragment.
const MIN_RESEND_MS: f64 = 20.0;

/// RTT and loss state of one connection.
#[derive(Debug, Clone, Copy)]
pub struct Rtt {
    srtt: f64,
    rttvar: f64,
    has_sample: bool,
    loss: f64,
}

impl Default for Rtt {
    fn default() -> Self {
        Self { srtt: INITIAL_SRTT_MS, rttvar: INITIAL_RTTVAR_MS, has_sample: false, loss: 0.0 }
    }
}

impl Rtt {
    /// Feeds one round-trip sample in milliseconds. The first sample sets both estimates; later ones move them with the
    /// RFC 6298 gains (1/8 for the mean, 1/4 for the variance).
    pub fn sample(&mut self, ms: f64) {
        if !self.has_sample {
            self.has_sample = true;
            self.srtt = ms;
            self.rttvar = ms / 2.0;
            return;
        }
        self.rttvar = 0.75 * self.rttvar + 0.25 * (self.srtt - ms).abs();
        self.srtt = 0.875 * self.srtt + 0.125 * ms;
    }

    /// Records one packet outcome: lost (1) or delivered (0) in the loss average.
    pub fn record_outcome(&mut self, lost: bool) {
        let outcome = f64::from(u8::from(lost));
        self.loss += (outcome - self.loss) * LOSS_GAIN;
    }

    /// Smoothed round-trip time in milliseconds.
    pub fn srtt_ms(&self) -> f64 {
        self.srtt
    }

    /// Round-trip time variance in milliseconds.
    pub fn rttvar_ms(&self) -> f64 {
        self.rttvar
    }

    /// Loss average between 0 and 1.
    pub fn loss(&self) -> f64 {
        self.loss
    }

    /// Milliseconds a reliable fragment waits after its last send before it is due again: `max(20, 1.5 * srtt)`.
    pub fn resend_after_ms(&self) -> f64 {
        (1.5 * self.srtt).max(MIN_RESEND_MS)
    }
}
