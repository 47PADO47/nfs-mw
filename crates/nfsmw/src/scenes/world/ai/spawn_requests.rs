//! The cop manager's active-car budget, its queued spawn requests and the over-budget removal.
//! Spec: `docs/specs/ai-pursuit-cops.md` sections 1 and 5, `docs/provenance/ai-pursuit-cop-fleet.md`.
//!
//! The original keeps at most eight active cop cars: the wave asks for a cop only below that count, and a request to place
//! a cop (`SpawnCop`, from scripts) first removes the lowest-priority active cop when it would break the budget. Every
//! police car on the road counts: patrols, the chase's cops, the support cars and the roadblock's cars, wrecks until they
//! are recycled. The helicopter is counted apart.

use blackbox_collision::CollisionWorld;
use blackbox_pursuit::fleet;
use blackbox_roads::{LaneType, NavKind, RandomSource, RoadNav, SegmentFilter};
use glam::Vec3;
use nfsmw_data::car::physics::SurfaceTable;

use super::car::{AiCar, Role, Start};
use super::cop::CopState;
use super::cop_manager::requests::{Request, Role as RequestRole};
use super::traffic::Cruise;
use super::{Focus, TrafficWorld};
use crate::scenes::world::drive::{CarSim, WorldGround};
use crate::scenes::world::road::Spawn;
use crate::scenes::world::space;

/// A cop placed by a request must not overlap another car: the distance between the centres is at least the two radii
/// plus this much (`DoesSpotOverlapCar`).
const OVERLAP_MARGIN: f32 = 2.0;
/// How far ahead of a patrol's spot its cursor starts (as the patrol spawn does).
const START_LOOK_AHEAD: f32 = 30.0;

impl TrafficWorld {
    /// An explicit chase request starts the pursuit once its car can be placed, without an offence or sighting.
    pub(super) fn requested_chase_ready(
        &self,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        !self.world_moment
            && self.cops.spawning_enabled()
            && self.police.can_begin()
            && self.cops.requests.iter().any(|request| {
                request.role == RequestRole::Chase
                    && self.request_spot_clear(request, focus)
                    && self.requested_chase_supported(request, collision, surfaces)
            })
    }

    /// Use the placement queries without allocating a render key or advancing the spawn RNG.
    /// Model loading and overlap alone cannot establish that a queued unit can stand on the world.
    fn requested_chase_supported(
        &self,
        request: &Request,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        if collision.resident_near(request.at.to_array(), 30.0) == Some(false) {
            return false;
        }
        let Some(model) = self.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case(&request.name))
        else {
            return false;
        };
        let filter = SegmentFilter { cop: true, no_decision: true, ..Default::default() };
        let mut nav = RoadNav::new(NavKind::Direction, LaneType::Cop, filter);
        if !nav.init_at_point(&self.network, &self.index, request.at, request.forward, false) {
            return false;
        }
        let render = space::to_render(request.at.to_array());
        let facing = space::to_render(request.forward.to_array());
        let spawn = Spawn { position: render, heading: facing.y.atan2(facing.x) };
        let mut sim = CarSim::for_rig(model.model.physics.clone(), &model.model.rig);
        sim.configure_ai();
        sim.place_traffic_moving(&WorldGround { collision, surfaces }, spawn, super::pursuit::START_SPEED)
    }

    fn request_spot_clear(&self, request: &Request, focus: Focus) -> bool {
        let Some(model) = self.cop_models.iter().find(|model| model.model.name.eq_ignore_ascii_case(&request.name))
        else {
            return false;
        };
        let radius = model.model.physics.spec.dimension.length() + OVERLAP_MARGIN;
        let overlaps_car =
            self.cars.iter().any(|car| car.physics_position().distance(request.at) < car.bounding_radius() + radius);
        let overlaps_player = focus
            .body
            .into_iter()
            .chain(self.guest_bodies())
            .any(|body| body.position.distance(request.at) < body.half_width.hypot(body.half_length) + radius);
        !overlaps_car && !overlaps_player
    }

    /// A scripted `NoNewPursuitsOrCops` cancels requests which have not been placed yet.
    pub fn cancel_cop_spawns(&mut self) {
        self.cops.requests = Default::default();
    }

    /// The police cars on the road now, helicopter apart (`mNumActiveCopCars`): patrols, chase cops, support cars and roadblock
    /// cars, wrecks included until the patrol pass recycles them.
    pub fn active_cop_cars(&self) -> usize {
        self.cars.iter().filter(|car| car.is_police()).count()
    }

    /// The most active cop cars (`mMaxActiveCopCars`): the Bartender setting, 8 in the install.
    pub fn cop_budget(&self) -> usize {
        self.police.settings().max_cops as usize
    }

    /// Whether the wave may ask for one more cop car: the budget has room (`mNumActiveCopCars < mMaxActiveCopCars`).
    pub(super) fn budget_has_room(&self) -> bool {
        self.active_cop_cars() < self.cop_budget()
    }

    /// Asks for a cop car of `name` at `at` (physics space) facing `forward`. It is placed on a later run when the cops are
    /// not locked out, its model is loaded and no car overlaps the spot (`SpawnCop` of the original's scripts).
    pub fn queue_cop_spawn(&mut self, name: &str, at: Vec3, forward: Vec3, role: RequestRole) {
        let request = Request { name: name.to_owned(), at, forward, role };
        self.cops.requests.push(request);
    }

    /// Cop spawn requests waiting to be placed.
    pub fn cop_requests_waiting(&self) -> usize {
        self.cops.requests.len()
    }

    /// Step 7 of the manager's run: tries queued requests in order. A chase request waits for scene pursuit construction.
    pub(super) fn service_spawn_requests(&mut self, focus: Focus, collision: &CollisionWorld, surfaces: &SurfaceTable) {
        if self.world_moment {
            return;
        }
        if self.cops.requests.is_empty() {
            return;
        }
        let mut requests = std::mem::take(&mut self.cops.requests);
        requests.service(|request| self.try_spawn_request(request, focus, collision, surfaces));
        self.cops.requests = requests;
    }

    /// `TrySpawnCop`: makes room when the budget would be broken, then places the car if it can.
    fn try_spawn_request(
        &mut self,
        request: &Request,
        focus: Focus,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> bool {
        if fleet::over_budget(self.active_cop_cars() + 1, self.cop_budget()) {
            self.remove_lowest_priority_cop(focus);
        }
        if !self.cops.spawning_enabled() {
            return false;
        }
        if !self.has_cop_model(&request.name) {
            self.request_cop(&request.name);
            return false;
        }
        if !self.request_spot_clear(request, focus) {
            return false;
        }
        let Some(car) = self.build_requested_car(request, collision, surfaces) else { return false };
        log::info!("cop manager: {} placed by a spawn request as {:?}", car.name, request.role);
        if request.role == RequestRole::Chase
            && let Some(pursuit) = self.pursuit.as_mut()
        {
            pursuit.involved += 1;
            self.police.ground_deployed(&car.name);
        }
        self.admit(car);
        true
    }

    /// The car of a request, standing exactly where it was asked for, or `None` when it cannot stand there: no road under
    /// the spot, no ground, another car in the way, or (for a chase cop) no chase.
    fn build_requested_car(
        &mut self,
        request: &Request,
        collision: &CollisionWorld,
        surfaces: &SurfaceTable,
    ) -> Option<AiCar> {
        let model = self.cop_models.iter().find(|m| m.model.name.eq_ignore_ascii_case(&request.name))?;
        let radius = model.model.physics.spec.dimension.length();
        if self
            .cars
            .iter()
            .any(|car| car.physics_position().distance(request.at) < car.bounding_radius() + radius + OVERLAP_MARGIN)
        {
            return None;
        }
        let chase = request.role == RequestRole::Chase;
        let performance = self.pursuit.as_ref().map(|pursuit| pursuit.performance());
        if chase && performance.is_none() {
            return None;
        }
        let (mut nav, cruise, speed) = match request.role {
            RequestRole::Roadblock => (
                RoadNav::new(NavKind::Direction, LaneType::Cop, SegmentFilter { cop: true, ..Default::default() }),
                Cruise::default(),
                0.0,
            ),
            RequestRole::Chase => {
                let filter = SegmentFilter { cop: true, no_decision: true, ..SegmentFilter::default() };
                (
                    RoadNav::new(NavKind::Direction, LaneType::Cop, filter),
                    Cruise::default(),
                    super::pursuit::START_SPEED,
                )
            }
            RequestRole::Patrol => {
                let cruise = self.patrol.as_ref()?.cruise;
                let nav = RoadNav::new(NavKind::Direction, LaneType::Racing, SegmentFilter::default());
                (nav, cruise, cruise.street.min(cruise.highway))
            }
        };
        if !nav.init_at_point(&self.network, &self.index, request.at, request.forward, false) {
            return None;
        }
        if request.role == RequestRole::Patrol && !nav.can_traffic_spawn(&self.network, &mut self.rng.police) {
            return None;
        }
        nav.half_width = model.model.physics.spec.dimension.x;
        nav.enable_trail(&self.network);
        // Script requests retain their explicit physical pose; only their navigation is prepared here.
        let render = space::to_render(request.at.to_array());
        let facing = space::to_render(request.forward.to_array());
        let spawn = Spawn { position: render, heading: facing.y.atan2(facing.x) };
        if request.role == RequestRole::Patrol {
            nav.advance_with_lookahead(
                &self.network,
                START_LOOK_AHEAD,
                Vec3::ZERO,
                START_LOOK_AHEAD,
                &mut self.rng.police,
            );
        }
        let role = match performance {
            Some(performance) if chase => Role::Cop(Box::new(CopState::new(model.car, performance, speed))),
            _ => Role::Traffic,
        };
        let start = Start {
            stagger: (self.rng.police.next_f32() * 8.0) as u32,
            speed,
            cruise,
            patrol: request.role == RequestRole::Patrol,
        };
        let mut car = AiCar::place(&model.model, role, nav, spawn, start, &WorldGround { collision, surfaces })?;
        if request.role == RequestRole::Roadblock {
            car.set_roadblock();
        }
        Some(car)
    }

    /// `UpdateCopPriorities`: takes the active cop with the lowest priority off the road. Cops behind the player and out of
    /// view go before cops ahead of it and in view.
    fn remove_lowest_priority_cop(&mut self, focus: Focus) {
        let Some(player) = focus.body else { return };
        let forward = Vec3::new(player.forward.x, 0.0, player.forward.y);
        let active: Vec<usize> = (0..self.cars.len()).filter(|&i| self.cars[i].is_police()).collect();
        let priorities = active.iter().map(|&i| {
            let car = &self.cars[i];
            fleet::priority(player.position, forward, car.physics_position(), car.offscreen_time() <= 0.0)
        });
        let Some(lowest) = fleet::lowest(priorities) else { return };
        let car = self.cars.remove(active[lowest]);
        log::info!("cop manager: {} is taken off the road, the cop budget of {} is full", car.name, self.cop_budget());
    }

    /// One line for the console: the cop cars against their budget and the requests waiting.
    pub fn fleet_status(&self) -> String {
        let count = |keep: fn(&AiCar) -> bool| self.cars.iter().filter(|car| keep(car)).count();
        format!(
            "cop cars {} of {} (patrol {}, chase {}, roadblock {}); {} spawn requests waiting; {} cop models loaded",
            self.active_cop_cars(),
            self.cop_budget(),
            count(|car| car.is_patrol() && !car.is_roadblock()),
            count(|car| car.is_cop() && !car.is_roadblock()),
            count(AiCar::is_roadblock),
            self.cop_requests_waiting(),
            self.cop_models.len(),
        )
    }
}
