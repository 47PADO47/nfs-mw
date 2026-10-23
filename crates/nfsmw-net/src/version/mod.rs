//! Protocol versions, the game-data id, the sim build id, the mod list, and the compatibility decision a server makes at
//! join (features.md MP-60 to MP-64, protocol.md section 8).
//!
//! - [`PROTO_MAJOR`] and [`PROTO_MINOR`]: this build's protocol. A major mismatch never connects (the Noise prologue
//!   [`noise_prologue`] differs); a minor mismatch connects at the lower minor ([`negotiate_minor`], [`Feature`]).
//! - [`DataId`]: a 16-byte hash of the identified `speed.exe` and the required file list; [`DataId::short`] is the
//!   4-byte prefix the discovery reply carries.
//! - [`SimBuild`]: 8 bytes naming the simulation build (git hash and target); informational unless the server requires
//!   a match.
//! - [`ModEntry`], [`ModPolicy`] and [`compare_mods`]: the sim-affecting mod list and the off, lenient and strict
//!   comparison of `security-gameplay.md` section 7.3. Client-only mods are never compared.
//! - [`check_join`]: the server's decision for one `Auth` plus `Join`, returning the negotiated minor or a
//!   [`Refusal`] with a [`RejectReason`] and a detail code. [`classify_server`] is the client's label for a discovery
//!   reply.

mod compat;
mod ids;
mod mods;

#[cfg(test)]
mod tests;

pub use compat::{
    Admitted, BuildInfo, ClientOffer, DETAIL_CLIENT_MUST_UPDATE, DETAIL_SERVER_MUST_UPDATE, Feature, Refusal,
    ServerLabel, ServerPolicy, check_join, classify_server, negotiate_minor,
};
pub use ids::{DataId, ProtoVersion, SimBuild};
pub use mods::{MAX_MODS, ModEntry, ModKind, ModList, ModMismatch, ModPolicy, compare_mods, is_mod_id};

use crate::message::macros::wire_enum;

/// The protocol major version: bumped for any incompatible change of a header, a known frame or a required message.
pub const PROTO_MAJOR: u8 = 1;
/// The protocol minor version: bumped for additive changes. Both sides use the lower value.
pub const PROTO_MINOR: u16 = 0;

/// The Noise prologue of protocol major `major`: `nfsmw-mp/<major>`, so a major mismatch fails the handshake.
pub fn noise_prologue(major: u8) -> String {
    format!("nfsmw-mp/{major}")
}

wire_enum! {
    /// Why a server refuses a join (`JoinRejected`, protocol.md section 7.1). Append-only.
    pub enum RejectReason {
        ServerFull = 0,
        /// The protocol versions are incompatible; the detail says which side must update.
        ProtocolMismatch = 1,
        /// The game data differs (MP-61).
        DataMismatch = 2,
        /// The sim-affecting mods differ (MP-63); the detail is the number of differences, at most 255.
        ModMismatch = 3,
        Banned = 4,
        /// The session is between phases that refuse joins.
        InProgress = 5,
        /// The loadout failed validation; the detail is the [`crate::loadout::LoadoutReject`] code.
        LoadoutRejected = 6,
        RateLimited = 7,
        ServerShuttingDown = 8,
        /// The server requires the same sim build and the client's differs (MP-62).
        SimBuildMismatch = 9,
    }
}

/// The `reason` byte of the handshake `Reject` packet (protocol.md section 4), sent only after a valid cookie.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum HandshakeReject {
    ServerFull = 1,
    /// The client's protocol major is not supported.
    ProtocolMajor = 2,
    ShuttingDown = 3,
}

impl HandshakeReject {
    /// The byte for `code`, or `None` for an unknown code.
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::ServerFull),
            2 => Some(Self::ProtocolMajor),
            3 => Some(Self::ShuttingDown),
            _ => None,
        }
    }
}
