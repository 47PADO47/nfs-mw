//! The one-line form of the network counters.

use std::fmt;

/// One reading of [`super::NetStats`], ready to print.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NetLine {
    pub rtt_ms: Option<f64>,
    pub jitter_ms: f64,
    /// 0..1.
    pub loss: f64,
    pub snapshot_age_ms: Option<u64>,
    pub corrections_per_min: u64,
    pub snaps_per_min: u64,
    pub bytes_in_per_s: f64,
    pub bytes_out_per_s: f64,
}

/// `net rtt 12 ms  jit 1.5 ms  loss 0.4%  snap 33 ms  corr 3/min  snaps 0/min  in 14.2 KB/s  out 2.1 KB/s`; a value not
/// yet measured prints as `-`.
impl fmt::Display for NetLine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("net rtt ")?;
        match self.rtt_ms {
            Some(rtt) => write!(f, "{rtt:.0} ms")?,
            None => f.write_str("-")?,
        }
        write!(f, "  jit {:.1} ms  loss {:.1}%  snap ", self.jitter_ms, self.loss * 100.0)?;
        match self.snapshot_age_ms {
            Some(age) => write!(f, "{age} ms")?,
            None => f.write_str("-")?,
        }
        write!(
            f,
            "  corr {}/min  snaps {}/min  in {:.1} KB/s  out {:.1} KB/s",
            self.corrections_per_min,
            self.snaps_per_min,
            self.bytes_in_per_s / 1000.0,
            self.bytes_out_per_s / 1000.0
        )
    }
}
