//! The secure channel (security.md section 4, protocol.md sections 4 and 5), behind the `netcode` feature. Sans-IO like
//! [`crate::conn`]: the caller moves datagrams and passes `now_ms` (and, for connect tokens, wall-clock seconds).
//!
//! - [`packet`]: the handshake datagrams (`Hello`, `Challenge`, `HelloCookie`, `NoiseMsg2`, `NoiseMsg3`, `Reject`) and
//!   [`classify`], which sorts any datagram before anything is decoded. Pure functions on `&[u8]`.
//! - [`CookieJar`]: the stateless cookie that proves a sender can receive at its address before any cryptography.
//! - [`IdentityKey`] and [`KeyId`]: the long-term X25519 key, the 16-byte id derived from it, fingerprints.
//! - [`PeerStore`], [`assess`], [`admit`]: trust on first use over a store the caller provides (no file I/O here).
//! - [`HandshakeClient`] and [`HandshakeServer`]: the cookie round trip and the Noise handshake (`XX` on a LAN, `XK`
//!   against a pinned server key), with retries, timeouts, the pre-authentication caps and the 3x amplification limit.
//!   Both end in an [`Established`], whose [`NoiseSealer`] goes into a [`crate::conn::Connection`].
//! - [`NoiseSealer`]: ChaCha20-Poly1305 sealing of every data packet under the Noise transport keys, the packet number
//!   as nonce, key phases with Noise `REKEY`, and the old key kept for reordered packets.
//! - [`token`]: connect tokens (online): sealed, short-lived, single-use, bound to the player's key and the server key.
//! - [`password`]: the optional room password, SPAKE2 bound to the Noise handshake hash.
//!
//! **Why the sealer does not use `snow`'s transport state.** `snow` 0.10's stateless transport mode takes no associated
//! data, encrypts out of place and cannot be cloned, but protocol.md 5.1 authenticates the clear header and the key
//! phase needs the old and the next key side by side. So `snow` runs the handshake and hands over the two transport keys
//! (`dangerously_get_raw_split`, feature `risky-raw-split`); sealing uses the same cipher (`chacha20poly1305`) with the
//! same nonce layout as Noise's `ChaChaPoly` (32 zero bits, then the 64-bit counter little-endian), and rekeying is
//! Noise's `REKEY` (encrypt 32 zero bytes at nonce 2^64 - 1, keep the first 32 bytes).
//!
//! The listen host's own client does none of this: it uses the in-memory link and [`crate::conn::Plain`] (ADR 0009
//! section 3).

mod client;
mod cookie;
mod crypto;
mod error;
mod identity;
mod noise;
pub mod packet;
pub mod password;
mod sealer;
mod server;
#[cfg(test)]
mod tests;
pub mod token;
mod trust;

pub use client::{ClientConfig, ClientStatus, HandshakeClient};
pub use cookie::{BUCKET_MS, CookieJar, SECRET_ROTATE_MS};
pub use error::HandshakeError;
pub use identity::{IdentityKey, KEY_LEN, KeyId};
pub use noise::Pattern;
pub use packet::{PacketKind, classify};
pub use sealer::{NoiseSealer, SealConfig, SealStats, TAG_LEN};
pub use server::{HandshakeServer, ServerAction, ServerConfig, ServerStats};
pub use trust::{Admission, JoinMode, MemoryPeerStore, PeerStore, Trust, admit, assess};

use std::net::SocketAddr;

use crate::conn::{ConnConfig, Connection};

/// A half-open handshake is freed after this long (security-ops.md 2.1).
pub const HANDSHAKE_TIMEOUT_MS: u64 = 5_000;
/// Until the handshake completes, a server sends an address at most this many times the bytes it received from it.
pub const AMPLIFICATION_FACTOR: usize = 3;

/// What both sides know about the other once the handshake is done.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PeerInfo {
    /// The address the handshake ran with.
    pub addr: SocketAddr,
    /// The connection id this side answers to.
    pub local_conn_id: u16,
    /// The connection id the peer answers to.
    pub remote_conn_id: u16,
    /// The peer's static public key, authenticated by the handshake.
    pub remote_static: [u8; KEY_LEN],
    /// The Noise handshake hash `h`: unique to this handshake, the channel binding of the room password.
    pub handshake_hash: [u8; 32],
}

impl PeerInfo {
    /// The peer's id (security.md 4.3).
    pub fn remote_id(&self) -> KeyId {
        KeyId::of(&self.remote_static)
    }
}

/// A finished handshake: who the peer is, and the sealer for the connection.
#[derive(Debug)]
pub struct Established {
    /// The peer.
    pub info: PeerInfo,
    /// Seals and opens the connection's datagrams.
    pub sealer: NoiseSealer,
}

impl Established {
    /// Builds the sealed connection. The ids come from the handshake; the timers from `timers`.
    pub fn into_connection(self, timers: ConnConfig) -> (Connection<NoiseSealer>, PeerInfo) {
        let config = ConnConfig { local_id: self.info.local_conn_id, remote_id: self.info.remote_conn_id, ..timers };
        (Connection::with_sealer(config, self.sealer), self.info)
    }
}
