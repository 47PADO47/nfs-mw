//! The [`ClientSession`] struct, its constructor and what the game reads from it.

use blackbox_net_core::endpoint::EndpointStats;
use blackbox_net_core::secure::password::PakeConfirm;
use nfsmw_sim::{EntityId, PlayerId, Tick};

use super::config::SessionConfig;
use super::events::{EventGate, SessionEvent};
use super::link::ClientLink;
use super::lobby::LobbyView;
use super::own_car::OwnCar;
use super::parts::PartBuffer;
use super::remote::Remotes;
use super::status::Status;
use super::views::Views;
use crate::clock::NetClock;
use crate::drop::{BulkReceiver, SESSION_DATA_LIMIT};
use crate::frames::GlobalBlock;
use crate::interp::{Drawn, InterpConfig};
use crate::message::{InvalidMessages, RaceResults};
use crate::netstats::{NetLine, NetStats};
use crate::predict::{Pose, PredictedCar};
use crate::schema::GameSchema;
use crate::spectate::{Camera, FollowCam};

/// Makes a new link to the same server for a reconnect, or `None` when it cannot right now.
pub type Connector<L> = Box<dyn FnMut() -> Option<L> + Send>;

/// The plain global state of the newest snapshot: session phase, pursuit and race state, time of day.
pub type GlobalView = GlobalBlock;

/// What admission gave this connection.
#[derive(Debug, Clone, Default)]
pub(super) struct Seat {
    pub slot: Option<u8>,
    pub entity: Option<EntityId>,
    pub resume_token: Option<Vec<u8>>,
    /// The token is being presented on the current connection.
    pub resuming: bool,
    pub snapshot_hz: u8,
    pub proto_minor: u16,
}

/// The password exchange in flight.
#[derive(Default)]
pub(super) struct Pake {
    pub confirm: Option<PakeConfirm>,
    /// The Noise handshake hash the connection reported, when its link has no hash of its own.
    pub hash: Option<[u8; 32]>,
}

/// Counters of what the session refused or lost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SessionCounters {
    /// Unreliable frames that did not decode, or of an unknown type.
    pub bad_frames: u64,
    /// Reliable messages that did not decode or came out of turn.
    pub bad_messages: u64,
    /// Snapshots whose baseline was gone.
    pub missing_baselines: u64,
    /// Snapshot bodies that did not decode for another reason.
    pub bad_snapshots: u64,
    /// Snapshots decoded.
    pub snapshots: u64,
    /// Reconnect attempts.
    pub reconnects: u64,
    /// INPUT frames sent.
    pub inputs_sent: u64,
    /// Snapshots given up with parts missing.
    pub parts_dropped: u64,
    /// Snapshot parts refused as duplicates or inconsistent with their siblings.
    pub part_conflicts: u64,
    /// Entity records the schema refused.
    pub bad_records: u64,
}

/// The client side of a multiplayer session: everything between the network link and the game. Pure apart from the
/// link: the caller passes the time, and nothing here reads a clock or blocks.
///
/// Generic over the link (`ClientEndpoint`, or a [`super::MockLink`] in tests) and the predicted car (the game's
/// vehicle, or `ToyCar` for tests and bots).
pub struct ClientSession<L: ClientLink, C: PredictedCar> {
    pub(super) link: Option<L>,
    pub(super) connector: Option<Connector<L>>,
    pub(super) config: SessionConfig,
    pub(super) status: Status,
    pub(super) schema: GameSchema,
    pub(super) clock: NetClock,
    pub(super) stats: NetStats,
    pub(super) endpoint_stats: Option<EndpointStats>,
    pub(super) own: OwnCar<C>,
    pub(super) remotes: Remotes,
    pub(super) parts: PartBuffer,
    pub(super) views: Views,
    pub(super) global: Option<(Tick, GlobalBlock)>,
    pub(super) lobby: LobbyView,
    pub(super) gate: EventGate,
    pub(super) events: Vec<SessionEvent>,
    pub(super) bulk: BulkReceiver,
    pub(super) invalid: InvalidMessages,
    pub(super) seat: Seat,
    pub(super) pake: Pake,
    pub(super) camera: FollowCam,
    pub(super) results: Option<RaceResults>,
    pub(super) counters: SessionCounters,
    pub(super) now_ms: u64,
    pub(super) last_reconnect_ms: Option<u64>,
    pub(super) last_baseline_report_ms: Option<u64>,
    pub(super) out: Vec<u8>,
}

impl<L: ClientLink, C: PredictedCar + Clone> ClientSession<L, C> {
    /// A session over `link`, which is connecting. `car` is the player's car built from the loadout; the first own
    /// block of a snapshot places it.
    pub fn new(link: L, config: SessionConfig, car: C) -> Self {
        let tuning = config.tuning;
        Self {
            link: Some(link),
            connector: None,
            own: OwnCar::new(car, tuning, config.uses_clutch()),
            remotes: Remotes::new(InterpConfig::from_tuning(&tuning)),
            stats: NetStats::new(tuning.snapshot_interval_ticks()),
            lobby: LobbyView::new(config.chat_lines),
            config,
            status: Status::Connecting,
            schema: GameSchema::new(),
            clock: NetClock::new(),
            endpoint_stats: None,
            parts: PartBuffer::default(),
            views: Views::new(),
            global: None,
            gate: EventGate::default(),
            events: Vec::new(),
            bulk: BulkReceiver::new(SESSION_DATA_LIMIT),
            invalid: InvalidMessages::new(),
            seat: Seat::default(),
            pake: Pake::default(),
            camera: FollowCam::new(),
            results: None,
            counters: SessionCounters::default(),
            now_ms: 0,
            last_reconnect_ms: None,
            last_baseline_report_ms: None,
            out: Vec::new(),
        }
    }

    /// The same, with a way to open a new link for a reconnect after a drop.
    pub fn with_connector(mut self, connector: Connector<L>) -> Self {
        self.connector = Some(connector);
        self
    }

    pub fn status(&self) -> &Status {
        &self.status
    }

    /// The lobby mirror.
    pub fn lobby(&self) -> &LobbyView {
        &self.lobby
    }

    /// The global block of the newest snapshot, with its tick.
    pub fn global(&self) -> Option<(Tick, &GlobalView)> {
        self.global.as_ref().map(|(tick, block)| (*tick, block))
    }

    /// The events since the last call, oldest first.
    pub fn take_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.events)
    }

    /// The seat's slot.
    pub fn slot(&self) -> Option<u8> {
        self.seat.slot
    }

    /// The entity id of the player's car.
    pub fn own_entity(&self) -> Option<EntityId> {
        self.seat.entity
    }

    /// Whether this client watches instead of driving: it joined as a spectator, or the lobby made it one.
    pub fn is_spectator(&self) -> bool {
        self.config.spectate_only() || self.lobby.my_role() == crate::message::Role::Spectator
    }

    /// The owner's car as it is drawn now (physics plus the smoothed correction); `None` before the first own block
    /// or for a spectator.
    pub fn own_pose(&self) -> Option<Pose> {
        self.own.pose()
    }

    /// The owner's predicted car.
    pub fn own_car(&self) -> Option<&C> {
        self.own.predictor().map(|predictor| predictor.car())
    }

    /// Every remote entity (other players, traffic, police, props) drawn at the display time, into `display`
    /// (cleared first).
    pub fn remote_entities(&mut self, display: &mut Vec<(EntityId, Drawn)>) {
        display.clear();
        let Some(server_tick) = self.clock.server_tick() else {
            return;
        };
        self.remotes.sample(server_tick, display);
    }

    /// The spectator camera's choice.
    pub fn camera(&self) -> Camera {
        self.camera.camera()
    }

    /// The car entity the spectator camera looks at: the followed racer, or for the leader camera the first racer in
    /// slot order (the client has no race order of other players yet); `None` for a free camera. Players are seated
    /// in the world with their slot as the player id.
    pub fn follow_target(&self) -> Option<EntityId> {
        let racers = self.lobby.racers();
        let slot = self.camera.target(&racers)?;
        self.remotes.entity_of(PlayerId(slot))
    }

    /// The last race result record.
    pub fn last_results(&self) -> Option<&RaceResults> {
        self.results.as_ref()
    }

    /// The `net` overlay line (MP-82).
    pub fn net_line(&mut self) -> NetLine {
        self.stats.line(self.now_ms)
    }

    /// The overlay counters.
    pub fn net_stats(&self) -> &NetStats {
        &self.stats
    }

    /// The net thread's last counters.
    pub fn endpoint_stats(&self) -> Option<&EndpointStats> {
        self.endpoint_stats.as_ref()
    }

    /// What the session refused or lost.
    pub fn counters(&self) -> SessionCounters {
        SessionCounters {
            parts_dropped: self.parts.dropped(),
            part_conflicts: self.parts.conflicts(),
            bad_records: self.remotes.bad_records(),
            ..self.counters
        }
    }

    /// `S_est`, the estimated server tick, once the clock is synced.
    pub fn server_tick(&self) -> Option<f64> {
        self.clock.server_tick()
    }

    /// The input lead in ticks, once prediction runs.
    pub fn lead_ticks(&self) -> Option<u32> {
        self.own.lead_ticks()
    }

    /// The correction statistics of the own car.
    pub fn corrections(&self) -> &crate::reconcile::CorrectionStats {
        self.own.reconciler().stats()
    }

    /// The link, while there is one.
    pub fn link(&self) -> Option<&L> {
        self.link.as_ref()
    }
}
