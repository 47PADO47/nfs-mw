//! The listen server: the game's own `Server` over the real world, on a thread of its own (integration.md section 7,
//! question 1). The world is opened inside that thread because `WorldSimHost` is not `Send` (the scene holds `Rc`s);
//! only the host's own client endpoint and the stop flag cross back to the game thread.

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::thread::{self, JoinHandle};

use anyhow::{Context, Result};
use blackbox_net_core::endpoint::{ClientEndpoint, EndpointConfig};
use blackbox_net_core::secure::{IdentityKey, KeyId};
use game_install::GameDir;
use nfsmw_server::{Server, TickHook};

use super::build;
use super::options::ServeOptions;
use crate::scenes::world::net::host::WorldSimHost;

/// What the server thread reports once.
pub enum ListenEvent {
    /// The world is open and the server listens on `addr`; `client` is the host's own client on the in-memory link.
    Up { addr: SocketAddr, client: ClientEndpoint },
    /// The world or the socket could not be opened.
    Failed(String),
}

/// A running listen server. Dropping it stops the server thread and waits for it.
pub struct ListenServer {
    stop: Arc<AtomicBool>,
    events: Receiver<ListenEvent>,
    thread: Option<JoinHandle<()>>,
    addr: Option<SocketAddr>,
}

impl ListenServer {
    /// Starts the server thread: it opens the world of `dir`, binds the LAN port with discovery, and attaches the host's
    /// own client with `player_key`. The result comes later through [`ListenServer::poll`]: the world takes seconds.
    pub fn start(dir: GameDir, options: ServeOptions, player_key: KeyId) -> Result<Self> {
        let config = options.server_config()?;
        let (sender, events) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));
        let flag = Arc::clone(&stop);
        let thread = thread::Builder::new()
            .name("net-listen-server".into())
            .spawn(move || {
                let mut server = match open(&dir, &options, config, player_key) {
                    Ok((server, addr, client)) => {
                        let _ = sender.send(ListenEvent::Up { addr, client });
                        server
                    }
                    Err(error) => {
                        log::error!("net: the listen server did not start: {error:#}");
                        let _ = sender.send(ListenEvent::Failed(format!("{error:#}")));
                        return;
                    }
                };
                let mut hook = StopHook { stop: flag };
                server.run(None, Some(&mut hook));
            })
            .context("the listen server thread did not start")?;
        Ok(Self { stop, events, thread: Some(thread), addr: None })
    }

    /// The host's own client once the server is up (`Ok(Some)` once), an error if it failed, `Ok(None)` meanwhile.
    pub fn poll(&mut self) -> Result<Option<ClientEndpoint>, String> {
        match self.events.try_recv() {
            Ok(ListenEvent::Up { addr, client }) => {
                log::info!("net: listen server up on {addr}");
                self.addr = Some(addr);
                Ok(Some(client))
            }
            Ok(ListenEvent::Failed(error)) => Err(error),
            Err(TryRecvError::Empty) => Ok(None),
            Err(TryRecvError::Disconnected) => match self.addr {
                Some(_) => Err("the listen server stopped".into()),
                None => Err("the listen server thread ended".into()),
            },
        }
    }

    /// The address the server is bound to, once it is up.
    pub fn addr(&self) -> Option<SocketAddr> {
        self.addr
    }

    /// Stops the server and waits for its thread.
    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let Some(thread) = self.thread.take() else { return };
        if thread.join().is_err() {
            log::error!("net: the listen server thread panicked");
        }
    }
}

impl Drop for ListenServer {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Opens the world, binds the port and attaches the host's client: everything that can fail before the loop.
fn open(
    dir: &GameDir,
    options: &ServeOptions,
    config: nfsmw_server::ServerConfig,
    player_key: KeyId,
) -> Result<(Server<WorldSimHost>, SocketAddr, ClientEndpoint)> {
    let seed = options.seed();
    let cars = nfsmw_data::car::list(dir);
    let host = WorldSimHost::open(dir, options.host_config(seed)).context("the server's world did not open")?;
    let identity = IdentityKey::generate().map_err(|err| anyhow::anyhow!("no identity key: {err}"))?;
    let endpoint =
        nfsmw_server::bind(&config, identity).with_context(|| format!("cannot bind {}", config.game_addr()))?;
    let addr = endpoint.local_addr();
    let setup = build::server_setup(&cars, seed);
    let mut server =
        Server::new(config, setup, endpoint, host, 0).map_err(|err| anyhow::anyhow!("server settings: {err:?}"))?;
    let client = server.connect_local(player_key, EndpointConfig::default()).context("the host's own client")?;
    Ok((server, addr, client))
}

/// Stops the server loop when the game sets its flag (the server's own flag is made by the server, after the game's).
struct StopHook {
    stop: Arc<AtomicBool>,
}

impl TickHook<WorldSimHost> for StopHook {
    fn before_tick(&mut self, server: &mut Server<WorldSimHost>, _now_ms: u64) {
        if self.stop.load(Ordering::Relaxed) {
            server.stop_flag().store(true, Ordering::Relaxed);
        }
    }
}
