//! The optional room password (security.md 4.6): SPAKE2 run inside the sealed channel right after the handshake, then
//! a confirmation tag from each side over the PAKE key and the Noise handshake hash. The password never travels and
//! cannot be tested offline from what an observer or a man in the middle sees; a man in the middle who ran two
//! handshakes has two different hashes and cannot relay a valid confirmation.
//!
//! Flow: both sides call [`start`] and send the 33-byte message; both call [`PakeStart::finish`] with the peer's
//! message and send [`PakeConfirm::tag`]; each checks the other's with [`PakeConfirm::verify`]. On a mismatch the host
//! closes with the generic "join refused" and counts a wrong guess ([`crate::ratelimit::FailureRule::PASSWORD`]).
//!
//! The password bytes are the caller's: normalise them (NFKC) first. `spake2` (RustCrypto) has had no independent
//! audit as far as we know, so this is marked experimental, as the design says.

use spake2::{Ed25519Group, Identity, Password, Spake2};

use super::crypto::{ct_eq, mac32};

/// Bytes of a PAKE message.
pub const PAKE_MSG_LEN: usize = 33;
/// Bytes of a confirmation tag.
pub const CONFIRM_LEN: usize = 32;
/// Longest password accepted, in bytes (64 characters of up to 4 bytes).
pub const MAX_PASSWORD_BYTES: usize = 256;

/// Which side of the exchange this is; the two confirmation tags differ by role.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PakeRole {
    /// The host checking the password.
    Host,
    /// The player joining.
    Joiner,
}

impl PakeRole {
    fn label(self) -> &'static [u8] {
        match self {
            Self::Host => b"host",
            Self::Joiner => b"join",
        }
    }

    fn other(self) -> Self {
        match self {
            Self::Host => Self::Joiner,
            Self::Joiner => Self::Host,
        }
    }
}

/// Why the exchange failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PakeError {
    /// The password is empty or longer than [`MAX_PASSWORD_BYTES`].
    BadPassword,
    /// The peer's message has the wrong length or is not a valid point.
    BadMessage,
}

/// One side after sending its message, waiting for the peer's.
pub struct PakeStart {
    state: Spake2<Ed25519Group>,
    role: PakeRole,
    handshake_hash: [u8; 32],
}

/// Starts the exchange for `password`, bound to the Noise `handshake_hash`. Returns the state and the message to send.
pub fn start(
    password: &[u8],
    handshake_hash: &[u8; 32],
    role: PakeRole,
) -> Result<(PakeStart, [u8; PAKE_MSG_LEN]), PakeError> {
    if password.is_empty() || password.len() > MAX_PASSWORD_BYTES {
        return Err(PakeError::BadPassword);
    }
    let (state, message) =
        Spake2::<Ed25519Group>::start_symmetric(&Password::new(password), &Identity::new(handshake_hash));
    let message: [u8; PAKE_MSG_LEN] = message.as_slice().try_into().map_err(|_| PakeError::BadMessage)?;
    Ok((PakeStart { state, role, handshake_hash: *handshake_hash }, message))
}

impl PakeStart {
    /// Finishes with the peer's message and returns the confirmation tags. Any 33 bytes are safe to pass; a bad point
    /// is an error, a wrong password only shows as a tag mismatch.
    pub fn finish(self, peer_message: &[u8]) -> Result<PakeConfirm, PakeError> {
        if peer_message.len() != PAKE_MSG_LEN {
            return Err(PakeError::BadMessage);
        }
        let key = self.state.finish(peer_message).map_err(|_| PakeError::BadMessage)?;
        let key: [u8; 32] = key.as_slice().try_into().map_err(|_| PakeError::BadMessage)?;
        let tag = |role: PakeRole| mac32(&key, &[b"room-password/1", role.label(), &self.handshake_hash]);
        Ok(PakeConfirm { mine: tag(self.role), theirs: tag(self.role.other()) })
    }
}

/// The two confirmation tags of a finished exchange.
pub struct PakeConfirm {
    mine: [u8; CONFIRM_LEN],
    theirs: [u8; CONFIRM_LEN],
}

impl PakeConfirm {
    /// The tag to send.
    pub fn tag(&self) -> [u8; CONFIRM_LEN] {
        self.mine
    }

    /// Whether the peer's tag proves it used the same password over the same handshake. Constant time.
    pub fn verify(&self, peer_tag: &[u8]) -> bool {
        peer_tag.len() == CONFIRM_LEN && ct_eq(&self.theirs, peer_tag)
    }
}

impl std::fmt::Debug for PakeStart {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PakeStart").field("role", &self.role).finish_non_exhaustive()
    }
}

impl std::fmt::Debug for PakeConfirm {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PakeConfirm").finish_non_exhaustive()
    }
}
