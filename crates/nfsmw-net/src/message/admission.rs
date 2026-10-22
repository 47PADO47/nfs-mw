//! Admission messages (protocol.md section 7.1, channel `control`): `Auth`, the PAKE exchange, `Join` and its answers.

use super::common::{ensure, is_slot};
use super::macros::{wire_enum, wire_struct};
use super::settings::Settings;
use super::text::Name;
use super::wire::Bytes;
use crate::loadout::Loadout;
use crate::version::{DataId, ModList, ProtoVersion, RejectReason, SimBuild};

/// Client capability bit: the player drives with a clutch pedal.
pub const CAP_CLUTCH: u8 = 1 << 0;
/// Client capability bit: the player drives with a wheel.
pub const CAP_WHEEL: u8 = 1 << 1;
/// Client capability bit: the client only spectates.
pub const CAP_SPECTATE_ONLY: u8 = 1 << 2;
/// Every defined client capability bit; the others are reserved and must be zero.
pub const CAP_MASK: u8 = CAP_CLUTCH | CAP_WHEEL | CAP_SPECTATE_ONLY;

/// Most bytes of a connect token (online only).
pub const MAX_CONNECT_TOKEN: usize = 256;
/// Most bytes of a resume token.
pub const MAX_RESUME_TOKEN: usize = 64;
/// Most bytes of one PAKE message.
pub const MAX_PAKE_MESSAGE: usize = 128;

wire_struct! {
    /// The client's first sealed message: its minor, builds and capabilities, and the optional tokens.
    pub struct Auth {
        pub proto_minor: u16,
        pub sim_build: SimBuild,
        pub data_id: DataId,
        /// [`CAP_CLUTCH`], [`CAP_WHEEL`], [`CAP_SPECTATE_ONLY`].
        pub caps: u8,
        pub connect_token: Option<Bytes<MAX_CONNECT_TOKEN>>,
        /// The `resume_token` of an earlier `JoinAccepted`, to take the same slot back.
        pub resume: Option<Bytes<MAX_RESUME_TOKEN>>,
    }
    check(auth) { ensure(auth.caps & !CAP_MASK == 0) }
}

wire_enum! {
    /// The password-authenticated key exchange in use.
    pub enum PakeScheme {
        Spake2 = 0,
    }
}

wire_struct! {
    /// The server starts the PAKE for a room with a password.
    pub struct PakeStart {
        pub scheme: PakeScheme,
        pub message: Bytes<MAX_PAKE_MESSAGE>,
    }
}

wire_struct! {
    /// One opaque PAKE message, either way.
    pub struct PakeMsg {
        pub message: Bytes<MAX_PAKE_MESSAGE>,
    }
}

wire_struct! {
    /// The confirmation tag over the PAKE key and the Noise handshake hash.
    pub struct PakeConfirm {
        pub tag: [u8; 32],
    }
}

wire_struct! {
    /// The client asks for a seat: its name, its car and its mods.
    pub struct Join {
        pub display_name: Name,
        pub loadout: Loadout,
        pub mod_manifest: ModList,
    }
}

wire_struct! {
    /// The server's capabilities: flag bits (none defined yet) and the fingerprint of its entity schema.
    #[derive(Copy, Eq, Hash, Default)]
    pub struct ServerCaps {
        pub flags: u32,
        /// [`crate::schema::GameSchema::schema_id`] of the server.
        pub schema_id: u64,
    }
}

wire_struct! {
    /// The seat is the client's.
    pub struct JoinAccepted {
        /// The hash of the client's identity key.
        pub player_id: [u8; 16],
        /// `0..32`.
        pub slot: u8,
        /// The entity id of the player's car (16 bits, ADR 0009).
        pub entity_id: u16,
        pub session_seed: u64,
        /// The server tick when the answer was written.
        pub tick: u32,
        /// Snapshots per second, 1 to 60.
        pub snapshot_hz: u8,
        pub settings: Settings,
        /// Half the side of the square the position quantiser covers, in metres (protocol.md `world_bounds`).
        pub world_half_extent: f32,
        pub resume_token: Bytes<MAX_RESUME_TOKEN>,
        pub server_caps: ServerCaps,
        pub proto_minor: u16,
    }
    check(accepted) {
        ensure(is_slot(accepted.slot) && (1..=60).contains(&accepted.snapshot_hz) && accepted.world_half_extent > 0.0)
    }
}

wire_struct! {
    /// The seat is refused, with a reason code and the server's versions so the UI can say which side must update.
    pub struct JoinRejected {
        pub reason: RejectReason,
        /// A reason-specific code, never free text ([`crate::version::Refusal::detail`]).
        pub detail: u16,
        pub server_proto: ProtoVersion,
        pub server_data_id: DataId,
    }
}

wire_enum! {
    /// Why the server removed a player.
    pub enum KickReason {
        /// The host or an admin kicked the player.
        Kicked = 0,
        /// The player is banned for `ban_minutes`.
        Banned = 1,
        /// Too many invalid messages or violations (`security-gameplay.md` section 5.2).
        Violations = 2,
        /// No input for too long.
        Idle = 3,
        /// A vote removed the player.
        Voted = 4,
    }
}

wire_struct! {
    /// The server removes the player.
    pub struct Kick {
        pub reason: KickReason,
        /// 0 for no ban.
        pub ban_minutes: u32,
    }
}
