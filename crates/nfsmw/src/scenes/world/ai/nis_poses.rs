//! The traffic cars under a NIS scene: the nearest car to a point, the nearest free police car, a police car a scene spawns,
//! a car's physics pose, and the poses a scene gives its cars (`scenes/world/nis/car_world.rs`). The cars' physics is frozen
//! while a scene plays, so the poses stay put.

use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::TrafficWorld;
use super::car::AiCar;
use super::car::{Role, Start};
use super::cop::CopState;
use super::pursuit::CopSpot;
use super::traffic::Cruise;
use crate::scenes::world::drive::CarPose;
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;
use blackbox_collision::CollisionWorld;
use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter};

/// The police car a NIS scene spawns for a cop channel: the install's COPMIDSIZE. The spec's `copmidsize_nis` has no folder
/// of its own (`docs/specs/nis-player.md`, section 7.1).
const NIS_COP_MODEL: &str = "copmidsize";
/// How far ahead of a spawned police car, in metres, the point it faces lies.
const FACING_REACH: f32 = 10.0;

impl TrafficWorld {
    /// A race channel binds by racer ID, independent of civilian proximity and vector order.
    pub fn racer_car_index(&self, id: usize) -> Option<usize> {
        self.cars.iter().position(|car| car.racer_id() == Some(id))
    }

    pub fn nis_ground_lift(&self, index: usize) -> Option<f32> {
        self.cars.get(index).map(AiCar::nis_ground_lift)
    }

    pub fn nis_geometry(&self, index: usize) -> Option<(f32, f32, [bool; 4])> {
        self.cars.get(index).map(AiCar::nis_geometry)
    }

    pub fn set_nis_body_tilt(&mut self, index: usize, tilt: glam::Quat) {
        if let Some(car) = self.cars.get_mut(index) {
            car.set_nis_body_tilt(tilt);
        }
    }

    /// The car's physics pose now, or `None` for an index no car has.
    pub fn car_pose(&self, index: usize) -> Option<CarPose> {
        self.cars.get(index).map(|car| car.pose(self.alpha))
    }

    /// The pose the car at `index` is drawn at now: a scene's pose while one drives it, else its physics pose.
    pub fn car_render_pose(&self, index: usize) -> Option<CarPose> {
        self.cars.get(index).map(|car| car.render_pose(self.alpha))
    }

    /// Every car's index, drawn position, whether it is a police car, and its racer id, for the NIS frame capture.
    pub fn car_census(&self) -> Vec<(usize, Vec3, bool, Option<usize>)> {
        let cars = self.cars.iter().enumerate();
        cars.map(|(i, car)| (i, car.render_pose(self.alpha).position, car.is_police(), car.racer_id())).collect()
    }

    /// The rolling radius of each wheel of the car at `index`, in the model's wheel order.
    pub fn car_wheel_radii(&self, index: usize) -> Option<[f32; 4]> {
        self.cars.get(index).map(AiCar::wheel_radii)
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

    /// Puts a police car of the scene at `at` (render space), facing `heading` (radians from +X, render space), on the
    /// ground under that point. Returns its index, or `None` when the car is not loaded or no ground is under the point yet
    /// (the world there has not streamed in). The cop spawn path (`spawn_cop_at`) is tried first, for a car that stands
    /// on the cop road; where its rules refuse (the gap to other cars, the no-spawn zones, no cop road), the car stands
    /// where the scene says, as the original makes the scene's police cars with the NIS driver (`docs/specs/nis-player.md`
    /// section 7.1). The car is not driven: its pose is the scene's, and the scene removes it at its end.
    pub fn spawn_nis_cop(
        &mut self,
        at: Vec3,
        heading: f32,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> Option<usize> {
        let model = self.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case(NIS_COP_MODEL))?;
        let facing = Vec3::new(heading.cos(), heading.sin(), 0.0);
        let physics_at = Vec3::from(space::to_physics(at));
        let target = Vec3::from(space::to_physics(at + facing * FACING_REACH));
        let player = model.car.performance;
        let spot = CopSpot { at: physics_at, target, speed: 0.0, player, band: None };
        if self.spawn_cop_at(NIS_COP_MODEL, spot, collision, surfaces) {
            return Some(self.cars.len() - 1);
        }
        let model = self.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case(NIS_COP_MODEL))?;
        let toward = target - physics_at;
        let filter = SegmentFilter { cop: true, ..SegmentFilter::default() };
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, filter);
        // The road is only the car's memory for after the scene; a spot off the cop road is still the scene's.
        let _ = nav.init_at_point(&self.network, &self.index, physics_at, toward, false);
        let state = CopState::new(model.car, model.car.performance, 0.0);
        let start = Start { stagger: 0, speed: 0.0, cruise: Cruise::default(), patrol: false };
        let spawn = Spawn { position: at, heading };
        let ground = WorldGround { collision, surfaces };
        let car = AiCar::place(&model.model, Role::Cop(Box::new(state)), nav, spawn, start, &ground)?;
        log::info!("NIS: a police car of the scene at ({:.0}, {:.0})", at.x, at.y);
        self.admit(car);
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
