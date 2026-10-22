//! The simulated bad network behind the console command `net.sim <loss> <rtt> <jitter>` (features.md MP-83): a
//! validated setting and its mapping to the lossy link simulator of `blackbox_net_core::link`.
//!
//! `loss` is a percentage applied to every datagram in each direction, `rtt` the added round trip in milliseconds
//! (half on send, half on receive) and `jitter` the added random delay in milliseconds (also split between the two
//! directions). `net.sim off` (or `net.sim 0 0 0`) turns it off.

#[cfg(test)]
mod tests;

use std::fmt;

use blackbox_net_core::link::LossConfig;

/// Highest loss accepted, percent.
pub const MAX_LOSS_PERCENT: f32 = 50.0;
/// Highest added round trip accepted, milliseconds.
pub const MAX_RTT_MS: u32 = 2000;
/// Highest added jitter accepted, milliseconds.
pub const MAX_JITTER_MS: u32 = 1000;

/// Why a `net.sim` setting was refused.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum NetSimError {
    #[error("usage: net.sim <loss %> <rtt ms> <jitter ms>, or net.sim off")]
    Usage,
    #[error("`{0}` is not a number")]
    NotANumber(String),
    #[error("loss must be 0 to {MAX_LOSS_PERCENT} %")]
    Loss,
    #[error("rtt must be 0 to {MAX_RTT_MS} ms")]
    Rtt,
    #[error("jitter must be 0 to {MAX_JITTER_MS} ms")]
    Jitter,
}

/// A validated `net.sim` setting.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct NetSim {
    loss_percent: f32,
    rtt_ms: u32,
    jitter_ms: u32,
}

/// The two link simulators of one end: one wraps what it sends, one what it receives.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LinkPair {
    pub send: LossConfig,
    pub receive: LossConfig,
}

impl NetSim {
    /// No impairment.
    pub const OFF: Self = Self { loss_percent: 0.0, rtt_ms: 0, jitter_ms: 0 };

    /// A setting of `loss_percent` percent loss, `rtt_ms` of added round trip and `jitter_ms` of added jitter.
    pub fn new(loss_percent: f32, rtt_ms: u32, jitter_ms: u32) -> Result<Self, NetSimError> {
        if !(0.0..=MAX_LOSS_PERCENT).contains(&loss_percent) {
            return Err(NetSimError::Loss);
        }
        if rtt_ms > MAX_RTT_MS {
            return Err(NetSimError::Rtt);
        }
        if jitter_ms > MAX_JITTER_MS {
            return Err(NetSimError::Jitter);
        }
        Ok(Self { loss_percent, rtt_ms, jitter_ms })
    }

    /// The setting of the console arguments after `net.sim`: three numbers, or `off`. A trailing `%` on the loss and
    /// `ms` on the times are accepted.
    pub fn parse(args: &str) -> Result<Self, NetSimError> {
        let words: Vec<&str> = args.split_whitespace().collect();
        if words.as_slice() == ["off"] {
            return Ok(Self::OFF);
        }
        let [loss, rtt, jitter] = words.as_slice() else {
            return Err(NetSimError::Usage);
        };
        let loss = number::<f32>(loss.trim_end_matches('%'))?;
        let rtt = number::<u32>(rtt.trim_end_matches("ms"))?;
        let jitter = number::<u32>(jitter.trim_end_matches("ms"))?;
        Self::new(loss, rtt, jitter)
    }

    pub fn loss_percent(&self) -> f32 {
        self.loss_percent
    }

    pub fn rtt_ms(&self) -> u32 {
        self.rtt_ms
    }

    pub fn jitter_ms(&self) -> u32 {
        self.jitter_ms
    }

    pub fn is_off(&self) -> bool {
        self.loss_percent == 0.0 && self.rtt_ms == 0 && self.jitter_ms == 0
    }

    /// The simulator settings for both directions of one end. Each direction drops `loss` of its datagrams and adds
    /// half the round trip and half the jitter (the odd millisecond goes to sending). `seed` makes the run repeatable;
    /// the receive side uses a different stream.
    pub fn link_configs(&self, seed: u64) -> LinkPair {
        let loss = f64::from(self.loss_percent) / 100.0;
        let base = LossConfig { loss, ..LossConfig::default() };
        LinkPair {
            send: LossConfig {
                latency_ms: self.rtt_ms.div_ceil(2),
                jitter_ms: self.jitter_ms.div_ceil(2),
                seed,
                ..base
            },
            receive: LossConfig {
                latency_ms: self.rtt_ms / 2,
                jitter_ms: self.jitter_ms / 2,
                seed: seed ^ 0x9E37_79B9_7F4A_7C15,
                ..base
            },
        }
    }
}

/// `net.sim 2 120 10`, or `net.sim off`.
impl fmt::Display for NetSim {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.is_off() {
            return f.write_str("net.sim off");
        }
        write!(f, "net.sim {} {} {}", self.loss_percent, self.rtt_ms, self.jitter_ms)
    }
}

fn number<T: std::str::FromStr>(word: &str) -> Result<T, NetSimError> {
    word.parse().map_err(|_| NetSimError::NotANumber(word.to_string()))
}
