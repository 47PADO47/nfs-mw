//! The discovery reply datagram (protocol.md section 3, table "Reply").

use super::name::{self, NAME_LEN};
use super::wire::{array, byte, field, is_zero, le_u16, le_u64};
use super::{DATAGRAM_LEN, DiscoveryError, FORMAT, Request};

/// The reply magic.
const MAGIC: &[u8; 4] = b"NMDR";
/// Offset of the name field.
const NAME_AT: usize = 33;

/// A discovery reply, unicast to the sender of the request. Fields follow the offsets in protocol.md section 3.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reply {
    /// The server's protocol major version.
    pub proto_major: u8,
    /// The nonce of the request this answers.
    pub nonce: u64,
    /// The server's protocol minor version.
    pub proto_minor: u8,
    /// The first 8 bytes of the hash of the server's static public key. Only used to match a saved entry.
    pub key_hash: [u8; 8],
    /// The UDP port of the game.
    pub game_port: u16,
    /// Players now in the session.
    pub players: u8,
    /// Player slots.
    pub max_players: u8,
    /// Flag bits, see the `FLAG_*` constants.
    pub flags: u8,
    /// The game mode code (protocol.md section 7.5).
    pub mode: u8,
    /// The first 4 bytes of the data id (protocol.md section 8.3).
    pub data_id: [u8; 4],
    /// The server name. Encoded as UTF-8 cut to 24 bytes on a character boundary.
    pub name: String,
}

impl Reply {
    /// The 64-byte datagram. The name is cut to at most 24 bytes and zero padded.
    pub fn encode(&self) -> [u8; DATAGRAM_LEN] {
        let mut out = [0u8; DATAGRAM_LEN];
        out[0..4].copy_from_slice(MAGIC);
        out[4] = self.proto_major;
        out[5] = FORMAT;
        out[6..14].copy_from_slice(&self.nonce.to_le_bytes());
        out[14] = self.proto_minor;
        out[15..23].copy_from_slice(&self.key_hash);
        out[23..25].copy_from_slice(&self.game_port.to_le_bytes());
        out[25] = self.players;
        out[26] = self.max_players;
        out[27] = self.flags;
        out[28] = self.mode;
        out[29..33].copy_from_slice(&self.data_id);
        out[NAME_AT..NAME_AT + NAME_LEN].copy_from_slice(&name::encode(&self.name));
        out
    }

    /// Reads a reply. Fails when the length is not 64, the magic or format is wrong, a padding byte is not zero, or
    /// the name is not UTF-8 up to its first zero byte.
    pub fn decode(bytes: &[u8]) -> Result<Reply, DiscoveryError> {
        if bytes.len() != DATAGRAM_LEN {
            return Err(DiscoveryError::Length);
        }
        if field(bytes, 0..4)? != MAGIC {
            return Err(DiscoveryError::Magic);
        }
        if byte(bytes, 5)? != FORMAT {
            return Err(DiscoveryError::Format);
        }
        if !is_zero(field(bytes, NAME_AT + NAME_LEN..DATAGRAM_LEN)?) {
            return Err(DiscoveryError::Padding);
        }
        Ok(Reply {
            proto_major: byte(bytes, 4)?,
            nonce: le_u64(bytes, 6)?,
            proto_minor: byte(bytes, 14)?,
            key_hash: array(bytes, 15)?,
            game_port: le_u16(bytes, 23)?,
            players: byte(bytes, 25)?,
            max_players: byte(bytes, 26)?,
            flags: byte(bytes, 27)?,
            mode: byte(bytes, 28)?,
            data_id: array(bytes, 29)?,
            name: name::decode(field(bytes, NAME_AT..NAME_AT + NAME_LEN)?)?,
        })
    }

    /// True when this reply answers `request`: same protocol major version and the same nonce.
    pub fn answers(&self, request: &Request) -> bool {
        self.proto_major == request.proto_major && self.nonce == request.nonce
    }
}
