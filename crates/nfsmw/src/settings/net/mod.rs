//! The LAN play settings (`docs/specs/multiplayer/integration.md` section 2): the player's name in a session, the game
//! port, the snapshot rate a host serves, the interpolation delay, the network line of the overlay and the identity key
//! file, and the address a Discord join invite carries. Each is a layered setting (command line > environment > config
//! file > defaults).
//!
//! Config (top-level keys): `net_name`, `net_port`, `net_snapshot_hz`, `net_interp_delay`, `net_show_stats`,
//! `net_identity_file`, `net_public_address`. Environment: `NFSMW_NET_NAME`, `NFSMW_NET_PORT`, `NFSMW_NET_SNAPSHOT_HZ`,
//! `NFSMW_NET_INTERP_DELAY`, `NFSMW_NET_SHOW_STATS`, `NFSMW_NET_IDENTITY_FILE`, `NFSMW_NET_PUBLIC_ADDRESS`. Command line:
//! `--net-name`, `--net-port`, `--net-snapshot-hz`, `--net-interp-delay`, `--net-show-stats` / `--no-net-show-stats`,
//! `--net-identity-file`, `--net-public-address`. Console: `set net_port 9760` and the same for the others.
//!
//! - `text.rs`: [`InlineText`], the `Copy` text the name and the path are kept in.
//! - `io.rs`: the config keys, the environment and the command line.
//! - `console.rs`: `get` and `set` of the console.

mod console;
mod io;
mod text;

pub use console::{flag, get, is_key, set, syntax};
pub use io::{NetArgs, from_env, put_into, read};
pub use text::InlineText;

/// The player's name: the protocol's name is 1 to 24 characters, at most 4 bytes each.
pub type NetName = InlineText<96>;
/// The identity key file's path.
pub type IdentityPath = InlineText<240>;
/// The IP address a Discord invite carries (an IPv6 address fits).
pub type PublicAddress = InlineText<48>;

/// The name a session shows when none is set.
pub const DEFAULT_NAME: &str = "Player";
/// The game port; discovery answers on the next one (`nfsmw-server`'s default too).
pub const DEFAULT_PORT: u16 = 9750;
/// The lowest port accepted: below are the system's. The highest leaves room for the discovery port.
pub const PORT_RANGE: (u16, u16) = (1024, 65534);
/// Snapshots a host sends a player per second (`nfsmw-server`'s default).
pub const DEFAULT_SNAPSHOT_HZ: u8 = 20;
/// The rates a host accepts: one per second to one per tick.
pub const SNAPSHOT_HZ_RANGE: (u8, u8) = (1, 60);
/// The interpolation delay of remote cars, in snapshot intervals (`nfsmw_net::reconcile::Tuning`'s default).
pub const DEFAULT_INTERP_DELAY: f32 = 2.0;
/// The delays accepted, in snapshot intervals.
pub const INTERP_DELAY_RANGE: (f32, f32) = (1.0, 4.0);
/// The network line shows only at the advanced metrics level unless this is on.
pub const DEFAULT_SHOW_STATS: bool = false;
/// The identity key next to the config file when no path is set (security.md).
pub const DEFAULT_IDENTITY_FILE: &str = "identity.key";

/// The LAN settings as the game holds them.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NetSettings {
    /// The player's name in a session (`net_name`).
    pub name: NetName,
    /// The game port a host binds and a direct connect uses by default (`net_port`).
    pub port: u16,
    /// Snapshots per second a host sends each player (`net_snapshot_hz`).
    pub snapshot_hz: u8,
    /// How far in the past remote cars are drawn, in snapshot intervals (`net_interp_delay`).
    pub interp_delay: f32,
    /// The overlay's network line shows at every metrics level, and on its own with the metrics off (`net_show_stats`).
    pub show_stats: bool,
    /// The identity key file; empty for [`DEFAULT_IDENTITY_FILE`] next to the config file (`net_identity_file`).
    pub identity_file: IdentityPath,
    /// The IP address Discord invites carry for a game this PC hosts; empty for this PC's LAN address
    /// (`net_public_address`). Set it to the public (or VPN) address when friends join from outside the LAN.
    pub public_address: PublicAddress,
}

impl Default for NetSettings {
    fn default() -> Self {
        Self::from_layer(NetLayer::default())
    }
}

impl NetSettings {
    /// The values of a merged layer, with the defaults where no layer set one and out-of-range values clamped.
    pub fn from_layer(layer: NetLayer) -> Self {
        Self {
            name: layer.name.filter(|name| valid_name(name.as_str()).is_ok()).unwrap_or_else(default_name),
            port: layer.port.map_or(DEFAULT_PORT, |port| port.clamp(PORT_RANGE.0, PORT_RANGE.1)),
            snapshot_hz: layer
                .snapshot_hz
                .map_or(DEFAULT_SNAPSHOT_HZ, |hz| hz.clamp(SNAPSHOT_HZ_RANGE.0, SNAPSHOT_HZ_RANGE.1)),
            interp_delay: layer.interp_delay.map_or(DEFAULT_INTERP_DELAY, clamp_delay),
            show_stats: layer.show_stats.unwrap_or(DEFAULT_SHOW_STATS),
            identity_file: layer.identity_file.unwrap_or_default(),
            public_address: layer.public_address.unwrap_or_default(),
        }
    }

    /// The address set for Discord invites, `None` for this PC's LAN address.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub fn public_ip(&self) -> Option<std::net::IpAddr> {
        self.public_address.as_str().parse().ok()
    }

    /// The identity key file: the set path, or [`DEFAULT_IDENTITY_FILE`] in `config_dir`.
    #[cfg_attr(not(feature = "multiplayer"), allow(dead_code))]
    pub fn identity_path(&self, config_dir: &std::path::Path) -> std::path::PathBuf {
        if self.identity_file.is_empty() {
            return config_dir.join(DEFAULT_IDENTITY_FILE);
        }
        std::path::PathBuf::from(self.identity_file.as_str())
    }
}

/// One layer of the LAN settings.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct NetLayer {
    pub name: Option<NetName>,
    pub port: Option<u16>,
    pub snapshot_hz: Option<u8>,
    pub interp_delay: Option<f32>,
    pub show_stats: Option<bool>,
    pub identity_file: Option<IdentityPath>,
    pub public_address: Option<PublicAddress>,
}

impl NetLayer {
    /// This layer, with `lower` filling in what it leaves unset.
    pub fn or(self, lower: Self) -> Self {
        Self {
            name: self.name.or(lower.name),
            port: self.port.or(lower.port),
            snapshot_hz: self.snapshot_hz.or(lower.snapshot_hz),
            interp_delay: self.interp_delay.or(lower.interp_delay),
            show_stats: self.show_stats.or(lower.show_stats),
            identity_file: self.identity_file.or(lower.identity_file),
            public_address: self.public_address.or(lower.public_address),
        }
    }
}

impl From<NetSettings> for NetLayer {
    /// Every value set, for writing the whole set to the config file.
    fn from(value: NetSettings) -> Self {
        Self {
            name: Some(value.name),
            port: Some(value.port),
            snapshot_hz: Some(value.snapshot_hz),
            interp_delay: Some(value.interp_delay),
            show_stats: Some(value.show_stats),
            identity_file: Some(value.identity_file),
            public_address: Some(value.public_address),
        }
    }
}

fn default_name() -> NetName {
    NetName::new(DEFAULT_NAME).unwrap_or_default()
}

fn clamp_delay(delay: f32) -> f32 {
    if !delay.is_finite() {
        return DEFAULT_INTERP_DELAY;
    }
    delay.clamp(INTERP_DELAY_RANGE.0, INTERP_DELAY_RANGE.1)
}

/// A name the protocol accepts: 1 to 24 characters, no control or bidirectional characters (`nfsmw_net` `Name`).
pub fn valid_name(text: &str) -> Result<NetName, String> {
    nfsmw_net::message::Name::new(text).map_err(|e| format!("not a player name: {e:?}"))?;
    NetName::new(text)
}

/// A port a host may bind: [`PORT_RANGE`].
pub fn parse_port(text: &str) -> Result<u16, String> {
    let port: u16 = text.trim().parse().map_err(|_| format!("{text:?} is not a port"))?;
    if !(PORT_RANGE.0..=PORT_RANGE.1).contains(&port) {
        return Err(format!("the port must be {} to {}", PORT_RANGE.0, PORT_RANGE.1));
    }
    Ok(port)
}

/// An IP address for Discord invites; the empty text is the default (this PC's LAN address).
pub fn parse_public_address(text: &str) -> Result<PublicAddress, String> {
    let text = text.trim();
    if !text.is_empty() && text.parse::<std::net::IpAddr>().is_err() {
        return Err(format!("{text:?} is not an IP address"));
    }
    PublicAddress::new(text)
}

/// A snapshot rate: [`SNAPSHOT_HZ_RANGE`].
pub fn parse_snapshot_hz(text: &str) -> Result<u8, String> {
    let hz: u8 = text.trim().parse().map_err(|_| format!("{text:?} is not a rate"))?;
    if !(SNAPSHOT_HZ_RANGE.0..=SNAPSHOT_HZ_RANGE.1).contains(&hz) {
        return Err(format!("the rate must be {} to {} per second", SNAPSHOT_HZ_RANGE.0, SNAPSHOT_HZ_RANGE.1));
    }
    Ok(hz)
}

/// An interpolation delay in snapshot intervals: [`INTERP_DELAY_RANGE`].
pub fn parse_interp_delay(text: &str) -> Result<f32, String> {
    let delay: f32 = text.trim().parse().map_err(|_| format!("{text:?} is not a number"))?;
    if !(INTERP_DELAY_RANGE.0..=INTERP_DELAY_RANGE.1).contains(&delay) {
        return Err(format!(
            "the delay must be {} to {} snapshot intervals",
            INTERP_DELAY_RANGE.0, INTERP_DELAY_RANGE.1
        ));
    }
    Ok(delay)
}

#[cfg(test)]
mod tests;
