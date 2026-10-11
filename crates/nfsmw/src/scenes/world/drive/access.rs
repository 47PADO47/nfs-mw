//! The car's physics for code that is not about the player, such as hits between cars.

use glam::Vec3;

use super::{CarSim, Drive};

impl Drive {
    pub fn update_events(
        &mut self,
        props: &mut crate::scenes::world::props::PropWorld,
        data: &nfsmw_data::car::physics::PhysicsData,
        view: Vec3,
    ) {
        if let Some(sim) = self.sim.as_mut() {
            crate::scenes::world::cargo::update(sim, &self.rig, props, data, view);
        }
    }

    /// What the car can do flat out (measured by driving the model flat out), for cops that are matched to it.
    pub(in crate::scenes::world) fn performance(&self) -> blackbox_vehicle::performance::Performance {
        blackbox_vehicle::performance::measure(&self.physics.spec)
    }

    /// The physics of the car, once it stands on a road.
    pub(in crate::scenes::world) fn sim_mut(&mut self) -> Option<&mut CarSim> {
        self.sim.as_mut()
    }
}
