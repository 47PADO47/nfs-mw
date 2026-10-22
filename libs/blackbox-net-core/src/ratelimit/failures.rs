use std::collections::HashMap;
use std::hash::Hash;

/// The rule of a [`FailureLimiter`]: `max_failures` within `window_ms` lock the key for `first_lock_ms`; each further
/// lock doubles, up to `max_lock_ms`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FailureRule {
    /// Failures that trigger a lock.
    pub max_failures: u32,
    /// Window in which they are counted.
    pub window_ms: u64,
    /// Length of the first lock.
    pub first_lock_ms: u64,
    /// Longest lock; the doubling stops here.
    pub max_lock_ms: u64,
}

impl FailureRule {
    /// Room password guesses (security.md 4.6): 5 wrong in 60 s, then 60 s of refusal doubling to 15 minutes.
    pub const PASSWORD: Self =
        Self { max_failures: 5, window_ms: 60_000, first_lock_ms: 60_000, max_lock_ms: 15 * 60_000 };
    /// Authorisation failures per address (security-ops.md 2.2): 10 per minute, then ignored for 5 minutes.
    pub const AUTH: Self =
        Self { max_failures: 10, window_ms: 60_000, first_lock_ms: 5 * 60_000, max_lock_ms: 5 * 60_000 };
}

#[derive(Debug, Clone, Copy)]
struct Entry {
    window_start: u64,
    failures: u32,
    locked_until: u64,
    next_lock_ms: u64,
}

/// Counts failures per key and locks keys that fail too often, with a doubling lock. Never locks the whole table: a
/// griefer can lock out only the keys it fails as. The table holds at most `max_keys` keys; when it is full, keys
/// with no lock and no recent failure are forgotten, and if none can be, a new key is treated as locked (fail closed).
#[derive(Debug, Clone)]
pub struct FailureLimiter<K> {
    rule: FailureRule,
    max_keys: usize,
    entries: HashMap<K, Entry>,
}

impl<K: Hash + Eq + Copy> FailureLimiter<K> {
    /// An empty limiter.
    pub fn new(rule: FailureRule, max_keys: usize) -> Self {
        Self { rule, max_keys: max_keys.max(1), entries: HashMap::new() }
    }

    /// Whether `key` is refused at `now_ms`.
    pub fn is_locked(&self, now_ms: u64, key: &K) -> bool {
        let Some(entry) = self.entries.get(key) else {
            return self.entries.len() >= self.max_keys && !self.has_stale(now_ms);
        };
        now_ms < entry.locked_until
    }

    /// Records a failure for `key`. Returns `true` when this failure started a lock.
    pub fn record_failure(&mut self, now_ms: u64, key: K) -> bool {
        if !self.entries.contains_key(&key) && self.entries.len() >= self.max_keys {
            self.forget_stale(now_ms);
        }
        if !self.entries.contains_key(&key) && self.entries.len() >= self.max_keys {
            return false;
        }
        let rule = self.rule;
        let entry = self.entries.entry(key).or_insert(Entry {
            window_start: now_ms,
            failures: 0,
            locked_until: 0,
            next_lock_ms: rule.first_lock_ms,
        });
        if now_ms.saturating_sub(entry.window_start) >= rule.window_ms {
            entry.window_start = now_ms;
            entry.failures = 0;
        }
        entry.failures = entry.failures.saturating_add(1);
        if entry.failures < rule.max_failures {
            return false;
        }
        entry.locked_until = now_ms.saturating_add(entry.next_lock_ms);
        entry.next_lock_ms = entry.next_lock_ms.saturating_mul(2).min(rule.max_lock_ms);
        entry.failures = 0;
        entry.window_start = now_ms;
        true
    }

    /// Forgets `key`'s failures and lock history (a correct password, a completed join).
    pub fn record_success(&mut self, key: &K) {
        self.entries.remove(key);
    }

    /// Keys tracked right now.
    pub fn tracked(&self) -> usize {
        self.entries.len()
    }

    fn is_stale(&self, now_ms: u64, entry: &Entry) -> bool {
        now_ms >= entry.locked_until && now_ms.saturating_sub(entry.window_start) >= self.rule.window_ms
    }

    fn has_stale(&self, now_ms: u64) -> bool {
        self.entries.values().any(|entry| self.is_stale(now_ms, entry))
    }

    fn forget_stale(&mut self, now_ms: u64) {
        let rule = self.rule;
        self.entries.retain(|_, entry| {
            now_ms < entry.locked_until || now_ms.saturating_sub(entry.window_start) < rule.window_ms
        });
    }
}
