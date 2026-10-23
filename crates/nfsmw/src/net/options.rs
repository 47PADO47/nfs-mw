//! How a server is started: the command line's `--port`, `--name` and `--server-config` over the config file, and the
//! world it opens.

use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use nfsmw_net::message::{ServerName, Settings};
use nfsmw_net::settings::validate;
use nfsmw_server::ServerConfig;

use crate::scenes::world::net::host::HostConfig;

/// Where a session's world starts, map position in render space: a road with room to drive away from the start (the
/// end-to-end test's, `scenes/world/net_client/installed.rs`). The client's world streams here once the server's first
/// own-car block places its car.
pub const START_AT: [f32; 2] = [1945.0, 1850.0];
/// The heading the host's car faces at the start, degrees.
pub const START_HEADING: f32 = 45.0;

/// What starts a server, listen or dedicated.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ServeOptions {
    /// The game port, over the config file's (`--port`).
    pub port: Option<u16>,
    /// The server name, over the config file's (`--name`).
    pub name: Option<String>,
    /// A server config file (`--server-config`, `docs/specs/multiplayer/server.md` section 5).
    pub config: Option<PathBuf>,
    /// The host's own car, `CAR[:PRESET]`: the world's lead (`PlayerId(0)`).
    pub car: String,
    /// The session seed; `None` takes one from the clock.
    pub seed: Option<u64>,
    /// The session settings the Host screen chose, over the config file's (checked).
    pub settings: Option<Settings>,
    /// The room password the Host screen chose, over the config file's. Never logged.
    pub password: Option<String>,
}

impl ServeOptions {
    /// The server's config: the file's (or the defaults), then the command line's port and name.
    pub fn server_config(&self) -> Result<ServerConfig> {
        let mut config = match &self.config {
            Some(path) => ServerConfig::load(path).with_context(|| format!("the server config {}", path.display()))?,
            None => ServerConfig::default(),
        };
        if let Some(settings) = &self.settings {
            validate(settings).map_err(|err| anyhow::anyhow!("the session settings: {err:?}"))?;
            config.settings = settings.clone();
        }
        if let Some(password) = &self.password {
            config.password = Some(password.clone());
            config.settings.password_required = true;
        }
        if let Some(port) = self.port {
            config.port = port;
        }
        if let Some(name) = &self.name {
            config.settings.name = ServerName::new(name).map_err(|err| anyhow::anyhow!("--name {name:?}: {err:?}"))?;
        }
        Ok(config)
    }

    /// The seed of this session.
    pub fn seed(&self) -> u64 {
        self.seed.unwrap_or_else(|| {
            SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |since| since.as_nanos() as u64).max(1)
        })
    }

    /// How the server's world opens: at [`START_AT`] in the host's car, waiting for no streaming (a missing tile holds
    /// the world still for the tick; `world-host.md` section 7).
    pub fn host_config(&self, seed: u64) -> HostConfig {
        let car = match self.car.is_empty() {
            true => crate::scenes::world::DEFAULT_CAR.to_owned(),
            false => self.car.clone(),
        };
        let mut config = HostConfig::new(START_AT, car, seed);
        config.heading = START_HEADING;
        config
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_command_line_wins_over_the_defaults() {
        let options = ServeOptions { port: Some(0), name: Some("Rockport LAN".into()), ..ServeOptions::default() };
        let config = options.server_config().unwrap();
        assert_eq!(config.port, 0);
        assert_eq!(config.settings.name.as_str(), "Rockport LAN");
        assert_eq!(ServeOptions::default().server_config().unwrap(), ServerConfig::default());
    }

    #[test]
    fn a_missing_config_file_is_an_error_that_names_it() {
        let options = ServeOptions { config: Some(PathBuf::from("no/such/server.toml")), ..ServeOptions::default() };
        let error = format!("{:#}", options.server_config().unwrap_err());
        assert!(error.contains("server.toml"), "{error}");
    }

    #[test]
    fn the_host_drives_the_default_car_unless_told() {
        assert_eq!(ServeOptions::default().host_config(1).car, crate::scenes::world::DEFAULT_CAR);
        let options = ServeOptions { car: "ELISE".into(), ..ServeOptions::default() };
        let config = options.host_config(7);
        assert_eq!((config.car.as_str(), config.seed, config.at), ("ELISE", 7, START_AT));
    }
}
