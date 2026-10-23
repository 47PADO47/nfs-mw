//! Lending the player's car and the static world to the predicted car for one frame's session calls (`car.rs`). The
//! car leaves the players list and the collision and props leave the residency (moves, no copies); they come back in
//! place before anything else of the frame runs.

use super::super::WorldScene;
use super::super::players::LEAD;
use super::car::{GameCar, Lent};

impl WorldScene {
    /// Lends the lead's car and the static world to `car`. `false` (and nothing lent) when the world has no car.
    pub(super) fn lend_world(&mut self, car: &GameCar) -> bool {
        let Some(drive) = self.players.remove(LEAD) else { return false };
        let (collision, props) = self.residency.take_world();
        car.lend(Lent { drive, collision, props });
        true
    }

    /// Takes back what [`WorldScene::lend_world`] lent.
    pub(super) fn return_world(&mut self, car: &GameCar) {
        let Some(lent) = car.take_back() else { return };
        self.players.set_lead(lent.drive);
        self.residency.put_world(lent.collision, lent.props);
    }
}
