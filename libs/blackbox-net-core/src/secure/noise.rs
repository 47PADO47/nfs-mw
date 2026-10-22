//! Building the `snow` handshake states (security.md 4.2): `Noise_XX_25519_ChaChaPoly_BLAKE2s` on a LAN, the `XK`
//! variant online, the caller's prologue mixed in on both sides, no pre-shared key and no early data.

use snow::params::NoiseParams;
use snow::{Builder, HandshakeState};

use super::{HandshakeError, IdentityKey};

/// The Noise pattern of a handshake.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pattern {
    /// Neither side knows the other's key in advance; trust is decided afterwards (LAN, trust on first use).
    Xx = 0,
    /// The client knows the server's key in advance (online, from the connect token).
    Xk = 1,
}

impl Pattern {
    /// The pattern of a `HelloCookie` pattern byte.
    pub fn from_u8(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Xx),
            1 => Some(Self::Xk),
            _ => None,
        }
    }

    fn params(self) -> Result<NoiseParams, HandshakeError> {
        let name = match self {
            Self::Xx => "Noise_XX_25519_ChaChaPoly_BLAKE2s",
            Self::Xk => "Noise_XK_25519_ChaChaPoly_BLAKE2s",
        };
        name.parse().map_err(|_| HandshakeError::Config)
    }
}

/// The initiator (client) side. `server_key` is required for `XK` and ignored for `XX`.
pub fn initiator(
    pattern: Pattern,
    prologue: &[u8],
    identity: &IdentityKey,
    server_key: Option<&[u8; 32]>,
) -> Result<HandshakeState, HandshakeError> {
    let secret = identity.secret_bytes();
    let builder = Builder::new(pattern.params()?).local_private_key(&secret[..])?.prologue(prologue)?;
    if pattern == Pattern::Xx {
        return Ok(builder.build_initiator()?);
    }
    let Some(key) = server_key else {
        return Err(HandshakeError::Config);
    };
    Ok(builder.remote_public_key(key)?.build_initiator()?)
}

/// The responder (server) side.
pub fn responder(pattern: Pattern, prologue: &[u8], identity: &IdentityKey) -> Result<HandshakeState, HandshakeError> {
    let secret = identity.secret_bytes();
    Ok(Builder::new(pattern.params()?).local_private_key(&secret[..])?.prologue(prologue)?.build_responder()?)
}

/// What a finished handshake hands over: the two transport keys (initiator to responder, responder to initiator), the
/// peer's static key and the handshake hash.
pub struct Finished {
    pub initiator_to_responder: [u8; 32],
    pub responder_to_initiator: [u8; 32],
    pub remote_static: [u8; 32],
    pub handshake_hash: [u8; 32],
}

/// Takes the keys out of a finished handshake. `Noise` if it is not finished or the peer's key is missing.
pub fn finish(state: &mut HandshakeState) -> Result<Finished, HandshakeError> {
    if !state.is_handshake_finished() {
        return Err(HandshakeError::Noise);
    }
    let remote_static: [u8; 32] =
        state.get_remote_static().and_then(|key| key.try_into().ok()).ok_or(HandshakeError::Noise)?;
    let handshake_hash: [u8; 32] = state.get_handshake_hash().try_into().map_err(|_| HandshakeError::Noise)?;
    let (initiator_to_responder, responder_to_initiator) = state.dangerously_get_raw_split();
    Ok(Finished { initiator_to_responder, responder_to_initiator, remote_static, handshake_hash })
}
