//! The host: a `WorldScene` under a session's rules, drawn by nobody, stepped one tick at a time.

use std::time::{Duration, Instant};

use anyhow::{Result, bail};
use blackbox_gfx::{Capabilities, GraphicsApi};
use game_install::GameDir;
use nfsmw_sim::{PlayerId, SimEvent, Tick};

use super::super::drive::{DriveInput, STEP};
use super::super::players::LEAD;
use super::super::props::MovingProp;
use super::super::session_rules::SessionRules;
use super::super::{DriveOptions, Options, WorldScene};
use super::catalog::CarCatalog;
use super::events::{MovedTracker, PursuitTracker, RaceTracker};
use super::ids::EntityTable;
use super::seat::MAX_PLAYERS;
use crate::app::null_backend::NullBackend;
use crate::input::ActionState;
use crate::settings::{Percent, TrafficSettings};
use crate::viewer::Scene;

/// How a host's world starts.
#[derive(Debug, Clone)]
pub struct HostConfig {
    /// Where the world starts: map position (x, y) in render space, and the heading the host's car faces (degrees).
    pub at: [f32; 2],
    pub heading: f32,
    /// The host's own car, `CAR[:PRESET]`: the car of `PlayerId(0)`, the world's lead.
    pub car: String,
    /// The session seed (`SessionRules::session`).
    pub seed: u64,
    /// The car budget of the traffic and the police's share of it in percent; `None` keeps the world's settings.
    pub traffic: Option<(u32, u8)>,
    /// Wait before every tick, at most this long, until every tile the world wants is resident and none is in flight
    /// (lockstep streaming, so two hosts with the same inputs stay equal). `None` never waits: while the players'
    /// surroundings are not resident the world holds still (its own loading gate) and the tick passes.
    pub settle_streaming: Option<Duration>,
    /// Ticks the opening may take to put the host's car on the road.
    pub open_ticks: u32,
}

impl HostConfig {
    /// A session from `seed` at `at`, in `car`, with the world's traffic, never waiting for streaming.
    pub fn new(at: [f32; 2], car: impl Into<String>, seed: u64) -> Self {
        Self { at, heading: 0.0, car: car.into(), seed, traffic: None, settle_streaming: None, open_ticks: 600 }
    }
}

/// The real world behind the `WorldSim` seam (`sim.rs`).
pub struct WorldSimHost {
    pub(super) scene: WorldScene,
    pub(super) renderer: NullBackend,
    pub(super) tick: Tick,
    pub(super) ids: EntityTable,
    pub(super) catalog: CarCatalog,
    /// The props in motion after the last tick: found once per tick (a pass over every prop collider of the loaded
    /// tiles) for the entity table and the export both.
    pub(super) props: Vec<MovingProp>,
    /// The host's own player has joined (its car is the lead and is always in the world).
    pub(super) lead_joined: bool,
    /// Players whose next tick resets their car.
    pub(super) resets: Vec<PlayerId>,
    pub(super) pursuit: PursuitTracker,
    pub(super) race: RaceTracker,
    pub(super) moved: MovedTracker,
    /// Ticks whose streaming wait ran out.
    pub(super) stalled: u32,
    pub(super) settle: Option<Duration>,
    /// The tick the server wants the staged race's GO on, until the countdown is released (`race/hold.rs`).
    pub(super) race_go_at: Option<Tick>,
}

impl WorldSimHost {
    /// Open the world of `dir` for a session: the scene with the host's car, a renderer that draws nothing, the session's
    /// rules, one tick per step, and the host's car on the road. The world's own ticks before that are not counted.
    pub fn open(dir: &GameDir, config: HostConfig) -> Result<Self> {
        let options = Options {
            at: Some(config.at),
            height: 40.0,
            heading: config.heading,
            pitch: -20.0,
            fog_distance: 1500.0,
            wait_for_load: true,
            drive: Some(DriveOptions { car: config.car.clone(), script: None }),
        };
        let mut scene = WorldScene::open(dir, options)?;
        let mut renderer = NullBackend::new(Capabilities::baseline("null", GraphicsApi::Vulkan));
        scene.init(&mut renderer)?;
        scene.set_session_rules(SessionRules::session(config.seed));
        if let Some((cars, cop_share)) = config.traffic {
            let setting = TrafficSettings { cars, cop_share: Percent(cop_share), ..scene.traffic_setting };
            scene.apply_traffic_setting(setting);
        }
        scene.physics_clock.set_one_per_update(true);
        let mut host = Self {
            scene,
            renderer,
            tick: 0,
            ids: EntityTable::default(),
            catalog: CarCatalog::new(nfsmw_data::car::list(dir)),
            props: Vec::new(),
            lead_joined: false,
            resets: Vec::new(),
            pursuit: PursuitTracker::default(),
            race: RaceTracker::default(),
            moved: MovedTracker::default(),
            stalled: 0,
            settle: config.settle_streaming,
            race_go_at: None,
        };
        host.place_lead(config.open_ticks)?;
        let mut ignored = Vec::new();
        host.collect_events(&mut ignored);
        Ok(host)
    }

    /// Runs the world until the host's car stands on the road.
    fn place_lead(&mut self, ticks: u32) -> Result<()> {
        for _ in 0..ticks {
            if let Some(drive) = self.scene.players.lead_mut() {
                drive.set_remote_input(Some(DriveInput::default()));
            }
            self.wait_for_streaming();
            self.scene.update(&mut self.renderer, &ActionState::default(), STEP);
            let placed = self.scene.players.lead().is_some_and(|d| d.has_sim() && d.waiting_for_road().is_none());
            if placed {
                return Ok(());
            }
        }
        bail!("the host's car {} found no road in {ticks} ticks", self.scene.last_car)
    }

    /// The world, to read.
    pub fn scene(&self) -> &WorldScene {
        &self.scene
    }

    /// The world, to change between ticks (console commands, a race to start).
    pub fn scene_mut(&mut self) -> (&mut WorldScene, &mut NullBackend) {
        (&mut self.scene, &mut self.renderer)
    }

    /// The car catalogue the type ids and numbers come from.
    pub fn catalog(&self) -> &CarCatalog {
        &self.catalog
    }

    /// Reset player `id`'s car on the next tick (the reset key: back on the nearest road, or the catch-up warp while the
    /// prompt is offered). `InputState` has no reset, so it comes as a request.
    pub fn request_reset(&mut self, id: PlayerId) {
        if !self.resets.contains(&id) {
            self.resets.push(id);
        }
    }

    /// Ticks whose wait for streaming ran out (`HostConfig::settle_streaming`).
    pub fn stalled_ticks(&self) -> u32 {
        self.stalled
    }

    /// Hand every player in the world (parked ones too) its input for the next tick: the seat's, else neutral, with a
    /// requested reset.
    pub(super) fn feed_inputs(&mut self, inputs: &nfsmw_sim::SimInputs) {
        for n in 0..MAX_PLAYERS {
            let id = PlayerId(n);
            let Some(drive) = self.scene.players.get_mut(id) else { continue };
            let mut input = inputs.get(id).map(DriveInput::from_vehicle).unwrap_or_default();
            input.reset = self.resets.contains(&id);
            drive.set_remote_input(Some(input));
        }
        self.resets.clear();
    }

    /// Waits for streaming as the configuration asks; counts a wait that ran out.
    pub(super) fn wait_for_streaming(&mut self) {
        let Some(limit) = self.settle else { return };
        if !settle_streaming(&mut self.scene, &mut self.renderer, limit) {
            self.stalled = self.stalled.saturating_add(1);
        }
    }

    /// The lead's input before it joined, and after it left: neutral.
    pub(super) fn idle_lead(&mut self) {
        if let Some(drive) = self.scene.players.get_mut(LEAD) {
            drive.set_remote_input(Some(DriveInput::default()));
        }
    }

    /// The events of the tick that just ran (`events/`), and the entity ids of what the world now has.
    pub(super) fn collect_events(&mut self, events: &mut Vec<SimEvent>) {
        self.scene.residency.props().moving_props(&mut self.props);
        self.ids.refresh(super::export::natives(&self.scene, &self.props));
        let contacts = std::mem::take(&mut self.scene.players.contacts);
        let ids = &self.ids;
        super::events::contact_events(&contacts, |party| super::sim::party_id(ids, party), events);
        let chase = super::sim::chase_state(&self.scene);
        self.pursuit.update(&chase, events);
        let racers = super::sim::racer_states(&self.scene);
        self.race.update(racers.as_deref(), events);
        let cars = super::sim::car_spots(&self.scene);
        self.moved.update(&cars, events);
    }
}

/// Waits until every tile the world wants is resident and no request is in flight, so a tick sees the same world on
/// every host; `false` when `limit` ran out first.
pub(in crate::scenes::world) fn settle_streaming(
    scene: &mut WorldScene,
    renderer: &mut NullBackend,
    limit: Duration,
) -> bool {
    let start = Instant::now();
    loop {
        // The frame asks for the traffic's support tiles before its streaming; asked here first, they are resident
        // when the frame looks, so its traffic does not wait on the disk in one host and not in another.
        scene.prepare_traffic_support();
        let centres = scene.foci();
        scene.residency.update_centres(renderer, &centres);
        if scene.residency.complete() && scene.residency.counts().1 == 0 {
            return true;
        }
        if start.elapsed() >= limit {
            return false;
        }
        std::thread::sleep(Duration::from_millis(1));
    }
}
