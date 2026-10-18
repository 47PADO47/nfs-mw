//! The four milestone states and the definition a milestone is built from (`docs/specs/career-milestones.md`
//! sections 2.3, 3.2 and 8).
//!
//! The definition is plain data. The caller reads the 55 milestones from the install and passes them in; this module
//! does not depend on `nfsmw-data`. The original keeps a second and third number per milestone (a "goal" and a
//! "step" ladder). No code of the PC exe reads them, so only the threshold is modelled **[unconfirmed]** (section 1.2).

use super::stat::StatType;

/// The state of a milestone (save byte at entry offset `0x08`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MilestoneState {
    /// 1: the bin is not reached yet. No progress counts.
    Locked = 1,
    /// 2: the bin is reached. Progress is tested in free roam.
    Active = 2,
    /// 3: the threshold was passed in free roam. Banked on escape; back to active on bust.
    AtStake = 3,
    /// 4: banked. Final.
    Complete = 4,
}

impl MilestoneState {
    /// The state byte stored in the save.
    pub const fn byte(self) -> u8 {
        self as u8
    }

    /// Decodes the state byte. Any byte other than 1 to 4 is not a state.
    pub const fn from_byte(b: u8) -> Option<MilestoneState> {
        match b {
            1 => Some(MilestoneState::Locked),
            2 => Some(MilestoneState::Active),
            3 => Some(MilestoneState::AtStake),
            4 => Some(MilestoneState::Complete),
            _ => None,
        }
    }
}

/// A milestone as the data gives it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MilestoneDef {
    /// The collection key of the milestone leaf (the save matches entries by this key).
    pub key: u32,
    /// The Blacklist bin, 1 to 15.
    pub bin: u8,
    /// The statistic the milestone is measured in.
    pub stat: StatType,
    /// The one number that completes it: seconds, points or counts (field `0x8445AF47`).
    pub threshold: f32,
    /// The reward in bounty points (field `0x8E1904C7`). It is paid into the career record, not as cash.
    pub bounty: u32,
}

/// One milestone with its state and the value it was banked at.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Milestone {
    /// The definition the milestone was built from.
    pub def: MilestoneDef,
    /// The current state.
    pub state: MilestoneState,
    /// The statistic value when it was banked; `None` before that.
    pub value: Option<f32>,
}

impl Milestone {
    /// A milestone that starts locked and not banked (section 3.1).
    pub const fn new(def: MilestoneDef) -> Milestone {
        Milestone { def, state: MilestoneState::Locked, value: None }
    }
}
