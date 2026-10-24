//! The authoritative server loop over any [`WorldSim`] (integration.md section 5, steps 3 and 4).
//!
//! [`Server`] owns the net thread's handle ([`ServerEndpoint`]), the lobby and its rules, the world and the replay
//! recorder. [`Server::tick_once`] is one 60 Hz step with the time given by the caller (tests drive a fake clock);
//! [`Server::run`] calls it from a drift-free timer. One step, in order:
//!
//! 1. **Inbox** (`inbox.rs`): every endpoint event. Reliable messages are decoded and routed (`route.rs`: admission in
//!    `admit.rs`, lobby commands, chat, votes, loadouts, resets); `INPUT` frames go to the player's
//!    [`nfsmw_net::input::InputWindow`]; a `PING` is answered with a `PONG` at once; `Delivered` moves the client's
//!    delta baseline.
//! 2. **Timers** (`drops.rs`): connections whose close is due, players silent for 10 s, resume windows that ended.
//! 3. **Lobby** (`actions.rs`, `phase.rs`): the start countdown, the load barrier, votes; the lobby's actions are
//!    carried out (sends, spawns, kicks, phase changes); the results screen returns to the lobby on its timer.
//! 4. **World** (`world.rs`): the inputs of the tick, [`WorldSim::step`], the events to the mode rules and the tally,
//!    the mode's events to the clients, a finished round to the results.
//! 5. **Snapshots** (`snapshot.rs`): the tick's entity frame, then for each client due a snapshot (player rate, or the
//!    spectator rate): own block, global block, and the replicate body against its last acknowledged view, with
//!    relevancy and the priority budget. The replay recorder (`replay.rs`) writes the same frame.
//! 6. **Housekeeping**: the late-join bulk transfer, the discovery reply, the status line (`stats.rs`).

mod actions;
mod admit;
mod drops;
mod housekeeping;
mod inbox;
mod password;
mod peer;
mod phase;
mod replay;
mod route;
mod send;
mod snapshot;
mod stamp;
mod stats;
mod strikes;
mod view;
mod world;

pub use stats::ServerStats;
pub use strikes::{KICK_SCORE, RESYNC_SCORE, Verdict, Violation, WARN_SCORE};

use std::collections::BTreeMap;
use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use blackbox_net_core::endpoint::{ClientEndpoint, EndpointConfig, ServerEndpoint};
use blackbox_net_core::secure::KeyId;
use blackbox_net_core::session::{PeerId, SessionConfig};
use nfsmw_net::chat::ChatGate;
use nfsmw_net::drop::{Presence, ResumeBook};
use nfsmw_net::fairness::ResetRule;
use nfsmw_net::join::JoinGate;
use nfsmw_net::lobby::{Lobby, LobbyConfig};
use nfsmw_net::schema::GameSchema;
use nfsmw_net::settings::SettingsError;
use nfsmw_net::version::{BuildInfo, DataId, ProtoVersion, ServerPolicy, SimBuild};
use nfsmw_net::vote::VoteBox;
use nfsmw_sim::SimInputs;

use crate::catalog::BoxedCatalog;
use crate::config::ServerConfig;
use crate::control::WorldControl;
use crate::timer::TickTimer;
use peer::Peer;
use phase::RaceState;
use replay::ReplayWriter;
use view::WorldView;

/// The largest snapshot payload: the frame space of a sealed packet (1,169 bytes) less the frame's type and length.
pub(crate) const MAX_SNAPSHOT_PAYLOAD: usize = blackbox_net_core::conn::FRAME_SPACE - 3;

/// What the server is beyond its config: its build, version policy, game data and entity schema.
pub struct ServerSetup {
    pub build: BuildInfo,
    pub policy: ServerPolicy,
    /// The game data loadouts are checked against.
    pub catalog: BoxedCatalog,
    pub schema: GameSchema,
    /// The session seed `JoinAccepted` carries.
    pub session_seed: u64,
}

impl ServerSetup {
    /// The setup of the toy server: [`toy_build`], the default policy and schema, and an open catalogue of
    /// `car_types` cars.
    pub fn toy(car_types: u32) -> Self {
        Self {
            build: toy_build(),
            policy: ServerPolicy::default(),
            catalog: Box::new(crate::catalog::OpenCatalog { car_types }),
            schema: GameSchema::new(),
            session_seed: 0x5EED,
        }
    }
}

/// The build a toy server and its test clients share: this protocol, a sim build named after the toy world, and an
/// all-zero data id (no game install).
pub fn toy_build() -> BuildInfo {
    BuildInfo {
        proto: ProtoVersion::CURRENT,
        sim_build: SimBuild::from_parts("toy-world", "any"),
        data_id: DataId { bytes: [0; 16] },
    }
}

/// Something that runs before each tick of [`Server::run`]: the bot clients, an admin console.
pub trait TickHook<W: WorldControl> {
    /// Called with the server and the tick's time before the tick runs.
    fn before_tick(&mut self, server: &mut Server<W>, now_ms: u64);
}

/// The authoritative server of one session.
pub struct Server<W: WorldControl> {
    config: ServerConfig,
    setup: ServerSetup,
    endpoint: ServerEndpoint,
    world: W,
    lobby: Lobby,
    gate: JoinGate,
    book: ResumeBook,
    presence: Presence,
    votes: VoteBox,
    chat: ChatGate,
    resets: ResetRule,
    peers: BTreeMap<PeerId, Peer>,
    /// The connection of each seated slot.
    slots: BTreeMap<u8, PeerId>,
    race: RaceState,
    frames: WorldView,
    inputs: SimInputs,
    replay: Option<ReplayWriter>,
    stats: ServerStats,
    now_ms: u64,
    shutting_down: bool,
    ended: bool,
    stop: Arc<AtomicBool>,
}

impl<W: WorldControl> Server<W> {
    /// A server for `config` over `world`, talking through `endpoint` (bound by [`bind`] or spawned over a test
    /// transport). `now_ms` is the caller's clock at the start.
    pub fn new(
        mut config: ServerConfig,
        setup: ServerSetup,
        endpoint: ServerEndpoint,
        world: W,
        now_ms: u64,
    ) -> Result<Self, SettingsError> {
        config.settings.password_required |= config.password.is_some();
        let lobby_config = LobbyConfig {
            session: SessionConfig::default(),
            countdown_seconds: config.countdown_seconds,
            auto_start: config.auto_start,
            ..LobbyConfig::default()
        };
        let lobby = Lobby::new(lobby_config, config.settings.clone(), config.password.is_some())?;
        let replay = config.replay_dir.as_deref().and_then(|dir| {
            ReplayWriter::create(dir, &setup, world.tick())
                .map_err(|err| log::warn!("no replay: cannot write to {}: {err}", dir.display()))
                .ok()
        });
        let mut server = Self {
            frames: WorldView::new(config.relevancy_radius),
            config,
            setup,
            endpoint,
            world,
            lobby,
            gate: JoinGate::new(),
            book: ResumeBook::new(),
            presence: Presence::new(),
            votes: VoteBox::new(),
            chat: ChatGate::new(),
            resets: ResetRule::new(),
            peers: BTreeMap::new(),
            slots: BTreeMap::new(),
            race: RaceState::default(),
            inputs: SimInputs::default(),
            replay,
            stats: ServerStats::default(),
            now_ms,
            shutting_down: false,
            ended: false,
            stop: Arc::new(AtomicBool::new(false)),
        };
        server.refresh_discovery();
        log::info!(
            "server \"{}\" up: mode {:?}, {} players, {} Hz snapshots",
            server.config.settings.name.as_str(),
            server.config.settings.mode,
            server.config.settings.max_players,
            server.config.snapshot_hz
        );
        Ok(server)
    }

    /// The config the server runs with.
    pub fn config(&self) -> &ServerConfig {
        &self.config
    }

    /// The world.
    pub fn world(&self) -> &W {
        &self.world
    }

    /// The world, to change it from outside the loop (the game's console, tests).
    pub fn world_mut(&mut self) -> &mut W {
        &mut self.world
    }

    /// The lobby.
    pub fn lobby(&self) -> &Lobby {
        &self.lobby
    }

    /// The entity schema.
    pub fn schema(&self) -> &GameSchema {
        &self.setup.schema
    }

    /// The counters.
    pub fn stats(&self) -> &ServerStats {
        &self.stats
    }

    /// The net thread's handle.
    pub fn endpoint(&self) -> &ServerEndpoint {
        &self.endpoint
    }

    /// A flag that stops [`Server::run`] when set, from any thread.
    pub fn stop_flag(&self) -> Arc<AtomicBool> {
        Arc::clone(&self.stop)
    }

    /// Whether the session ended (the listen host left).
    pub fn has_ended(&self) -> bool {
        self.ended
    }

    /// Attaches a client in this process (the listen host's own client, a bot) through the in-memory link.
    pub fn connect_local(&mut self, key: KeyId, config: EndpointConfig) -> io::Result<ClientEndpoint> {
        self.endpoint.connect_local(key, config, None)
    }

    /// Snapshot bytes sent to `slot`'s connection so far, and the snapshots.
    pub fn snapshot_traffic(&self, slot: u8) -> Option<(u64, u64)> {
        let peer = self.peers.get(self.slots.get(&slot)?)?;
        Some((peer.snapshot_bytes, peer.snapshots))
    }

    /// One 60 Hz step at `now_ms` (module docs).
    pub fn tick_once(&mut self, now_ms: u64) {
        self.now_ms = self.now_ms.max(now_ms);
        let started = std::time::Instant::now();
        self.drain_inbox();
        self.run_timers();
        self.update_lobby();
        self.step_world();
        self.send_snapshots();
        self.send_bulk();
        self.housekeeping();
        self.stats.on_tick(started.elapsed());
    }

    /// Runs the loop at 60 Hz until the stop flag is set, the session ends, or `seconds` pass. `hook` runs before each
    /// tick. Shuts down cleanly at the end.
    pub fn run(&mut self, seconds: Option<u64>, mut hook: Option<&mut dyn TickHook<W>>) {
        let mut timer = TickTimer::new(nfsmw_sim::TICK_HZ);
        let limit = seconds.map(|seconds| seconds.saturating_mul(1000));
        loop {
            timer.wait();
            let now_ms = timer.now_ms();
            if self.stop.load(Ordering::Relaxed) || self.ended || limit.is_some_and(|limit| now_ms >= limit) {
                break;
            }
            if let Some(hook) = hook.as_deref_mut() {
                hook.before_tick(self, now_ms);
            }
            self.tick_once(now_ms);
            self.stats.skipped_ticks = timer.skipped();
        }
        self.shutdown();
    }

    /// Closes every connection, finishes the replay and logs the totals. Idempotent.
    pub fn shutdown(&mut self) {
        if self.shutting_down {
            return;
        }
        self.shutting_down = true;
        if let Some(replay) = self.replay.as_mut() {
            replay.finish();
        }
        self.endpoint.shutdown();
        log::info!("server down: {}", self.stats.summary());
    }
}

/// Binds the game and discovery ports of `config` with the LAN handshake of this protocol and `identity`.
pub fn bind(config: &ServerConfig, identity: blackbox_net_core::secure::IdentityKey) -> io::Result<ServerEndpoint> {
    ServerEndpoint::bind(config.game_addr(), config.discovery_addr(), endpoint_config(identity))
}

/// The server endpoint config of this protocol: the LAN handshake limits, the protocol's prologue and `identity`.
pub fn endpoint_config(
    identity: blackbox_net_core::secure::IdentityKey,
) -> blackbox_net_core::endpoint::ServerEndpointConfig {
    use nfsmw_net::version::{PROTO_MAJOR, noise_prologue};
    let handshake = blackbox_net_core::secure::ServerConfig::lan(PROTO_MAJOR, noise_prologue(PROTO_MAJOR).as_bytes());
    blackbox_net_core::endpoint::ServerEndpointConfig::lan(handshake, identity)
}
