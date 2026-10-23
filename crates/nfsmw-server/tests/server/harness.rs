//! A server over the toy world on a fake clock, with test clients on the in-memory link (the listen host's path:
//! no socket, no handshake) or on the lossy simulator with the real handshake. The net threads run in real time, so
//! every tick sleeps a moment to let the datagrams through; the server's clock is the fake one.

use std::collections::BTreeMap;
use std::net::SocketAddr;
use std::thread;
use std::time::Duration;

use blackbox_net_core::endpoint::{ClientEndpoint, ClientEndpointConfig, EndpointConfig, ServerEndpoint};
use blackbox_net_core::link::{LossConfig, LossyLink, MemoryNet};
use blackbox_net_core::ratelimit::PreAuthLimits;
use blackbox_net_core::secure::ClientConfig;
use nfsmw_net::input::InputCode;
use nfsmw_net::message::Message;
use nfsmw_net::version::{PROTO_MAJOR, noise_prologue};
use nfsmw_server::{Server, ServerConfig, ServerSetup, WorldControl, endpoint_config};
use nfsmw_sim::toy::{ToyConfig, ToyWorld};
use nfsmw_sim::{OwnState, PlayerId};

use super::client::{TestClient, identity};

/// The server's address on the in-memory network of the lossy tests.
const SERVER_ADDR: SocketAddr = SocketAddr::new(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 0, 0, 1)), 7000);
/// Ticks ahead of the server the clients send their input: the net threads take a moment to carry it.
const INPUT_LEAD: u32 = 5;
/// Real time each tick waits for the net threads. One millisecond was not always enough for a datagram to cross two
/// threads on a busy machine, so a ping went unanswered inside its window.
const TICK_SLEEP: Duration = Duration::from_millis(3);

/// A config for tests: quiet status line, a 1 s countdown, short results.
pub fn test_config() -> ServerConfig {
    let mut config = ServerConfig::from_toml("countdown_seconds = 1\nresults_seconds = 2\nstatus_seconds = 0\n")
        .expect("a valid test config");
    config.discovery = false;
    config
}

/// The server, its clients and the fake clock.
pub struct Harness<W: WorldControl = ToyWorld> {
    pub server: Server<W>,
    pub clients: Vec<TestClient>,
    pub tick: u64,
    /// The world's own state of each slot after each tick, to check own blocks against.
    pub own: BTreeMap<(u8, u32), OwnState>,
    net: Option<MemoryNet>,
}

impl Harness {
    /// A server for `config` over a toy world of `toy`, its endpoint on an in-memory link.
    pub fn new(config: ServerConfig, toy: ToyConfig) -> Self {
        let net = MemoryNet::new();
        let endpoint = ServerEndpoint::spawn(net.endpoint(SERVER_ADDR), None, server_endpoint_config())
            .expect("the server endpoint starts");
        Self::with_endpoint(config, toy, endpoint, None)
    }

    /// A server whose endpoint sits on the lossy simulator with `loss`, for clients that run the real handshake.
    pub fn lossy(config: ServerConfig, toy: ToyConfig, loss: f64) -> Self {
        let net = MemoryNet::new();
        let link = LossyLink::new(net.endpoint(SERVER_ADDR), loss_config(loss, 1));
        let endpoint = ServerEndpoint::spawn(link, None, server_endpoint_config()).expect("the server endpoint starts");
        Self::with_endpoint(config, toy, endpoint, Some(net))
    }

    fn with_endpoint(config: ServerConfig, toy: ToyConfig, endpoint: ServerEndpoint, net: Option<MemoryNet>) -> Self {
        let world = ToyWorld::new(7, toy);
        Harness::around(config, world, endpoint, net)
    }
}

impl<W: WorldControl> Harness<W> {
    /// A server for `config` over any `world` of the toy world's car types, its endpoint on an in-memory link.
    pub fn over(config: ServerConfig, world: W) -> Self {
        let net = MemoryNet::new();
        let endpoint = ServerEndpoint::spawn(net.endpoint(SERVER_ADDR), None, server_endpoint_config())
            .expect("the server endpoint starts");
        Self::around(config, world, endpoint, None)
    }

    fn around(config: ServerConfig, world: W, endpoint: ServerEndpoint, net: Option<MemoryNet>) -> Self {
        let setup = ServerSetup::toy(64);
        let server = Server::new(config, setup, endpoint, world, 0).expect("valid settings");
        Self { server, clients: Vec::new(), tick: 0, own: BTreeMap::new(), net }
    }

    /// Adds a client on the in-memory link with identity `seed`; returns its index.
    pub fn connect(&mut self, seed: u8) -> usize {
        let key = identity(seed).key_id();
        let endpoint = self.server.connect_local(key, quiet()).expect("the local client attaches");
        self.clients.push(TestClient::new(endpoint));
        self.clients.len() - 1
    }

    /// Adds a client on the lossy simulator that runs the real handshake; returns its index.
    pub fn connect_lossy(&mut self, seed: u8, loss: f64) -> usize {
        let net = self.net.as_ref().expect("a lossy harness");
        let addr = SocketAddr::from(([10, 0, 1, seed], 7001));
        let link = LossyLink::new(net.endpoint(addr), loss_config(loss, 100 + u64::from(seed)));
        let mut handshake = ClientConfig::lan(PROTO_MAJOR, noise_prologue(PROTO_MAJOR).as_bytes());
        handshake.retry_ms = vec![250; 16];
        let config =
            ClientEndpointConfig { endpoint: quiet(), handshake, identity: identity(seed), latest_frame_type: None };
        let endpoint = ClientEndpoint::spawn(link, SERVER_ADDR, config).expect("the client starts");
        self.clients.push(TestClient::new(endpoint));
        self.clients.len() - 1
    }

    /// The fake clock now.
    pub fn now_ms(&self) -> u64 {
        self.tick * 1000 / 60
    }

    /// One server tick: every client that is not silent drives with `code` [`INPUT_LEAD`] ticks ahead and pings twice a
    /// second; the server steps; the world's own states are recorded; the clients read their events.
    pub fn step(&mut self, code: InputCode) {
        self.step_by(|_, _| code);
    }

    /// One server tick as [`Harness::step`], with each client's input from `pilot(index, client)`.
    pub fn step_by(&mut self, mut pilot: impl FnMut(usize, &TestClient) -> InputCode) {
        let next = self.server.world().tick();
        let now_ms = self.now_ms();
        for (index, client) in self.clients.iter_mut().enumerate() {
            if client.slot().is_none() {
                continue;
            }
            let code = pilot(index, client);
            client.drive(next + INPUT_LEAD, code);
            if (self.tick + index as u64) % 30 == 0 {
                client.ping(self.tick as u8, now_ms as u32);
            }
        }
        self.server.tick_once(self.now_ms());
        let done = self.server.world().tick().saturating_sub(1);
        for client in &self.clients {
            let Some(slot) = client.slot() else {
                continue;
            };
            if let Some(state) = self.server.world().own_state(PlayerId(slot)) {
                self.own.insert((slot, done), state);
            }
        }
        self.tick += 1;
        thread::sleep(TICK_SLEEP);
        for client in &mut self.clients {
            client.pump();
        }
    }

    /// Steps until `done` holds, at most `max_ticks`; panics with `what` otherwise.
    pub fn until(&mut self, what: &str, max_ticks: u32, code: InputCode, mut done: impl FnMut(&Self) -> bool) {
        for _ in 0..max_ticks {
            if done(self) {
                return;
            }
            self.step(code);
        }
        assert!(done(self), "timed out waiting for {what}");
    }

    /// Steps `ticks` times.
    pub fn run(&mut self, ticks: u32, code: InputCode) {
        for _ in 0..ticks {
            self.step(code);
        }
    }

    /// Joins clients `indices` (car type = index) and waits until each is seated.
    pub fn join_all(&mut self, indices: &[usize]) {
        for &index in indices {
            let name = format!("player{index}");
            self.clients[index].join(&name, index as u32 % 8, None, 0);
        }
        self.until("every join", 600, InputCode::NEUTRAL, |h| {
            indices.iter().all(|&index| h.clients[index].accepted.is_some())
        });
    }

    /// Readies clients `indices` and runs until the race is running (`PhaseChange` to running for each).
    pub fn start_race(&mut self, indices: &[usize]) {
        for &index in indices {
            self.clients[index].ready(true);
        }
        self.until("the race start", 900, InputCode::NEUTRAL, |h| {
            indices.iter().all(|&index| h.clients[index].got(is_running))
        });
    }
}

/// A `PhaseChange` to running.
pub fn is_running(message: &Message) -> bool {
    matches!(message, Message::PhaseChange(change) if change.phase == nfsmw_net::message::SessionPhase::Running)
}

/// Full throttle, wheel straight.
pub fn throttle() -> InputCode {
    InputCode { throttle: nfsmw_net::input::PEDAL_MAX, ..InputCode::NEUTRAL }
}

/// Endpoint timers without stats events.
fn quiet() -> EndpointConfig {
    EndpointConfig { stats_ms: 0, ..EndpointConfig::default() }
}

/// The server endpoint of the tests: this protocol's handshake with roomy pre-authentication limits.
fn server_endpoint_config() -> blackbox_net_core::endpoint::ServerEndpointConfig {
    let mut config = endpoint_config(identity(1));
    config.handshake.limits = PreAuthLimits {
        conns_per_ip: 64,
        handshakes_per_ip: 64,
        half_open_soft: 64,
        new_handshakes_per_s: 64,
        max_connections: 64,
        ..PreAuthLimits::LAN
    };
    config.endpoint = quiet();
    config
}

fn loss_config(loss: f64, seed: u64) -> LossConfig {
    LossConfig { loss, latency_ms: 2, jitter_ms: 2, duplicate: 0.0, reorder: 0.01, seed }
}
