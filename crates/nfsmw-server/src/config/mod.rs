//! The server's config file (MP-25, MP-84): one flat TOML table, `key = value`, read by [`ServerConfig::load`].
//!
//! Two kinds of keys share the table:
//!
//! - **Session keys** are the stable keys of [`nfsmw_net::settings::Field`] (`name`, `mode`, `track`, `laps`,
//!   `traffic_density`, `cop_level_cap`, `max_players`, `max_spectators`, `public`, `join_in_progress`, `collision`,
//!   `pursuits`, `catch_up`, `nitrous`, `damage`, `allowed_cars`, `password_required`, `sim_build_match`). Their values
//!   are parsed by [`nfsmw_net::settings::set`] over the dedicated defaults, then the whole set is checked by
//!   [`nfsmw_net::settings::validate`].
//! - **Server keys** belong to this crate (`keys.rs`): the address and port, discovery, the snapshot rate and budget,
//!   the relevancy radius, the lobby timers, the replay folder, the log file, the log level and the status period.
//!
//! An unknown key, a wrong type, a value out of range or a session set that does not hold together is a
//! [`ConfigError`] that names the key. A non-empty `password` turns `password_required` on; the server checks it with
//! the PAKE exchange of `blackbox-net-core` (`server/password.rs`), and `password_required = true` without a
//! password is refused.

mod error;
mod keys;

#[cfg(test)]
mod tests;

pub use error::ConfigError;

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};

use log::LevelFilter;
use nfsmw_net::message::{ServerName, Settings};
use nfsmw_net::settings::{SettingsError, dedicated_defaults, validate};

/// The default game port. The discovery port is the next one.
pub const DEFAULT_PORT: u16 = 9750;
/// The default snapshot rate, per second.
pub const DEFAULT_SNAPSHOT_HZ: u8 = 20;
/// The hard cap on snapshot bytes per second to one client (features.md section 4).
pub const MAX_SNAPSHOT_BUDGET: u32 = 32 * 1024;
/// The default snapshot budget per client, bytes per second: the typical LAN figure of features.md section 4.
pub const DEFAULT_SNAPSHOT_BUDGET: u32 = 16 * 1024;
/// The default relevancy radius, metres.
pub const DEFAULT_RELEVANCY_RADIUS: f32 = 600.0;

/// Everything a server reads from its config file. [`ServerConfig::default`] is a valid dedicated server.
#[derive(Debug, Clone, PartialEq)]
pub struct ServerConfig {
    /// The session settings (name, mode, track, limits, rules).
    pub settings: Settings,
    /// Where the identity key is kept (made the first time), or `None` for a new key every start. A server that keeps
    /// its key stays the same server to the clients that pinned it.
    pub identity_file: Option<PathBuf>,
    /// The room password, or `None` for an open room. Never logged, never sent: the PAKE proves knowledge of it.
    pub password: Option<String>,
    /// The address the game port binds to.
    pub bind: IpAddr,
    /// The game port; 0 lets the system pick one.
    pub port: u16,
    /// Answer LAN discovery on `port + 1`.
    pub discovery: bool,
    /// Snapshots per second to a player, 1 to 60. Spectators get at most 10.
    pub snapshot_hz: u8,
    /// Snapshot bytes per second to one client, at most [`MAX_SNAPSHOT_BUDGET`].
    pub snapshot_budget: u32,
    /// Entities farther than this from a client's car are left out of its snapshots (player cars never are).
    pub relevancy_radius: f32,
    /// Start the countdown by itself once every racer is ready.
    pub auto_start: bool,
    /// Seconds of the start countdown.
    pub countdown_seconds: u8,
    /// Seconds the results stay up before the lobby comes back.
    pub results_seconds: u16,
    /// Where replays are written, or `None` for no recording.
    pub replay_dir: Option<PathBuf>,
    /// The log file, or `None` for stderr only.
    pub log_file: Option<PathBuf>,
    /// The most detailed log level written.
    pub log_level: LevelFilter,
    /// Seconds between status lines in the log; 0 for none.
    pub status_seconds: u32,
}

impl Default for ServerConfig {
    fn default() -> Self {
        let name = ServerName::new("nfsmw-server").unwrap_or_default();
        Self {
            settings: dedicated_defaults(name),
            identity_file: None,
            password: None,
            bind: IpAddr::V4(Ipv4Addr::UNSPECIFIED),
            port: DEFAULT_PORT,
            discovery: true,
            snapshot_hz: DEFAULT_SNAPSHOT_HZ,
            snapshot_budget: DEFAULT_SNAPSHOT_BUDGET,
            relevancy_radius: DEFAULT_RELEVANCY_RADIUS,
            auto_start: true,
            countdown_seconds: 5,
            results_seconds: 10,
            replay_dir: None,
            log_file: None,
            log_level: LevelFilter::Info,
            status_seconds: 10,
        }
    }
}

/// The key file name a config file's folder keeps the server's identity in, when the file names none.
pub const DEFAULT_IDENTITY_FILE: &str = "server.key";

impl ServerConfig {
    /// Reads and checks the config file at `path`. A file that does not name an `identity_file` keeps the server's
    /// key in [`DEFAULT_IDENTITY_FILE`] beside it, so the clients that pinned the server know it again.
    pub fn load(path: &Path) -> Result<Self, ConfigError> {
        let text =
            std::fs::read_to_string(path).map_err(|source| ConfigError::Read { path: path.to_path_buf(), source })?;
        let dir = path.parent().unwrap_or(Path::new(""));
        Self::parse(&text, Some(dir))
    }

    /// Parses and checks config text: the defaults with every key of `text` applied. Text has no folder, so a key file
    /// is kept only where `identity_file` says.
    pub fn from_toml(text: &str) -> Result<Self, ConfigError> {
        Self::parse(text, None)
    }

    fn parse(text: &str, dir: Option<&Path>) -> Result<Self, ConfigError> {
        let table: toml::Table = text.parse().map_err(|err: toml::de::Error| ConfigError::Syntax(err.to_string()))?;
        let mut config = Self::default();
        for (key, value) in &table {
            keys::apply(&mut config, key, value)?;
        }
        if let (Some(dir), false) = (dir, table.contains_key("identity_file")) {
            config.identity_file = Some(dir.join(DEFAULT_IDENTITY_FILE));
        }
        config.settings.password_required |= config.password.is_some();
        if config.settings.password_required && config.password.is_none() {
            return Err(ConfigError::Settings(SettingsError::NoPassword));
        }
        validate(&config.settings).map_err(ConfigError::Settings)?;
        Ok(config)
    }

    /// The game port's address.
    pub fn game_addr(&self) -> SocketAddr {
        SocketAddr::new(self.bind, self.port)
    }

    /// The discovery port's address (`port + 1`), when discovery is on.
    pub fn discovery_addr(&self) -> Option<SocketAddr> {
        if !self.discovery {
            return None;
        }
        let port = match self.port {
            0 => 0,
            port => port.checked_add(1)?,
        };
        Some(SocketAddr::new(self.bind, port))
    }

    /// Ticks between two snapshots to a client at `hz` snapshots per second (at least 1).
    pub fn snapshot_interval(hz: u8) -> u32 {
        let hz = u32::from(hz.max(1));
        (nfsmw_sim::TICK_HZ + hz / 2) / hz
    }
}
