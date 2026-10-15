//! Native camera-query registration for a resident physical trailer.

use super::TrailerCar;
use crate::viewer::camera::original::{Avoidable, VehicleBounds};

impl TrailerCar {
    pub(in crate::scenes::world::ai) fn camera_bodies(&self, out: &mut Vec<Avoidable>) {
        if self.sim.is_finite() {
            out.push(self.sim.camera_avoidable());
        }
    }

    pub(in crate::scenes::world::ai) fn camera_vehicle_bounds(&self, out: &mut Vec<VehicleBounds>) {
        if !self.sim.is_finite() {
            return;
        }
        out.push(VehicleBounds {
            world_id: self.render_id,
            body: self.sim.camera_avoidable(),
            radius: self.sim.body().radius(),
            active: true,
            loading: false,
        });
    }
}
