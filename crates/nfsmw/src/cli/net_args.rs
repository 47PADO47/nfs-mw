//! The LAN multiplayer switches (feature `multiplayer`, `docs/specs/multiplayer/game-wiring.md` section 2). They are
//! global: `nfsmw --host`, `nfsmw play --skip-boot --connect 192.168.1.5`.

use std::net::{SocketAddr, ToSocketAddrs};
use std::path::PathBuf;

use clap::Args;

/// The default game port (`nfsmw_server::config::DEFAULT_PORT`); discovery answers on the next one.
pub const DEFAULT_PORT: u16 = 9750;

#[derive(Args, Debug, Clone, Default, PartialEq)]
pub struct NetArgs {
    /// Host a LAN game: a server on this PC with you in it (free roam; others join with --connect).
    #[arg(long, global = true, conflicts_with_all = ["connect", "dedicated", "discover"])]
    pub host: bool,
    /// Join the LAN game at ADDR: an address or a host name, with :PORT unless it is the default 9750.
    #[arg(long, global = true, value_name = "ADDR", value_parser = parse_server_addr, conflicts_with_all = ["dedicated", "discover"])]
    pub connect: Option<SocketAddr>,
    /// Run a dedicated LAN server: no window, no sound, no GPU. Ctrl-C stops it.
    #[arg(long, global = true, conflicts_with = "discover")]
    pub dedicated: bool,
    /// List the LAN servers that answer within two seconds, then quit.
    #[arg(long, global = true)]
    pub discover: bool,
    /// The game port of --host and --dedicated, and the port --discover asks about (default 9750).
    #[arg(long, global = true, value_name = "N")]
    pub port: Option<u16>,
    /// The server's name for --host and --dedicated (1 to 32 characters).
    #[arg(long = "name", id = "server_name", global = true, value_name = "S")]
    pub server_name: Option<String>,
    /// A server config file for --host and --dedicated (docs/specs/multiplayer/server.md section 5).
    #[arg(long, global = true, value_name = "FILE")]
    pub server_config: Option<PathBuf>,
    /// Stop a --dedicated server after this many seconds.
    #[arg(long, global = true, value_name = "N")]
    pub seconds: Option<u64>,
}

impl NetArgs {
    /// Whether the command line starts a session in the game window (`--host` or `--connect`).
    pub fn starts_session(&self) -> bool {
        self.host || self.connect.is_some()
    }

    /// The port --discover asks about.
    pub fn discover_port(&self) -> u16 {
        self.port.unwrap_or(DEFAULT_PORT)
    }
}

/// A server address: `HOST:PORT`, or `HOST` alone for the default port. Host names are resolved (the first IPv4
/// address wins, as the LAN is IPv4).
pub(super) fn parse_server_addr(text: &str) -> Result<SocketAddr, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err("expected an address".into());
    }
    if let Ok(addr) = text.parse::<SocketAddr>() {
        return Ok(addr);
    }
    if let Ok(ip) = text.parse::<std::net::IpAddr>() {
        return Ok(SocketAddr::new(ip, DEFAULT_PORT));
    }
    let with_port = match text.rsplit_once(':').is_some_and(|(_, port)| port.parse::<u16>().is_ok()) {
        true => text.to_owned(),
        false => format!("{text}:{DEFAULT_PORT}"),
    };
    let addrs: Vec<SocketAddr> =
        with_port.to_socket_addrs().map_err(|err| format!("cannot resolve {text:?}: {err}"))?.collect();
    addrs
        .iter()
        .find(|addr| addr.is_ipv4())
        .or_else(|| addrs.first())
        .copied()
        .ok_or_else(|| format!("{text:?} has no address"))
}
