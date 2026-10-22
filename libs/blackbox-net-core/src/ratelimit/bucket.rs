/// A token bucket on a caller-driven clock: up to `burst` tokens, refilled at `refill` tokens per `period_ms`.
///
/// Integer arithmetic only. Tokens are kept in units of `1 / period_ms`, so a refill of 5 tokens per 10 seconds adds
/// exactly 5 tokens after 10,000 ms with no rounding drift. A clock that goes backwards refills nothing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenBucket {
    /// Tokens held, scaled by `period_ms`.
    scaled: u64,
    /// Capacity, scaled by `period_ms`.
    capacity: u64,
    /// Tokens added per period.
    refill: u64,
    /// Length of one period in milliseconds (at least 1).
    period_ms: u64,
    /// The time of the last refill; `None` before the first call.
    last_ms: Option<u64>,
}

impl TokenBucket {
    /// A full bucket of `burst` tokens that regains `refill` tokens every `period_ms` milliseconds.
    pub fn new(burst: u32, refill: u32, period_ms: u64) -> Self {
        let period_ms = period_ms.max(1);
        let capacity = u64::from(burst).saturating_mul(period_ms);
        Self { scaled: capacity, capacity, refill: u64::from(refill), period_ms, last_ms: None }
    }

    /// Takes `n` tokens if the bucket holds them after refilling to `now_ms`; otherwise takes nothing.
    pub fn try_take(&mut self, now_ms: u64, n: u32) -> bool {
        self.refill_to(now_ms);
        let need = u64::from(n).saturating_mul(self.period_ms);
        if need > self.scaled {
            return false;
        }
        self.scaled -= need;
        true
    }

    /// Whether `n` tokens could be taken at `now_ms`, without taking them.
    pub fn would_allow(&self, now_ms: u64, n: u32) -> bool {
        let mut probe = *self;
        probe.try_take(now_ms, n)
    }

    /// Whole tokens held after refilling to `now_ms`.
    pub fn available(&mut self, now_ms: u64) -> u64 {
        self.refill_to(now_ms);
        self.scaled / self.period_ms
    }

    /// Whether the bucket is full at `now_ms` (it has been idle long enough to forget).
    pub fn is_full(&self, now_ms: u64) -> bool {
        let mut probe = *self;
        probe.refill_to(now_ms);
        probe.scaled >= probe.capacity
    }

    fn refill_to(&mut self, now_ms: u64) {
        let Some(last) = self.last_ms else {
            self.last_ms = Some(now_ms);
            return;
        };
        let elapsed = now_ms.saturating_sub(last);
        self.last_ms = Some(last.max(now_ms));
        let added = elapsed.saturating_mul(self.refill);
        self.scaled = self.scaled.saturating_add(added).min(self.capacity);
    }
}
