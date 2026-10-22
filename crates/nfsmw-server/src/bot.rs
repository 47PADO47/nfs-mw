//! Scripted bots: clients that join a server through the same [`ClientSession`] a player's game uses, predict a toy
//! car locally and drive it with a [`WanderScript`], so tests, soak runs and `--bots N` can fill a server.
//!
//! - [`Bot`]: one session and its script. [`Bot::step`] runs one frame: update, ready up in the lobby, answer
//!   `LoadWorld`, and drive every due tick. Over a `ClientEndpoint` ([`Bot::connect`] by address, [`Bot::spawn`] over
//!   any transport such as the lossy simulator) or any other [`ClientLink`] ([`Bot::over`]).
//! - [`BotFleet`]: many bots stepped together on the caller's thread with the caller's (fake or real) clock.
//! - [`ThreadedFleet`]: one thread per bot on the real clock, stopped and joined with a [`BotReport`] each.
//!
//! The bot's car mirrors `ToyWorld`'s player car of the same type (power `1 + (type % 4) × 0.05`, top speed 60 m/s,
//! bounds 500 m), so its prediction matches the server's step; the first own block places it.

use std::io;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use blackbox_net_core::endpoint::{ClientEndpoint, ClientEndpointConfig, Transport};
use blackbox_net_core::secure::{ClientConfig, IdentityKey};
use nfsmw_net::client_session::{ClientLink, ClientSession, EndReason, SessionConfig, SessionEvent};
use nfsmw_net::loadout::Loadout;
use nfsmw_net::message::Name;
use nfsmw_net::predict::ToyCar;
use nfsmw_net::version::{DataId, PROTO_MAJOR, SimBuild, noise_prologue};
use nfsmw_sim::PlayerId;
use nfsmw_sim::runner::WanderScript;
use nfsmw_sim::toy::Car;

/// The plane of the default toy world, metres.
pub const BOT_BOUNDS: f32 = 500.0;
/// The toy world's player top speed, metres per second.
const TOP_SPEED: f32 = 60.0;
/// The frame period of a threaded bot, milliseconds.
const FRAME_MS: u64 = 16;

/// What a bot needs beyond its link.
#[derive(Debug, Clone, PartialEq)]
pub struct BotConfig {
    /// The bot's number: its name is `bot<index>`.
    pub index: u32,
    /// The script's seed.
    pub seed: u64,
    /// The car type (`ToyWorld` knows `0..64`).
    pub car_type: u32,
    /// The data id and sim build the server expects.
    pub data_id: DataId,
    pub sim_build: SimBuild,
}

impl BotConfig {
    /// Bot `index` with a seed derived from `seed`, car type `index % 4`, and the test build ids.
    pub fn new(index: u32, seed: u64) -> Self {
        Self {
            index,
            seed: seed ^ u64::from(index).wrapping_mul(0x9E37_79B9_7F4A_7C15),
            car_type: index % 4,
            data_id: DataId { bytes: [0; 16] },
            sim_build: SimBuild::from_parts("bot", "toy"),
        }
    }

    /// The session settings of this bot.
    pub fn session(&self) -> SessionConfig {
        let name = Name::new(&format!("bot{}", self.index)).unwrap_or_else(|_| Name::new("bot").expect("a name"));
        SessionConfig::new(name, Loadout::stock(self.car_type), self.sim_build, self.data_id)
    }

    /// The bot's car before the first own block places it.
    pub fn car(&self) -> ToyCar {
        let power = 1.0 + (self.car_type % 4) as f32 * 0.05;
        ToyCar::new(Car::new([0.0, 0.0], [0.0, 1.0], power, TOP_SPEED), BOT_BOUNDS)
    }
}

/// A client endpoint for a LAN server of this protocol, with a fresh identity key.
pub fn endpoint_config() -> io::Result<ClientEndpointConfig> {
    let handshake = ClientConfig::lan(PROTO_MAJOR, noise_prologue(PROTO_MAJOR).as_bytes());
    let identity = IdentityKey::generate().map_err(|err| io::Error::other(err.to_string()))?;
    Ok(ClientEndpointConfig::new(handshake, identity))
}

/// One scripted client.
pub struct Bot<L: ClientLink = ClientEndpoint> {
    session: ClientSession<L, ToyCar>,
    script: WanderScript,
    last_ms: Option<u64>,
    joined: bool,
    end: Option<EndReason>,
}

impl Bot<ClientEndpoint> {
    /// Connects to `server` over UDP. A dropped connection is retried with a new endpoint for the resume window.
    pub fn connect(server: SocketAddr, config: BotConfig) -> io::Result<Self> {
        let endpoint = ClientEndpoint::connect(server, endpoint_config()?)?;
        let connector = Box::new(move || ClientEndpoint::connect(server, endpoint_config().ok()?).ok());
        let mut bot = Self::over(endpoint, config);
        bot.session = bot.session.with_connector(connector);
        Ok(bot)
    }

    /// Connects to `server` over a transport the caller made (the lossy simulator, a memory link).
    pub fn spawn<T: Transport + 'static>(net: T, server: SocketAddr, config: BotConfig) -> io::Result<Self> {
        let endpoint = ClientEndpoint::spawn(net, server, endpoint_config()?)?;
        Ok(Self::over(endpoint, config))
    }
}

impl<L: ClientLink> Bot<L> {
    /// A bot over any link (a mock link in tests, the listen host's local client).
    pub fn over(link: L, config: BotConfig) -> Self {
        let session = ClientSession::new(link, config.session(), config.car());
        Self { session, script: WanderScript::new(config.seed, []), last_ms: None, joined: false, end: None }
    }

    /// One frame at `now_ms`: the session update, the lobby answers, and the driving.
    pub fn step(&mut self, now_ms: u64) {
        let dt = self.last_ms.map_or(0, |last| now_ms.saturating_sub(last)) as f32;
        self.last_ms = Some(now_ms);
        self.session.update(now_ms, dt);
        for event in self.session.take_events() {
            self.on_event(event);
        }
        let player = PlayerId(self.session.slot().unwrap_or(0));
        let script = &self.script;
        self.session.drive(|tick| script.input(player, tick));
    }

    fn on_event(&mut self, event: SessionEvent) {
        match event {
            SessionEvent::Admitted { .. } => {
                self.joined = true;
                self.session.set_ready(true);
            }
            SessionEvent::LoadWorld(_) => {
                self.session.load_complete(true);
            }
            SessionEvent::Ended(why) => self.end = Some(why),
            _ => {}
        }
    }

    /// The session.
    pub fn session(&self) -> &ClientSession<L, ToyCar> {
        &self.session
    }

    /// The session, to send commands.
    pub fn session_mut(&mut self) -> &mut ClientSession<L, ToyCar> {
        &mut self.session
    }

    /// Whether the bot was ever seated.
    pub fn has_joined(&self) -> bool {
        self.joined
    }

    /// Why the session ended, if it did.
    pub fn end(&self) -> Option<&EndReason> {
        self.end.as_ref()
    }

    /// Leaves the server.
    pub fn leave(&mut self) {
        self.session.leave();
    }

    /// The bot's numbers.
    pub fn report(&self) -> BotReport {
        let counters = self.session.counters();
        BotReport {
            joined: self.joined,
            seated: self.session.status().is_joined(),
            snapshots: counters.snapshots,
            inputs_sent: counters.inputs_sent,
            corrections: self.session.corrections().corrections(),
            snaps: self.session.corrections().snaps(),
            end: self.end.clone(),
        }
    }
}

/// What a bot did.
#[derive(Debug, Clone, PartialEq)]
pub struct BotReport {
    /// It was seated at some point.
    pub joined: bool,
    /// It is seated now.
    pub seated: bool,
    pub snapshots: u64,
    pub inputs_sent: u64,
    pub corrections: u64,
    pub snaps: u64,
    pub end: Option<EndReason>,
}

/// Bots stepped together on the caller's thread.
pub struct BotFleet<L: ClientLink = ClientEndpoint> {
    bots: Vec<Bot<L>>,
}

impl BotFleet<ClientEndpoint> {
    /// `count` bots connected to `server` over UDP, seeded from `seed`.
    pub fn connect(server: SocketAddr, count: u32, seed: u64) -> io::Result<Self> {
        let bots =
            (0..count).map(|index| Bot::connect(server, BotConfig::new(index, seed))).collect::<io::Result<_>>()?;
        Ok(Self { bots })
    }
}

impl<L: ClientLink> BotFleet<L> {
    /// A fleet of bots the caller made.
    pub fn from_bots(bots: Vec<Bot<L>>) -> Self {
        Self { bots }
    }

    /// One frame of every bot at `now_ms`.
    pub fn step(&mut self, now_ms: u64) {
        for bot in &mut self.bots {
            bot.step(now_ms);
        }
    }

    /// Bots seated now.
    pub fn seated(&self) -> usize {
        self.bots.iter().filter(|bot| bot.session().status().is_joined()).count()
    }

    pub fn bots(&self) -> &[Bot<L>] {
        &self.bots
    }

    pub fn bots_mut(&mut self) -> &mut [Bot<L>] {
        &mut self.bots
    }

    /// Every bot leaves.
    pub fn leave(&mut self) {
        for bot in &mut self.bots {
            bot.leave();
        }
    }

    /// Every bot's numbers.
    pub fn reports(&self) -> Vec<BotReport> {
        self.bots.iter().map(Bot::report).collect()
    }
}

/// One thread per bot on the real clock.
pub struct ThreadedFleet {
    stop: Arc<AtomicBool>,
    threads: Vec<JoinHandle<BotReport>>,
}

impl ThreadedFleet {
    /// `count` bots connecting to `server`, each on its own thread, stepping every 16 ms until [`ThreadedFleet::stop`].
    pub fn spawn(server: SocketAddr, count: u32, seed: u64) -> io::Result<Self> {
        let stop = Arc::new(AtomicBool::new(false));
        let mut threads = Vec::new();
        for index in 0..count {
            let mut bot = Bot::connect(server, BotConfig::new(index, seed))?;
            let stop = Arc::clone(&stop);
            let thread = thread::Builder::new().name(format!("bot{index}")).spawn(move || {
                let origin = Instant::now();
                while !stop.load(Ordering::Relaxed) && bot.end().is_none() {
                    bot.step(origin.elapsed().as_millis() as u64);
                    thread::sleep(Duration::from_millis(FRAME_MS));
                }
                bot.leave();
                bot.report()
            })?;
            threads.push(thread);
        }
        Ok(Self { stop, threads })
    }

    /// Stops every bot (each leaves the server) and returns their reports.
    pub fn stop(self) -> Vec<BotReport> {
        self.stop.store(true, Ordering::Relaxed);
        self.threads.into_iter().filter_map(|thread| thread.join().ok()).collect()
    }
}

#[cfg(test)]
mod tests {
    use blackbox_net_core::endpoint::ClientEvent;
    use blackbox_net_core::secure::KeyId;
    use nfsmw_net::client_session::{MockLink, Sent, mock_link};
    use nfsmw_net::message::{Message, decode};

    use super::*;

    #[test]
    fn a_bot_presents_itself_on_connect() {
        let (link, peer) = mock_link();
        let mut fleet: BotFleet<MockLink> = BotFleet::from_bots(vec![Bot::over(link, BotConfig::new(3, 1))]);
        peer.push(ClientEvent::Connected { server_key: KeyId([1; 16]) });
        fleet.step(16);
        let sent: Vec<Message> = peer
            .take_sent()
            .into_iter()
            .filter_map(|sent| match sent {
                Sent::Reliable { bytes, .. } => decode(&bytes).ok(),
                Sent::Unreliable { .. } => None,
            })
            .collect();
        assert!(matches!(&sent[..], [Message::Auth(_), Message::Join(join)] if join.display_name.as_str() == "bot3"));
        assert_eq!(fleet.seated(), 0);
    }

    /// Bots join the real server loop over loopback UDP with the real handshake, race, and drive for two seconds.
    #[test]
    fn bots_join_the_real_server() {
        use crate::{Server, ServerConfig, ServerSetup, bind};
        use nfsmw_sim::toy::{ToyConfig, ToyWorld};

        let mut config = ServerConfig::from_toml(
            "bind = \"127.0.0.1\"
port = 0
discovery = false
countdown_seconds = 1
status_seconds = 0
",
        )
        .expect("a valid config");
        config.discovery = false;
        let identity = IdentityKey::generate().expect("a key");
        let endpoint = bind(&config, identity).expect("the server binds");
        let addr = endpoint.local_addr();
        let world = ToyWorld::new(7, ToyConfig::default());
        let setup = ServerSetup::toy(world.config().car_types);
        let mut server = Server::new(config, setup, endpoint, world, 0).expect("valid settings");
        let mut fleet = BotFleet::connect(addr, 4, 7).expect("the bots start");
        let mut driving = None;
        for step in 0..6000u64 {
            let now_ms = step * 1000 / 60;
            fleet.step(now_ms);
            server.tick_once(now_ms);
            thread::sleep(Duration::from_millis(1));
            let all_driving = fleet.reports().iter().all(|report| report.inputs_sent > 0 && report.snapshots > 0);
            if driving.is_none() && all_driving {
                driving = Some(step);
            }
            if driving.is_some_and(|since| step >= since + 120) {
                break;
            }
        }
        assert!(driving.is_some(), "every bot drives: {:?}", fleet.reports());
        assert_eq!(fleet.seated(), 4);
        for report in fleet.reports() {
            assert!(report.snapshots > 0 && report.inputs_sent > 0, "{report:?}");
            assert_eq!(report.snaps, 0, "{report:?}");
        }
        fleet.leave();
    }
}
