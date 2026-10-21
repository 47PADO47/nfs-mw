//! The traffic cars under a NIS scene: the nearest car to a point, the nearest free police car, a police car a scene spawns,
//! a car's physics pose, and the poses a scene gives its cars (`scenes/world/nis/car_world.rs`). The cars' physics is frozen
//! while a scene plays, so the poses stay put.

use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::TrafficWorld;
use super::car::AiCar;
use super::pursuit::CopSpot;
use crate::scenes::world::drive::CarPose;
use crate::scenes::world::space;
use blackbox_collision::CollisionWorld;

/// The police car a NIS scene spawns for a cop channel: the install's COPMIDSIZE. The spec's `copmidsize_nis` has no folder
/// of its own (`docs/specs/nis-player.md`, section 7.1).
const NIS_COP_MODEL: &str = "copmidsize";
/// How far ahead of a spawned police car, in metres, the point it faces lies.
const FACING_REACH: f32 = 10.0;

impl TrafficWorld {
    /// The car's physics pose now, or `None` for an index no car has.
    pub fn car_pose(&self, index: usize) -> Option<CarPose> {
        self.cars.get(index).map(|car| car.pose(self.alpha))
    }

    /// The index of the civilian car nearest `point` that no index in `taken` names. Police cars are left out.
    pub fn nearest_car(&self, point: Vec3, taken: &[usize]) -> Option<usize> {
        self.nearest_where(point, taken, |car| !car.is_police())
    }

    /// The index of the police car nearest `point` that no index in `taken` names. A wrecked car is left out.
    pub fn nearest_police(&self, point: Vec3, taken: &[usize]) -> Option<usize> {
        self.nearest_where(point, taken, AiCar::is_live_police)
    }

    /// Asks for the police car a NIS scene spawns, unless it is loaded. The world asks once when it opens, so a scene that
    /// wants one usually finds it loaded; a scene that does not finds it waiting (`scenes/world/nis/cop_wait.rs`).
    pub fn request_nis_cop(&mut self) {
        self.request_cop(NIS_COP_MODEL);
    }

    /// Puts a police car of the scene at `at` (render space), facing `heading` (radians from +X, render space), on the cop
    /// road through the cop spawn path (`spawn_cop_at`). Returns its index, or `None` when the car is not loaded or the road
    /// has no place for it. The car is not driven: its pose is the scene's, and the scene removes it at its end.
    pub fn spawn_nis_cop(
        &mut self,
        at: Vec3,
        heading: f32,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> Option<usize> {
        let model = self.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case(NIS_COP_MODEL))?;
        let facing = Vec3::new(heading.cos(), heading.sin(), 0.0);
        let spot = CopSpot {
            at: Vec3::from(space::to_physics(at)),
            target: Vec3::from(space::to_physics(at + facing * FACING_REACH)),
            speed: 0.0,
            player: model.car.performance,
            band: None,
        };
        if !self.spawn_cop_at(NIS_COP_MODEL, spot, collision, surfaces) {
            return None;
        }
        Some(self.cars.len() - 1)
    }

    /// Removes the cars at `indices` from the traffic world. The indices are the ones a scene holds, so they are read
    /// before any car is removed: the highest goes first, so the rest stay where they are.
    pub fn remove_cars(&mut self, indices: &[usize]) {
        let mut sorted = indices.to_vec();
        sorted.sort_unstable_by(|a, b| b.cmp(a));
        sorted.dedup();
        for index in sorted {
            if index < self.cars.len() {
                self.cars.remove(index);
            }
        }
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

    /// The index of the car nearest `point` that no index in `taken` names and that `wanted` accepts.
    fn nearest_where(&self, point: Vec3, taken: &[usize], wanted: impl Fn(&AiCar) -> bool) -> Option<usize> {
        self.cars
            .iter()
            .enumerate()
            .filter(|(index, car)| !taken.contains(index) && wanted(car))
            .map(|(index, car)| (index, car.pose(self.alpha).position.distance_squared(point)))
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(index, _)| index)
    }
}
