//! Small primitives shared by the secure modules: a keyed MAC, a constant-time compare, system randomness and the
//! Noise ChaChaPoly nonce.

use blake2::Blake2sMac256;
use blake2::digest::generic_array::GenericArray;
use blake2::digest::{KeyInit, Mac};
use subtle::ConstantTimeEq;

use super::HandshakeError;

/// BLAKE2s-256 in keyed mode over the concatenation of `parts`.
pub fn mac32(key: &[u8; 32], parts: &[&[u8]]) -> [u8; 32] {
    let mut mac = <Blake2sMac256 as KeyInit>::new(GenericArray::from_slice(key));
    for part in parts {
        Mac::update(&mut mac, part);
    }
    mac.finalize().into_bytes().into()
}

/// Compares two byte strings in time that does not depend on where they differ.
pub fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    a.ct_eq(b).into()
}

/// `N` bytes from the operating system's random source.
pub fn random<const N: usize>() -> Result<[u8; N], HandshakeError> {
    let mut bytes = [0u8; N];
    getrandom::fill(&mut bytes).map_err(|_| HandshakeError::Random)?;
    Ok(bytes)
}

/// The 12-byte ChaCha20-Poly1305 nonce of Noise's `ChaChaPoly`: four zero bytes, then the counter little-endian.
pub fn noise_nonce(counter: u64) -> [u8; 12] {
    let mut nonce = [0u8; 12];
    let (_, tail) = nonce.split_at_mut(4);
    tail.copy_from_slice(&counter.to_le_bytes());
    nonce
}
