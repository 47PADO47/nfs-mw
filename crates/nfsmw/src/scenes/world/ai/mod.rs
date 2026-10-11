//! Computer-driven cars in the streamed city. Traffic today: cars that follow the lanes of the road
//! network, spawned around the player and removed when they are far away.
//! Specs: `docs/specs/ai-traffic.md`, `docs/specs/ai-traffic-spawning.md`.

mod car;
mod commands;
mod cop;
mod data;
mod manager;
mod models;
mod pattern;
mod population;
mod pursuit;
mod scene;
mod spawn;
mod traffic;

use std::collections::HashSet;
use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use blackbox_render::Instance;
use blackbox_roads::{Body, RoadNetwork, SegmentIndex, SplitMix};
use glam::Vec3;
use nfsmw_data::car::physics::{CarPhysics, SurfaceTable};

use super::drive::{CarRig, CarSim, FixedClock, STEP};
use super::props::PropWorld;
use car::{AiCar, Ctx};
pub(super) use commands::{pursuit_command, traffic_command};
pub use cop::CopCar;
use manager::TypeTimers;
pub(super) use models::load_requested;
pub use pattern::{Pattern, Patterns};
pub use population::{ModelRequest, PatrolPlan};
pub use pursuit::{CopModel, Pursuit};

/// Physics steps a screenshot run simulates per update.
const BATCH_STEPS: u32 = 6;
/// The traffic manager runs twice a second.
const MANAGER_PERIOD: f32 = 0.5;

/// Where the player (or the free camera) is: traffic spawns ahead of it and stays near it.
#[derive(Debug, Clone, Copy)]
pub struct Focus {
    /// Map position (x, y).
    pub position: [f32; 2],
    /// The way it faces or moves, radians from +X.
    pub heading: f32,
    /// Speed along that way, m/s.
    pub speed: f32,
    /// The player's car for the traffic to avoid, in physics space (none for the free camera).
    pub body: Option<Body>,
}

/// A car model traffic can use: the rig is shared by every car of the model.
pub struct TrafficModel {
    pub name: String,
    pub rig: Rc<CarRig>,
    pub physics: CarPhysics,
}

pub struct TrafficWorld {
    network: RoadNetwork,
    index: SegmentIndex,
    models: Vec<TrafficModel>,
    cop_models: Vec<CopModel>,
    pursuit: Option<Pursuit>,
    cars: Vec<AiCar>,
    rng: SplitMix,
    clock: FixedClock,
    /// How many cars the manager keeps on the road.
    target: usize,
    /// The share of them that are patrol cops, in percent.
    cop_share: u32,
    /// The patterns of the neighbourhoods, when the install has them, and their type timers.
    patterns: Option<Patterns>,
    timers: TypeTimers,
    /// The patrol cop cars and how fast they cruise.
    patrol: Option<PatrolPlan>,
    /// Models to load before the cars of that type can spawn; the scene loads them when it has a renderer.
    requests: Vec<ModelRequest>,
    /// Names already asked for (loaded, loading or failed).
    known: HashSet<String>,
    spawn_timer: f32,
    /// Seconds of simulation a screenshot run still has to run in batches (it has no frame time).
    warmup_left: f32,
    /// Simulated seconds since the last debug status line.
    log_timer: f32,
}

impl TrafficWorld {
    pub fn new(network: RoadNetwork) -> Self {
        let index = SegmentIndex::build(&network);
        Self {
            network,
            index,
            models: Vec::new(),
            cop_models: Vec::new(),
            pursuit: None,
            cars: Vec::new(),
            rng: SplitMix(0x6E66_736D_7721),
            clock: FixedClock::default(),
            target: 0,
            cop_share: 0,
            patterns: None,
            timers: TypeTimers::default(),
            patrol: None,
            requests: Vec::new(),
            known: HashSet::new(),
            spawn_timer: 0.0,
            warmup_left: 0.0,
            log_timer: 0.0,
        }
    }

    pub fn add_model(&mut self, model: TrafficModel) {
        self.known.insert(model.name.to_ascii_lowercase());
        self.models.push(model);
    }

    pub fn model_names(&self) -> Vec<&str> {
        self.models.iter().map(|m| m.name.as_str()).collect()
    }

    /// Asks a screenshot run to simulate `seconds` before it captures.
    pub fn set_warmup(&mut self, seconds: f32) {
        // Nothing to simulate without cars to simulate them.
        self.warmup_left = seconds;
    }

    /// Whether a screenshot run may capture: the warm-up is over.
    pub fn settled(&self) -> bool {
        self.warmup_left <= 0.0
    }

    /// Runs the cars for `dt` seconds of frame time and keeps the traffic at its target count around
    /// `focus` (map x, y). `loaded` is false while the map around the player is still streaming in.
    pub fn update(
        &mut self,
        dt: f32,
        focus: Focus,
        loaded: bool,
        collision: &CollisionWorld,
        props: &mut PropWorld,
        surfaces: &SurfaceTable,
    ) {
        if !loaded {
            return;
        }
        if self.cars.is_empty() && self.target == 0 && self.pursuit.is_none() {
            // Nothing to simulate: a screenshot run does not wait for it.
            self.warmup_left = 0.0;
            return;
        }
        // A screenshot run has no frame time: it simulates its warm-up in batches.
        let batch = dt == 0.0 && self.warmup_left > 0.0;
        let finishing = batch && self.warmup_left <= BATCH_STEPS as f32 * STEP;
        let steps = match batch {
            true => {
                self.warmup_left -= BATCH_STEPS as f32 * STEP;
                BATCH_STEPS
            }
            false => self.clock.advance(dt),
        };
        for _ in 0..steps {
            // Every car as the others' trails see it, with whether it is a cop.
            let bodies: Vec<(Body, bool)> = self.cars.iter().map(|c| (c.body(), c.is_cop())).collect();
            for (me, car) in self.cars.iter_mut().enumerate() {
                let mut ctx = Ctx {
                    net: &self.network,
                    index: &self.index,
                    rng: &mut self.rng,
                    bodies: &bodies,
                    me,
                    player: focus.body,
                };
                car.step(&mut ctx, collision, props, surfaces);
            }
        }
        let simulated = match batch {
            true => steps as f32 * STEP,
            false => dt,
        };
        self.update_pursuit(simulated, focus, collision, surfaces);
        self.log_timer += simulated;
        if self.log_timer >= 5.0 && log::log_enabled!(log::Level::Debug) {
            self.log_timer = 0.0;
            log::debug!("traffic status: {}", self.status(focus));
        }
        if finishing {
            log::info!("traffic after the screenshot warm-up: {}", self.status(focus));
        }
        self.recycle(simulated, focus);
        self.spawn_timer += simulated;
        if (batch || self.spawn_timer >= MANAGER_PERIOD) && self.target > 0 {
            self.spawn_timer = 0.0;
            self.manage(MANAGER_PERIOD, focus, collision, surfaces);
        }
    }

    /// Lets the cars hit each other and the player's car (`player`).
    pub fn collide(&mut self, mut player: Option<&mut CarSim>) {
        const NEAR: f32 = 8.0;
        for i in 0..self.cars.len() {
            let (head, tail) = self.cars.split_at_mut(i + 1);
            let car = &mut head[i];
            if let Some(player) = player.as_deref_mut()
                && car.physics_position().distance(Vec3::from(player.collision_box().centre.to_array())) < NEAR
                && let Some(hit) = player.collide_with(car.sim_mut())
            {
                car.on_hit(hit.impulse, true);
            }
            for other in tail {
                if car.physics_position().distance(other.physics_position()) >= NEAR {
                    continue;
                }
                if let Some(hit) = car.sim_mut().collide_with(other.sim_mut()) {
                    car.on_hit(hit.impulse, false);
                    other.on_hit(hit.impulse, false);
                }
            }
        }
    }

    /// Appends the instances of every car.
    pub fn instances(&self, out: &mut Vec<Instance>) {
        let alpha = self.clock.alpha();
        for car in &self.cars {
            car.rig().instances(&car.pose(alpha), out);
        }
    }

    /// Removes every car (the models stay loaded).
    pub fn clear_cars(&mut self) {
        self.cars.clear();
        self.pursuit = None;
    }

    /// Puts requests the scene has not got to yet back.
    pub fn requeue(&mut self, requests: Vec<ModelRequest>) {
        self.requests.extend(requests);
    }

    /// Whether the manager is waiting for a model.
    pub fn wants_models(&self) -> bool {
        !self.requests.is_empty()
    }

    /// One line per car for the `traffic status` command.
    pub fn status(&self, focus: Focus) -> String {
        let mut lines = vec![format!("{} of {} cars; {}", self.cars.len(), self.target, self.pursuit_status())];
        for car in self.cars.iter().take(12) {
            let p = car.position();
            let c = car.controls();
            lines.push(format!(
                "{} at ({:.0}, {:.0}), {:.0} m away, {:.1} m/s, gas {:.0} brake {:.0} steer {:+.2}; {}",
                car.name,
                p.x,
                p.y,
                (p.x - focus.position[0]).hypot(p.y - focus.position[1]),
                car.speed(),
                c.gas,
                c.brake,
                c.steer,
                car.debug()
            ));
        }
        lines.join("\n")
    }
}
