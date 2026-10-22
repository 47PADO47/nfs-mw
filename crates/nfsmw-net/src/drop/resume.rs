//! Resume tokens (MP-51, security.md 4.7): single-use, bound to the player id, valid for 30 s after a drop.
//!
//! A token is [`RESUME_TOKEN_BYTES`] fresh random bytes the caller draws from a CSPRNG (this crate has none and
//! takes no clock or entropy of its own). The book keeps only the SHA-256 of each token, compares hashes in constant
//! time, and forgets a token the moment it is used, so a replay of a used token finds nothing. A token presented with
//! another player id is refused without burning it, so a thief cannot lock the owner out. The transport seals the
//! token under the session key; this module never sees the key.

use std::collections::BTreeMap;

use sha2::{Digest, Sha256};

use crate::message::{Bytes, MAX_RESUME_TOKEN};

/// The window after a drop in which the slot is kept (MP-51).
pub const RESUME_WINDOW_MS: u64 = 30_000;
/// Bytes of a resume token.
pub const RESUME_TOKEN_BYTES: usize = 32;

/// Why a resume was refused. Both mean "make it a normal join"; the client is not told which.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResumeError {
    /// No live token matches: never issued, used already, released, or presented by another player.
    Unknown,
    /// The token matches a slot whose window has passed.
    Expired,
}

/// A successful resume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Resumed {
    pub slot: u8,
    /// The slot's old connection had not timed out yet: the caller closes it before the new one takes over.
    pub was_live: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Entry {
    player_id: [u8; 16],
    /// SHA-256 of the live token, `None` once it was used.
    token: Option<[u8; 32]>,
    /// The end of the window while the slot is held after a drop.
    held_until: Option<u64>,
}

/// The server's resume tokens, one per slot.
#[derive(Debug, Clone, Default)]
pub struct ResumeBook {
    entries: BTreeMap<u8, Entry>,
}

impl ResumeBook {
    /// No tokens.
    pub fn new() -> Self {
        Self::default()
    }

    /// Issues the token `fresh` for `slot` and `player_id` (at join, and again after each resume), replacing any
    /// earlier one, and returns it for `JoinAccepted`.
    pub fn issue(&mut self, slot: u8, player_id: [u8; 16], fresh: [u8; RESUME_TOKEN_BYTES]) -> Bytes<MAX_RESUME_TOKEN> {
        let entry = Entry { player_id, token: Some(hash(&fresh)), held_until: None };
        self.entries.insert(slot, entry);
        // 32 bytes always fit the 64-byte limit.
        Bytes::new(fresh.to_vec()).unwrap_or_default()
    }

    /// `slot` dropped at `now_ms`: hold it for [`RESUME_WINDOW_MS`]. Returns the end of the window, or `None` when the
    /// slot has no token (it never joined, or used its token without a new one).
    pub fn hold(&mut self, slot: u8, now_ms: u64) -> Option<u64> {
        let entry = self.entries.get_mut(&slot).filter(|entry| entry.token.is_some())?;
        let until = now_ms.saturating_add(RESUME_WINDOW_MS);
        entry.held_until = Some(until);
        Some(until)
    }

    /// Whether `slot` is held after a drop.
    pub fn is_held(&self, slot: u8) -> bool {
        self.entries.get(&slot).is_some_and(|entry| entry.held_until.is_some())
    }

    /// Checks `token` from a client that authenticated as `player_id` at `now_ms`, and uses it up. Call
    /// [`ResumeBook::issue`] again for the new `JoinAccepted`.
    pub fn resume(&mut self, token: &[u8], player_id: [u8; 16], now_ms: u64) -> Result<Resumed, ResumeError> {
        if token.len() != RESUME_TOKEN_BYTES {
            return Err(ResumeError::Unknown);
        }
        let presented = hash(token);
        let mut found = None;
        for (slot, entry) in &self.entries {
            let matches = entry.token.is_some_and(|stored| same(&stored, &presented));
            if matches {
                found = Some(*slot);
            }
        }
        let slot = found.ok_or(ResumeError::Unknown)?;
        let entry = self.entries.get_mut(&slot).ok_or(ResumeError::Unknown)?;
        if !same16(&entry.player_id, &player_id) {
            return Err(ResumeError::Unknown);
        }
        if entry.held_until.is_some_and(|until| now_ms >= until) {
            return Err(ResumeError::Expired);
        }
        let was_live = entry.held_until.is_none();
        entry.token = None;
        entry.held_until = None;
        Ok(Resumed { slot, was_live })
    }

    /// The held slots whose window ended by `now_ms`, in slot order; their tokens are forgotten. The caller releases
    /// each slot (see [`super::on_expiry`]).
    pub fn take_expired(&mut self, now_ms: u64) -> Vec<u8> {
        let ended: Vec<u8> = self
            .entries
            .iter()
            .filter(|(_, entry)| entry.held_until.is_some_and(|until| now_ms >= until))
            .map(|(slot, _)| *slot)
            .collect();
        for slot in &ended {
            self.entries.remove(slot);
        }
        ended
    }

    /// Forgets `slot`'s token (the player quit, was kicked, or the slot was released).
    pub fn release(&mut self, slot: u8) {
        self.entries.remove(&slot);
    }
}

fn hash(bytes: &[u8]) -> [u8; 32] {
    let mut out = [0u8; 32];
    out.copy_from_slice(&Sha256::digest(bytes));
    out
}

/// Constant-time equality of two hashes.
fn same(a: &[u8; 32], b: &[u8; 32]) -> bool {
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}

fn same16(a: &[u8; 16], b: &[u8; 16]) -> bool {
    a.iter().zip(b).fold(0u8, |acc, (x, y)| acc | (x ^ y)) == 0
}
