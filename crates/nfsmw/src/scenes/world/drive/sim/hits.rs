//! What the car touched since the traffic last asked: a prop at all, and the props with their cost to state.

use super::CarSim;

impl CarSim {
    /// The props the car hit since the last call, with their cost to state (prop id, cost).
    pub fn take_smacked(&mut self) -> Vec<(u32, u32)> {
        std::mem::take(&mut self.smacked)
    }

    /// Whether the car touched a prop since the last call.
    pub fn take_prop_struck(&mut self) -> bool {
        std::mem::take(&mut self.prop_struck)
    }
}
