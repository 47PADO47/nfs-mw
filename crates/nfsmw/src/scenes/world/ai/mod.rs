//! Computer-driven cars in the streamed city. Traffic today: cars that follow the lanes of the road
//! network, spawned around the player and removed when they are far away.
//! Specs: `docs/specs/ai-traffic.md`, `docs/specs/ai-traffic-spawning.md`.

mod activation;
mod air;
mod car;
mod collide;
mod collide_guests;
mod collision_sounds;
mod commands;
pub(in crate::scenes::world) mod coop_chase;
mod cop;
mod cop_aims;
mod cop_health;
mod cop_lights;
mod cop_manager;
mod damage_effects;
mod data;
mod entities;
mod goal_aims;
mod hit_count;
mod hitch;
mod inspect;
#[cfg(test)]
pub(in crate::scenes::world) mod installed_support;
mod lamps;
mod manager;
mod model_loader;
mod models;
mod moment_feed;
mod net_export;
mod status;
mod water;
pub(in crate::scenes::world) use water::RainView;
mod nis_poses;
mod original_camera;
mod parallel;
mod pattern;
mod police;
mod policy;
mod population;
mod population_stats;
mod pursuit;
mod pursuit_budget;
mod race_route;
mod racer;
mod racer_route;
mod racers;
mod radar;
mod release;
mod road_barriers;
mod road_reflection;
mod roadblock;
mod scene;
mod search;
mod seek_spawn;
mod shading;
pub(super) mod signal_props;
mod simulation;
mod sirens;
mod spawn;
mod spawn_requests;
mod spikes;
mod state_hash;
mod stop;
mod streams;
mod support;
mod tally;
mod touches;
mod traffic;
#[allow(dead_code)] // the hooks in collide.rs and cop.rs are not wired yet (docs/specs/traffic-sound-events.md Â§4)
mod traffic_events;
mod trailer;

use std::collections::HashSet;
use std::rc::Rc;

use blackbox_render::{EffectVertex, Instance};
use blackbox_roads::{Body, RoadNetwork, SegmentIndex, SignalController};
use glam::Vec3;
use nfsmw_data::car::physics::CarPhysics;

use super::drive::CarRig;
use car::AiCar;
pub(super) use commands::{
    pursuit_command, roadblock_command, scripted_cops, signals_command, speeding_pursuit, traffic_command,
};
pub use cop::CopCar;
pub use lamps::LampMeshes;
use manager::TypeTimers;
pub(super) use models::load_requested;
pub use net_export::{NetCar, NetCarKind};
pub use pattern::{Pattern, Patterns};
pub use police::PlayerView;
pub use population::{ModelRequest, PatrolPlan};
#[cfg(test)]
pub(crate) use pursuit::tests::arrest_fixture::{ArrestFixture, handbrake, open_arrest_fixture};
pub use pursuit::{CopModel, Pursuit};
pub use racers::{ModelState, RacerSpawn};
pub use tally::RoadblockTally;
pub use touches::{CarTouch, Toucher};

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
    /// The world's car render keys: each car placed from the model takes the next (`keys.rs`).
    pub keys: Rc<super::keys::CarKeys>,
}

pub struct TrafficWorld {
    water: water::Water,
    network: RoadNetwork,
    index: SegmentIndex,
    /// The track barrier lines (`road_barriers.rs`).
    track_barriers: Vec<blackbox_roads::TrackBarrier>,
    /// The signals of the junctions and the simulated seconds on their clock.
    signals: SignalController,
    signal_time: f32,
    /// The real traffic-light props that serve the approaches, and the debug switch (`signal_props.rs`).
    signal_lamps: signal_props::SignalLamps,
    /// Cars stop for the lights (the `traffic_lights` setting).
    lights_on: bool,
    /// Cars go round a car that blocks their lane (the `traffic_avoidance` setting; `car/avoid.rs`).
    avoidance: bool,
    models: Vec<TrafficModel>,
    cop_models: Vec<CopModel>,
    /// The worker that reads the car models the traffic asks for, made by its first request (`model_loader.rs`).
    model_loader: Option<model_loader::ModelLoader>,
    /// The police helicopter's rig. It is not a car, so it has no `CarPhysics` (`models.rs`).
    helicopter_rig: Option<Rc<CarRig>>,
    pursuit: Option<Pursuit>,
    /// The backup values of the heat levels, from the install.
    backup_rows: Vec<nfsmw_data::pursuit::BackupRow>,
    police: police::Police,
    /// The cop manager's lockout timer and no-spawn zones (`cop_manager.rs`).
    cops: cop_manager::CopManager,
    /// The cars in entity-id order (ntities.rs).
    cars: Vec<AiCar>,
    /// The entity id the last admitted car took (ntities.rs).
    next_entity: u32,
    /// One random stream per system (`streams.rs`).
    rng: streams::Streams,
    alpha: f32,
    /// How many cars the manager keeps on the road.
    target: usize,
    policy: crate::settings::TrafficPolicy,
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
    /// Names of the models that failed to load.
    failed: HashSet<String>,
    /// Last applied appearance mode; live actors share these uploaded rigs.
    shading: Option<crate::settings::CarShading>,
    spawn_timer: f32,
    population_stats: population_stats::PopulationStats,
    /// Seconds of simulation a screenshot run still has to run in batches (it has no frame time).
    warmup_left: f32,
    /// Simulated seconds since the last debug status line.
    log_timer: f32,
    /// The live pursuit breaker zones: a police car inside one is destroyed (`blackbox_pursuit::breaker`).
    breaker_zones: blackbox_pursuit::breaker::Zones,
    /// The thresholds of the sound and pursuit rules (`traffic_events.rs`).
    events: traffic_events::EventTuning,
    /// Sounds the cars asked for since the scene last took them (`collide.rs`).
    sounds: Vec<crate::audio::traffic::TrafficSound>,
    /// The sirens of the police cars (`sirens.rs`).
    sirens: sirens::Sirens,
    /// Seconds left in which a hit by the player counts as recent.
    hit_timer: f32,
    /// The player's speed (m/s) and whether a patrol cop can see the player, as of the last collision step.
    player_speed: f32,
    /// The player's physics position as of the last collision step (`None` when not driving).
    player_position: Option<Vec3>,
    /// Current simulation camera in physics space for source damage suppression.
    damage_view: Option<Vec3>,
    /// Whether a cop sees the player, as of the last sight test (`collide.rs`).
    seen_by_cops: bool,
    /// Simulated seconds since the last sight test, against `blackbox_pursuit::sight::TEST_PERIOD`.
    sight_clock: f32,
    /// What the player struck since the police last read it (`collide.rs`).
    strikes: collide::Strikes,
    /// The standing roadblocks with their spike strips and sawhorses, and the request lottery (`roadblock.rs`).
    roadblocks: roadblock::Roadblocks,
    spike_patches: Option<[blackbox_pursuit::spikes::Patch; 4]>,
    /// Tires punctured by the strips (`spikes.rs`).
    spike_hits: u32,
    /// The player's collisions with traffic, for the post-race statistics (`hit_count.rs`).
    player_hits: hit_count::HitCount,
    /// The co-op guests' focus and camera this tick (`population/guests.rs`); empty outside co-op.
    guests: Vec<(Focus, crate::viewer::camera::Lens)>,
    /// The car pairs that touched in the last tick (`touches.rs`).
    touches: Vec<CarTouch>,
    /// Which player the population manager spawns around next (`population/guests.rs`).
    spawn_turn: usize,
    /// A co-op chase's targets after the primary (`coop_chase.rs`); empty outside co-op.
    secondary_targets: Vec<(blackbox_pursuit::targets::TargetId, Body)>,
    /// A cop sees a secondary target: the cool-down waits (`coop_chase.rs`).
    secondary_seen: bool,
    /// The world runs a busted meter per target, so the police's own holds (`coop_chase.rs`).
    shared_bust_held: bool,
    /// Per-step scratch, kept so a step does not allocate: every body as the others see it (`simulation.rs`).
    bodies: Vec<(Body, bool)>,
    /// The cops' aims of the step, by car index (`cop_aims.rs`).
    aims: Vec<Option<Vec3>>,
    /// When the step integrates the cars on several threads, and the starts of their steps by car index (`parallel.rs`).
    parallel: parallel::Parallel,
    starts: Vec<Option<super::drive::StepStart>>,
    /// The hits and close passes of the collision step (`collide.rs`).
    hits: Vec<collide::Hit>,
    passes: Vec<collide::Pass>,
    /// The rigs and window states of the last `update_damaged_windows`, kept so the frame does not allocate
    /// (`car/glass.rs`).
    window_requests: std::cell::RefCell<Vec<(Rc<CarRig>, Vec<super::drive::SurfaceDamage>)>>,
    /// The player as the police last saw it, for the radio's feed (`police/speech_feed.rs`).
    speech_view: Option<PlayerView>,
    /// What the chase radio watches: the world's events and the player's attitude (`police/speech_events.rs`).
    speech_watch: police::SpeechWatch,
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
            water: Default::default(),
            signals: SignalController::new(&network),
            signal_time: 0.0,
            signal_lamps: signal_props::SignalLamps::default(),
            lights_on: false,
            avoidance: false,
            network,
            index,
            track_barriers: Vec::new(),
            models: Vec::new(),
            cop_models: Vec::new(),
            model_loader: None,
            helicopter_rig: None,
            pursuit: None,
            backup_rows: Vec::new(),
            police: police::Police::default(),
            cops: Default::default(),
            cars: Vec::new(),
            next_entity: 0,
            rng: streams::Streams::default(),
            alpha: 0.0,
            target: 0,
            policy: Default::default(),
            cop_share: 0,
            patterns: None,
            timers: TypeTimers::default(),
            patrol: None,
            requests: Vec::new(),
            known: HashSet::new(),
            failed: HashSet::new(),
            shading: None,
            spawn_timer: 0.0,
            population_stats: Default::default(),
            warmup_left: 0.0,
            log_timer: 0.0,
            breaker_zones: Default::default(),
            events: traffic_events::EventTuning::default(),
            sounds: Vec::new(),
            sirens: sirens::Sirens::default(),
            hit_timer: 0.0,
            player_speed: 0.0,
            player_position: None,
            damage_view: None,
            seen_by_cops: false,
            sight_clock: 0.0,
            strikes: collide::Strikes::default(),
            roadblocks: Default::default(),
            spike_patches: None,
            spike_hits: 0,
            player_hits: Default::default(),
            guests: Vec::new(),
            touches: Vec::new(),
            spawn_turn: 0,
            secondary_targets: Vec::new(),
            secondary_seen: false,
            shared_bust_held: false,
            bodies: Vec::new(),
            aims: Vec::new(),
            parallel: parallel::Parallel::default(),
            starts: Vec::new(),
            hits: Vec::new(),
            passes: Vec::new(),
            window_requests: Default::default(),
            speech_view: None,
            speech_watch: Default::default(),
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

    /// The heat of the chase, 0 with none.
    pub fn heat_value(&self) -> f32 {
        self.police.heat_value()
    }

    /// The escape's status: the chase, the cool-down, or evaded.
    pub fn cooldown_status(&self) -> blackbox_pursuit::CooldownStatus {
        self.police.cooldown_status()
    }

    /// What a patrol cop knows about the player at `heat`, as of the last collision step.
    pub fn sighting(&self, heat: u32) -> traffic_events::Sighting {
        traffic_events::Sighting {
            heat,
            player_speed: self.player_speed,
            seen: self.seen_by_cops,
            recent_hit: self.hit_timer > 0.0,
            has_911: self.cops.roam.active(),
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

    /// Whether the cars go round a car that blocks their lane (a rewrite extension: the original does not).
    pub fn set_avoidance(&mut self, on: bool) {
        self.avoidance = on;
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

    /// The road graph the cars drive on, and its segment index: the GPS route is searched on it.
    pub fn road_graph(&self) -> (&RoadNetwork, &SegmentIndex) {
        (&self.network, &self.index)
    }

    /// Where the standing roadblocks are, in render space.
    pub fn roadblock_sites(&self) -> Vec<[f32; 2]> {
        self.roadblocks.sites()
    }

    /// The cops for the minimap (`docs/specs/hud-cop-blips.md`). A pursuit is on while the world has one.
    pub fn map_blips(&self, out: &mut Vec<crate::hud::MapBlip>) {
        let pursuing = self.pursuit.is_some();
        self.helicopter_blips(pursuing, out);
        for at in self.roadblock_sites() {
            out.push(crate::hud::MapBlip { style: crate::hud::ROADBLOCK, position: at, active: false, heading: None });
        }
        for car in self.cars.iter().filter(|c| c.is_live_police()) {
            let at = car.position();
            out.push(crate::hud::MapBlip {
                style: crate::hud::COP,
                position: [at.x, at.y],
                active: pursuing && !car.is_patrol(),
                heading: Some(car.heading()),
            });
        }
    }

    /// Appends the sprites of the cop light bars of every car.
    pub fn cop_particles(&self, camera: Vec3, forward: Vec3, out: &mut Vec<blackbox_render::TexturedEffect>) {
        for car in &self.cars {
            car.cop_particles(camera, forward, out);
        }
    }

    /// Authored directional head, brake and reverse flares of every actor and trailer.
    pub fn car_flares(&self, view: super::car_light_flares::FlareView, out: &mut Vec<blackbox_render::TexturedEffect>) {
        for car in &self.cars {
            car.flares(self.alpha, view, out);
        }
    }

    /// Appends the glows of the lamps of every car and of the signal lamps within sight.
    pub fn car_glows(&self, turn_signals: bool, camera: Vec3, forward: Vec3, out: &mut Vec<EffectVertex>) {
        for car in &self.cars {
            car.glows(self.alpha, self.signal_time, turn_signals, camera, forward, out);
        }
    }

    pub fn instances(&self, out: &mut Vec<Instance>) {
        self.instances_with_detail(None, out);
    }
    pub(super) fn instances_with_detail(&self, detail: Option<&super::drive::DetailView>, out: &mut Vec<Instance>) {
        let alpha = self.alpha;
        for car in &self.cars {
            car.instances_with_detail(alpha, detail, out);
        }
        self.police.instances(out);
        self.roadblock_instances(out);
    }

    /// Removes every car (the models stay loaded).
    pub fn clear_cars(&mut self) {
        self.cars.clear();
        self.pursuit = None;
    }

    /// Whether the manager is waiting for a model.
    pub fn wants_models(&self) -> bool {
        !self.requests.is_empty()
    }
}
