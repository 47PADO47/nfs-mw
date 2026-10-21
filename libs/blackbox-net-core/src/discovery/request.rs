//! The discovery request datagram (protocol.md section 3, table "Request").

use super::wire::{byte, field, is_zero, le_u64};
use super::{DATAGRAM_LEN, DiscoveryError, FORMAT};

/// The request magic.
const MAGIC: &[u8; 4] = b"NMDQ";

/// A discovery request, sent to the subnet broadcast address and to the recent-servers list.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    /// The protocol major version the client speaks.
    pub proto_major: u8,
    /// Random per request. A reply must echo it to be accepted.
    pub nonce: u64,
}

impl Request {
    /// The 64-byte datagram: magic, major, format, nonce, then 50 zero bytes.
    pub fn encode(&self) -> [u8; DATAGRAM_LEN] {
        let mut out = [0u8; DATAGRAM_LEN];
        out[0..4].copy_from_slice(MAGIC);
        out[4] = self.proto_major;
        out[5] = FORMAT;
        out[6..14].copy_from_slice(&self.nonce.to_le_bytes());
        out
    }

    /// Reads a request. Fails when the length is not 64, the magic or format is wrong, or the padding is not zero.
    pub fn decode(bytes: &[u8]) -> Result<Request, DiscoveryError> {
        if bytes.len() != DATAGRAM_LEN {
            return Err(DiscoveryError::Length);
        }
        if field(bytes, 0..4)? != MAGIC {
            return Err(DiscoveryError::Magic);
        }
        if byte(bytes, 5)? != FORMAT {
            return Err(DiscoveryError::Format);
        }
        if !is_zero(field(bytes, 14..DATAGRAM_LEN)?) {
            return Err(DiscoveryError::Padding);
        }
        Ok(Request { proto_major: byte(bytes, 4)?, nonce: le_u64(bytes, 6)? })
    }
}
