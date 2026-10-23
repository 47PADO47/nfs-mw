use std::collections::HashMap;
use std::hash::Hash;

use super::TokenBucket;

/// One token bucket per key (a source address, a peer id), with a bound on how many keys are tracked.
///
/// When the table is full, buckets that have refilled completely are forgotten (they hold no information). If every
/// tracked key is still active, a new key is refused: under a flood from many sources the limiter fails closed rather
/// than growing without bound.
#[derive(Debug, Clone)]
pub struct KeyedLimiter<K> {
    template: TokenBucket,
    max_keys: usize,
    buckets: HashMap<K, TokenBucket>,
}

impl<K: Hash + Eq + Copy> KeyedLimiter<K> {
    /// A limiter that gives each key `burst` tokens, refilled at `refill` per `period_ms`, for at most `max_keys` keys.
    pub fn new(burst: u32, refill: u32, period_ms: u64, max_keys: usize) -> Self {
        Self {
            template: TokenBucket::new(burst, refill, period_ms),
            max_keys: max_keys.max(1),
            buckets: HashMap::new(),
        }
    }

    /// Takes `n` tokens from `key`'s bucket. `false` when the bucket is short or the table is full of active keys.
    pub fn try_take(&mut self, now_ms: u64, key: K, n: u32) -> bool {
        if let Some(bucket) = self.buckets.get_mut(&key) {
            return bucket.try_take(now_ms, n);
        }
        if self.buckets.len() >= self.max_keys {
            self.buckets.retain(|_, bucket| !bucket.is_full(now_ms));
        }
        if self.buckets.len() >= self.max_keys {
            return false;
        }
        let mut bucket = self.template;
        let allowed = bucket.try_take(now_ms, n);
        self.buckets.insert(key, bucket);
        allowed
    }

    /// Whether `key` could take `n` tokens now, without taking them. A key not tracked yet could, unless the table is
    /// full of active keys.
    pub fn would_allow(&self, now_ms: u64, key: K, n: u32) -> bool {
        if let Some(bucket) = self.buckets.get(&key) {
            return bucket.would_allow(now_ms, n);
        }
        if self.buckets.len() < self.max_keys {
            return self.template.would_allow(now_ms, n);
        }
        self.buckets.values().any(|bucket| bucket.is_full(now_ms))
    }

    /// Keys tracked right now.
    pub fn tracked(&self) -> usize {
        self.buckets.len()
    }

    /// Forgets `key`.
    pub fn forget(&mut self, key: &K) {
        self.buckets.remove(key);
    }
}
