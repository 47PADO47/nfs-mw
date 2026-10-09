//! The car's physics for code that is not about the player, such as hits between cars.

use super::{CarSim, Drive};

impl Drive {
    /// The physics of the car, once it stands on a road.
    pub(in crate::scenes::world) fn sim_mut(&mut self) -> Option<&mut CarSim> {
        self.sim.as_mut()
    }
}
