//! Computer-driven cars in the streamed city. Traffic today: cars that follow the lanes of the road
//! network, spawned around the player and removed when they are far away.
//! Specs: `docs/specs/ai-traffic.md`, `docs/specs/ai-traffic-spawning.md`.

mod car;
mod scene;
mod spawn;
mod traffic;

use std::rc::Rc;

use blackbox_collision::CollisionWorld;
use blackbox_render::{Instance, Renderer};
use blackbox_roads::{Body, RoadNetwork, SegmentIndex, SplitMix};
use glam::Vec3;
use nfsmw_data::car::physics::{CarPhysics, SurfaceTable};

use super::drive::{CarRig, CarSim, FixedClock, STEP};
use super::props::PropWorld;
use car::AiCar;

/// Cars farther than this from the player (metres) are removed.
const DESPAWN_DISTANCE: f32 = 350.0;
/// A car with no wheel on the ground for this long (seconds) fell off the map and is removed.
const MAX_AIRBORNE: f32 = 4.0;
/// A car standing this long (seconds) is removed when it is more than `STUCK_DISTANCE` metres from the player.
const STUCK_TIME: f32 = 20.0;
const STUCK_DISTANCE: f32 = 40.0;
/// Physics steps a screenshot run simulates per update.
const BATCH_STEPS: u32 = 6;
/// Seconds between spawn attempts.
const SPAWN_PERIOD: f32 = 0.5;

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
    cars: Vec<AiCar>,
    rng: SplitMix,
    clock: FixedClock,
    /// How many cars the manager keeps on the road.
    target: usize,
    spawn_timer: f32,
    /// Seconds of simulation a screenshot run still has to run in batches (it has no frame time).
    warmup_left: f32,
}

impl TrafficWorld {
    pub fn new(network: RoadNetwork) -> Self {
        let index = SegmentIndex::build(&network);
        Self {
            network,
            index,
            models: Vec::new(),
            cars: Vec::new(),
            rng: SplitMix(0x6E66_736D_7721),
            clock: FixedClock::default(),
            target: 0,
            spawn_timer: 0.0,
            warmup_left: 0.0,
        }
    }

    pub fn has_models(&self) -> bool {
        !self.models.is_empty()
    }

    pub fn add_model(&mut self, model: TrafficModel) {
        self.models.push(model);
    }

    pub fn model_names(&self) -> Vec<&str> {
        self.models.iter().map(|m| m.name.as_str()).collect()
    }

    /// Asks a screenshot run to simulate `seconds` before it captures.
    pub fn set_warmup(&mut self, seconds: f32) {
        // Nothing to simulate without cars to simulate them.
        self.warmup_left = if self.target > 0 && self.has_models() { seconds } else { 0.0 };
    }

    /// Whether a screenshot run may capture: the warm-up is over.
    pub fn settled(&self) -> bool {
        self.warmup_left <= 0.0
    }

    pub fn set_target(&mut self, target: usize) {
        self.target = target;
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
        if !loaded || (self.cars.is_empty() && self.target == 0) {
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
            // Every car as the others' trails see it: the traffic, then the player.
            let mut bodies: Vec<Body> = self.cars.iter().map(AiCar::body).collect();
            bodies.extend(focus.body);
            for (i, car) in self.cars.iter_mut().enumerate() {
                car.step(&self.network, &mut self.rng, (&bodies, i), collision, props, surfaces);
            }
        }
        if finishing {
            log::info!("traffic after the screenshot warm-up: {}", self.status(focus));
        }
        self.cars.retain(|car| {
            let p = car.position();
            let away = (p.x - focus.position[0]).hypot(p.y - focus.position[1]);
            // A car that has been stuck for a long time (a jam at a junction) goes once the player is not near.
            car.is_finite()
                && car.airborne_time() < MAX_AIRBORNE
                && away < DESPAWN_DISTANCE
                && !(car.idle_time() > STUCK_TIME && away > STUCK_DISTANCE)
        });
        self.spawn_timer += dt;
        if (batch || self.spawn_timer >= SPAWN_PERIOD) && self.cars.len() < self.target {
            self.spawn_timer = 0.0;
            self.spawn_one(focus, collision, surfaces);
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

    /// Removes every car and frees the models.
    pub fn clear(&mut self, renderer: &mut Renderer) {
        self.cars.clear();
        self.target = 0;
        for model in self.models.drain(..) {
            if let Ok(rig) = Rc::try_unwrap(model.rig) {
                rig.release(renderer);
            }
        }
    }

    /// One line per car for the `traffic status` command.
    pub fn status(&self, focus: Focus) -> String {
        let mut lines = vec![format!("{} of {} cars", self.cars.len(), self.target)];
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
