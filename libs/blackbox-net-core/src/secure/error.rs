use std::fmt;

use super::packet::PacketError;

/// Why a handshake step failed. On the wire every failure is a silent drop; these are for counters and the plain
/// reason the player sees.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeError {
    /// A handshake datagram failed to decode.
    Packet(PacketError),
    /// The Noise step failed: a bad message, a bad key or a version (prologue) mismatch.
    Noise,
    /// The system random source failed.
    Random,
    /// The configuration is unusable (an `XK` client without a server key, a prologue over 255 bytes).
    Config,
    /// The server stopped answering: every retry went out and nothing came back.
    NoAnswer,
    /// The server refused with this `Reject` reason (1 full, 2 protocol major unsupported, 3 shutting down).
    Rejected(u8),
}

impl fmt::Display for HandshakeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Packet(err) => write!(f, "malformed handshake packet: {err}"),
            Self::Noise => f.write_str("handshake failed (key or version mismatch)"),
            Self::Random => f.write_str("the system random source failed"),
            Self::Config => f.write_str("unusable handshake configuration"),
            Self::NoAnswer => f.write_str("no answer"),
            Self::Rejected(1) => f.write_str("host full"),
            Self::Rejected(2) => f.write_str("protocol version not supported by the host"),
            Self::Rejected(3) => f.write_str("host shutting down"),
            Self::Rejected(reason) => write!(f, "refused by the host (reason {reason})"),
        }
    }
}

impl std::error::Error for HandshakeError {}

impl From<PacketError> for HandshakeError {
    fn from(err: PacketError) -> Self {
        Self::Packet(err)
    }
}

impl From<snow::Error> for HandshakeError {
    fn from(_: snow::Error) -> Self {
        Self::Noise
    }
}
