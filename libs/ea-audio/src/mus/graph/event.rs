//! Events: short programs of 12-byte actions that the player runs by id.

use crate::bytes::u32_le;
use crate::error::Result;

/// Opcode of the action that jumps to a node.
pub const OP_BRANCH_TO: u8 = 4;
/// Size of an event's header in bytes.
const HEADER: usize = 20;
/// Size of an action in bytes.
const ACTION: usize = 12;
/// The node number of "no node" in a branch action.
const NO_NODE: u32 = 0xFFFF;

/// One action of an event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    pub mask: u32,
    pub w1: u32,
    pub w2: u32,
}

impl Action {
    /// Bits 8 to 14 of the second word.
    pub fn opcode(&self) -> u8 {
        ((self.w1 >> 8) & 0x7F) as u8
    }

    /// For a branch action, the node it jumps to; `None` for other actions and for "no node" (stop).
    pub fn branch_node(&self) -> Option<usize> {
        if self.opcode() != OP_BRANCH_TO {
            return None;
        }
        let node = self.w2 & 0xFFFF;
        (node != NO_NODE).then_some(node as usize)
    }
}

/// An event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// The 24-bit id. The game's ids have a project bit above it; match on these 24 bits.
    pub id: u32,
    pub actions: Vec<Action>,
}

impl Event {
    pub(super) fn parse(data: &[u8], at: usize) -> Result<Self> {
        let word = u32_le(data, at + 12)?;
        let actions = (0..(word >> 24) as usize)
            .map(|i| {
                let base = at + HEADER + ACTION * i;
                Ok(Action { mask: u32_le(data, base)?, w1: u32_le(data, base + 4)?, w2: u32_le(data, base + 8)? })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Self { id: word & 0x00FF_FFFF, actions })
    }

    /// The node a song event starts at: the last action that branches to a real node. A song event is "stop
    /// everything" followed by that branch; an event with no such action returns `None`.
    pub fn start_node(&self) -> Option<usize> {
        self.actions.iter().rev().find_map(Action::branch_node)
    }
}
