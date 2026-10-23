//! Identity keys (security.md 4.3): one X25519 key pair per install, the Noise static key. The public half is the
//! identity; [`KeyId`], the first 16 bytes of its SHA-256, is the player id that bans, pins and friends use. Storing the
//! secret (`identity.key`, owner-only) is the caller's job; this module never touches a file.

use std::fmt;

use sha2::{Digest, Sha256};
use snow::params::DHChoice;
use snow::resolvers::{CryptoResolver, DefaultResolver};
use zeroize::Zeroizing;

use super::HandshakeError;
use super::crypto::random;

/// Bytes of an X25519 key, secret or public.
pub const KEY_LEN: usize = 32;

/// A long-term identity key pair. The secret is wiped from memory when the key is dropped and never printed.
#[derive(Clone)]
pub struct IdentityKey {
    secret: Zeroizing<[u8; KEY_LEN]>,
    public: [u8; KEY_LEN],
}

impl IdentityKey {
    /// A new key pair from the operating system's random source.
    pub fn generate() -> Result<Self, HandshakeError> {
        let secret = Zeroizing::new(random::<KEY_LEN>()?);
        Self::from_secret(&secret)
    }

    /// The key pair of a stored secret (any 32 bytes; X25519 clamps them).
    pub fn from_secret(secret: &[u8; KEY_LEN]) -> Result<Self, HandshakeError> {
        let mut dh = DefaultResolver.resolve_dh(&DHChoice::Curve25519).ok_or(HandshakeError::Config)?;
        dh.set(secret);
        let public: [u8; KEY_LEN] = dh.pubkey().try_into().map_err(|_| HandshakeError::Config)?;
        Ok(Self { secret: Zeroizing::new(*secret), public })
    }

    /// A copy of the secret, for the caller to store. Wiped when dropped.
    pub fn secret_bytes(&self) -> Zeroizing<[u8; KEY_LEN]> {
        Zeroizing::new(*self.secret)
    }

    /// The public key: the identity.
    pub fn public(&self) -> &[u8; KEY_LEN] {
        &self.public
    }

    /// The id of this key.
    pub fn key_id(&self) -> KeyId {
        KeyId::of(&self.public)
    }
}

impl fmt::Debug for IdentityKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("IdentityKey").field("id", &self.key_id()).finish_non_exhaustive()
    }
}

/// The id of a public key: the first 16 bytes of its SHA-256. The display name is a label; this is the identity.
#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct KeyId(pub [u8; 16]);

impl KeyId {
    /// The id of `public`.
    pub fn of(public: &[u8; KEY_LEN]) -> Self {
        let digest = Sha256::digest(public);
        let mut id = [0u8; 16];
        id.copy_from_slice(&digest[..16]);
        Self(id)
    }

    /// The first 8 bytes: the `key_hash` of a discovery reply, only for matching a saved entry.
    pub fn short(&self) -> [u8; 8] {
        let mut short = [0u8; 8];
        short.copy_from_slice(&self.0[..8]);
        short
    }

    /// The fingerprint to compare aloud: eight groups of four lowercase hexadecimal digits, space separated.
    pub fn fingerprint(&self) -> String {
        let hex: Vec<String> = self.0.chunks(2).map(|pair| pair.iter().map(|b| format!("{b:02x}")).collect()).collect();
        hex.join(" ")
    }

    /// Four 11-bit indices into a 2048-word list, from the first 44 bits, for the four-word phrase of security.md 4.5.
    /// The game supplies the word list.
    pub fn phrase_indices(&self) -> [u16; 4] {
        let mut head = [0u8; 8];
        head.copy_from_slice(&self.0[..8]);
        let bits = u64::from_be_bytes(head);
        [0u32, 1, 2, 3].map(|i| ((bits >> (64 - 11 * (i + 1))) & 0x7FF) as u16)
    }
}

impl fmt::Display for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.fingerprint())
    }
}

impl fmt::Debug for KeyId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "KeyId({})", self.fingerprint())
    }
}
