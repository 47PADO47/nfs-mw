//! The cars a NIS scene makes for its police channels from its special car list (`nfsmw_data::world::scene_cars`):
//! police models load and stand as police cars of the scene, the others as traffic cars of the scene. They are not
//! driven: the scene gives them their poses and removes them at its end. Spec: `docs/specs/nis-player.md` section 7.

use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::world::scene_cars;

use super::TrafficWorld;
use super::car::{AiCar, Role, Start};
use super::racers::ModelState;
use super::traffic::Cruise;
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;
use blackbox_collision::CollisionWorld;
use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter};

impl TrafficWorld {
    /// Asks for the model of a list vehicle unless it is loaded or asked for.
    pub fn request_nis_vehicle(&mut self, vehicle: &str) {
        let model = scene_cars::model_of(vehicle);
        match scene_cars::is_police(vehicle) {
            true => self.request_cop(model),
            false => self.request_racer_model(model),
        }
    }

    /// Whether a list vehicle's model is loaded, loading or failed.
    pub fn nis_vehicle_state(&self, vehicle: &str) -> ModelState {
        let model = scene_cars::model_of(vehicle);
        if !scene_cars::is_police(vehicle) {
            return self.racer_model_state(model);
        }
        match self.has_cop_model(model) {
            true => ModelState::Loaded,
            false => ModelState::Pending,
        }
    }

    /// Puts a list vehicle of the scene at `at` (render space), facing `heading`, on the ground there. Its index, or
    /// `None` when its model is not loaded or no ground is under the point yet.
    pub fn spawn_nis_vehicle(
        &mut self,
        vehicle: &str,
        at: Vec3,
        heading: f32,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> Option<usize> {
        let model = scene_cars::model_of(vehicle);
        if scene_cars::is_police(vehicle) {
            return self.spawn_nis_cop_model(model, at, heading, collision, surfaces);
        }
        let model = self.models.iter().find(|m| m.name.eq_ignore_ascii_case(model))?.clone();
        let physics_at = Vec3::from(space::to_physics(at));
        let facing = Vec3::from(space::to_physics(Vec3::new(heading.cos(), heading.sin(), 0.0)));
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Traffic, SegmentFilter::default());
        // The road is only the car's memory for after the scene; a spot off the road is still the scene's.
        let _ = nav.init_at_point(&self.network, &self.index, physics_at, facing, false);
        let start = Start { stagger: 0, speed: 0.0, cruise: Cruise::default(), patrol: false };
        let ground = WorldGround { collision, surfaces };
        let car = AiCar::place(&model, Role::Traffic, nav, Spawn { position: at, heading }, start, &ground)?;
        log::info!("NIS: a {vehicle} of the scene at ({:.0}, {:.0})", at.x, at.y);
        self.admit(car);
        Some(self.cars.len() - 1)
    }
}
