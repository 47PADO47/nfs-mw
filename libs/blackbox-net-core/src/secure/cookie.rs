//! The stateless cookie (security.md 4.4, security-ops.md 2.4). A server answers `Hello` with a MAC over the client's
//! address, its nonce and both connection ids in the current 15-second bucket, and keeps nothing. A `HelloCookie`
//! that echoes a valid cookie proves the sender receives at its address; only then is any state or cryptography spent.
//!
//! The MAC is keyed BLAKE2s-256 (the hash Noise already uses here) under a secret that rotates every hour, with the
//! previous one kept, so a cookie is valid for the current and the previous bucket: 15 to 30 seconds.

use std::net::SocketAddr;

use zeroize::Zeroizing;

use super::crypto::{ct_eq, mac32};

/// Length of one cookie time bucket.
pub const BUCKET_MS: u64 = 15_000;
/// How often the cookie secret should rotate.
pub const SECRET_ROTATE_MS: u64 = 3_600_000;

/// What a cookie is bound to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CookieInput<'a> {
    /// The client's source address.
    pub addr: SocketAddr,
    /// The client's nonce from `Hello`.
    pub client_nonce: &'a [u8; 16],
    /// The client's connection id.
    pub client_conn_id: u16,
    /// The connection id the server offered.
    pub server_conn_id: u16,
}

/// The cookie secrets: the current one and the one before it.
pub struct CookieJar {
    current: Zeroizing<[u8; 32]>,
    previous: Option<Zeroizing<[u8; 32]>>,
    rotated_ms: u64,
}

impl CookieJar {
    /// A jar with `secret` as the current secret, rotated at `now_ms`.
    pub fn new(secret: [u8; 32], now_ms: u64) -> Self {
        Self { current: Zeroizing::new(secret), previous: None, rotated_ms: now_ms }
    }

    /// Makes `secret` current and keeps the old current one as the previous secret.
    pub fn rotate(&mut self, secret: [u8; 32], now_ms: u64) {
        let old = std::mem::replace(&mut self.current, Zeroizing::new(secret));
        self.previous = Some(old);
        self.rotated_ms = now_ms;
    }

    /// Whether an hour has passed since the last rotation.
    pub fn rotation_due(&self, now_ms: u64) -> bool {
        now_ms.saturating_sub(self.rotated_ms) >= SECRET_ROTATE_MS
    }

    /// The cookie for `input` at `now_ms`, with the low 8 bits of its bucket.
    pub fn issue(&self, now_ms: u64, input: &CookieInput<'_>) -> (u8, [u8; 32]) {
        let bucket = now_ms / BUCKET_MS;
        (bucket as u8, compute(&self.current, bucket, input))
    }

    /// Whether `cookie` was issued for `input` in the current or the previous bucket (named by `bucket`), under either
    /// secret. Compared in constant time.
    pub fn verify(&self, now_ms: u64, input: &CookieInput<'_>, bucket: u8, cookie: &[u8; 32]) -> bool {
        let now_bucket = now_ms / BUCKET_MS;
        let candidates = [Some(now_bucket), now_bucket.checked_sub(1)];
        let mut valid = false;
        for full in candidates.into_iter().flatten().filter(|full| *full as u8 == bucket) {
            valid |= ct_eq(&compute(&self.current, full, input), cookie);
            let Some(previous) = self.previous.as_ref() else {
                continue;
            };
            valid |= ct_eq(&compute(previous, full, input), cookie);
        }
        valid
    }
}

/// The 18-byte form of an address: the IPv6 (or IPv4-mapped) address and the port.
fn address_bytes(addr: SocketAddr) -> [u8; 18] {
    let ip = match addr {
        SocketAddr::V4(v4) => v4.ip().to_ipv6_mapped(),
        SocketAddr::V6(v6) => *v6.ip(),
    };
    let mut out = [0u8; 18];
    out[..16].copy_from_slice(&ip.octets());
    out[16..].copy_from_slice(&addr.port().to_le_bytes());
    out
}

fn compute(secret: &[u8; 32], bucket: u64, input: &CookieInput<'_>) -> [u8; 32] {
    let addr = address_bytes(input.addr);
    let [c0, c1] = input.client_conn_id.to_le_bytes();
    let [s0, s1] = input.server_conn_id.to_le_bytes();
    let ids = [c0, c1, s0, s1];
    mac32(secret, &[b"cookie/1", &addr, input.client_nonce, &ids, &bucket.to_le_bytes()])
}

impl std::fmt::Debug for CookieJar {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CookieJar").field("rotated_ms", &self.rotated_ms).finish_non_exhaustive()
    }
}
