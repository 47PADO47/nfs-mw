//! The racers of a race, in the traffic world: their models, placing them on the grid and what the race reads of them.
//! Specs: `docs/specs/ai-racers.md`, `docs/specs/ai-racers-route.md`.

use blackbox_collision::CollisionWorld;
use blackbox_roads::{LaneType, NavKind, RoadNav, SegmentFilter};
use blackbox_vehicle::performance::{Performance, measure_with_integration};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;
use nfsmw_data::pursuit::AiVehicle;

use super::TrafficWorld;
use super::car::{AiCar, Role, Start};
use super::racer::RacerState;
use super::traffic::Cruise;
use crate::scenes::world::drive::WorldGround;
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// Where a racer's model stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelState {
    Loaded,
    Pending,
    Failed,
}

/// Everything a racer is created from.
pub struct RacerSpawn {
    /// The racer's index in the race.
    pub id: usize,
    /// The `pvehicle` name of its car.
    pub model: String,
    /// The grid slot in render space and its heading (radians from +X).
    pub spawn: Spawn,
    pub ai: AiVehicle,
    /// The player's car, for matching.
    pub player: Performance,
    /// `MinimumAIPerformance`.
    pub bias: f32,
    pub base_skill: f32,
    /// The start speed, m/s (a rolling start).
    pub speed: f32,
    /// Held on the grid until GO.
    pub held: bool,
    /// A person drives this racer (split screen): `Some(automatic gearbox)`.
    pub person: Option<bool>,
}

impl TrafficWorld {
    /// Asks for the racer's model (a `pvehicle` name) unless it is loaded or already asked for.
    pub fn request_racer_model(&mut self, name: &str) {
        if self.racer_model_state(name) == ModelState::Pending {
            self.request_traffic_model(name);
        }
    }

    /// Whether the racer's model is loaded, still loading, or failed to load.
    pub fn racer_model_state(&self, name: &str) -> ModelState {
        let lower = name.to_ascii_lowercase();
        if self.models.iter().any(|m| m.name.eq_ignore_ascii_case(name)) {
            return ModelState::Loaded;
        }
        match self.failed.contains(&lower) {
            true => ModelState::Failed,
            false => ModelState::Pending,
        }
    }

    /// Takes every racer out of the world.
    pub fn clear_racers(&mut self) {
        self.cars.retain(|car| !car.is_racer());
    }

    /// Puts a racer on the grid. `false` when its model is not loaded or the ground or road is missing there.
    pub fn spawn_racer(&mut self, spec: &RacerSpawn, collision: &CollisionWorld, surfaces: &SurfaceTable) -> bool {
        let Some(model) = self.models.iter().find(|m| m.name.eq_ignore_ascii_case(&spec.model)) else { return false };
        let physics_at = Vec3::from(space::to_physics(spec.spawn.position));
        let dir = Vec3::from(space::to_physics(Vec3::new(spec.spawn.heading.cos(), spec.spawn.heading.sin(), 0.0)));
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
        if !nav.init_at_point(&self.network, &self.index, physics_at, dir, false) {
            log::warn!("race: no road at the grid slot of racer {}", spec.id);
            return false;
        }
        nav.half_width = model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        let own = measure_with_integration(&model.physics.spec, model.physics.tire_integration);
        let mut state = RacerState::new(spec.id, own, spec.ai, spec.player, spec.bias, spec.base_skill, spec.speed);
        state.held = spec.held;
        let start = Start { stagger: spec.id as u32 % 8, speed: spec.speed, cruise: Cruise::default(), patrol: false };
        let ground = WorldGround { collision, surfaces };
        let Some(car) = AiCar::place(model, Role::Racer(Box::new(state)), nav, spec.spawn, start, &ground) else {
            return false;
        };
        log::info!(
            "race: racer {} drives {} from ({:.0}, {:.0})",
            spec.id,
            car.name,
            spec.spawn.position.x,
            spec.spawn.position.y
        );
        self.cars.push(car);
        if let (Some(automatic), Some(car)) = (spec.person, self.cars.last_mut()) {
            car.seat_person(automatic);
        }
        true
    }

    /// The pedals a person holds in the racer `id` this frame. `false` when no person drives it.
    pub fn set_racer_pedals(&mut self, id: usize, pedals: crate::scenes::world::drive::DriveInput) -> bool {
        self.cars.iter_mut().find(|car| car.racer_id() == Some(id)).is_some_and(|car| car.set_pedals(pedals))
    }

    /// The racer's car as a camera follows it: place (render space), heading (radians from +X) and speed (m/s), blended
    /// `alpha` of the way between its last two physics steps.
    pub fn racer_followed(&self, id: usize, alpha: f32) -> Option<crate::viewer::camera::Followed> {
        let car = self.cars.iter().find(|car| car.racer_id() == Some(id))?;
        let pose = car.pose(alpha);
        let nose = pose.rotation * Vec3::X;
        Some(crate::viewer::camera::Followed {
            position: pose.position,
            heading: nose.y.atan2(nose.x),
            speed: car.speed(),
        })
    }

    /// The HUD state of the racer `id`'s car (split screen's second player).
    pub fn racer_hud(&self, id: usize) -> Option<crate::hud::HudState> {
        self.cars.iter().find(|car| car.racer_id() == Some(id)).map(AiCar::hud_state)
    }

    /// The racers: index, place in physics space and speed.
    pub fn racers(&self) -> impl Iterator<Item = (usize, Vec3, f32)> + '_ {
        self.cars.iter().filter_map(|car| car.racer_id().map(|id| (id, car.race_position(), car.race_speed())))
    }

    /// The racer with this index.
    pub fn racer_mut(&mut self, id: usize) -> Option<&mut RacerState> {
        self.cars.iter_mut().filter_map(AiCar::racer_mut).find(|racer| racer.id == id)
    }

    /// Takes the racer with this index out of the world (a racer that was knocked out).
    pub fn remove_racer(&mut self, id: usize) {
        self.cars.retain(|car| car.racer_id() != Some(id));
    }

    /// Releases every racer from the grid.
    pub fn release_racers(&mut self) {
        for racer in self.cars.iter_mut().filter_map(AiCar::racer_mut) {
            racer.held = false;
        }
    }

    /// Which way the racer points on the minimap (render space); `None` for an emulated one, which has no car moving.
    pub fn racer_heading(&self, id: usize) -> Option<[f32; 2]> {
        let car = self.cars.iter().find(|car| car.racer_id() == Some(id))?;
        let emulated = car.is_racer_emulated();
        (!emulated).then(|| car.heading())
    }

    /// Puts the emulated racer `id` back on the road at `at` (physics space), facing `forward`. `false` when it cannot
    /// stand there yet (the ground is not loaded): it stays emulated.
    pub fn relocate_racer(
        &mut self,
        id: usize,
        at: Vec3,
        forward: Vec3,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        let Some(car) = self.cars.iter_mut().find(|car| car.racer_id() == Some(id)) else { return false };
        let speed = car.race_speed().max(15.0);
        car.relocate_racer(&self.network, &self.index, at, forward, speed, &WorldGround { collision, surfaces })
    }
}
