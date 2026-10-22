//! The late-join session data (MP-53): what a joiner of a running session needs before its first full snapshot, as one
//! bulk blob of kind `SessionData`. No world state is in it: traffic and police come in the snapshot stream.

use crate::loadout::Loadout;
use crate::message::macros::wire_struct;
use crate::message::{DecodeError, List, LobbyState, MAX_SLOTS, Reader, Wire, Writer, is_slot};

/// Most bytes of a session data blob a client accepts (the receiver's [`super::BulkReceiver`] limit). A full lobby with
/// every loadout at its part limit stays under it.
pub const SESSION_DATA_LIMIT: u32 = 64 * 1024;

wire_struct! {
    /// One member's car.
    pub struct LoadoutRow {
        pub slot: u8,
        pub loadout: Loadout,
    }
    check(row) { is_slot(row.slot).then_some(()).ok_or(DecodeError::BadValue) }
}

wire_struct! {
    /// The session data: the lobby (roster, settings, phase, host) and every member's loadout, so the joiner can build
    /// the other cars from ids.
    pub struct SessionData {
        pub lobby: LobbyState,
        pub loadouts: List<LoadoutRow, MAX_SLOTS>,
    }
}

impl SessionData {
    /// The encoded blob for a [`super::BulkSender`].
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::new();
        self.put(&mut Writer::new(&mut out));
        out
    }

    /// The data from a verified blob. Trailing bytes are an error.
    pub fn decode(bytes: &[u8]) -> Result<Self, DecodeError> {
        let mut reader = Reader::new(bytes);
        let data = Self::take(&mut reader)?;
        if !reader.is_empty() {
            return Err(DecodeError::TrailingBytes);
        }
        Ok(data)
    }
}
