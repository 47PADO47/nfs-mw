//! Lobby and session messages (protocol.md section 7.2, channel `control`).

use super::codec::{DecodeError, Reader, Writer};
use super::common::{MAX_SLOTS, Mode, Role, SessionPhase, ensure, is_slot};
use super::macros::{wire_enum, wire_struct};
use super::settings::{MAX_TRAFFIC_DENSITY, Settings, SettingsPatch};
use super::text::Name;
use super::wire::{List, Wire};
use crate::loadout::Loadout;

/// Most options of a vote.
pub const MAX_VOTE_OPTIONS: usize = 8;

wire_struct! {
    /// One row of the lobby roster.
    pub struct LobbyPlayer {
        pub slot: u8,
        pub player_id: [u8; 16],
        pub name: Name,
        /// The loadout summary: the car type.
        pub car_type: u32,
        pub ready: bool,
        pub role: Role,
        pub team: u8,
        pub ping_ms: u16,
        /// Game-defined bits (host, muted, loading, ...).
        pub flags: u8,
    }
    check(row) { ensure(is_slot(row.slot)) }
}

wire_struct! {
    /// The whole lobby, sent on join and after a reconnect.
    pub struct LobbyState {
        pub players: List<LobbyPlayer, MAX_SLOTS>,
        pub settings: Settings,
        pub phase: SessionPhase,
        pub host_slot: u8,
    }
    check(state) {
        let unique = state.players.iter().enumerate().all(|(index, row)| {
            !state.players.iter().take(index).any(|earlier| earlier.slot == row.slot)
        });
        ensure(unique && is_slot(state.host_slot))
    }
}

/// One change of a lobby roster.
#[derive(Debug, Clone, PartialEq)]
pub enum LobbyChange {
    Add(LobbyPlayer),
    Update(LobbyPlayer),
    Remove { slot: u8 },
}

impl Wire for LobbyChange {
    fn put(&self, w: &mut Writer<'_>) {
        match self {
            Self::Add(row) => {
                w.varint(0);
                row.put(w);
            }
            Self::Update(row) => {
                w.varint(1);
                row.put(w);
            }
            Self::Remove { slot } => {
                w.varint(2);
                slot.put(w);
            }
        }
    }

    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        match r.varint(2).map_err(|_| DecodeError::BadEnum)? {
            0 => Ok(Self::Add(LobbyPlayer::take(r)?)),
            1 => Ok(Self::Update(LobbyPlayer::take(r)?)),
            _ => {
                let slot = u8::take(r)?;
                ensure(is_slot(slot))?;
                Ok(Self::Remove { slot })
            }
        }
    }
}

wire_struct! {
    /// The rows that changed since the last lobby message.
    pub struct LobbyDelta {
        pub changes: List<LobbyChange, MAX_SLOTS>,
    }
}

wire_struct! {
    pub struct SetReady {
        pub ready: bool,
    }
}

wire_struct! {
    pub struct SelectLoadout {
        pub loadout: Loadout,
    }
}

wire_struct! {
    /// The player asks for a role; the server checks it against the settings.
    pub struct SetRole {
        pub role: Role,
    }
}

wire_struct! {
    /// Host or admin only: change some settings.
    pub struct UpdateSettings {
        pub patch: SettingsPatch,
    }
}

wire_struct! {
    pub struct StartCountdown {
        /// The server tick at which the race starts.
        pub start_tick: u32,
        pub seconds: u8,
    }
}

wire_enum! {
    /// Why a countdown stopped.
    pub enum CancelReason {
        HostCancelled = 0,
        PlayerLeft = 1,
        NotReady = 2,
        LoadTimeout = 3,
    }
}

wire_struct! {
    pub struct CancelCountdown {
        pub reason: CancelReason,
    }
}

wire_struct! {
    /// Load the world for the next event.
    pub struct LoadWorld {
        pub track: u32,
        pub mode: Mode,
        pub laps: u8,
        /// Minutes after midnight.
        pub time_of_day: u16,
        pub weather: u8,
        /// Percent.
        pub traffic_density: u8,
        pub cop_level_cap: u8,
        pub pursuits: bool,
        /// The grid or pad slot the player starts on.
        pub spawn_slot: u8,
    }
    check(load) { ensure(load.traffic_density <= MAX_TRAFFIC_DENSITY && load.time_of_day < 24 * 60) }
}

wire_struct! {
    pub struct LoadComplete {
        pub streaming_ready: bool,
    }
}

wire_struct! {
    /// The owner's input is neutral until `start_tick`.
    pub struct StartAt {
        pub start_tick: u32,
    }
}

wire_enum! {
    /// What a vote decides.
    pub enum VoteKind {
        Kick = 0,
        Restart = 1,
        ChangeMode = 2,
        ChangeTrack = 3,
        SkipResults = 4,
    }
}

wire_struct! {
    /// A player's vote, or the start of one.
    pub struct Vote {
        pub kind: VoteKind,
        /// The option id (a slot, a mode, a track).
        pub option: u32,
    }
}

wire_struct! {
    /// One option of a running vote and its count.
    #[derive(Copy, Eq, Hash)]
    pub struct VoteOption {
        pub id: u32,
        pub count: u8,
    }
}

wire_struct! {
    pub struct VoteState {
        pub kind: VoteKind,
        pub options: List<VoteOption, MAX_VOTE_OPTIONS>,
        pub ends_tick: u32,
    }
}

/// An admin command, checked against the sender's role.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AdminCommand {
    Kick { slot: u8 },
    Ban { slot: u8, minutes: u32 },
    Mute { slot: u8, muted: bool },
    Restart,
    ChangeMode { mode: Mode },
}

impl Wire for AdminCommand {
    fn put(&self, w: &mut Writer<'_>) {
        match self {
            Self::Kick { slot } => {
                w.varint(0);
                slot.put(w);
            }
            Self::Ban { slot, minutes } => {
                w.varint(1);
                slot.put(w);
                minutes.put(w);
            }
            Self::Mute { slot, muted } => {
                w.varint(2);
                slot.put(w);
                muted.put(w);
            }
            Self::Restart => w.varint(3),
            Self::ChangeMode { mode } => {
                w.varint(4);
                mode.put(w);
            }
        }
    }

    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        let tag = r.varint(4).map_err(|_| DecodeError::BadEnum)?;
        let command = match tag {
            0 => Self::Kick { slot: u8::take(r)? },
            1 => Self::Ban { slot: u8::take(r)?, minutes: u32::take(r)? },
            2 => Self::Mute { slot: u8::take(r)?, muted: bool::take(r)? },
            3 => Self::Restart,
            _ => Self::ChangeMode { mode: Mode::take(r)? },
        };
        let slot_ok = match command {
            Self::Kick { slot } | Self::Ban { slot, .. } | Self::Mute { slot, .. } => is_slot(slot),
            Self::Restart | Self::ChangeMode { .. } => true,
        };
        ensure(slot_ok)?;
        Ok(command)
    }
}

wire_struct! {
    pub struct HostCommand {
        pub command: AdminCommand,
    }
}

wire_struct! {
    /// The client's streaming fell behind (or caught up).
    pub struct StreamingStall {
        pub stalled: bool,
    }
}

wire_struct! {
    /// Ask for the last result record again, after a reconnect.
    pub struct ResultRequest {}
}

wire_struct! {
    /// Ask for a reset at `tick`; at most one per 10 s. The answer is a `Respawn` event.
    pub struct ResetRequest {
        pub tick: u32,
    }
}

wire_struct! {
    /// The client could not decode the snapshot of `tick` because its baseline is gone: the server forgets the client's
    /// views (`ClientState::reset`) and sends the next snapshot full. At most one per 500 ms.
    pub struct BaselineLost {
        pub tick: u32,
    }
}
