//! `NetSession`: the game's multiplayer state, one per program (integration.md section 2). A `NonSend` Bevy resource:
//! the client it holds shares the player's car with the world scene through `Rc`s.
//!
//! | Mode | What runs |
//! |---|---|
//! | `Idle` | Nothing; a LAN scan may run |
//! | `Host` | The listen server on its own thread ([`ListenServer`]); this program's player joins it as a client over the in-memory link |
//! | `Join` | A client of a server at an address (`ClientEndpoint::connect`) |
//! | `Dedicated` | The server loop on the main thread with no window (`dedicated.rs`); no Bevy app, so only [`NetSession::run_dedicated`] sees it |

use std::cell::RefCell;
use std::net::SocketAddr;
use std::rc::Rc;
use std::time::Duration;

use anyhow::{Context, Result};
use blackbox_net_core::endpoint::{DiscoveryScan, Found};
use blackbox_net_core::secure::IdentityKey;
use game_install::GameDir;

use super::client::{ClientHandle, NetClient, PlayerSetup, connect};
use super::discover::{Servers, scan_config};
use super::listen::ListenServer;
use super::options::ServeOptions;

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
    mode: Mode,
    dir: Option<GameDir>,
    player: PlayerSetup,
    /// The player's identity for this run: one key, so a reconnect resumes the seat.
    identity: Option<IdentityKey>,
    listen: Option<ListenServer>,
    client: Option<ClientHandle>,
    scan: Option<DiscoveryScan>,
    servers: Servers,
    /// The last thing that went wrong, for the front end.
    error: Option<String>,
    /// Ready on a seat by itself (the command line's `--host` and `--connect`).
    auto_ready: bool,
}

impl NetSession {
    /// An idle session for the install at `dir` (none: hosting and joining refuse).
    pub fn new(dir: Option<GameDir>, player: PlayerSetup) -> Self {
        Self {
            mode: Mode::Idle,
            dir,
            player,
            identity: None,
            listen: None,
            client: None,
            scan: None,
            servers: Servers::default(),
            error: None,
            auto_ready: false,
        }
    }

    /// Say ready by itself once seated.
    pub fn set_auto_ready(&mut self, on: bool) {
        self.auto_ready = on;
    }

    pub fn mode(&self) -> Mode {
        self.mode
    }

    pub fn player(&self) -> &PlayerSetup {
        &self.player
    }

    pub fn set_player(&mut self, player: PlayerSetup) {
        self.player = player;
    }

    /// The client, once there is one (`Host` after the server is up, `Join` at once).
    pub fn client(&self) -> Option<&ClientHandle> {
        self.client.as_ref()
    }

    /// The listen server's game port, once it is up.
    pub fn listen_addr(&self) -> Option<SocketAddr> {
        self.listen.as_ref().and_then(ListenServer::addr)
    }

    /// The servers the last scan found.
    pub fn servers(&self) -> &[Found] {
        self.servers.list()
    }

    /// Whether a LAN scan runs.
    pub fn scanning(&self) -> bool {
        self.scan.is_some()
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    fn identity(&mut self) -> Result<IdentityKey> {
        if let Some(identity) = &self.identity {
            return Ok(identity.clone());
        }
        let identity = IdentityKey::generate().map_err(|err| anyhow::anyhow!("no identity key: {err}"))?;
        self.identity = Some(identity.clone());
        Ok(identity)
    }

    fn cars(&self) -> Vec<String> {
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

    /// Joins the server at `addr`.
    pub fn join(&mut self, addr: SocketAddr) -> Result<()> {
        self.leave();
        let identity = self.identity()?;
        let (endpoint, connector) = connect(addr, identity).with_context(|| format!("cannot reach {addr}"))?;
        let config = self.player.session_config(&self.cars());
        let client = NetClient::new(endpoint, config, self.auto_ready).with_connector(connector);
        self.client = Some(Rc::new(RefCell::new(client)));
        self.mode = Mode::Join;
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
    }

    /// Starts a LAN scan for servers on `port` for `duration`; the list fills as answers come.
    pub fn start_scan(&mut self, port: u16, duration: Duration) -> Result<()> {
        self.servers.clear();
        self.scan = Some(DiscoveryScan::start(scan_config(port, duration)).context("cannot scan the LAN")?);
        Ok(())
    }

    /// Once a frame: the listen server's start, the scan's answers, and the client while no world drives it.
    pub fn update(&mut self) {
        self.poll_listen();
        self.poll_scan();
        let Some(client) = self.client.as_ref() else { return };
        let Ok(mut client) = client.try_borrow_mut() else { return };
        if !client.is_attached() {
            client.pump();
        }
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
            self.servers.note(found);
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

impl Drop for NetSession {
    fn drop(&mut self) {
        self.leave();
    }
}
