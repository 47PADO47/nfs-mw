//! Lockstep uploads for a session (`session_rules.rs`, `docs/specs/multiplayer/determinism.md` D3 and D5).
//!
//! Normally an arrived tile is uploaded in the first frame that has room for it, so the order tiles join the world (and
//! with it the ids of their props and the order of their collision) follows the loader threads' timing. In lockstep the
//! uploads wait until every requested tile has arrived, then all go in at once in tile order: the same requests give the
//! same world, however fast the disk is.

use super::{Residency, TileState};

impl Residency {
    /// Turns lockstep uploads on or off.
    pub fn set_lockstep(&mut self, on: bool) {
        self.lockstep = on;
    }

    /// In lockstep: a requested tile has not arrived yet, so this frame uploads nothing.
    pub(super) fn holds_uploads(&self) -> bool {
        let arrived = |index: usize| self.arrived.iter().any(|data| data.index == index);
        self.lockstep
            && self.tiles.iter().any(|(&index, state)| matches!(state, TileState::Requested) && !arrived(index))
    }

    /// In lockstep: the arrived tiles in tile order, and no byte budget (`upload_arrived`).
    pub(super) fn lockstep_order(&mut self) -> bool {
        if !self.lockstep {
            return false;
        }
        self.arrived.make_contiguous().sort_unstable_by_key(|data| data.index);
        true
    }
}
