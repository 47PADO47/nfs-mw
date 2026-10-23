//! The join-storm limit per source address and the password guess limit (security.md 4.6), both bounded.

use std::collections::{BTreeMap, VecDeque};

use crate::lobby::{JOIN_RATE, RateLimit};

/// Most source addresses the join limit tracks.
pub const MAX_SOURCES: usize = 1024;
/// Wrong passwords allowed per player id and per source inside [`GUESS_WINDOW_MS`].
pub const GUESS_LIMIT: usize = 5;
/// The guess window.
pub const GUESS_WINDOW_MS: u64 = 60_000;
/// The first refusal after too many guesses; it doubles each time up to [`MAX_LOCK_MS`].
pub const LOCK_MS: u64 = 60_000;
/// The longest refusal: 15 minutes.
pub const MAX_LOCK_MS: u64 = 15 * 60_000;
/// Most keys the password guard tracks.
pub const MAX_GUESS_KEYS: usize = 2048;

/// Joins per source address ([`JOIN_RATE`]: 5 per 10 s). When the table is full of active sources it refuses new
/// ones (fail closed); idle sources are forgotten first.
#[derive(Debug, Clone, Default)]
pub struct JoinGuard {
    sources: BTreeMap<u64, RateLimit>,
}

impl JoinGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether a join from `source` (the caller's key for the address) is allowed at `now_ms`.
    pub fn allow(&mut self, source: u64, now_ms: u64) -> bool {
        if !self.sources.contains_key(&source) && self.sources.len() >= MAX_SOURCES {
            self.sources.retain(|_, bucket| bucket.tokens(now_ms) < JOIN_RATE.burst);
        }
        if !self.sources.contains_key(&source) && self.sources.len() >= MAX_SOURCES {
            return false;
        }
        self.sources.entry(source).or_insert_with(|| RateLimit::new(JOIN_RATE, now_ms)).allow(now_ms)
    }
}

/// Who a password guess is counted against.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum GuessKey {
    Player([u8; 16]),
    Source(u64),
}

#[derive(Debug, Clone, Default)]
struct Guesses {
    fails: VecDeque<u64>,
    locked_until: u64,
    next_lock_ms: u64,
    last_ms: u64,
}

/// Wrong-password limits: [`GUESS_LIMIT`] per player id and per source in [`GUESS_WINDOW_MS`], then a refusal of
/// [`LOCK_MS`] that doubles up to [`MAX_LOCK_MS`]. The room itself is never locked, so a griefer cannot lock out
/// friends.
#[derive(Debug, Clone, Default)]
pub struct PasswordGuard {
    keys: BTreeMap<GuessKey, Guesses>,
}

impl PasswordGuard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Whether the player or the source is in a refusal at `now_ms`. When the table is full of refused keys, a key it
    /// does not hold is refused too (fail closed).
    pub fn locked(&self, player_id: [u8; 16], source: u64, now_ms: u64) -> bool {
        let keys = [GuessKey::Player(player_id), GuessKey::Source(source)];
        if keys.iter().any(|key| self.keys.get(key).is_some_and(|guesses| now_ms < guesses.locked_until)) {
            return true;
        }
        let unknown = keys.iter().any(|key| !self.keys.contains_key(key));
        unknown && self.saturated(now_ms)
    }

    /// Counts a wrong (or missing) password.
    pub fn failed(&mut self, player_id: [u8; 16], source: u64, now_ms: u64) {
        for key in [GuessKey::Player(player_id), GuessKey::Source(source)] {
            self.make_room(key, now_ms);
            if !self.keys.contains_key(&key) && self.keys.len() >= MAX_GUESS_KEYS {
                continue;
            }
            let guesses = self.keys.entry(key).or_default();
            guesses.last_ms = now_ms;
            while guesses.fails.front().is_some_and(|first| now_ms.saturating_sub(*first) >= GUESS_WINDOW_MS) {
                guesses.fails.pop_front();
            }
            guesses.fails.push_back(now_ms);
            if guesses.fails.len() < GUESS_LIMIT {
                continue;
            }
            let lock = guesses.next_lock_ms.max(LOCK_MS);
            guesses.locked_until = now_ms.saturating_add(lock);
            guesses.next_lock_ms = lock.saturating_mul(2).min(MAX_LOCK_MS);
            guesses.fails.clear();
        }
    }

    /// A right password clears the player's count (not the source's: other players may share the address).
    pub fn passed(&mut self, player_id: [u8; 16]) {
        self.keys.remove(&GuessKey::Player(player_id));
    }

    /// Whether the table is full and every key in it is refused.
    fn saturated(&self, now_ms: u64) -> bool {
        self.keys.len() >= MAX_GUESS_KEYS && self.keys.values().all(|guesses| now_ms < guesses.locked_until)
    }

    /// Before adding `key` to a full table: forget the unlocked key heard from longest ago.
    fn make_room(&mut self, key: GuessKey, now_ms: u64) {
        if self.keys.contains_key(&key) || self.keys.len() < MAX_GUESS_KEYS {
            return;
        }
        let oldest = self
            .keys
            .iter()
            .filter(|(_, guesses)| now_ms >= guesses.locked_until)
            .min_by_key(|(_, guesses)| guesses.last_ms)
            .map(|(key, _)| *key);
        if let Some(oldest) = oldest {
            self.keys.remove(&oldest);
        }
    }
}
