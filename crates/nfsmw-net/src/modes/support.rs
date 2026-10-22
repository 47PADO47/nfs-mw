//! The modes this build does not run, and why (ADR 0011 section 5). The lobby refuses them with the reason code;
//! there are no stand-in rules.

use crate::message::Mode;

/// A mode the lobby refuses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, thiserror::Error)]
pub enum Unsupported {
    /// MP-04: the dragster is not modelled in single player.
    #[error("drag races are not built: the drag race is not modelled in single player")]
    Drag,
    /// MP-08: what a human cop can do is open question 2 of features.md.
    #[error("cops vs racers is not built: the human cop's tools are not decided")]
    CopsVsRacers,
    /// A mod's mode (`128..=255`): needs the mod API (milestone 12).
    #[error("mod mode {0} needs the mod API")]
    ModMode(u8),
}

impl Unsupported {
    /// The reason code a lobby notice carries: 1 drag, 2 cops vs racers, 3 a mod's mode.
    pub fn code(self) -> u8 {
        match self {
            Self::Drag => 1,
            Self::CopsVsRacers => 2,
            Self::ModMode(_) => 3,
        }
    }
}

/// `Ok` when this build runs `mode`, else why not.
pub fn support(mode: Mode) -> Result<(), Unsupported> {
    match mode {
        Mode::Drag => Err(Unsupported::Drag),
        Mode::CopsVsRacers => Err(Unsupported::CopsVsRacers),
        Mode::Custom(id) => Err(Unsupported::ModMode(id)),
        Mode::FreeRoam
        | Mode::Circuit
        | Mode::Sprint
        | Mode::Speedtrap
        | Mode::Knockout
        | Mode::PursuitCoop
        | Mode::Tag => Ok(()),
    }
}
