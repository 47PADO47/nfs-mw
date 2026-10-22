//! Types several messages share: the session phase, the player role, the game mode and the slot bound.

use blackbox_net_core::session::Phase;

use super::codec::{DecodeError, Reader, Writer};
use super::macros::wire_enum;
use super::wire::Wire;

/// Player slots in a session's id space (5 bits): a slot is `0..MAX_SLOTS`.
pub const MAX_SLOTS: usize = 32;

/// Whether `slot` names a slot of the id space.
pub fn is_slot(slot: u8) -> bool {
    usize::from(slot) < MAX_SLOTS
}

/// [`DecodeError::BadValue`] unless `ok`: the shape of every `check` block.
pub fn ensure(ok: bool) -> Result<(), DecodeError> {
    match ok {
        true => Ok(()),
        false => Err(DecodeError::BadValue),
    }
}

wire_enum! {
    /// The session phase on the wire (ADR 0009 section 6). The countdown's start tick travels in its own field.
    pub enum SessionPhase {
        Lobby = 0,
        Loading = 1,
        WaitLoads = 2,
        Intro = 3,
        Countdown = 4,
        Running = 5,
        Results = 6,
    }
}

impl From<Phase> for SessionPhase {
    fn from(phase: Phase) -> Self {
        match phase {
            Phase::Lobby => Self::Lobby,
            Phase::Loading => Self::Loading,
            Phase::WaitLoads => Self::WaitLoads,
            Phase::Intro => Self::Intro,
            Phase::Countdown { .. } => Self::Countdown,
            Phase::Running => Self::Running,
            Phase::Results => Self::Results,
        }
    }
}

wire_enum! {
    /// What a player does in a session.
    pub enum Role {
        Racer = 0,
        Cop = 1,
        Spectator = 2,
    }
}

/// A game mode (protocol.md section 7.5): one byte, `9..=127` reserved, `128..=255` a mod's mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    #[default]
    FreeRoam,
    Circuit,
    Sprint,
    Drag,
    Speedtrap,
    Knockout,
    PursuitCoop,
    CopsVsRacers,
    Tag,
    /// A mod's mode id, `128..=255`.
    Custom(u8),
}

impl Mode {
    /// The mode's byte.
    pub fn code(self) -> u8 {
        match self {
            Self::FreeRoam => 0,
            Self::Circuit => 1,
            Self::Sprint => 2,
            Self::Drag => 3,
            Self::Speedtrap => 4,
            Self::Knockout => 5,
            Self::PursuitCoop => 6,
            Self::CopsVsRacers => 7,
            Self::Tag => 8,
            Self::Custom(id) => id,
        }
    }

    /// The mode with byte `code`, or `None` for a reserved code.
    pub fn from_code(code: u8) -> Option<Self> {
        match code {
            0 => Some(Self::FreeRoam),
            1 => Some(Self::Circuit),
            2 => Some(Self::Sprint),
            3 => Some(Self::Drag),
            4 => Some(Self::Speedtrap),
            5 => Some(Self::Knockout),
            6 => Some(Self::PursuitCoop),
            7 => Some(Self::CopsVsRacers),
            8 => Some(Self::Tag),
            128..=255 => Some(Self::Custom(code)),
            _ => None,
        }
    }
}

impl Wire for Mode {
    fn put(&self, w: &mut Writer<'_>) {
        w.u8(self.code());
    }
    fn take(r: &mut Reader<'_>) -> Result<Self, DecodeError> {
        Self::from_code(r.u8()?).ok_or(DecodeError::BadEnum)
    }
}
