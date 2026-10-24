//! What the server asks of a world beyond [`WorldSim`]: the session rules that move or park a car, and the player's
//! view of the pursuit and the race for the global block. Every call has a default that does nothing and says so, so
//! a world that cannot do one yet still runs; the server then skips the effect (a refused reset is not announced).
//! These calls belong in `WorldSim` itself once the game's world has them.

use nfsmw_net::frames::GlobalBlock;
use nfsmw_net::message::Mode;
use nfsmw_sim::toy::ToyWorld;
use nfsmw_sim::{PlayerId, WorldSim};

/// What a round asks of a world that runs its own races: the session's mode and track, and who races.
#[derive(Debug, Clone, PartialEq)]
pub struct RaceRequest {
    pub mode: Mode,
    /// The track id of the session settings (the FEng hash of the race's collection name).
    pub track: u32,
    /// Laps, 0 for the track's own.
    pub laps: u8,
    /// Traffic, percent.
    pub traffic_density: u8,
    /// The players who race, in slot order. Each has a car in the world already.
    pub players: Vec<PlayerId>,
}

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

    /// Starts the race of `request` in the world, with `request.players` as its people. The world runs its own grid,
    /// intro and countdown; the laps and finishes come back as [`nfsmw_sim::SimEvent`]s. An `Err` says why this world
    /// cannot (the server logs it and the round goes on without a world race).
    fn start_race(&mut self, _request: &RaceRequest) -> Result<(), String> {
        Err("this world runs no races".into())
    }

    /// Whether the race of [`WorldControl::start_race`] stands ready on its grid. The server holds its load barrier until
    /// it does, so nobody is counted down to a race that is still loading. True for a world with nothing to wait for.
    fn race_staged(&self) -> bool {
        true
    }

    /// The race should go (the first racer's GO) on server tick `go_tick`; a world that staged its race starts the
    /// countdown so that it ends there, or at once when `go_tick` is too close. Called when the lobby's countdown begins.
    fn schedule_race_go(&mut self, _go_tick: u32) {}

    /// Ends the world's race, if one runs, and puts the players back on the road.
    fn stop_race(&mut self) {}

    /// Fills what the global block tells player `id` beyond the phase: the heat, the busted and cooldown meters and
    /// the chase's state, the race place and lap, the time of day. The default leaves the block as the server made it
    /// (pursuit and race fields zero, the lobby's time of day).
    fn fill_global(&self, _id: PlayerId, _global: &mut GlobalBlock) {}
}

impl WorldControl for ToyWorld {
    fn reset_player(&mut self, id: PlayerId) -> bool {
        ToyWorld::reset_player(self, id)
    }
}
