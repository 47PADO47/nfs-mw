//! Sealing of data packets under the Noise transport keys (security.md 4.4, protocol.md 5.1).
//!
//! - **Cipher**: ChaCha20-Poly1305, one key per direction. The 7 clear header bytes are the associated data.
//! - **Nonce**: the packet number expanded to 64 bits (as QUIC reconstructs packet numbers: the candidate nearest to
//!   the next expected number), in Noise's `ChaChaPoly` nonce layout. A sender refuses to seal a number that is not
//!   above the last one it sealed, so a nonce is never reused under one key.
//! - **Key phase** (flag bits 6 and 5): the sender moves to the next phase after [`SealConfig::rekey_packets`] packets
//!   or [`SealConfig::rekey_ms`], deriving the new key with Noise `REKEY`. A receiver that sees the next phase derives
//!   the next key, and keeps the old one for [`SealConfig::old_key_ms`] for reordered packets, then erases it.
//! - **Failure**: a bad tag or an unknown phase changes nothing and is counted; the caller drops the packet silently.

use chacha20poly1305::aead::{AeadInPlace, KeyInit};
use chacha20poly1305::{ChaCha20Poly1305, Key, Nonce, Tag};
use zeroize::Zeroizing;

use super::crypto::noise_nonce;
use crate::conn::{CLEAR_HEADER_LEN, ConnError, KEY_PHASE_MASK, Sealer};

/// Bytes of the authentication tag.
pub const TAG_LEN: usize = 16;

/// When to rekey and how long to keep an old key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SealConfig {
    /// Packets sealed under one key before the next phase (2^24).
    pub rekey_packets: u64,
    /// Time under one key before the next phase (one hour).
    pub rekey_ms: u64,
    /// How long a receiver keeps the previous key for reordered packets (3 s).
    pub old_key_ms: u64,
}

impl Default for SealConfig {
    fn default() -> Self {
        Self { rekey_packets: 1 << 24, rekey_ms: 3_600_000, old_key_ms: 3_000 }
    }
}

/// Counters of one sealer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SealStats {
    /// Packets sealed.
    pub sealed: u64,
    /// Packets opened.
    pub opened: u64,
    /// Packets whose tag did not verify.
    pub bad_tags: u64,
    /// Packets whose key phase named no key held (two phases ahead, or an old key already erased).
    pub unknown_phase: u64,
    /// Times this side moved its sending key to the next phase.
    pub rekeys_sent: u64,
    /// Times this side followed the peer to its next phase.
    pub rekeys_received: u64,
}

type SecretKey = Zeroizing<[u8; 32]>;

/// The [`Sealer`] of a connection that finished a Noise handshake.
pub struct NoiseSealer {
    config: SealConfig,
    send_key: SecretKey,
    send_phase: u8,
    send_in_phase: u64,
    send_phase_start: Option<u64>,
    last_sent: Option<u64>,
    recv_key: SecretKey,
    recv_phase: u8,
    old_recv: Option<(SecretKey, u64)>,
    recv_highest: Option<u64>,
    stats: SealStats,
}

impl NoiseSealer {
    /// A sealer that sends under `send_key` and receives under `recv_key`, both in phase 0.
    pub fn new(send_key: [u8; 32], recv_key: [u8; 32], config: SealConfig) -> Self {
        Self {
            config,
            send_key: Zeroizing::new(send_key),
            send_phase: 0,
            send_in_phase: 0,
            send_phase_start: None,
            last_sent: None,
            recv_key: Zeroizing::new(recv_key),
            recv_phase: 0,
            old_recv: None,
            recv_highest: None,
            stats: SealStats::default(),
        }
    }

    /// The counters.
    pub fn stats(&self) -> SealStats {
        self.stats
    }

    /// The key phase packets are sent in (0 to 3).
    pub fn send_phase(&self) -> u8 {
        self.send_phase
    }

    /// The key phase packets are received in (0 to 3).
    pub fn recv_phase(&self) -> u8 {
        self.recv_phase
    }

    /// Moves the sending key to the next phase now, whatever the counters say.
    pub fn rekey_now(&mut self, now_ms: u64) {
        self.send_key = rekey(&self.send_key);
        self.send_phase = (self.send_phase + 1) & 3;
        self.send_in_phase = 0;
        self.send_phase_start = Some(now_ms);
        self.stats.rekeys_sent += 1;
    }

    fn rekey_if_due(&mut self, now_ms: u64) {
        let start = *self.send_phase_start.get_or_insert(now_ms);
        let by_count = self.send_in_phase >= self.config.rekey_packets;
        let by_time = now_ms.saturating_sub(start) >= self.config.rekey_ms;
        if by_count || by_time {
            self.rekey_now(now_ms);
        }
    }

    fn erase_expired(&mut self, now_ms: u64) {
        if self.old_recv.as_ref().is_some_and(|(_, until)| now_ms >= *until) {
            self.old_recv = None;
        }
    }
}

impl Sealer for NoiseSealer {
    fn overhead(&self) -> usize {
        TAG_LEN
    }

    fn seal(&mut self, now_ms: u64, packet: &mut [u8], len: usize) -> Result<usize, ConnError> {
        if len < CLEAR_HEADER_LEN || packet.len() < len + TAG_LEN {
            return Err(ConnError::SealFailed);
        }
        let truncated = number_of(packet).ok_or(ConnError::SealFailed)?;
        let full = expand(self.last_sent.map_or(0, |last| last + 1), truncated);
        if self.last_sent.is_some_and(|last| full <= last) {
            return Err(ConnError::SealFailed);
        }
        self.rekey_if_due(now_ms);
        let Some(first) = packet.first_mut() else {
            return Err(ConnError::SealFailed);
        };
        *first = (*first & !KEY_PHASE_MASK) | (self.send_phase << 5);
        let (head, rest) = packet.split_at_mut(CLEAR_HEADER_LEN);
        let (body, tail) = rest.split_at_mut(len - CLEAR_HEADER_LEN);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&self.send_key[..]));
        let nonce = noise_nonce(full);
        let tag = cipher
            .encrypt_in_place_detached(Nonce::from_slice(&nonce), head, body)
            .map_err(|_| ConnError::SealFailed)?;
        let Some(tag_slot) = tail.get_mut(..TAG_LEN) else {
            return Err(ConnError::SealFailed);
        };
        tag_slot.copy_from_slice(&tag);
        self.last_sent = Some(full);
        self.send_in_phase += 1;
        self.stats.sealed += 1;
        Ok(len + TAG_LEN)
    }

    fn open(&mut self, now_ms: u64, datagram: &mut [u8]) -> Result<usize, ConnError> {
        self.erase_expired(now_ms);
        let len = datagram.len();
        if len < CLEAR_HEADER_LEN + TAG_LEN {
            return Err(ConnError::Truncated);
        }
        let truncated = number_of(datagram).ok_or(ConnError::Truncated)?;
        let phase = datagram.first().map_or(0, |first| (first & KEY_PHASE_MASK) >> 5);
        let full = expand(self.recv_highest.map_or(0, |highest| highest + 1), truncated);

        let next_phase = (self.recv_phase + 1) & 3;
        let previous_phase = (self.recv_phase + 3) & 3;
        let next_key = (phase == next_phase).then(|| rekey(&self.recv_key));
        let key: &[u8; 32] = match (phase, next_key.as_ref(), self.old_recv.as_ref()) {
            (p, _, _) if p == self.recv_phase => &self.recv_key,
            (_, Some(next), _) => next,
            (p, _, Some((old, _))) if p == previous_phase => old,
            _ => {
                self.stats.unknown_phase += 1;
                return Err(ConnError::BadTag);
            }
        };

        let body_len = len - CLEAR_HEADER_LEN - TAG_LEN;
        let (head, rest) = datagram.split_at_mut(CLEAR_HEADER_LEN);
        let (body, tag) = rest.split_at_mut(body_len);
        let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));
        let nonce = noise_nonce(full);
        let opened = cipher.decrypt_in_place_detached(Nonce::from_slice(&nonce), head, body, Tag::from_slice(tag));
        if opened.is_err() {
            self.stats.bad_tags += 1;
            return Err(ConnError::BadTag);
        }

        if let Some(next) = next_key {
            let old = std::mem::replace(&mut self.recv_key, next);
            self.old_recv = Some((old, now_ms.saturating_add(self.config.old_key_ms)));
            self.recv_phase = phase;
            self.stats.rekeys_received += 1;
        }
        if self.recv_highest.is_none_or(|highest| full > highest) {
            self.recv_highest = Some(full);
        }
        if let Some(first) = datagram.first_mut() {
            *first &= !KEY_PHASE_MASK;
        }
        self.stats.opened += 1;
        Ok(len - TAG_LEN)
    }
}

impl std::fmt::Debug for NoiseSealer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NoiseSealer")
            .field("send_phase", &self.send_phase)
            .field("recv_phase", &self.recv_phase)
            .field("stats", &self.stats)
            .finish_non_exhaustive()
    }
}

/// The low 32 bits of the packet number from the clear header.
fn number_of(packet: &[u8]) -> Option<u32> {
    let bytes: [u8; 4] = packet.get(3..7)?.try_into().ok()?;
    Some(u32::from_le_bytes(bytes))
}

/// The 64-bit packet number nearest to `expected` whose low 32 bits are `truncated` (RFC 9000 appendix A.3).
pub(crate) fn expand(expected: u64, truncated: u32) -> u64 {
    const WIN: u64 = 1 << 32;
    const HALF: u64 = WIN / 2;
    let candidate = (expected & !(WIN - 1)) | u64::from(truncated);
    if candidate.saturating_add(HALF) <= expected && candidate < u64::MAX - WIN {
        return candidate + WIN;
    }
    if candidate > expected.saturating_add(HALF) && candidate >= WIN {
        return candidate - WIN;
    }
    candidate
}

/// Noise `REKEY(k)`: the first 32 bytes of encrypting 32 zero bytes under `k` at nonce 2^64 - 1 with no associated
/// data.
fn rekey(key: &[u8; 32]) -> SecretKey {
    let cipher = ChaCha20Poly1305::new(Key::from_slice(&key[..]));
    let nonce = noise_nonce(u64::MAX);
    let mut next = Zeroizing::new([0u8; 32]);
    // Encrypting 32 bytes cannot fail (the only error is a buffer over the cipher's 256 GiB limit).
    let _tag = cipher.encrypt_in_place_detached(Nonce::from_slice(&nonce), &[], &mut next[..]);
    next
}
