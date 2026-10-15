//! Computer-driven cars in the streamed city. Traffic today: cars that follow the lanes of the road
//! network, spawned around the player and removed when they are far away.
//! Specs: `docs/specs/ai-traffic.md`, `docs/specs/ai-traffic-spawning.md`.

mod car;
mod collide;
mod commands;
mod cop;
mod cop_aims;
mod cop_lights;
mod data;
mod hitch;
mod lamps;
mod manager;
mod models;
mod original_camera;
mod pattern;
mod police;
mod population;
mod pursuit;
mod roadblock;
mod scene;
mod simulation;
mod spawn;
mod spikes;
mod stop;
mod support;
mod traffic;
#[allow(dead_code)] // the hooks in collide.rs and cop.rs are not wired yet (docs/specs/traffic-sound-events.md §4)
mod traffic_events;
mod trailer;

use std::collections::HashSet;
use std::rc::Rc;

use blackbox_render::{EffectVertex, Instance};
use blackbox_roads::{Body, RoadNetwork, SegmentIndex, SignalController, SplitMix};
use glam::Vec3;
use nfsmw_data::car::physics::CarPhysics;

use super::drive::CarRig;
use car::AiCar;
pub(super) use commands::{pursuit_command, roadblock_command, speeding_pursuit, traffic_command};
pub use cop::CopCar;
pub use lamps::LampMeshes;
use manager::TypeTimers;
pub(super) use models::load_requested;
pub use pattern::{Pattern, Patterns};
pub use police::PlayerView;
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
    /// Its installed rigid-body behaviour is a tractor, whether or not it currently tows a trailer.
    pub tractor: bool,
    /// The trailer a semi tractor pulls.
    pub trailer: Option<Box<TrafficModel>>,
    /// A cop model's light bars (`None` for traffic).
    pub cop_lights: Option<Rc<cop_lights::CopLightSet>>,
    /// Installed class IDs for the authored traffic producers.
    pub audio_profile: crate::audio::traffic::TrafficProfile,
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
    /// The police helicopter's rig. It is not a car, so it has no `CarPhysics` (`models.rs`).
    helicopter_rig: Option<Rc<CarRig>>,
    pursuit: Option<Pursuit>,
    police: police::Police,
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
    /// The thresholds of the sound and pursuit rules (`traffic_events.rs`).
    events: traffic_events::EventTuning,
    /// Sounds the cars asked for since the scene last took them (`collide.rs`).
    sounds: Vec<crate::audio::traffic::TrafficSound>,
    /// Seconds left in which a hit by the player counts as recent.
    hit_timer: f32,
    /// The player's speed (m/s) and whether a patrol cop can see the player, as of the last collision step.
    player_speed: f32,
    /// The player's physics position as of the last collision step (`None` when not driving).
    player_position: Option<Vec3>,
    /// Whether a cop sees the player, as of the last sight test (`collide.rs`).
    seen_by_cops: bool,
    /// Simulated seconds since the last sight test, against `blackbox_pursuit::sight::TEST_PERIOD`.
    sight_clock: f32,
    /// What the player struck since the police last read it (`collide.rs`).
    strikes: collide::Strikes,
    /// The spike strips of the standing roadblocks, on the road plane (`spikes.rs`). Cleared when the chase ends.
    spikes: Vec<blackbox_pursuit::spikes::Strip>,
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
            helicopter_rig: None,
            pursuit: None,
            police: police::Police::default(),
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
            events: traffic_events::EventTuning::default(),
            sounds: Vec::new(),
            hit_timer: 0.0,
            player_speed: 0.0,
            player_position: None,
            seen_by_cops: false,
            sight_clock: 0.0,
            strikes: collide::Strikes::default(),
            spikes: Vec::new(),
        }
    }

    /// The thresholds the sound and pursuit rules use.
    pub fn events(&self) -> traffic_events::EventTuning {
        self.events
    }

    /// Whether a chase is on.
    pub fn is_pursuing(&self) -> bool {
        self.pursuit.is_some()
    }

    /// What a patrol cop knows about the player at `heat`, as of the last collision step.
    pub fn sighting(&self, heat: u32) -> traffic_events::Sighting {
        traffic_events::Sighting {
            heat,
            player_speed: self.player_speed,
            seen: self.seen_by_cops,
            recent_hit: self.hit_timer > 0.0,
            // No 911 calls in the rewrite yet.
            has_911: false,
        }
    }

    /// The sounds the cars asked for since the last call.
    pub fn take_sounds(&mut self) -> Vec<crate::audio::traffic::TrafficSound> {
        std::mem::take(&mut self.sounds)
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

    /// The cops for the minimap (`docs/specs/hud-cop-blips.md`). A pursuit is on while the world has one.
    pub fn map_blips(&self, out: &mut Vec<crate::hud::MapBlip>) {
        let pursuing = self.pursuit.is_some();
        self.helicopter_blips(pursuing, out);
        for car in self.cars.iter().filter(|c| c.is_cop()) {
            let at = car.position();
            let forward = car.pose(1.0).rotation * Vec3::X;
            out.push(crate::hud::MapBlip {
                style: crate::hud::COP,
                position: [at.x, at.y],
                heading: [forward.x, forward.y],
                active: pursuing,
            });
        }
    }

    /// Appends the sprites of the cop light bars of every car.
    pub fn cop_particles(&self, camera: Vec3, forward: Vec3, out: &mut Vec<blackbox_render::TexturedEffect>) {
        for car in &self.cars {
            car.cop_particles(camera, forward, out);
        }
    }

    /// Appends the headlight beams of every car.
    pub fn car_beams(&self, out: &mut Vec<EffectVertex>) {
        for car in &self.cars {
            car.beams(self.alpha, out);
        }
    }

    /// Appends the glows of the lamps of every car and of the signal lamps within sight.
    pub fn car_glows(&self, turn_signals: bool, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for car in &self.cars {
            car.glows(self.alpha, self.signal_time, turn_signals, camera, forward, out);
        }
    }

    pub fn instances(&self, out: &mut Vec<Instance>) {
        let alpha = self.alpha;
        for car in &self.cars {
            car.instances(alpha, out);
        }
        self.police.instances(out);
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
