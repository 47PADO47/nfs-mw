//! The static world lent out for a multiplayer client's prediction (`net_client/car.rs`): the collision and the props
//! leave the residency for the length of the session calls of one frame and come back unchanged in place. Nothing
//! streams while they are out (the frame's streaming runs before and after, never during).

use blackbox_collision::CollisionWorld;

use super::Residency;
use crate::scenes::world::props::PropWorld;

impl Residency {
    /// Takes the collision world and the props out, leaving empty ones (no allocation) until [`Residency::put_world`].
    pub(in crate::scenes::world) fn take_world(&mut self) -> (CollisionWorld, PropWorld) {
        (std::mem::take(&mut self.collision), std::mem::take(&mut self.props))
    }

    /// Puts back what [`Residency::take_world`] took.
    pub(in crate::scenes::world) fn put_world(&mut self, collision: CollisionWorld, props: PropWorld) {
        self.collision = collision;
        self.props = props;
    }
}
