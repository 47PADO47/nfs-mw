//! Computer-driven cars in the streamed city. Traffic today: cars that follow the lanes of the road
//! network, spawned around the player and removed when they are far away.
//! Specs: `docs/specs/ai-traffic.md`, `docs/specs/ai-traffic-spawning.md`.

mod car;
mod collide;
mod commands;
mod cop;
mod data;
mod hitch;
mod lamps;
mod manager;
mod models;
mod pattern;
mod population;
mod pursuit;
mod scene;
mod simulation;
mod spawn;
mod stop;
mod traffic;
mod trailer;

use std::collections::HashSet;
use std::rc::Rc;

use blackbox_render::{EffectVertex, Instance};
use blackbox_roads::{Body, RoadNetwork, SegmentIndex, SignalController, SplitMix};
use glam::Vec3;
use nfsmw_data::car::physics::CarPhysics;

use super::drive::CarRig;
use car::AiCar;
pub(super) use commands::{pursuit_command, traffic_command};
pub use cop::CopCar;
pub use lamps::LampMeshes;
use manager::TypeTimers;
pub(super) use models::load_requested;
pub use pattern::{Pattern, Patterns};
pub use population::{ModelRequest, PatrolPlan};
pub use pursuit::{CopModel, Pursuit};

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
    /// The trailer a semi tractor pulls.
    pub trailer: Option<Box<TrafficModel>>,
}

pub struct TrafficWorld {
    network: RoadNetwork,
    index: SegmentIndex,
    /// The signals of the junctions and the simulated seconds on their clock.
    signals: SignalController,
    signal_time: f32,
    /// Cars stop for the lights (the `traffic_lights` setting).
    lights_on: bool,
    models: Vec<TrafficModel>,
    cop_models: Vec<CopModel>,
    pursuit: Option<Pursuit>,
    cars: Vec<AiCar>,
    rng: SplitMix,
    alpha: f32,
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
    breaker_areas: Vec<super::props::scenery::StopArea>,
}

impl TrafficWorld {
    pub fn update_events(
        &mut self,
        props: &mut super::props::PropWorld,
        data: &nfsmw_data::car::physics::PhysicsData,
        view: glam::Vec3,
    ) {
        for car in &mut self.cars {
            car.update_events(props, data, view);
        }
    }

    pub fn new(network: RoadNetwork) -> Self {
        let index = SegmentIndex::build(&network);
        Self {
            signals: SignalController::new(&network),
            signal_time: 0.0,
            lights_on: false,
            network,
            index,
            models: Vec::new(),
            cop_models: Vec::new(),
            pursuit: None,
            cars: Vec::new(),
            rng: SplitMix(0x6E66_736D_7721),
            alpha: 0.0,
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
            breaker_areas: Vec::new(),
        }
    }

    pub fn add_model(&mut self, model: TrafficModel) {
        self.known.insert(model.name.to_ascii_lowercase());
        self.models.push(model);
    }

    pub fn model_names(&self) -> Vec<&str> {
        self.models.iter().map(|m| m.name.as_str()).collect()
    }

    /// Whether the cars stop for the traffic lights (a rewrite extension: the original has none).
    pub fn set_lights(&mut self, on: bool) {
        self.lights_on = on;
    }

    /// Asks a screenshot run to simulate `seconds` before it captures.
    pub fn set_warmup(&mut self, seconds: f32) {
        // Nothing to simulate without cars to simulate them.
        self.warmup_left = seconds;
    }

    /// Whether a screenshot run may capture: the warm-up is over.
    pub fn settled(&self) -> bool {
        self.warmup_left <= 0.0 || !self.has_work()
    }

    /// Appends the headlight beams of every car.
    pub fn car_beams(&self, out: &mut Vec<EffectVertex>) {
        for car in &self.cars {
            car.beams(self.alpha, out);
        }
    }

    /// Appends the glows of the lamps of every car and of the signal lamps within sight.
    pub fn car_glows(&self, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for car in &self.cars {
            car.glows(self.alpha, self.signal_time, camera, forward, out);
        }
    }

    /// Appends the instances of every car.
    pub fn instances(&self, out: &mut Vec<Instance>) {
        let alpha = self.alpha;
        for car in &self.cars {
            car.instances(alpha, out);
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
