//! Refused discovery datagrams.

use std::fmt;

/// Why a discovery datagram was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiscoveryError {
    /// The datagram is not [`super::DATAGRAM_LEN`] bytes long.
    Length,
    /// The magic is not the one for this datagram (`NMDQ` or `NMDR`).
    Magic,
    /// The format version is not [`super::FORMAT`].
    Format,
    /// A padding byte is not zero.
    Padding,
    /// The name is not valid UTF-8 up to its first zero byte.
    Name,
}

impl fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::Length => "discovery datagram is not 64 bytes",
            Self::Magic => "discovery datagram has the wrong magic",
            Self::Format => "discovery datagram has an unknown format version",
            Self::Padding => "discovery datagram has nonzero padding",
            Self::Name => "discovery reply name is not valid UTF-8",
        };
        f.write_str(text)
    }
}

impl std::error::Error for DiscoveryError {}
