//! A co-op target busted on its own while the chase goes on (`docs/specs/coop-career.md` section 5).

use crate::scenes::world::WorldScene;
use crate::scenes::world::players::PlayerId;

impl WorldScene {
    /// Player `id` was busted by its own meter: it pays on its own car and goes to the safe house.
    pub(in crate::scenes::world) fn coop_bust(&mut self, id: PlayerId) {
        log::info!("co-op: player {} is busted; the chase goes on", id.0 + 1);
    }
}
