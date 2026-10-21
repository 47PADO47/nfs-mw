//! The traffic cars under a NIS scene: the nearest car to a point, a car's physics pose, and the poses a scene gives its
//! cars (`scenes/world/nis/car_world.rs`). The cars' physics is frozen while a scene plays, so the poses stay put.

use glam::Vec3;

use super::TrafficWorld;
use crate::scenes::world::drive::CarPose;

impl TrafficWorld {
    /// The car's physics pose now, or `None` for an index no car has.
    pub fn car_pose(&self, index: usize) -> Option<CarPose> {
        self.cars.get(index).map(|car| car.pose(self.alpha))
    }

    /// The index of the car nearest `point` that no index in `taken` names. Police cars are left out.
    pub fn nearest_car(&self, point: Vec3, taken: &[usize]) -> Option<usize> {
        self.cars
            .iter()
            .enumerate()
            .filter(|(index, car)| !taken.contains(index) && !car.is_cop())
            .map(|(index, car)| (index, car.pose(self.alpha).position.distance_squared(point)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }

    /// Gives the car at `index` the pose a NIS scene drives it at, or clears it with `None`. An index no car has does
    /// nothing.
    pub fn set_nis_pose(&mut self, index: usize, pose: Option<CarPose>) {
        if let Some(car) = self.cars.get_mut(index) {
            car.set_nis_pose(pose);
        }
    }

    /// Clears every car's NIS pose, so each car draws its physics pose again.
    pub fn clear_nis_poses(&mut self) {
        for car in &mut self.cars {
            car.set_nis_pose(None);
        }
    }
}
