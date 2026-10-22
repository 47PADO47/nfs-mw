//! The game's side of a client session: `ClientSession` over the net thread's `ClientEndpoint` and the player's real
//! car (`GameCar`), the clock it is driven with, and the answers the game gives by itself (ready on a seat when asked
//! to, `LoadComplete` on `LoadWorld`). The front end and the world scene share it through a [`ClientHandle`]: the world
//! drives it while it runs in client mode (`scenes/world/net_client/`), the `NetSession` systems pump it otherwise.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::io;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Instant;

use blackbox_net_core::endpoint::{ClientEndpoint, ClientEndpointConfig};
use blackbox_net_core::secure::{ClientConfig, IdentityKey};
use nfsmw_net::client_session::{ClientSession, EndReason, SessionConfig, SessionEvent};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::Name;
use nfsmw_net::version::{PROTO_MAJOR, noise_prologue};

use super::build;
use crate::frontend::lan::car::car_type;
use crate::scenes::world::net_client::GameCar;

/// The session of a game client.
pub type GameSession = ClientSession<ClientEndpoint, GameCar>;

/// The client shared by the `NetSession` resource, the front end and the world scene (all on the main thread).
pub type ClientHandle = Rc<RefCell<NetClient>>;

/// Session events kept for the world scene, and for the screens, before they take them.
const SCENE_EVENTS: usize = 256;

/// Who the player is in a session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlayerSetup {
    /// The name shown in the lobby, 1 to 16 characters.
    pub name: String,
    /// The car folder the player asks for (the listen host's own car is the world's lead whatever it asks).
    pub car: String,
}

impl PlayerSetup {
    /// The session settings of this player for a server with the install's `cars`.
    pub fn session_config(&self, cars: &[String]) -> SessionConfig {
        let name = Name::new(&self.name).unwrap_or_else(|_| Name::new("Player").expect("a valid name"));
        SessionConfig::new(name, Loadout::stock(car_type(&self.car)), build::sim_build(), build::data_id(cars))
    }
}

/// A LAN client endpoint config of this protocol with `identity` (kept across reconnects, so a resume is accepted).
pub fn endpoint_config(identity: &IdentityKey) -> ClientEndpointConfig {
    let handshake = ClientConfig::lan(PROTO_MAJOR, noise_prologue(PROTO_MAJOR).as_bytes());
    ClientEndpointConfig::new(handshake, identity.clone())
}

/// What makes a new connection to the same server for a reconnect.
pub type Reconnector = Box<dyn FnMut() -> Option<ClientEndpoint> + Send>;

/// Connects to `addr` with a reconnecting connector: a dropped connection is retried with the same identity.
pub fn connect(addr: SocketAddr, identity: IdentityKey) -> io::Result<(ClientEndpoint, Reconnector)> {
    let endpoint = ClientEndpoint::connect(addr, endpoint_config(&identity))?;
    let connector = Box::new(move || ClientEndpoint::connect(addr, endpoint_config(&identity)).ok());
    Ok((endpoint, connector))
}

/// A game client: the session, its car handle and its clock.
pub struct NetClient {
    session: GameSession,
    car: GameCar,
    start: Instant,
    last_ms: Option<u64>,
    /// Say ready as soon as the seat is given (`--host`, `--connect`): the lobby starts by itself.
    auto_ready: bool,
    /// A world scene drives the session (its client mode).
    attached: bool,
    scene_events: Vec<SessionEvent>,
    /// A copy of every event for the front end's screens, which take them once a frame (`NetFrontend::take_events`).
    frontend_events: VecDeque<SessionEvent>,
    /// A clock set by the caller instead of the wall clock (tests that run a server on a fake clock).
    manual_ms: Option<u64>,
}

impl NetClient {
    /// A client over `link` with `config`; `auto_ready` answers the seat with ready.
    pub fn new(link: ClientEndpoint, config: SessionConfig, auto_ready: bool) -> Self {
        let car = GameCar::new();
        let session = ClientSession::new(link, config, car.clone());
        Self {
            session,
            car,
            start: Instant::now(),
            last_ms: None,
            auto_ready,
            attached: false,
            scene_events: Vec::new(),
            frontend_events: VecDeque::new(),
            manual_ms: None,
        }
    }

    /// Reconnects through `connector` after a silent drop.
    pub fn with_connector(mut self, connector: Reconnector) -> Self {
        self.session = self.session.with_connector(connector);
        self
    }

    /// Milliseconds since the client started: the session's clock.
    pub fn now_ms(&self) -> u64 {
        if let Some(ms) = self.manual_ms {
            return ms;
        }
        u64::try_from(self.start.elapsed().as_millis()).unwrap_or(u64::MAX)
    }

    /// Runs the session on the caller's clock from now on (tests with a server on a fake clock).
    #[cfg(test)]
    pub fn set_now_ms(&mut self, ms: u64) {
        self.manual_ms = Some(ms);
    }

    /// One session update at the current time, then its events. The world scene calls it inside its lend; the
    /// `NetSession` systems call it while no world drives the client (a step then does nothing to the car).
    pub fn pump(&mut self) {
        let now = self.now_ms();
        let dt = self.last_ms.map_or(0, |last| now.saturating_sub(last));
        self.last_ms = Some(now);
        self.session.update(now, dt as f32);
        for event in self.session.take_events() {
            self.on_event(event);
        }
    }

    fn on_event(&mut self, event: SessionEvent) {
        match &event {
            SessionEvent::Admitted { slot, resumed, .. } => {
                self.notice(format!("seated in slot {slot}{}", if *resumed { " (resumed)" } else { "" }));
                if self.auto_ready {
                    self.session.set_ready(true);
                }
            }
            // The world runs already (free roam): its streaming is the client's own business (`StreamingStall`).
            SessionEvent::LoadWorld(_) => {
                self.session.load_complete(true);
            }
            SessionEvent::Ended(reason) => self.notice(end_line(reason)),
            SessionEvent::ConnectionLost => self.notice("connection lost: reconnecting".into()),
            _ => {}
        }
        if self.frontend_events.len() >= SCENE_EVENTS {
            self.frontend_events.pop_front();
        }
        self.frontend_events.push_back(event.clone());
        if !self.attached {
            return;
        }
        if self.scene_events.len() >= SCENE_EVENTS {
            self.scene_events.remove(0);
        }
        self.scene_events.push(event);
    }

    fn notice(&self, line: String) {
        log::info!("net: {line}");
    }

    pub fn session(&self) -> &GameSession {
        &self.session
    }

    pub fn session_mut(&mut self) -> &mut GameSession {
        &mut self.session
    }

    /// The car handle the session predicts with.
    pub fn car(&self) -> &GameCar {
        &self.car
    }

    /// Whether a world scene drives the client.
    pub fn is_attached(&self) -> bool {
        self.attached
    }

    pub fn set_attached(&mut self, attached: bool) {
        self.attached = attached;
        self.scene_events.clear();
    }

    /// The session events since the world scene last took them (gameplay, results, snaps, the end).
    pub fn take_scene_events(&mut self) -> Vec<SessionEvent> {
        std::mem::take(&mut self.scene_events)
    }

    /// The session events since the screens last took them, oldest first.
    pub fn take_frontend_events(&mut self) -> Vec<SessionEvent> {
        self.frontend_events.drain(..).collect()
    }
}

/// The line that says why a session ended.
pub fn end_line(reason: &EndReason) -> String {
    match reason {
        EndReason::Left => "left the session".into(),
        EndReason::Rejected(rejected) => format!("the server refused the seat ({:?})", rejected.reason),
        EndReason::Kicked(kick) => format!("kicked ({:?})", kick.reason),
        EndReason::Link(why) => format!("disconnected ({why:?})"),
        EndReason::ResumeExpired => "the connection did not come back in time".into(),
        EndReason::PasswordRequired => "the server asks for a password".into(),
        EndReason::PasswordRejected => "wrong password".into(),
        EndReason::SchemaMismatch => "the server runs another entity schema".into(),
        EndReason::Protocol => "the server broke the protocol".into(),
    }
}
