//! `NetSession`: the game's multiplayer state, one per program (integration.md section 2). It lives in the front end's
//! `LanBackend` resource as its `NetFrontend` (`frontend.rs`); the client it holds shares the player's car with the
//! world scene through `Rc`s, so the resource is `NonSend`.
//!
//! | Mode | What runs |
//! |---|---|
//! | `Idle` | Nothing; a LAN scan may run |
//! | `Host` | The listen server on its own thread ([`ListenServer`]); this program's player joins it as a client over the in-memory link |
//! | `Join` | A client of a server at an address (`ClientEndpoint::connect`) |
//! | `Dedicated` | The server loop on the main thread with no window (`dedicated.rs`); no Bevy app, so only [`NetSession::run_dedicated`] sees it |

use std::cell::RefCell;
use std::net::{IpAddr, SocketAddr};
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context, Result};
use blackbox_net_core::endpoint::DiscoveryScan;
use blackbox_net_core::secure::IdentityKey;
use game_install::GameDir;
use nfsmw_net::client_session::{LobbyView, SessionEvent};
use nfsmw_net::message::RaceResults;
use nfsmw_net::version::PROTO_MAJOR;

use super::client::{ClientHandle, NetClient, PlayerSetup, connect};
use super::discover::scan_config;
use super::listen::ListenServer;
use super::options::ServeOptions;
use crate::discord::LanGame;
use crate::frontend::lan::backend::ServerEntry;

/// What the session does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Idle,
    Host,
    Join,
    Dedicated,
}

/// The game's multiplayer state.
pub struct NetSession {
    pub(super) mode: Mode,
    dir: Option<GameDir>,
    pub(super) player: PlayerSetup,
    /// The player's identity: one key, so a reconnect resumes the seat; from `identity_file` when set.
    identity: Option<IdentityKey>,
    identity_file: Option<std::path::PathBuf>,
    listen: Option<ListenServer>,
    pub(super) client: Option<ClientHandle>,
    pub(super) scan: Option<DiscoveryScan>,
    /// The servers the last scan found, as the browser lists them.
    pub(super) servers: Vec<ServerEntry>,
    /// What hosting or connecting last failed with, before a session existed.
    pub(super) error: Option<String>,
    /// Ready on a seat by itself (the command line's `--host` and `--connect`).
    auto_ready: bool,
    /// The lobby and the last result record as of this frame, for the screens (`frontend.rs`).
    pub(super) lobby: Option<LobbyView>,
    pub(super) results: Option<RaceResults>,
    /// The server a `Join` session was started at.
    joined: Option<SocketAddr>,
}

impl NetSession {
    /// An idle session for the install at `dir` (none: hosting and joining refuse).
    pub fn new(dir: Option<GameDir>, player: PlayerSetup) -> Self {
        Self {
            mode: Mode::Idle,
            dir,
            player,
            identity: None,
            identity_file: None,
            listen: None,
            client: None,
            scan: None,
            servers: Vec::new(),
            error: None,
            auto_ready: false,
            lobby: None,
            results: None,
            joined: None,
        }
    }

    /// Read (or make) the player's identity key in `path` (`net_identity_file`) instead of a new key every run.
    pub fn set_identity_file(&mut self, path: Option<std::path::PathBuf>) {
        self.identity_file = path;
    }

    /// Say ready by itself once seated.
    pub fn set_auto_ready(&mut self, on: bool) {
        self.auto_ready = on;
    }

    /// The client, once there is one (`Host` after the server is up, `Join` at once).
    pub fn client(&self) -> Option<&ClientHandle> {
        self.client.as_ref()
    }

    fn identity(&mut self) -> Result<IdentityKey> {
        if let Some(identity) = &self.identity {
            return Ok(identity.clone());
        }
        let stored = self.identity_file.as_deref().map(super::identity::load_or_create).transpose();
        let identity = match stored {
            Ok(Some(identity)) => identity,
            Ok(None) => IdentityKey::generate().map_err(|err| anyhow::anyhow!("no identity key: {err}"))?,
            Err(error) => {
                log::warn!("net: {error:#}; this run uses a new key");
                IdentityKey::generate().map_err(|err| anyhow::anyhow!("no identity key: {err}"))?
            }
        };
        self.identity = Some(identity.clone());
        Ok(identity)
    }

    pub(super) fn cars(&self) -> Vec<String> {
        self.dir.as_ref().map(nfsmw_data::car::list).unwrap_or_default()
    }

    /// Hosts a listen server with `options`; the player joins it once its world is open.
    pub fn host(&mut self, mut options: ServeOptions) -> Result<()> {
        self.leave();
        let dir = self.dir.clone().context("no install to host from")?;
        if options.car.is_empty() {
            options.car = self.player.car.clone();
        }
        let key = self.identity()?.key_id();
        self.listen = Some(ListenServer::start(dir, options, key)?);
        self.mode = Mode::Host;
        self.error = None;
        Ok(())
    }

    /// Joins the server at `addr`, with the room password if there is one.
    pub fn join(&mut self, addr: SocketAddr, password: Option<&str>) -> Result<()> {
        self.leave();
        let identity = self.identity()?;
        let (endpoint, connector) = connect(addr, identity).with_context(|| format!("cannot reach {addr}"))?;
        let mut config = self.player.session_config(&self.cars());
        config.password = password.map(str::to_owned);
        let client = NetClient::new(endpoint, config, self.auto_ready).with_connector(connector);
        self.client = Some(Rc::new(RefCell::new(client)));
        self.mode = Mode::Join;
        self.joined = Some(addr);
        self.error = None;
        log::info!("net: joining {addr}");
        Ok(())
    }

    /// Leaves the session: the client says goodbye, a listen server stops.
    pub fn leave(&mut self) {
        if let Some(client) = self.client.take() {
            client.borrow_mut().session_mut().leave();
        }
        if let Some(mut listen) = self.listen.take() {
            listen.stop();
        }
        self.mode = Mode::Idle;
        self.joined = None;
        self.lobby = None;
    }

    /// The LAN game this player is seated in, as the Discord party shows it. A game this PC hosts is reached at
    /// `advertise` (when known) and the server's port; a game joined is at the address it was joined at, and is not
    /// offered on.
    pub fn lan_game(&self, advertise: Option<IpAddr>) -> Option<LanGame> {
        let lobby = self.lobby.as_ref()?;
        lobby.my_slot?;
        let settings = lobby.settings.as_ref()?;
        let (host, hosting) = match self.mode {
            Mode::Host => (SocketAddr::new(advertise?, self.listen.as_ref()?.addr()?.port()), true),
            Mode::Join => (self.joined?, false),
            Mode::Idle | Mode::Dedicated => return None,
        };
        let players = u8::try_from(lobby.players.len()).unwrap_or(u8::MAX);
        let open = hosting && !settings.password_required && players < settings.max_players;
        Some(LanGame { host, players, max_players: settings.max_players, open })
    }

    /// A friend's Discord invite to the server at `addr`: join it, unless this player hosts or is seated in a game
    /// already (a click on a stale invite must not drop them out of it).
    pub fn join_from_discord(&mut self, addr: SocketAddr) {
        let seated = self.lobby.as_ref().is_some_and(|lobby| lobby.my_slot.is_some());
        if self.mode == Mode::Host || seated {
            log::info!("net: the Discord invite to {addr} was ignored: already in a game");
            return;
        }
        if let Err(error) = self.join(addr, None) {
            log::warn!("net: the Discord invite to {addr} failed: {error:#}");
            self.error = Some(format!("{error:#}"));
        }
    }

    /// Starts a LAN scan for servers on `port` for `duration`; the list fills as answers come.
    pub fn start_scan(&mut self, port: u16, duration: Duration) -> Result<()> {
        self.servers.clear();
        self.scan = Some(DiscoveryScan::start(scan_config(port, duration)).context("cannot scan the LAN")?);
        Ok(())
    }

    /// Once a frame: the listen server's start, the scan's answers, the client while no world drives it, and the
    /// screens' copies of the lobby and the results.
    pub fn update(&mut self) {
        self.poll_listen();
        self.poll_scan();
        self.refresh();
        let Some(client) = self.client.as_ref() else { return };
        let Ok(mut client) = client.try_borrow_mut() else { return };
        if !client.is_attached() {
            client.pump();
        }
    }

    /// The screens' copies of the lobby and the last result record.
    pub(super) fn refresh(&mut self) {
        let Some(client) = self.client.as_ref() else { return };
        let Ok(client) = client.try_borrow() else { return };
        self.lobby = Some(client.session().lobby().clone());
        if let Some(results) = client.session().last_results() {
            self.results = Some(results.clone());
        }
    }

    /// The session events the screens have not taken yet.
    pub(super) fn take_frontend_events(&mut self) -> Vec<SessionEvent> {
        let Some(client) = self.client.as_ref() else { return Vec::new() };
        let Ok(mut client) = client.try_borrow_mut() else { return Vec::new() };
        client.take_frontend_events()
    }

    fn poll_listen(&mut self) {
        let Some(listen) = self.listen.as_mut() else { return };
        if self.client.is_some() {
            return;
        }
        match listen.poll() {
            Ok(None) => {}
            Ok(Some(endpoint)) => {
                let config = self.player.session_config(&self.cars());
                let client = NetClient::new(endpoint, config, self.auto_ready);
                self.client = Some(Rc::new(RefCell::new(client)));
            }
            Err(error) => {
                self.error = Some(error);
                self.listen = None;
                self.mode = Mode::Idle;
            }
        }
    }

    fn poll_scan(&mut self) {
        let Some(scan) = self.scan.as_mut() else { return };
        while let Some(found) = scan.poll() {
            let entry = ServerEntry::from_reply(found.from, &found.reply, PROTO_MAJOR, None);
            match self.servers.iter_mut().find(|known| known.addr == entry.addr) {
                Some(known) => *known = entry,
                None => self.servers.push(entry),
            }
        }
        if scan.is_finished() {
            self.scan = None;
        }
    }

    /// Runs a dedicated server (no window) until Ctrl-C or `seconds`; the session is `Dedicated` meanwhile.
    pub fn run_dedicated(&mut self, options: &ServeOptions, seconds: Option<u64>) -> Result<()> {
        let dir = self.dir.clone().context("no install to serve")?;
        let config = super::dedicated::init_logging(options)?;
        self.mode = Mode::Dedicated;
        let result = super::dedicated::run(&dir, options, config, seconds);
        self.mode = Mode::Idle;
        result
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;

impl Drop for NetSession {
    fn drop(&mut self) {
        self.leave();
    }
}
