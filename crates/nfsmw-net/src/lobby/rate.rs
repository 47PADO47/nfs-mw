//! A token bucket for the per-player limits of `security-gameplay.md` section 5.1. The bucket counts milliseconds of
//! refill instead of fractional tokens, so it is exact in integers and many fast calls lose no refill.

/// A limit: one token every `per_token_ms`, at most `burst` saved up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RateSpec {
    pub per_token_ms: u64,
    pub burst: u32,
}

/// Chat lines and quick messages: 1 per second, burst 3.
pub const CHAT_RATE: RateSpec = RateSpec { per_token_ms: 1_000, burst: 3 };
/// Lobby actions (ready, car change, role, vote): 2 per second, burst 6.
pub const LOBBY_RATE: RateSpec = RateSpec { per_token_ms: 500, burst: 6 };
/// Votes started by one player: 1 per 60 seconds.
pub const VOTE_START_RATE: RateSpec = RateSpec { per_token_ms: 60_000, burst: 1 };
/// Joins from one source address: 5 per 10 seconds.
pub const JOIN_RATE: RateSpec = RateSpec { per_token_ms: 2_000, burst: 5 };

/// A token bucket that starts full.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateLimit {
    spec: RateSpec,
    /// Refill saved up, in milliseconds; a token costs `per_token_ms`.
    credit_ms: u64,
    last_ms: u64,
}

impl RateLimit {
    /// A full bucket of `spec` at `now_ms`. A zero `per_token_ms` is treated as 1.
    pub fn new(spec: RateSpec, now_ms: u64) -> Self {
        let spec = RateSpec { per_token_ms: spec.per_token_ms.max(1), burst: spec.burst };
        Self { spec, credit_ms: spec.per_token_ms.saturating_mul(u64::from(spec.burst)), last_ms: now_ms }
    }

    /// Takes a token at `now_ms`. `false` when none is left: the action is refused. A clock that goes backwards refills
    /// nothing.
    pub fn allow(&mut self, now_ms: u64) -> bool {
        self.refill(now_ms);
        if self.credit_ms < self.spec.per_token_ms {
            return false;
        }
        self.credit_ms -= self.spec.per_token_ms;
        true
    }

    /// Whole tokens left at `now_ms`, without taking one.
    pub fn tokens(&mut self, now_ms: u64) -> u32 {
        self.refill(now_ms);
        u32::try_from(self.credit_ms / self.spec.per_token_ms).unwrap_or(u32::MAX)
    }

    fn refill(&mut self, now_ms: u64) {
        let elapsed = now_ms.saturating_sub(self.last_ms);
        self.last_ms = self.last_ms.max(now_ms);
        let cap = self.spec.per_token_ms.saturating_mul(u64::from(self.spec.burst));
        self.credit_ms = self.credit_ms.saturating_add(elapsed).min(cap);
    }
}
