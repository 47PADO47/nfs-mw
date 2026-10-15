//! Read-only access to each physical AI actor and its trailer for camera queries.

use super::AiCar;
use crate::viewer::camera::original::{Avoidable, VehicleBounds};

impl AiCar {
    pub(in crate::scenes::world::ai) fn camera_bodies(&self, out: &mut Vec<Avoidable>) {
        if !self.sim.is_finite() {
            return;
        }
        out.push(self.sim.camera_avoidable());
        if let Some(trailer) = self.trailer() {
            trailer.camera_bodies(out);
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
        if let Some(trailer) = self.trailer() {
            trailer.camera_vehicle_bounds(out);
        }
    }
}
