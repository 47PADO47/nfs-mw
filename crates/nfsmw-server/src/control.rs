//! What the server asks of a world beyond [`WorldSim`]: the session rules that move or park a car. Every call has a
//! default that does nothing and says so, so a world that cannot do one yet still runs; the server then skips the
//! effect (a refused reset is not announced). These calls belong in `WorldSim` itself once the game's world has them.

use nfsmw_sim::toy::ToyWorld;
use nfsmw_sim::{PlayerId, WorldSim};

/// The server's extra calls on a world.
pub trait WorldControl: WorldSim {
    /// Puts the player's car back on the road (MP-35). Returns whether the world did.
    fn reset_player(&mut self, _id: PlayerId) -> bool {
        false
    }

    /// Makes the player's car a parked, non-colliding ghost or a normal car again (MP-50, MP-51). Returns whether the
    /// world did.
    fn set_ghost(&mut self, _id: PlayerId, _ghost: bool) -> bool {
        false
    }
}

impl WorldControl for ToyWorld {
    fn reset_player(&mut self, id: PlayerId) -> bool {
        ToyWorld::reset_player(self, id)
    }
}
