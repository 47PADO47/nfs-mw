//! What the cop sirens read of the police state (`ai/sirens.rs`). Child of `police`, so it reads the state directly.

use super::*;

impl Police {
    /// Seconds since any cop last saw the player (0 with no chase).
    pub fn seconds_unseen(&self) -> f32 {
        self.cooldown.since_seen()
    }
}
