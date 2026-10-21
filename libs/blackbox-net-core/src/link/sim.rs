use std::collections::BTreeMap;
use std::io;
use std::net::SocketAddr;

use super::Link;
use crate::rng::SplitMix64;

/// Impairments applied by [`LossyLink`]. The default is a perfect link.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct LossConfig {
    /// Probability in `0.0..=1.0` that a sent datagram is dropped.
    pub loss: f64,
    /// Fixed one-way delay in milliseconds.
    pub latency_ms: u32,
    /// Extra random delay, uniform in `0..=jitter_ms` milliseconds.
    pub jitter_ms: u32,
    /// Probability in `0.0..=1.0` that a second copy of a surviving datagram is delivered.
    pub duplicate: f64,
    /// Probability in `0.0..=1.0` that a datagram is held back by an extra `latency_ms + jitter_ms`, so later ones overtake it.
    pub reorder: f64,
    /// Seed for the simulator's random generator. The same seed and the same calls reproduce the same run.
    pub seed: u64,
}

/// A datagram scheduled for delivery, keyed in the queue by `(deliver_at_ms, sequence)`.
type Pending = (SocketAddr, Vec<u8>);

/// A [`Link`] wrapper that simulates an impaired network on a clock the caller advances with [`LossyLink::set_now`].
///
/// Sends are dropped, delayed, duplicated or reordered as [`LossConfig`] says. Datagrams due at the same time keep
/// send order. A datagram the inner link refuses when it is finally forwarded counts as dropped.
#[derive(Debug)]
pub struct LossyLink<L: Link> {
    inner: L,
    config: LossConfig,
    rng: SplitMix64,
    now_ms: u64,
    seq: u64,
    pending: BTreeMap<(u64, u64), Pending>,
    sent: u64,
    dropped: u64,
    duplicated: u64,
}

impl<L: Link> LossyLink<L> {
    /// Wraps `inner` with the impairments in `config`. Simulated time starts at 0.
    pub fn new(inner: L, config: LossConfig) -> Self {
        Self {
            inner,
            config,
            rng: SplitMix64(config.seed),
            now_ms: 0,
            seq: 0,
            pending: BTreeMap::new(),
            sent: 0,
            dropped: 0,
            duplicated: 0,
        }
    }

    /// Advances simulated time to `now_ms` and forwards every datagram whose delivery time has come.
    pub fn set_now(&mut self, now_ms: u64) {
        self.now_ms = now_ms;
        self.flush();
    }

    /// Datagrams passed to [`Link::send`], including dropped ones.
    pub fn sent(&self) -> u64 {
        self.sent
    }

    /// Datagrams dropped by loss, or refused by the inner link on forwarding.
    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Extra copies scheduled by the duplicate impairment.
    pub fn duplicated(&self) -> u64 {
        self.duplicated
    }

    /// The wrapped link.
    pub fn inner(&self) -> &L {
        &self.inner
    }

    /// The wrapped link, mutably.
    pub fn inner_mut(&mut self) -> &mut L {
        &mut self.inner
    }

    /// A delay of `latency_ms` plus a uniform draw in `0..=jitter_ms`.
    fn jittered_delay(&mut self) -> u64 {
        let jitter = self.rng.below(u64::from(self.config.jitter_ms) + 1);
        u64::from(self.config.latency_ms) + jitter
    }

    fn schedule(&mut self, to: SocketAddr, bytes: &[u8], delay_ms: u64) {
        self.seq += 1;
        let deliver_at = self.now_ms + delay_ms;
        self.pending.insert((deliver_at, self.seq), (to, bytes.to_vec()));
    }

    /// Forwards, in order, every pending datagram whose delivery time is not after `now_ms`.
    fn flush(&mut self) {
        while let Some(entry) = self.pending.first_entry() {
            if entry.key().0 > self.now_ms {
                return;
            }
            let (to, bytes) = entry.remove();
            if self.inner.send(to, &bytes).is_err() {
                self.dropped += 1;
            }
        }
    }
}

impl<L: Link> Link for LossyLink<L> {
    fn send(&mut self, to: SocketAddr, bytes: &[u8]) -> io::Result<()> {
        self.sent += 1;
        if self.rng.next_f64() < self.config.loss {
            self.dropped += 1;
            return Ok(());
        }
        let mut delay = self.jittered_delay();
        if self.rng.next_f64() < self.config.reorder {
            delay += u64::from(self.config.latency_ms) + u64::from(self.config.jitter_ms);
        }
        self.schedule(to, bytes, delay);
        if self.rng.next_f64() < self.config.duplicate {
            self.duplicated += 1;
            let copy_delay = self.jittered_delay();
            self.schedule(to, bytes, copy_delay);
        }
        self.flush();
        Ok(())
    }

    fn recv(&mut self, buf: &mut [u8]) -> io::Result<Option<(SocketAddr, usize)>> {
        self.inner.recv(buf)
    }

    fn local_addr(&self) -> SocketAddr {
        self.inner.local_addr()
    }
}
