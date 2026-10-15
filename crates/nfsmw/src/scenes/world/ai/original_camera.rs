//! Physical resident actors used by the original gameplay camera's body queries.

use super::TrafficWorld;
use crate::viewer::camera::original::{Avoidable, VehicleBounds};

impl TrafficWorld {
    pub fn camera_bodies(&self, out: &mut Vec<Avoidable>) {
        for car in &self.cars {
            car.camera_bodies(out);
        }
    }

    pub fn camera_vehicle_bounds(&self, out: &mut Vec<VehicleBounds>) {
        for car in &self.cars {
            car.camera_vehicle_bounds(out);
        }
    }
}
